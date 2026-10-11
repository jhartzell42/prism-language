{-# LANGUAGE RankNTypes, ScopedTypeVariables, RecursiveDo, LambdaCase, FlexibleContexts #-}
{-# LANGUAGE TypeFamilies, ConstraintKinds, TupleSections #-}
-- | The evaluator on Reflex: expressions with lifting over `Event t` and `Dynamic t`, widget
-- blocks built in the push monad, rebuild, for, elements, state cells, and the prelude.
--
-- Review checklist (chapter 14, "What must not be used"):
--   * `current` appears in `lifted` and `EBin "on"` (the tag law), in `rebuildOver` and
--     `buildFor` (networkView's `tag (current d) now`, the first build), and under `switch`
--     for the members-as-opened terms. `sample` appears nowhere.
--   * `coincidence` appears once, in `registerBirth`: a block born in n is heard in n, for its
--     requests and for a `for`'s consolidated event. The result event of a `rebuild` is not
--     prompt: the program applies `!Switch`, which is `switchHold never`.
--   * No `switchHoldPromptly`, `switchPromptlyDyn`, `tagPromptlyDyn`, `getPostBuild`,
--     `performEvent`, `newTriggerEvent`. No `Adjustable`: `PerformEventT`'s instance builds a
--     replacement in a follow-up frame, so blocks are built inside `push` closures instead, in
--     the frame of the change that caused them.
module Prism.Eval where

import Reflex hiding (apply)
import Prism.Syntax
import Prism.Value
import Data.IORef
import qualified Data.Map as M
import Data.Char (isUpper, toUpper, toLower)
import Data.List (isPrefixOf, isSuffixOf, intercalate)
import Data.Maybe (fromMaybe, isJust, listToMaybe)
import Data.Functor.Identity
import Data.Functor.Misc (Const2(..))
import Control.Monad
import Control.Monad.IO.Class
import Control.Exception (throw)

-- | What the evaluator needs of a timeline: Reflex, and IORefs in the push monad for the
-- position and generation counters and the state cells.
type Rx t = (Reflex t, MonadIO (PushM t))

data Ctx t = Ctx
  { cTags   :: M.Map String TagDecl
  , cTypes  :: M.Map String TypeDecl
  , cOracle :: Value t -> IO (Maybe (Value t))               -- the World's snapshot, read when a state slot is made
  , cFan    :: EventSelector t (Const2 (Path t) (Value t))   -- responses by path
  , cPushes :: Event t (M.Map (Value t) (Value t))           -- this turn's pushes by element value
  , cCells  :: IORef (M.Map (Value t) (Dynamic t (Value t))) -- state cells, one per element value
  }

-- | How an expression reaches the build monad. At a statement it is the push monad itself;
-- inside a lifted closure (a plain function over the value of a dynamic or an event) there is
-- none, and a hold or a switch there is an error.
newtype Build t m = Build { runBuild :: forall a. PushM t a -> m a }

atBuild :: Build t (PushM t)
atBuild = Build id

inClosure :: Build t Identity
inClosure = Build (\_ -> prismError "Build" "a hold, switch or widget inside a lifted expression")

prismError :: String -> String -> a
prismError k d = throw (PrismError k d)

splitOn :: String -> String -> [String]
splitOn sep t = go t
  where
    go s = case breakOn s of
      (a, Nothing) -> [a]
      (a, Just rest) -> a : go rest
    breakOn s | null s = ("", Nothing)
              | sep `isPrefixOf` s = ("", Just (drop (length sep) s))
              | otherwise = let (a, r) = breakOn (tail s) in (head s : a, r)

-- ---------------------------------------------------------------------------
-- Lifting

isFunctor :: Value t -> Bool
isFunctor (VEvent _) = True
isFunctor (VDyn _) = True
isFunctor _ = False

-- | Apply a plain function to operands, lifting over any event or dynamic among them. One
-- event and any dynamics: `attachWith` over the dynamics' behaviors, which is the tag law
-- (the dynamics are read as they were before the occurrence). Dynamics only: a derived
-- dynamic, prompt and glitch-free by Reflex's merge.
lifted :: Reflex t => String -> [Value t] -> ([Value t] -> Value t) -> Value t
lifted label vs f
  | not (any isFunctor vs) = f vs
  | otherwise = case [i | (i, VEvent _) <- zip [0 :: Int ..] vs] of
      (_:_:_) -> prismError "Build" ("two events in one operation: " ++ label)
      [i] ->
        let e = case vs !! i of { VEvent x -> x; _ -> never }
            dyns = [(j, d) | (j, VDyn d) <- zip [0 ..] vs]
            befores = sequenceA [current d | (_, d) <- dyns]          -- the tag law
        in VEvent (forceE (attachWith (\bs ev -> f (fill ((i, ev) : zip (map fst dyns) bs))) befores e))
      [] ->
        let dyns = [(j, d) | (j, VDyn d) <- zip [0 ..] vs]
        in VDyn (distributeListOverDynWith (\nows -> f (fill (zip (map fst dyns) nows))) (map snd dyns))
  where
    fill m = [fromMaybe v (lookup j m) | (j, v) <- zip [0 :: Int ..] vs]

-- | Force an event's values as they are computed, so that an error in one aborts the
-- occurrence instead of surfacing when the transcript is printed.
forceE :: Reflex t => Event t (Value t) -> Event t (Value t)
forceE = push (\v -> forceValue v `seq` return (Just v))

asDyn :: Reflex t => Value t -> Dynamic t (Value t)
asDyn (VDyn d) = d
asDyn v = constDyn v

asEvent :: String -> Value t -> Event t (Value t)
asEvent _ (VEvent e) = e
asEvent what v = prismError "Build" (what ++ ": expected an event, got " ++ showValue v)

-- ---------------------------------------------------------------------------
-- Expressions

evalPure :: Rx t => Ctx t -> Env t -> Expr -> Value t
evalPure ctx env e = runIdentity (eval inClosure ctx env e)

applyPure :: Rx t => Ctx t -> Value t -> [Value t] -> Value t
applyPure ctx f args = runIdentity (apply inClosure ctx f args)

eval :: forall t m. (Rx t, Monad m) => Build t m -> Ctx t -> Env t -> Expr -> m (Value t)
eval b ctx env expr = case expr of
  ELit l -> return (litValue l)
  EVar nm -> return (lookupName env (key nm))
  EText parts -> do
    vs <- mapM partVal parts
    return (lifted "text" vs (VText . concatMap textOf))
  EApp hd args -> do
    f <- ev hd
    as <- mapM ev args
    apply b ctx f as
  ECons n fields -> do
    vs <- mapM (ev . snd) fields
    return (lifted n vs (\vs' -> construct ctx n (zip (map fst fields) vs')))
  EField e f -> do
    v <- ev e
    return (lifted ("." ++ f) [v] (\[x] -> field x f))
  EIndex e i -> do
    v <- ev e
    iv <- ev i
    return (lifted "index" [v, iv] (\[x, k] -> index x k))
  EBin "|" a c -> do
    x <- ev a
    y <- ev c
    return (VEvent (leftmost [asEvent "|" x, asEvent "|" y]))
  EBin "on" a c -> do
    x <- ev a
    y <- ev c
    let e = asEvent "on" y
    return $ case x of
      VDyn d -> VEvent (tag (current d) e)              -- the tag law: the one window onto before
      VEvent _ -> prismError "Build" "`on` with an event on the left"
      v -> VEvent (v <$ e)
  EBin "and" a c -> do
    x <- ev a
    case x of
      VBool False -> return x
      VBool True -> ev c
      _ -> do y <- ev c; return (lifted "and" [x, y] (\[p, q] -> binop "and" p q))
  EBin "or" a c -> do
    x <- ev a
    case x of
      VBool True -> return x
      VBool False -> ev c
      _ -> do y <- ev c; return (lifted "or" [x, y] (\[p, q] -> binop "or" p q))
  EBin op a c -> do
    x <- ev a
    y <- ev c
    return (lifted op [x, y] (\[p, q] -> binop op p q))
  ENot a -> do
    x <- ev a
    return (lifted "not" [x] (\[v] -> case v of { VBool p -> VBool (not p); _ -> prismError "Build" "not on a non-boolean" }))
  ENeg a -> do
    x <- ev a
    return $ lifted "neg" [x] $ \[v] -> case v of
      VInt n -> VInt (negate n)
      VFloat d -> VFloat (negate d)
      _ -> prismError "Build" "negating a non-number"
  EThen _ _ -> prismError "Build" "`then` outside a hold statement"
  EIf c t e -> do
    cv <- ev c
    case cv of
      VBool True -> ev t
      VBool False -> ev e
      _ -> return $ lifted "if" [cv] $ \[v] -> case v of
             VBool True -> evalPure ctx env t
             VBool False -> evalPure ctx env e
             _ -> prismError "Build" "if on a non-boolean"
  EMatch s arms -> do
    sv <- ev s
    if isFunctor sv
      then return (lifted "match" [sv] (\[v] -> runIdentity (matchArms inClosure ctx env v arms)))
      else matchArms b ctx env sv arms
  ELam ps body -> return (VFn (Closure ps body env))
  EBlock stmts -> return (VBlueprint (Blueprint (\sc -> buildBlock ctx env sc stmts) True))
  ERebuild d stmts -> return (VBlueprint (Blueprint (\sc -> buildRebuild ctx env sc d stmts) False))
  EFor binders m stmts -> return (VBlueprint (Blueprint (\sc -> buildFor ctx env sc binders m stmts) False))
  EReturn e -> ev e
  EFree "$" e -> VDyn . asDyn <$> ev e
  EFree _ e -> ev e
  EElement e -> return (VBlueprint (Blueprint (\sc -> buildElement ctx env sc e) False))
  EStarted -> return (VBlueprint (Blueprint (\_ -> started) True))
  EArray es -> VArray <$> mapM ev es          -- an array of events is plain (chapter 3)
  EMap kvs -> do
    ks <- mapM (ev . fst) kvs
    vs <- mapM (ev . snd) kvs
    return $ lifted "map" (ks ++ vs) $ \xs ->
      let (ks', vs') = splitAt (length ks) xs in VMap (M.fromList (zip ks' vs'))
  ETuple es -> VTuple <$> mapM ev es          -- a bundle: never lifted
  ELet nm v body -> do
    vv <- ev v
    eval b ctx (M.insert (key nm) vv env) body
  where
    ev = eval b ctx env
    partVal (TLit s) = return (VText s)
    partVal (TExpr e) = ev e

-- | `@started`: fires once, in the occurrence the widget is built. Chapter 14 spells it as a
-- `rebuild` over a constant, which reduces to `now`.
started :: Rx t => PushM t (Value t)
started = VEvent . (VUnit <$) <$> now

litValue :: Lit -> Value t
litValue l = case l of
  LNat n -> VInt n
  LInt n -> VInt n
  LFloat d -> VFloat d
  LBool b -> VBool b
  LUnit -> VUnit

key :: Name -> String
key (Name s b) = s ++ b

baseOf :: String -> String
baseOf = dropWhile (`elem` "!$@?")

lookupName :: Reflex t => Env t -> String -> Value t
lookupName env n = case M.lookup n env of
  Just v -> v
  Nothing
    | b == "_" , Just v <- firstJust [M.lookup (s ++ "_") env | s <- ["", "$", "!", "@", "!!", "$$", "!$"]] -> v
    | b == "never" -> VEvent never
    | b `elem` primNames -> VPrim b
    | b == "Nil" -> VNil
    | Just v <- M.lookup b env -> v          -- a sigil-less reference to a sigiled name
    | otherwise -> VCons b []   -- a nullary constructor or a unit-type value (click, tick)
  where
    b = baseOf n
    firstJust xs = case [x | Just x <- xs] of { (x:_) -> Just x; [] -> Nothing }

primNames :: [String]
primNames = ["Switch", "Join", "Union", "Filter", "Leftmost", "Updated", "Distinct", "HoldFirst",
             "Insert", "Remove", "Fold", "MapValues", "Keys", "Values", "Concat", "Len", "Append",
             "Size", "Show", "ToText", "Or", "Sqrt", "Min", "Max", "Some", "Get", "Member", "Range",
             "Abs", "Mod", "Take", "Drop", "DropLast", "Prepend", "Upper", "Lower", "JoinText", "Count", "Prefix", "Suffix", "Split", "Reverse", "First", "Last", "Second", "Third"]

-- | Construct a record, choice value, or element from a constructor name and fields.
construct :: Ctx t -> String -> [(Maybe String, Value t)] -> Value t
construct ctx n fields
  | n == "Some", [(_, v)] <- fields = VSome v
  | all (isJust . fst) fields && not (null fields) = VRec n [(f, v) | (Just f, v) <- fields]
  | null fields && isJust (M.lookup n (cTags ctx)) = VRec n []
  | otherwise = case M.lookup n (cTypes ctx) of
      Just (TDRecord _ fs) | length fs == length fields -> VRec n (zip fs (map snd fields))
      _ -> case M.lookup n (cTags ctx) of
        Just td | length (tdFields td) == length fields -> VRec n (zip (tdFields td) (map snd fields))
        _ -> VCons n (map snd fields)

field :: Value t -> String -> Value t
field v f = case v of
  VRec _ fs | Just x <- lookup f fs -> x
  VCons _ [x] | f == "value" -> x
  _ -> prismError "Build" ("no field " ++ f ++ " on " ++ showValue v)

index :: Value t -> Value t -> Value t
index (VMap m) k = maybe VNil VSome (M.lookup k m)
index (VArray xs) (VInt i) | i >= 0 && fromIntegral i < length xs = VSome (xs !! fromIntegral i)
index _ _ = VNil

binop :: String -> Value t -> Value t -> Value t
binop op a b = case (op, a, b) of
  ("+", VInt x, VInt y) -> VInt (x + y)
  ("-", VInt x, VInt y) -> VInt (x - y)
  ("*", VInt x, VInt y) -> VInt (x * y)
  ("/", VInt _, VInt 0) -> VNil
  ("/", VInt x, VInt y) -> VSome (VInt (x `div` y))
  ("+", VText x, VText y) -> VText (x ++ y)
  ("+", _, _) | isFloatish a && isFloatish b -> VFloat (toF a + toF b)
  ("-", _, _) | isFloatish a && isFloatish b -> VFloat (toF a - toF b)
  ("*", _, _) | isFloatish a && isFloatish b -> VFloat (toF a * toF b)
  ("/", _, _) | isFloatish a && isFloatish b -> VFloat (toF a / toF b)
  ("==", _, _) -> VBool (a == b)
  ("!=", _, _) -> VBool (a /= b)
  ("<", _, _) -> VBool (a < b)
  ("<=", _, _) -> VBool (a <= b)
  (">", _, _) -> VBool (a > b)
  (">=", _, _) -> VBool (a >= b)
  ("and", VBool x, VBool y) -> VBool (x && y)
  ("or", VBool x, VBool y) -> VBool (x || y)
  _ -> prismError "Build" ("bad operands for " ++ op ++ ": " ++ showValue a ++ ", " ++ showValue b)
  where
    isFloatish (VFloat _) = True
    isFloatish (VInt _) = True
    isFloatish _ = False
    toF (VFloat d) = d
    toF (VInt n) = fromIntegral n
    toF _ = 0

-- ---------------------------------------------------------------------------
-- Application

apply :: forall t m. (Rx t, Monad m) => Build t m -> Ctx t -> Value t -> [Value t] -> m (Value t)
apply b ctx f args = case f of
  VFn (Closure ps body cenv)
    | isBlueprintBody body ->
        -- a blueprint function: no lifting, sigiled or not
        eval b ctx (bindParams ps args cenv) body
    | otherwise ->
        let raw = [isSigiled p | p <- ps] ++ repeat False
            positions = [i | (i, (a, False)) <- zip [0 :: Int ..] (zip args raw), isFunctor a]
        in if null positions
          then eval b ctx (bindParams ps args cenv) body
          else return $ lifted "apply" [args !! i | i <- positions] $ \plain ->
            let m = M.fromList (zip positions plain)
                args' = [fromMaybe a (M.lookup i m) | (i, a) <- zip [0 ..] args]
            in evalPure ctx (bindParams ps args' cenv) body
  VPrim n -> prim b ctx n args
  VCons n [] -> return (lifted n args (\vs -> construct ctx n (map (Nothing,) vs)))
  VRec n [] -> return (lifted n args (\vs -> construct ctx n (map (Nothing,) vs)))
  _ -> prismError "Build" ("applying a non-function: " ++ showValue f)
  where
    isSigiled (Name s _) = s /= ""
    isBlueprintBody (EBlock _) = True
    isBlueprintBody _ = False

bindParams :: [Name] -> [Value t] -> Env t -> Env t
bindParams ps args env
  | length ps /= length args = prismError "Build" ("wrong number of arguments: expected " ++ show (length ps) ++ ", got " ++ show (length args))
  | otherwise = foldr (\(nm, v) e -> M.insert (key nm) v e) env (zip ps args)

matchArms :: (Rx t, Monad m) => Build t m -> Ctx t -> Env t -> Value t -> [(Pat, Maybe Expr, Expr)] -> m (Value t)
matchArms _ _ _ v [] = prismError "Build" ("no match arm for " ++ showValue v)
matchArms b ctx env v ((p, guard, body) : rest) = case matchPat ctx p v of
  Just binds -> do
    let env' = M.union (M.fromList binds) env
    ok <- case guard of
      Nothing -> return True
      Just g -> do
        gv <- eval b ctx env' g
        case gv of
          VBool x -> return x
          _ -> prismError "Build" "match guard is not a boolean"
    if ok then eval b ctx env' body else matchArms b ctx env v rest
  Nothing -> matchArms b ctx env v rest

matchPat :: Ctx t -> Pat -> Value t -> Maybe [(String, Value t)]
matchPat ctx p v = case (p, v) of
  (PWild, _) -> Just []
  (PVar nm, _) -> Just [(key nm, v)]
  (PLit l, _) -> if litValue l == v then Just [] else Nothing
  (PText s, VText t) -> if s == t then Just [] else Nothing
  (PCons "Some" [q], VSome x) -> matchPat ctx q x
  (PCons "Nil" [], VNil) -> Just []
  (PCons n qs, VCons m xs) | n == m && length qs == length xs -> concat <$> zipWithM (matchPat ctx) qs xs
  (PCons n [], VRec m []) | n == m -> Just []
  (PCons n qs, VRec m fs) | n == m && length qs == length fs -> concat <$> zipWithM (matchPat ctx) qs (map snd fs)
  (PRec n fps _, VRec m fs)
    | tagMatches n m ->
        let one (f, q) = case lookup f fs of
              Just x -> matchPat ctx q x
              Nothing -> Nothing
        in concat <$> mapM one fps
  (PRec n [] _, VCons m []) | tagMatches n m -> Just []
  (PCons n [], VRec m _) | n == m -> Just []
  (PTuple qs, VTuple xs) | length qs == length xs -> concat <$> zipWithM (matchPat ctx) qs xs
  _ -> Nothing
  where
    tagMatches pat tg
      | ".*" `isSuffixOf` pat = take (length pat - 1) pat `isPrefixOf` tg
      | otherwise = pat == tg

-- ---------------------------------------------------------------------------
-- Prelude

prim :: forall t m. (Rx t, Monad m) => Build t m -> Ctx t -> String -> [Value t] -> m (Value t)
prim b ctx n args = case (n, args) of
  ("Switch", [VEvent outer]) -> VEvent <$> runBuild b (switchHold never (asEvent "Switch" <$> outer))   -- !Switch: non-prompt
  ("Join", [VDyn outer]) -> return (VDyn (join (asDyn <$> outer)))                                     -- $Join: prompt
  ("Updated", [VDyn d]) -> return (VEvent (updated d))
  ("Distinct", [VDyn d]) -> VDyn <$> runBuild b (holdUniqDyn d)
  ("Leftmost", [VArray evs]) -> return (VEvent (leftmost (map (asEvent "Leftmost") evs)))
  ("Union", [VEvent x, VEvent y]) -> return (VEvent (mergeWith unionV [x, y]))
  ("Filter", [VEvent e]) -> return (VEvent (fmapMaybe (\case { VSome v -> Just v; _ -> Nothing }) e))
  ("HoldFirst", [VEvent e]) -> return (VBlueprint (Blueprint (\_ -> VDyn <$> holdDyn VNil e) True))
  ("Some", [v]) -> return (lifted "Some" [v] (\[x] -> VSome x))
  ("Fold", [m, z, f]) -> return $ lifted "Fold" [m, z] $ \[coll, acc0] -> case coll of
    VMap mm -> foldl (\acc (k, v) -> applyPure ctx f [acc, k, v]) acc0 (M.toList mm)
    VArray xs -> foldl (\acc x -> applyPure ctx f [acc, x]) acc0 xs
    _ -> prismError "Build" "Fold over a non-collection"
  ("MapValues", [m, f]) -> return $ lifted "MapValues" [m] $ \[mm] -> case mm of
    VMap mm' -> VMap (M.map (\v -> applyPure ctx f [v]) mm')
    _ -> prismError "Build" "MapValues over a non-map"
  _ -> return (lifted n args (plainPrim n))
  where
    unionV (VMap m1) (VMap m2) = VMap (M.union m1 m2)
    unionV v _ = v

plainPrim :: String -> [Value t] -> Value t
plainPrim n args = case (n, args) of
  ("Insert", [VMap m, k, v]) -> VMap (M.insert k v m)
  ("Remove", [VMap m, k]) -> VMap (M.delete k m)
  ("Get", [VMap m, k]) -> maybe VNil VSome (M.lookup k m)
  ("Member", [VMap m, k]) -> VBool (M.member k m)
  ("Keys", [VMap m]) -> VArray (M.keys m)
  ("Values", [VMap m]) -> VArray (M.elems m)
  ("Size", [VMap m]) -> VInt (fromIntegral (M.size m))
  ("Size", [VArray xs]) -> VInt (fromIntegral (length xs))
  ("Len", [VText s]) -> VInt (fromIntegral (length s))
  ("Take", [VText s, VInt k]) -> VText (take (fromIntegral k) s)
  ("Drop", [VText s, VInt k]) -> VText (drop (fromIntegral k) s)
  ("DropLast", [VText s]) -> VText (if null s then s else init s)
  ("Len", [VArray xs]) -> VInt (fromIntegral (length xs))
  ("Append", [VArray xs, x]) -> VArray (xs ++ [x])
  ("Prepend", [VArray xs, x]) -> VArray (x : xs)
  ("Upper", [VText t]) -> VText (map toUpper t)
  ("JoinText", [VArray xs, VText sep]) -> VText (intercalate sep (map textOf xs))
  ("Count", [VText t, VText c]) -> VInt (fromIntegral (length (filter (`elem` c) t)))
  ("Prefix", [VText t, VInt k]) -> VText (take (fromIntegral k) t)
  ("Suffix", [VText t, VInt k]) -> VText (drop (fromIntegral k) t)
  ("Split", [VText t, VText sep]) -> VArray (map VText (splitOn sep t))
  ("Reverse", [VArray xs]) -> VArray (reverse xs)
  ("First", [VTuple (x:_)]) -> x
  ("Second", [VTuple (_:y:_)]) -> y
  ("Third", [VTuple (_:_:z:_)]) -> z
  ("First", [VArray (x:_)]) -> VSome x
  ("First", [VArray []]) -> VNil
  ("Last", [VArray xs]) | not (null xs) -> VSome (last xs)
  ("Last", [VArray []]) -> VNil
  ("Lower", [VText t]) -> VText (map toLower t)
  ("Concat", [VArray xs]) -> VText (concatMap textOf xs)
  ("Show", [v]) -> VText (textOf v)
  ("ToText", [v]) -> VText (textOf v)
  ("Or", [VSome v, _]) -> v
  ("Or", [VNil, d]) -> d
  ("Sqrt", [VFloat d]) -> VFloat (sqrt d)
  ("Min", [a, b]) -> min a b
  ("Max", [a, b]) -> max a b
  ("Abs", [VInt x]) -> VInt (abs x)
  ("Mod", [VInt x, VInt y]) | y /= 0 -> VInt (x `mod` y)
  ("Range", [VInt a, VInt b]) -> VArray (map VInt [a .. b - 1])
  _ -> prismError "Build" ("bad arguments to " ++ n ++ ": " ++ unwords (map showValue args))

-- ---------------------------------------------------------------------------
-- Scopes: where a block's slots and child blocks register

newScope :: MonadIO m => Path t -> m (Scope t)
newScope path = liftIO (Scope path <$> newIORef 0 <*> newIORef [] <*> newIORef [])

nextPos :: MonadIO m => Scope t -> m Int
nextPos sc = liftIO (atomicModifyIORef' (scPos sc) (\n -> (n + 1, n)))

addList :: MonadIO m => Scope t -> Dynamic t (ListMap t) -> m ()
addList sc l = liftIO (modifyIORef' (scLists sc) (l :))

addReq :: MonadIO m => Scope t -> Event t (Requests t) -> m ()
addReq sc q = liftIO (modifyIORef' (scReqs sc) (q :))

-- | The scope's list, a dynamic of the union of its children's lists, and its requests, the
-- merge of its children's request events.
closeScope :: (Reflex t, MonadIO m) => Scope t -> m (Dynamic t (ListMap t), Event t (Requests t))
closeScope sc = do
  ls <- liftIO (readIORef (scLists sc))
  rs <- liftIO (readIORef (scReqs sc))
  let l = case ls of
        [] -> constDyn M.empty
        _ -> distributeListOverDynWith M.unions (reverse ls)
      q = case rs of
        [] -> never
        _ -> mergeWith (++) (reverse rs)
  return (l, q)

-- | What one build of a block produces: its result, its list and its requests.
data Gen t = Gen
  { genResult :: Value t
  , genList   :: Dynamic t (ListMap t)
  , genReqs   :: Event t (Requests t)
  }

-- | Register an event of generations with a scope: the list follows the latest generation
-- (a hold of the generations' lists, joined, so the old block's slots leave the list in the
-- occurrence of the rebuild), and the requests are those of the members as the occurrence
-- opened plus the birth term.
registerGenerations :: Rx t => Scope t -> Event t (Gen t) -> PushM t ()
registerGenerations sc genE = do
  listH <- holdDyn (constDyn M.empty) (genList <$> genE)
  addList sc (join listH)
  reqH <- holdDyn never (genReqs <$> genE)
  addReq sc (mergeWith (++) [switch (current reqH), registerBirth (genReqs <$> genE)])

-- | The birth term, the evaluator's one `coincidence`: a block born in occurrence n is heard
-- in n. `switch` over the members as the occurrence opened hears a block torn down in n
-- (its nodes propagate until commit) and not a block born in n; this term supplies the
-- newborn's firing. Prism's cycle check (not implemented here) is what makes it safe: nothing
-- a newborn fires at birth can be downstream of the consolidation that hears it.
registerBirth :: Reflex t => Event t (Event t a) -> Event t a
registerBirth = coincidence

-- ---------------------------------------------------------------------------
-- Widget blocks

-- | Build a block's statements into a scope and return the block's result. Names bound later
-- in the block are placeholders tied with `mdo`: an event name is a `switch` over the real
-- event, a dynamic name a `join` over the real dynamic, so that building the earlier
-- statements never forces the knot (Spider subscribes at the end of the frame).
buildBlock :: forall t. Rx t => Ctx t -> Env t -> Scope t -> Block -> PushM t (Value t)
buildBlock ctx env0 sc stmts = mdo
  let bound = concatMap boundNames stmts
      env1 = M.union (M.fromList [(key nm, placeholder nm (finalEnv M.! key nm)) | nm <- bound]) env0
  (finalEnv, result) <- foldM step (env1, VUnit) stmts
  -- every named event is subscribed, so that a prompt cycle through it is detected (Spider
  -- checks causality at subscription, and an unused event is never subscribed)
  forM_ [nm | nm <- bound, eventSigil (nSigil nm)] $ \nm -> case M.lookup (key nm) finalEnv of
    Just (VEvent e) -> void (hold () (() <$ e))
    _ -> return ()
  return result
  where
    boundNames s = case s of
      SBind nm _ -> [nm]
      SLet nm _ -> [nm]
      SBindPat p _ -> patNames p
      SExpr _ -> []
    patNames p = case p of
      PVar nm -> [nm]
      PCons _ ps -> concatMap patNames ps
      PTuple ps -> concatMap patNames ps
      PRec _ fs _ -> concatMap (patNames . snd) fs
      _ -> []
    placeholder (Name sig _) real
      | eventSigil sig = VEvent (switch (constant (asEvent "forward" real)))
      | dynSigil sig = VDyn (join (constDyn (asDyn real)))
      | otherwise = real
    eventSigil s = s `elem` ["!", "!!", "!$"]
    dynSigil s = s `elem` ["$", "$$"]
    step (env, _) s = case s of
      SLet nm e -> do
        v <- eval atBuild ctx env e
        return (M.insert (key nm) v env, VUnit)
      SBind nm (EThen initE e) -> do
        iv <- eval atBuild ctx env initE
        evv <- eval atBuild ctx env e
        d <- holdDyn iv (asEvent ("hold " ++ key nm) evv)
        return (M.insert (key nm) (VDyn d) env, VUnit)
      SBind nm e -> do
        v <- eval atBuild ctx env e >>= buildIf
        return (M.insert (key nm) v env, VUnit)
      SBindPat p e -> do
        v <- eval atBuild ctx env e >>= buildIf
        case matchPat ctx p v of
          Just binds -> return (M.union (M.fromList binds) env, VUnit)
          Nothing -> prismError "Build" ("bind pattern does not match " ++ showValue v)
      SExpr e -> do
        v <- eval atBuild ctx env e >>= buildIf
        return (env, v)
    buildIf (VBlueprint bp) = buildChild bp sc
    buildIf v = return v

-- | Build a blueprint as a child of a scope. Every widget bind takes a position (chapter 2's
-- conventions): a widget block and `@started` take it here and a block numbers its own binds
-- under it; `rebuild`, `for` and `@Element` take theirs inside their build, so a bind of one of
-- those is one segment, not two.
buildChild :: Rx t => Blueprint t -> Scope t -> PushM t (Value t)
buildChild bp sc
  | bpOwnPos bp = do
      pos <- nextPos sc
      sub <- newScope (scPath sc ++ [SPos pos])
      r <- bpBuild bp sub
      (l, q) <- closeScope sub
      addList sc l
      addReq sc q
      return r
  | otherwise = bpBuild bp sc

buildRebuild :: Rx t => Ctx t -> Env t -> Scope t -> Expr -> Block -> PushM t (Value t)
buildRebuild ctx env sc dExpr stmts = do
  dv <- eval atBuild ctx env dExpr
  VEvent <$> rebuildOver ctx sc (asDyn dv) (\gsc v -> buildBlock ctx (M.insert "_" v env) gsc stmts)

-- | The rebuild machinery, Reflex's `networkView` with the block built inside `push`: the
-- block is built when `leftmost [updated d, tag (current d) now]` fires, so once at the
-- rebuild's own build (with the new value if the dynamic changed in this occurrence) and
-- again in every occurrence the dynamic changes. Returns the event of the generations' results.
rebuildOver :: forall t. Rx t => Ctx t -> Scope t -> Dynamic t (Value t) -> (Scope t -> Value t -> PushM t (Value t)) -> PushM t (Event t (Value t))
rebuildOver _ sc d buildGen = do
  pos <- nextPos sc
  genRef <- liftIO (newIORef (0 :: Int))
  nowE <- now
  let trigger = leftmost [updated d, tag (current d) nowE]
      build v = do
        g <- liftIO (atomicModifyIORef' genRef (\n -> (n + 1, n)))
        gsc <- newScope (scPath sc ++ [SPos pos, SGen g])
        r <- buildGen gsc (forceValue v `seq` v)
        (l, q) <- closeScope gsc
        return (Just (Gen r l q))
      genE = push build trigger
  registerGenerations sc genE
  return (genResult <$> genE)

-- | One row of a `for`.
data Row t = Row
  { rowResult :: Value t
  , rowList   :: Dynamic t (ListMap t)
  , rowReqs   :: Event t (Requests t)
  }

data ForPatch t = ForPatch
  { fpRows :: M.Map (Value t) (Row t)      -- the members after this occurrence
  , fpBorn :: M.Map (Value t) (Row t)      -- the members born in it
  }

buildFor :: forall t. Rx t => Ctx t -> Env t -> Scope t -> Maybe (Name, Name) -> Expr -> Block -> PushM t (Value t)
buildFor ctx env sc binders mExpr stmts = do
  mv0 <- eval atBuild ctx env mExpr
  -- an array is a map keyed by index (second class, as the book says)
  let mv = lifted "for-source" [mv0] $ \[v] -> case v of
        VArray xs -> VMap (M.fromList (zip (map VInt [0 ..]) xs))
        other -> other
      md = asMap <$> asDyn mv
      asMap (VMap m) = m
      asMap v = prismError "Build" ("for over a non-collection: " ++ showValue v)
      (kName, vName) = case binders of
        Just (k, v) -> (key k, key v)
        Nothing -> ("key", "$_")
  pos <- nextPos sc
  nowE <- now
  rowsRef <- liftIO (newIORef M.empty)
  let trigger = leftmost [updated md, tag (current md) nowE]
      step m = do
        live <- liftIO (readIORef rowsRef)
        let newKeys = M.keys (m `M.difference` live)
            kept = live `M.intersection` m
        born <- forM newKeys $ \k -> do
          rsc <- newScope (scPath sc ++ [SPos pos, SKey k])
          let entry = fromMaybe VUnit . M.lookup k <$> md
              env' = M.insert kName k (M.insert vName (VDyn entry) env)
          r <- buildBlock ctx env' rsc stmts
          (l, q) <- closeScope rsc
          return (k, Row r l q)
        let live' = M.union kept (M.fromList born)
        liftIO (writeIORef rowsRef live')
        return (Just (ForPatch live' (M.fromList born)))
      patchE = push step trigger
  rowsD <- holdDyn M.empty (fpRows <$> patchE)
  addList sc (M.unions . M.elems <$> joinDynThroughMap (fmap rowList <$> rowsD))
  let reqsOf rows = mergeWith (++) (map rowReqs (M.elems rows))
  addReq sc (mergeWith (++) [switch (current (reqsOf <$> rowsD)), registerBirth (reqsOf . fpBorn <$> patchE)])
  -- the result kind is read off the block's last statement (the rows are built later)
  case guessKind stmts of
    "event" -> do
      -- consolidation is prompt: the members as the occurrence opened, plus the newborns. The
      -- birth term is left out when the block cannot fire at birth (chapter 14 reads that off
      -- the typed AST; here it is read off the syntax), which keeps a block whose result feeds
      -- the collection, the todo list, from being a prompt cycle through the term.
      let fired rows = VMap <$> mergeMap (M.map (asEvent "for" . rowResult) rows)
          unionMaps (VMap a) (VMap b) = VMap (M.union a b)
          unionMaps a _ = a
          opened = switch (current (fired <$> rowsD))
      return $ VEvent $ if canFireAtBirth stmts
        then mergeWith unionMaps [opened, registerBirth (fired . fpBorn <$> patchE)]
        else opened
    _ -> return (VDyn (VMap <$> joinDynThroughMap (fmap (asDyn . rowResult) <$> rowsD)))
  where
    guessKind ss = case reverse ss of
      (SExpr e : _) -> kindOf e
      _ -> "plain"
    kindOf e = case e of
      EReturn x -> kindOf x
      EBin "on" _ _ -> "event"
      EBin "|" _ _ -> "event"
      EVar (Name s _) | s `elem` ["!", "!!"] -> "event"
                      | s `elem` ["$", "$$"] -> "dyn"
      EApp (EVar (Name "!" _)) _ -> "event"
      EApp (EVar (Name "$" _)) _ -> "dyn"
      EApp _ as | any ((== "event") . kindOf) as -> "event"
                | any ((== "dyn") . kindOf) as -> "dyn"
      EBin _ a b | kindOf a == "event" || kindOf b == "event" -> "event"
                 | kindOf a == "dyn" || kindOf b == "dyn" -> "dyn"
      EText parts | any (\p -> case p of { TExpr x -> kindOf x /= "plain"; _ -> False }) parts -> "dyn"
      ELit _ -> "plain"
      _ -> "plain"

-- | @Element: make a slot. Dynamic fields lift the element into a rebuild.
buildElement :: forall t. Rx t => Ctx t -> Env t -> Scope t -> Expr -> PushM t (Value t)
buildElement ctx env sc e = do
  let (plainExpr, requestsExpr, tagName) = case e of
        ECons n fields ->
          ( ECons n [fv | fv@(f, _) <- fields, f /= Just "requests"]
          , listToMaybe [v | (Just "requests", v) <- fields]
          , Just n )
        _ -> (e, Nothing, Nothing)
  ev <- eval atBuild ctx env plainExpr
  reqE <- fmap (asEvent "requests") <$> mapM (eval atBuild ctx env) requestsExpr
  case ev of
    VDyn d -> do
      -- the shorthand: a fresh slot per change
      resultEv <- rebuildOver ctx sc d (\gsc v -> makeSlot ctx gsc v reqE)
      case tagName >>= \n -> M.lookup n (cTags ctx) of
        Just td | tdResponds td -> VEvent <$> switchHold never (asEvent "element" <$> resultEv)
                | tdState td -> do
                    -- a hold of the generations' state dynamics, joined; the first generation
                    -- arrives in the build occurrence and the join is prompt
                    outer <- holdDyn (VDyn (constDyn VUnit)) resultEv
                    return (VDyn (join (asDyn <$> outer)))
        _ -> return VUnit
    v -> makeSlot ctx sc v reqE

-- | Create one slot for a plain element value: its path, its response event selected from
-- the root fan by path, its state cell, its request event told to the scope, and the slot
-- itself told as a constant one-entry list.
makeSlot :: forall t. Rx t => Ctx t -> Scope t -> Value t -> Maybe (Event t (Value t)) -> PushM t (Value t)
makeSlot ctx sc v reqE = do
  let v' = case v of { VCons n [] -> VRec n []; _ -> v }
      tg = case v' of { VRec n _ -> n; VCons n _ -> n; _ -> "" }
      td = fromMaybe (TagDecl tg [] False False False) (M.lookup tg (cTags ctx))
  pos <- nextPos sc
  let path = scPath sc ++ [SPos pos]
      responds = if tdResponds td then Just (select (cFan ctx) (Const2 path)) else Nothing
  state <- if tdState td then Just <$> cellFor ctx (forceValue v' `seq` v') else return Nothing
  forM_ reqE $ \r -> addReq sc ((\x -> [(path, x)]) <$> r)
  addList sc (constDyn (M.singleton path (Slot path v' (tdResponds td))))
  return $ case (responds, state) of
    (Just e, Just d) -> VTuple [VEvent e, VDyn d]
    (Just e, _) -> VEvent e
    (_, Just d) -> VDyn d
    _ -> VUnit

-- | A state cell: a hold initialised from the oracle's answer and fed by this and later turns'
-- pushes for its element value. Two slots with equal values share one cell.
cellFor :: forall t. Rx t => Ctx t -> Value t -> PushM t (Dynamic t (Value t))
cellFor ctx ev = do
  cells <- liftIO (readIORef (cCells ctx))
  case M.lookup ev cells of
    Just d -> return d
    Nothing -> do
      o <- liftIO (cOracle ctx ev)
      case o of
        Nothing -> prismError "UnsuppliedState" ("element=" ++ showValue ev)
        Just s -> do
          d <- holdDyn s (fmapMaybe (M.lookup ev) (cPushes ctx))
          liftIO (modifyIORef' (cCells ctx) (M.insert ev d))
          return d

-- | Whether a `for`'s block can fire in the occurrence of its birth: it mentions `@started`,
-- a `rebuild`, `!Updated`, or an event bound outside the block (which may be firing as the
-- block is born). A block whose events are all its own elements' responses cannot, since no
-- response reaches a slot in the occurrence it was created.
canFireAtBirth :: Block -> Bool
canFireAtBirth stmts = any (exprHas . stmtExprs) stmts || any (`notElem` bound) (concatMap (freeEvents . stmtExprs) stmts)
  where
    bound = concatMap boundIn stmts
    boundIn s = case s of
      SBind nm _ -> [key nm]
      SLet nm _ -> [key nm]
      SBindPat p _ -> map key (patNames p)
      SExpr _ -> []
    patNames p = case p of
      PVar nm -> [nm]
      PCons _ ps -> concatMap patNames ps
      PTuple ps -> concatMap patNames ps
      PRec _ fs _ -> concatMap (patNames . snd) fs
      _ -> []
    stmtExprs s = case s of
      SBind _ e -> [e]
      SBindPat _ e -> [e]
      SLet _ e -> [e]
      SExpr e -> [e]
    exprHas = any has
    has e = case e of
      EStarted -> True
      ERebuild _ _ -> True
      EApp (EVar (Name _ "Updated")) _ -> True
      _ -> any has (children e)
    freeEvents = concatMap free
    free e = case e of
      EVar (Name s b) | s `elem` ["!", "!!", "!$"] -> [s ++ b]
      ELam ps body -> filter (`notElem` map key ps) (free body)
      EBlock ss -> filter (`notElem` concatMap boundIn ss) (concatMap (freeEvents . stmtExprs) ss)
      ERebuild d ss -> free d ++ filter (`notElem` concatMap boundIn ss) (concatMap (freeEvents . stmtExprs) ss)
      EFor bs m ss -> free m ++ filter (`notElem` (concatMap boundIn ss ++ maybe [] (\(k, v) -> [key k, key v]) bs)) (concatMap (freeEvents . stmtExprs) ss)
      _ -> concatMap free (children e)
    children e = case e of
      EText parts -> [x | TExpr x <- parts]
      EApp f as -> f : as
      ECons _ fs -> map snd fs
      EField x _ -> [x]
      EIndex x i -> [x, i]
      EBin _ a b -> [a, b]
      ENot x -> [x]
      ENeg x -> [x]
      EThen a b -> [a, b]
      EIf c a b -> [c, a, b]
      EMatch s arms -> s : concat [maybe [] (: []) g ++ [body] | (_, g, body) <- arms]
      ELam _ body -> [body]
      EBlock ss -> concatMap stmtExprs ss
      ERebuild d ss -> d : concatMap stmtExprs ss
      EFor _ m ss -> m : concatMap stmtExprs ss
      EReturn x -> [x]
      EFree _ x -> [x]
      EElement x -> [x]
      EArray xs -> xs
      EMap kvs -> concat [[k, v] | (k, v) <- kvs]
      ETuple xs -> xs
      ELet _ v body -> [v, body]
      _ -> []
