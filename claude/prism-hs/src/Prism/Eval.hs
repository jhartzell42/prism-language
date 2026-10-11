-- | The evaluator: expressions with lifting over the event and dynamic functors, widget
-- blocks built into scopes, rebuild, for, elements, and the prelude.
module Prism.Eval where

import Prism.Syntax
import Prism.Value
import Prism.Runtime
import Data.IORef
import qualified Data.Map.Strict as M
import Data.Char (isUpper, toUpper, toLower)
import Data.List (isPrefixOf, isSuffixOf, intercalate)

import Data.Maybe (fromMaybe, isJust)
import Control.Monad
import Control.Exception

data Ctx = Ctx
  { cRT     :: RT
  , cTags   :: M.Map String TagDecl
  , cTypes  :: M.Map String TypeDecl
  , cOracle :: Value -> IO (Maybe Value)      -- the World's snapshot
  }

splitOn :: String -> String -> [String]
splitOn sep t = go t
  where
    go s = case breakOn s of
      (a, Nothing) -> [a]
      (a, Just rest) -> a : go rest
    breakOn s | null s = ("", Nothing)
              | sep `isPrefixOf` s = ("", Just (drop (length sep) s))
              | otherwise = let (a, r) = breakOn (tail s) in (head s : a, r)

prismError :: String -> String -> IO a
prismError k d = throwIO (PrismError k d)

-- ---------------------------------------------------------------------------
-- Scopes

newScope :: [Seg] -> IO Scope
newScope path = Scope path <$> newIORef 0 <*> newIORef []

nextPos :: Scope -> IO Int
nextPos sc = atomicModifyIORef' (scPos sc) (\n -> (n + 1, n))

addChild :: Scope -> Child -> IO ()
addChild sc c = modifyIORef' (scChildren sc) (++ [c])

-- | Every slot under a scope, in tree order.
slotsOf :: Scope -> IO [Slot]
slotsOf sc = do
  cs <- readIORef (scChildren sc)
  concat <$> mapM one cs
  where
    one (CSlot s) = return [s]
    one (CScope ref) = readIORef ref >>= maybe (return []) slotsOf
    one (CKeyed ref) = readIORef ref >>= fmap concat . mapM slotsOf . M.elems

-- ---------------------------------------------------------------------------
-- Lifting

isFunctor :: Value -> Bool
isFunctor (VEvent _) = True
isFunctor (VDyn _) = True
isFunctor _ = False

-- | Apply a plain function to operands, lifting over any event or dynamic among them.
lifted :: Ctx -> String -> [Value] -> ([Value] -> IO Value) -> IO Value
lifted ctx label vs f
  | not (any isFunctor vs) = f vs
  | otherwise = do
      let rt = cRT ctx
          evs = [i | (i, VEvent _) <- zip [0 :: Int ..] vs]
      case evs of
        (_:_:_) -> prismError "Build" ("two events in one operation: " ++ label)
        [i] -> do
          let VEvent e = vs !! i
              dyns = [(j, d) | (j, VDyn d) <- zip [0 ..] vs]
          ev <- tagMap rt label e (map snd dyns) $ \ev befores -> do
            let m = M.fromList ((i, ev) : zip (map fst dyns) befores)
            f [fromMaybe v (M.lookup j m) | (j, v) <- zip [0 ..] vs]
          return (VEvent ev)
        [] -> do
          let dyns = [(j, d) | (j, VDyn d) <- zip [0 ..] vs]
          d <- derivedDyn rt label (map snd dyns) $ \nows -> do
            let m = M.fromList (zip (map fst dyns) nows)
            f [fromMaybe v (M.lookup j m) | (j, v) <- zip [0 ..] vs]
          return (VDyn d)

asDyn :: Ctx -> Value -> IO Dyn
asDyn _ (VDyn d) = return d
asDyn ctx v = constDyn (cRT ctx) v

asEvent :: String -> Value -> IO Ev
asEvent _ (VEvent e) = return e
asEvent what v = prismError "Build" (what ++ ": expected an event, got " ++ showValue v)

-- ---------------------------------------------------------------------------
-- Expressions

eval :: Ctx -> Env -> Expr -> IO Value
eval ctx env expr = case expr of
  ELit l -> return (litValue l)
  EVar nm -> lookupName ctx env (key nm)
  EText parts -> do
    vs <- mapM partVal parts
    lifted ctx "text" vs (return . VText . concatMap textOf)
  EApp hd args -> do
    f <- eval ctx env hd
    as <- mapM (eval ctx env) args
    apply ctx f as
  ECons n fields -> do
    vs <- mapM (eval ctx env . snd) fields
    lifted ctx n vs $ \vs' -> return (construct ctx n (zip (map fst fields) vs'))
  EField e f -> do
    v <- eval ctx env e
    lifted ctx ("." ++ f) [v] $ \[x] -> field x f
  EIndex e i -> do
    v <- eval ctx env e
    iv <- eval ctx env i
    lifted ctx "index" [v, iv] $ \[x, k] -> return (index x k)
  EBin "|" a b -> do
    x <- eval ctx env a
    y <- eval ctx env b
    ex <- asEvent "|" x
    ey <- asEvent "|" y
    VEvent <$> leftmost (cRT ctx) [ex, ey]
  EBin "on" a b -> do
    x <- eval ctx env a
    y <- eval ctx env b
    e <- asEvent "on" y
    case x of
      VDyn d -> VEvent <$> tagMap (cRT ctx) "on" e [d] (\_ [bv] -> return bv)
      VEvent _ -> prismError "Build" "`on` with an event on the left"
      v -> VEvent <$> tagMap (cRT ctx) "on" e [] (\_ _ -> return v)
  EBin "and" a b -> do
    x <- eval ctx env a
    case x of
      VBool False -> return x
      VBool True -> eval ctx env b
      _ -> do y <- eval ctx env b; lifted ctx "and" [x, y] (\[p, q] -> binop "and" p q)
  EBin "or" a b -> do
    x <- eval ctx env a
    case x of
      VBool True -> return x
      VBool False -> eval ctx env b
      _ -> do y <- eval ctx env b; lifted ctx "or" [x, y] (\[p, q] -> binop "or" p q)
  EBin op a b -> do
    x <- eval ctx env a
    y <- eval ctx env b
    lifted ctx op [x, y] (\[p, q] -> binop op p q)
  ENot a -> do
    x <- eval ctx env a
    lifted ctx "not" [x] (\[VBool b] -> return (VBool (not b)))
  ENeg a -> do
    x <- eval ctx env a
    lifted ctx "neg" [x] $ \[v] -> case v of
      VInt n -> return (VInt (negate n))
      VFloat d -> return (VFloat (negate d))
      _ -> prismError "Build" "negating a non-number"
  EThen _ _ -> prismError "Build" "`then` outside a hold statement"
  EIf c t e -> do
    cv <- eval ctx env c
    case cv of
      VBool True -> eval ctx env t
      VBool False -> eval ctx env e
      _ -> lifted ctx "if" [cv] $ \[b] -> case b of
             VBool True -> eval ctx env t
             VBool False -> eval ctx env e
             _ -> prismError "Build" "if on a non-boolean"
  EMatch s arms -> do
    sv <- eval ctx env s
    lifted ctx "match" [sv] $ \[v] -> matchArms ctx env v arms
  ELam ps body -> return (VFn (Closure ps body env))
  EBlock stmts -> return (VBlueprint (Blueprint (\sc -> buildBlock ctx env sc stmts) True))
  ERebuild d stmts -> return (VBlueprint (Blueprint (\sc -> buildRebuild ctx env sc d stmts) False))
  EFor binders m stmts -> return (VBlueprint (Blueprint (\sc -> buildFor ctx env sc binders m stmts) False))
  EReturn e -> eval ctx env e
  EFree "$" e -> do
    v <- eval ctx env e
    case v of
      VDyn _ -> return v
      _ -> VDyn <$> constDyn (cRT ctx) v
  EFree _ e -> eval ctx env e
  EElement e -> return (VBlueprint (Blueprint (\sc -> buildElement ctx env sc e) False))
  EStarted -> return (VBlueprint (Blueprint (\_ -> VEvent <$> started (cRT ctx)) True))
  EArray es -> VArray <$> mapM (eval ctx env) es        -- an array of events is plain (chapter 3)
  EMap kvs -> do
    ks <- mapM (eval ctx env . fst) kvs
    vs <- mapM (eval ctx env . snd) kvs
    lifted ctx "map" (ks ++ vs) $ \xs ->
      let (ks', vs') = splitAt (length ks) xs in return (VMap (M.fromList (zip ks' vs')))
  ETuple es -> VTuple <$> mapM (eval ctx env) es        -- a bundle: never lifted
  ELet nm v body -> do
    vv <- eval ctx env v
    eval ctx (M.insert (key nm) vv env) body
  where
    partVal (TLit s) = return (VText s)
    partVal (TExpr e) = eval ctx env e

litValue :: Lit -> Value
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

lookupName :: Ctx -> Env -> String -> IO Value
lookupName ctx env n0 = case M.lookup n env of
  Just (VThunk ref) -> do
    st <- readIORef ref
    case st of
      Right v -> return v
      Left act -> do
        writeIORef ref (Left (prismError "Cycle" ("let " ++ n ++ " depends on itself")))
        v <- act
        writeIORef ref (Right v)
        return v
  Just v -> return v
  Nothing
    | b == "_" , Just v <- firstJust [M.lookup (s ++ "_") env | s <- ["", "$", "!", "@", "!!", "$$", "!$"]] -> force v
    | b == "never" -> VEvent <$> newExternal (cRT ctx) "never"
    | b `elem` primNames -> return (VPrim b)
    | b == "Nil" -> return VNil
    | Just v <- M.lookup b env -> return v          -- a sigil-less reference to a sigiled name
    | otherwise -> return (VCons b [])   -- a nullary constructor or a unit-type value (click, tick)
  where
    n = n0
    b = baseOf n0
    firstJust xs = case [x | Just x <- xs] of { (x:_) -> Just x; [] -> Nothing }
    force (VThunk ref) = do
      st <- readIORef ref
      case st of
        Right v -> return v
        Left act -> do v <- act; writeIORef ref (Right v); return v
    force v = return v

primNames :: [String]
primNames = ["Switch", "Join", "Union", "Filter", "Leftmost", "Updated", "Distinct", "HoldFirst",
             "Insert", "Remove", "Fold", "MapValues", "Keys", "Values", "Concat", "Len", "Append",
             "Size", "Show", "ToText", "Or", "Sqrt", "Min", "Max", "Some", "Get", "Member", "Range",
             "Abs", "Mod", "Take", "Drop", "DropLast", "Prepend", "Upper", "Lower", "JoinText", "Count", "Prefix", "Suffix", "Split", "Reverse", "First", "Last", "Second", "Third"]

-- | Construct a record, choice value, or element from a constructor name and fields.
construct :: Ctx -> String -> [(Maybe String, Value)] -> Value
construct ctx n fields
  | n == "Some", [(_, v)] <- fields = VSome v
  | all (isJust . fst) fields && not (null fields) = VRec n [(f, v) | (Just f, v) <- fields]
  | null fields && isJust (M.lookup n (cTags ctx)) = VRec n []
  | otherwise = case M.lookup n (cTypes ctx) of
      Just (TDRecord _ fs) | length fs == length fields -> VRec n (zip fs (map snd fields))
      _ -> case M.lookup n (cTags ctx) of
        Just td | length (tdFields td) == length fields -> VRec n (zip (tdFields td) (map snd fields))
        _ -> VCons n (map snd fields)

field :: Value -> String -> IO Value
field v f = case v of
  VRec _ fs | Just x <- lookup f fs -> return x
  VCons _ [x] | f == "value" -> return x
  _ -> prismError "Build" ("no field " ++ f ++ " on " ++ showValue v)

index :: Value -> Value -> Value
index (VMap m) k = maybe VNil VSome (M.lookup k m)
index (VArray xs) (VInt i) | i >= 0 && fromIntegral i < length xs = VSome (xs !! fromIntegral i)
index _ _ = VNil

binop :: String -> Value -> Value -> IO Value
binop op a b = case (op, a, b) of
  ("+", VInt x, VInt y) -> return (VInt (x + y))
  ("-", VInt x, VInt y) -> return (VInt (x - y))
  ("*", VInt x, VInt y) -> return (VInt (x * y))
  ("/", VInt _, VInt 0) -> return VNil
  ("/", VInt x, VInt y) -> return (VSome (VInt (x `div` y)))
  ("+", VText x, VText y) -> return (VText (x ++ y))
  ("+", _, _) | isFloatish a && isFloatish b -> return (VFloat (toF a + toF b))
  ("-", _, _) | isFloatish a && isFloatish b -> return (VFloat (toF a - toF b))
  ("*", _, _) | isFloatish a && isFloatish b -> return (VFloat (toF a * toF b))
  ("/", _, _) | isFloatish a && isFloatish b -> return (VFloat (toF a / toF b))
  ("==", _, _) -> return (VBool (a == b))
  ("!=", _, _) -> return (VBool (a /= b))
  ("<", _, _) -> return (VBool (a < b))
  ("<=", _, _) -> return (VBool (a <= b))
  (">", _, _) -> return (VBool (a > b))
  (">=", _, _) -> return (VBool (a >= b))
  ("and", VBool x, VBool y) -> return (VBool (x && y))
  ("or", VBool x, VBool y) -> return (VBool (x || y))
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

apply :: Ctx -> Value -> [Value] -> IO Value
apply ctx f args = case f of
  VFn (Closure ps body cenv)
    | isBlueprintBody body -> do
        -- a blueprint function: no lifting, sigiled or not
        env' <- bindParams ps args cenv
        eval ctx env' body
    | otherwise -> do
        let raw = [isSigiled p | p <- ps] ++ repeat False
            toLift = [a | (a, False) <- zip args raw, isFunctor a]
        if null toLift
          then do env' <- bindParams ps args cenv; eval ctx env' body
          else do
            let positions = [i | (i, (a, False)) <- zip [0 :: Int ..] (zip args raw), isFunctor a]
            lifted ctx "apply" [args !! i | i <- positions] $ \plain -> do
              let m = M.fromList (zip positions plain)
                  args' = [fromMaybe a (M.lookup i m) | (i, a) <- zip [0 ..] args]
              env' <- bindParams ps args' cenv
              eval ctx env' body
  VPrim n -> prim ctx n args
  VCons n [] -> lifted ctx n args (\vs -> return (construct ctx n (map ((,) Nothing) vs)))
  VRec n [] -> lifted ctx n args (\vs -> return (construct ctx n (map ((,) Nothing) vs)))
  _ -> prismError "Build" ("applying a non-function: " ++ showValue f)
  where
    isSigiled (Name s _) = s /= ""
    isBlueprintBody (EBlock _) = True
    isBlueprintBody _ = False

bindParams :: [Name] -> [Value] -> Env -> IO Env
bindParams ps args env
  | length ps /= length args = prismError "Build" ("wrong number of arguments: expected " ++ show (length ps) ++ ", got " ++ show (length args))
  | otherwise = return (foldr (\(nm, v) e -> M.insert (key nm) v e) env (zip ps args))

matchArms :: Ctx -> Env -> Value -> [(Pat, Maybe Expr, Expr)] -> IO Value
matchArms ctx env v [] = prismError "Build" ("no match arm for " ++ showValue v)
matchArms ctx env v ((p, guard, body) : rest) = case matchPat ctx p v of
  Just binds -> do
    let env' = M.union (M.fromList binds) env
    ok <- case guard of
      Nothing -> return True
      Just g -> do
        gv <- eval ctx env' g
        case gv of
          VBool b -> return b
          _ -> prismError "Build" "match guard is not a boolean"
    if ok then eval ctx env' body else matchArms ctx env v rest
  Nothing -> matchArms ctx env v rest

matchPat :: Ctx -> Pat -> Value -> Maybe [(String, Value)]
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
  (PRec n fps dots, VRec m fs)
    | tagMatches n m ->
        let one (f, q) = case lookup f fs of
              Just x -> matchPat ctx q x
              Nothing -> Nothing
        in if not dots && any (`notElem` map fst fps) (map fst fs) && not (null fps) && False
             then Nothing
             else concat <$> mapM one fps
  (PRec n [] _, VCons m []) | tagMatches n m -> Just []
  (PCons n [], VRec m _) | n == m -> Just []
  (PTuple qs, VTuple xs) | length qs == length xs -> concat <$> zipWithM (matchPat ctx) qs xs
  _ -> Nothing
  where
    tagMatches pat tag
      | ".*" `isSuffixOf` pat = take (length pat - 1) pat `isPrefixOf` tag
      | otherwise = pat == tag

-- ---------------------------------------------------------------------------
-- Prelude

prim :: Ctx -> String -> [Value] -> IO Value
prim ctx n args = case (n, args) of
  ("Switch", [VEvent outer]) -> VEvent <$> switchE rt outer
  ("Join", [VDyn outer]) -> VDyn <$> joinD rt outer
  ("Updated", [VDyn d]) -> return (VEvent (dynChange d))
  ("Distinct", [VDyn d]) -> return (VDyn d)
  ("Leftmost", [VArray evs]) -> do
    es <- mapM (asEvent "Leftmost") evs
    VEvent <$> leftmost rt es
  ("Union", [VEvent a, VEvent b]) ->
    VEvent <$> derived rt "union" [a, b] (do
      x <- firingOf rt a
      y <- firingOf rt b
      return $ case (x, y) of
        (Just (VMap m1), Just (VMap m2)) -> Just (VMap (M.union m1 m2))
        (Just v, Nothing) -> Just v
        (Nothing, Just v) -> Just v
        _ -> Nothing)
  ("Filter", [VEvent e]) ->
    VEvent <$> derived rt "filter" [e] (do
      x <- firingOf rt e
      return $ case x of
        Just (VSome v) -> Just v
        _ -> Nothing)
  ("HoldFirst", [VEvent e]) -> return $ VBlueprint $ flip Blueprint True $ \_ -> do
    f <- firingOf rt e
    case f of
      Just v -> VDyn <$> mkDyn rt v e
      Nothing -> prismError "Build" "@HoldFirst: the event did not fire in the build occurrence"
  ("Some", [v]) -> lifted ctx "Some" [v] (\[x] -> return (VSome x))
  ("Fold", [m, z, f]) -> lifted ctx "Fold" [m, z] $ \[coll, acc0] -> case coll of
    VMap mm -> foldM (\acc (k, v) -> apply ctx f [acc, k, v]) acc0 (M.toList mm)
    VArray xs -> foldM (\acc x -> apply ctx f [acc, x]) acc0 xs
    _ -> prismError "Build" "Fold over a non-collection"
  ("MapValues", [m, f]) -> lifted ctx "MapValues" [m] $ \[VMap mm] ->
    VMap <$> traverse (\v -> apply ctx f [v]) mm
  _ -> lifted ctx n args (plainPrim n)
  where rt = cRT ctx

plainPrim :: String -> [Value] -> IO Value
plainPrim n args = case (n, args) of
  ("Insert", [VMap m, k, v]) -> return (VMap (M.insert k v m))
  ("Remove", [VMap m, k]) -> return (VMap (M.delete k m))
  ("Get", [VMap m, k]) -> return (maybe VNil VSome (M.lookup k m))
  ("Member", [VMap m, k]) -> return (VBool (M.member k m))
  ("Keys", [VMap m]) -> return (VArray (M.keys m))
  ("Values", [VMap m]) -> return (VArray (M.elems m))
  ("Size", [VMap m]) -> return (VInt (fromIntegral (M.size m)))
  ("Size", [VArray xs]) -> return (VInt (fromIntegral (length xs)))
  ("Len", [VText s]) -> return (VInt (fromIntegral (length s)))
  ("Take", [VText s, VInt n]) -> return (VText (take (fromIntegral n) s))
  ("Drop", [VText s, VInt n]) -> return (VText (drop (fromIntegral n) s))
  ("DropLast", [VText s]) -> return (VText (if null s then s else init s))
  ("Len", [VArray xs]) -> return (VInt (fromIntegral (length xs)))
  ("Append", [VArray xs, x]) -> return (VArray (xs ++ [x]))
  ("Prepend", [VArray xs, x]) -> return (VArray (x : xs))
  ("Upper", [VText t]) -> return (VText (map toUpper t))
  ("JoinText", [VArray xs, VText sep]) -> return (VText (intercalate sep (map textOf xs)))
  ("Count", [VText t, VText c]) -> return (VInt (fromIntegral (length (filter (`elem` c) t))))
  ("Prefix", [VText t, VInt n]) -> return (VText (take (fromIntegral n) t))
  ("Suffix", [VText t, VInt n]) -> return (VText (drop (fromIntegral n) t))
  ("Split", [VText t, VText sep]) -> return (VArray (map VText (splitOn sep t)))
  ("Reverse", [VArray xs]) -> return (VArray (reverse xs))
  ("First", [VTuple (x:_)]) -> return x
  ("Second", [VTuple (_:y:_)]) -> return y
  ("Third", [VTuple (_:_:z:_)]) -> return z
  ("First", [VArray (x:_)]) -> return (VSome x)
  ("First", [VArray []]) -> return VNil
  ("Last", [VArray xs]) | not (null xs) -> return (VSome (last xs))
  ("Last", [VArray []]) -> return VNil
  ("Lower", [VText t]) -> return (VText (map toLower t))
  ("Concat", [VArray xs]) -> return (VText (concatMap textOf xs))
  ("Show", [v]) -> return (VText (textOf v))
  ("ToText", [v]) -> return (VText (textOf v))
  ("Or", [VSome v, _]) -> return v
  ("Or", [VNil, d]) -> return d
  ("Sqrt", [VFloat d]) -> return (VFloat (sqrt d))
  ("Min", [a, b]) -> return (min a b)
  ("Max", [a, b]) -> return (max a b)
  ("Abs", [VInt x]) -> return (VInt (abs x))
  ("Mod", [VInt x, VInt y]) | y /= 0 -> return (VInt (x `mod` y))
  ("Range", [VInt a, VInt b]) -> return (VArray (map VInt [a .. b - 1]))
  _ -> prismError "Build" ("bad arguments to " ++ n ++ ": " ++ unwords (map showValue args))

-- Fold needs the evaluator to call the closure; handled through apply when the prim sees a VFn.
-- (Implemented in applyPrimWithFn below and dispatched from prim via the generic path.)

-- ---------------------------------------------------------------------------
-- Widget blocks

-- | Build a block's statements into a scope and return the block's result.
buildBlock :: Ctx -> Env -> Scope -> Block -> IO Value
buildBlock ctx env0 sc stmts = do
  let rt = cRT ctx
  -- pre-pass: holds get their dynamic (with a forward change event); event names get forwards
  (env1, holds, fwds) <- foldM prepass (env0, M.empty, M.empty) stmts
  -- plain lets get thunks that evaluate in the environment as it stands when forced; the
  -- environment ref lets a thunk see bindings made after the pre-pass
  envRef <- newIORef env1
  forM_ stmts $ \s -> case s of
    SLet nm@(Name sig _) e | not (eventSigil sig), let n = key nm -> case M.lookup n env1 of
      Just (VThunk ref) -> writeIORef ref (Left (readIORef envRef >>= \e' -> eval ctx e' e))
      _ -> return ()
    _ -> return ()
  (env2, result) <- foldM (step holds fwds envRef) (env1, VUnit) (zip [0 :: Int ..] stmts)
  return result
  where
    rt = cRT ctx
    prepass (env, holds, fwds) s = case s of
      SBind nm@(Name sig b) (EThen initE _) -> do
        let n = key nm
        iv <- eval ctx env initE `catch` \(PrismError _ _) ->
                prismError "Build" ("the initial value of hold " ++ n ++ " must not depend on later bindings (prototype limit)")
        (fwd, connect) <- newForward rt ("hold " ++ n)
        d <- mkDyn rt iv fwd
        return (M.insert n (VDyn d) env, M.insert n (d, connect) holds, fwds)
      SBind nm@(Name sig _) _ | eventSigil sig -> do
        let n = key nm
        (fwd, connect) <- newForward rt ("fwd " ++ n)
        return (M.insert n (VEvent fwd) env, holds, M.insert n (evConnector connect) fwds)
      SBind nm@(Name sig _) _ | dynSigil sig -> do
        let n = key nm
        (fd, connect) <- newForwardDyn rt n
        return (M.insert n (VDyn fd) env, holds, M.insert n connect fwds)
      SLet nm@(Name sig _) _ | eventSigil sig -> do
        let n = key nm
        (fwd, connect) <- newForward rt ("fwd " ++ n)
        return (M.insert n (VEvent fwd) env, holds, M.insert n (evConnector connect) fwds)
      SLet nm _ -> do
        let n = key nm
        ref <- newIORef (Left (prismError "Build" ("let " ++ n ++ " used before the block could define it")))
        return (M.insert n (VThunk ref) env, holds, fwds)
      SBindPat p _ -> foldM (\(e, h, f) nm@(Name sig _) ->
          let n = key nm in
          if eventSigil sig then do
            (fwd, connect) <- newForward rt ("fwd " ++ n)
            return (M.insert n (VEvent fwd) e, h, M.insert n (evConnector connect) f)
          else if dynSigil sig then do
            (fd, connect) <- newForwardDyn rt n
            return (M.insert n (VDyn fd) e, h, M.insert n connect f)
          else return (e, h, f)) (env, holds, fwds) (patNames p)
      _ -> return (env, holds, fwds)
    patNames p = case p of
      PVar nm -> [nm]
      PCons _ ps -> concatMap patNames ps
      PTuple ps -> concatMap patNames ps
      PRec _ fs _ -> concatMap (patNames . snd) fs
      _ -> []
    eventSigil s = s == "!" || s == "!!"
    dynSigil s = s == "$" || s == "$$"
    evConnector connect v = case v of
      VEvent e -> connect e
      _ -> return ()
    step holds fwds envRef (env, _) (i, s) = case s of
      SLet nm e | let n = key nm -> do
        v <- lookupName ctx env n >>= \cur -> case cur of
          VThunk _ -> lookupName ctx env n      -- forces the thunk in the current environment
          VEvent _ -> eval ctx env e            -- an event let: evaluate now
          _ -> eval ctx env e
        connectIf fwds n v
        let env' = M.insert n v env
        writeIORef envRef env'
        return (env', VUnit)
      SBind nm (EThen _ e) | let n = key nm -> do
        v <- eval ctx env e
        ev <- asEvent ("hold " ++ n) v
        case M.lookup n holds of
          Just (_, connect) -> connect ev
          Nothing -> return ()
        return (env, VUnit)
      SBind nm e | let n = key nm -> do
        v <- eval ctx env e
        r <- case v of
          VBlueprint bp -> buildChild bp sc
          _ -> return v
        connectIf fwds n r
        let env' = M.insert n r env
        writeIORef envRef env'
        return (env', VUnit)
      SBindPat p e -> do
        v <- eval ctx env e
        r <- case v of
          VBlueprint bp -> buildChild bp sc
          _ -> return v
        case matchPat ctx p r of
          Just binds -> do
            forM_ binds $ \(n, x) -> connectIf fwds n x
            let env' = M.union (M.fromList binds) env
            writeIORef envRef env'
            return (env', VUnit)
          Nothing -> prismError "Build" ("bind pattern does not match " ++ showValue r)
      SExpr e -> do
        v <- eval ctx env e
        r <- case v of
          VBlueprint bp -> buildChild bp sc
          _ -> return v
        return (env, r)
    connectIf fwds n v = case M.lookup n fwds of
      Just connect -> connect v
      Nothing -> return ()

-- | A forward dynamic: a dynamic bound later in the block (read through a tag until then).
-- Connecting it copies the real dynamic's committed value and forwards its change event.
newForwardDyn :: RT -> String -> IO (Dyn, Value -> IO ())
newForwardDyn rt n = do
  (fwd, connectEv) <- newForward rt ("fwd " ++ n)
  d <- mkDyn rt (VCons "<unbound>" []) fwd
  let connect v = case v of
        VDyn real -> do
          beforeOf real >>= writeIORef (dynCur d)
          connectEv (dynChange real)
        _ -> return ()
  return (d, connect)

-- | Build a blueprint as a child of a scope. Every widget bind takes a position (chapter 2's
-- conventions): a widget block and `@started` take it here and a block numbers its own binds
-- under it; `rebuild`, `for` and `@Element` take theirs inside their build, so a bind of one of
-- those is one segment, not two.
buildChild :: Blueprint -> Scope -> IO Value
buildChild bp sc
  | bpOwnPos bp = do
      pos <- nextPos sc
      sub <- newScope (scPath sc ++ [SPos pos])
      ref <- newIORef (Just sub)
      addChild sc (CScope ref)
      bpBuild bp sub
  | otherwise = bpBuild bp sc

buildRebuild :: Ctx -> Env -> Scope -> Expr -> Block -> IO Value
buildRebuild ctx env sc dExpr stmts = do
  dv <- eval ctx env dExpr
  d <- asDyn ctx dv
  VEvent <$> rebuildOver ctx sc d ( \gsc v -> buildBlock ctx (M.insert "_" v env) gsc stmts)

-- | The rebuild machinery: a node over a dynamic that builds a generation when it first runs
-- (in height order, so after the dynamic's change in the build occurrence, once) and again on
-- every change. Returns the event of the generations' results.
rebuildOver :: Ctx -> Scope -> Dyn -> (Scope -> Value -> IO Value) -> IO Ev
rebuildOver ctx sc d buildGen = do
  let rt = cRT ctx
  pos <- nextPos sc
  genRef <- newIORef (0 :: Int)
  scopeRef <- newIORef Nothing
  addChild sc (CScope scopeRef)
  resultEv <- newEv rt "rebuild" (return [dynChange d])
  let build v = do
        g <- atomicModifyIORef' genRef (\n -> (n + 1, n))
        gsc <- newScope (scPath sc ++ [SPos pos, SGen g])
        writeIORef scopeRef (Just gsc)
        buildGen gsc v
  node <- newNode rt resultEv $ do
    v <- nowOf rt d
    Just <$> build v
  subscribe rt (dynChange d) node
  schedule rt node                       -- the first build, in height order
  return resultEv

buildFor :: Ctx -> Env -> Scope -> Maybe (Name, Name) -> Expr -> Block -> IO Value
buildFor ctx env sc binders mExpr stmts = do
  let rt = cRT ctx
  mv0 <- eval ctx env mExpr
  -- an array is a map keyed by index (second class, as the book says)
  mv <- lifted ctx "for-source" [mv0] $ \[v] -> return $ case v of
    VArray xs -> VMap (M.fromList (zip (map VInt [0 ..]) xs))
    other -> other
  md <- asDyn ctx mv
  pos <- nextPos sc
  rowsRef <- newIORef M.empty          -- key -> (scope, result)
  scopesRef <- newIORef M.empty
  addChild sc (CKeyed scopesRef)
  -- consolidated event: fires with the map of the rows' firings
  consEv <- newEv rt "for" (do rows <- readIORef rowsRef; return [e | (_, VEvent e) <- M.toList rows])
  consNode <- newNode rt consEv $ do
    rows <- readIORef rowsRef
    fired <- forM (M.toList rows) $ \(k, r) -> case r of
      VEvent e -> fmap ((,) k) <$> firingOf rt e
      _ -> return Nothing
    let m = M.fromList [kv | Just kv <- fired]
    return (if M.null m then Nothing else Just (VMap m))
  -- the for node's own event: fires after the rows have been brought up to date
  forEv <- newEv rt "for-rows" (return [dynChange md])
  -- consolidated dynamic: the map of the rows' current values (for dynamic and plain results);
  -- it runs after the for node, so births are in it
  consDynEv <- newEv rt "for-dyn" (do rows <- readIORef rowsRef; return (forEv : [dynChange d | (_, VDyn d) <- M.toList rows]))
  consDynNode <- newNode rt consDynEv $ do
    rows <- readIORef rowsRef
    vals <- forM (M.toList rows) $ \(k, r) -> case r of
      VDyn d -> (,) k <$> nowOf rt d
      v -> return (k, v)
    return (Just (VMap (M.fromList vals)))
  let (kName, vName) = case binders of
        Just (k, v) -> (key k, key v)
        Nothing -> ("key", "$_")
      buildRow k = do
        rsc <- newScope (scPath sc ++ [SPos pos, SKey k])
        modifyIORef' scopesRef (M.insert k rsc)
        entry <- derivedDyn rt "entry" [md] $ \[VMap m] -> return (fromMaybe VUnit (M.lookup k m))
        let env' = M.insert kName k (M.insert vName (VDyn entry) env)
        r <- buildBlock ctx env' rsc stmts
        modifyIORef' rowsRef (M.insert k r)
        case r of
          VEvent e -> subscribe rt e consNode
          VDyn d -> subscribe rt (dynChange d) consDynNode
          _ -> return ()
      dropRow k = do
        rows <- readIORef rowsRef
        case M.lookup k rows of
          Just (VEvent e) -> unsubscribe e consNode
          Just (VDyn d) -> unsubscribe (dynChange d) consDynNode
          _ -> return ()
        modifyIORef' rowsRef (M.delete k)
        modifyIORef' scopesRef (M.delete k)
  forNode <- newNode rt forEv $ do
    VMap m <- nowOf rt md
    scopes <- readIORef scopesRef
    let new = M.keys (m `M.difference` scopes)
        gone = M.keys (scopes `M.difference` m)
    mapM_ buildRow new
    unless (null gone) $ commitLater rt (mapM_ dropRow gone)
    return (Just VUnit)
  subscribe rt (dynChange md) forNode
  schedule rt forNode                      -- the first rows, in height order
  subscribe rt forEv consDynNode
  -- the result kind is read off the block's last statement (the rows are built later)
  case guessKind stmts of
    "event" -> return (VEvent consEv)
    _ -> VDyn <$> mkDyn rt (VMap M.empty) consDynEv      -- dynamics and plain values: a dynamic map
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
buildElement :: Ctx -> Env -> Scope -> Expr -> IO Value
buildElement ctx env sc e = do
  let rt = cRT ctx
  (plainExpr, requestsExpr) <- case e of
    ECons n fields ->
      let reqs = [v | (Just "requests", v) <- fields]
          others = [fv | fv@(f, _) <- fields, f /= Just "requests"]
      in return (ECons n others, case reqs of { (r:_) -> Just r; [] -> Nothing })
    _ -> return (e, Nothing)
  ev <- eval ctx env plainExpr
  reqV <- mapM (eval ctx env) requestsExpr
  reqE <- mapM (asEvent "requests") reqV
  case ev of
    VDyn d -> do
      -- the shorthand: a fresh slot per change
      resultEv <- rebuildOver ctx sc d $ \gsc v -> makeSlot ctx gsc v reqE
      v0 <- beforeOf d
      let tag = case v0 of { VRec n _ -> n; VCons n _ -> n; _ -> "" }
      case M.lookup tag (cTags ctx) of
        Just td | tdResponds td -> VEvent <$> switchE rt resultEv
                | tdState td -> do
                    -- a hold of the generations' state dynamics, joined; the first generation
                    -- is built when the rebuild node runs, so the hold starts from a placeholder
                    -- that the first firing replaces
                    outer <- holdFromEvent rt resultEv
                    VDyn <$> joinD rt outer
        _ -> return VUnit
    v -> makeSlot ctx sc v reqE

-- | A dynamic holding the last firing of an event of dynamics, starting from an empty constant.
holdFromEvent :: RT -> Ev -> IO Dyn
holdFromEvent rt ev = do
  placeholder <- constDyn rt VUnit
  mkDyn rt (VDyn placeholder) ev

-- | Create one slot for a plain element value.
makeSlot :: Ctx -> Scope -> Value -> Maybe Ev -> IO Value
makeSlot ctx sc v reqE = do
  let rt = cRT ctx
      tag = case v of { VRec n _ -> n; VCons n _ -> n; _ -> "" }
      v' = case v of { VCons n [] -> VRec n []; _ -> v }
  pos <- nextPos sc
  let path = scPath sc ++ [SPos pos]
  td <- case M.lookup tag (cTags ctx) of
    Just t -> return t
    Nothing -> return (TagDecl tag [] False False False)
  responds <- if tdResponds td then Just <$> newExternal rt ("responds " ++ showPath path) else return Nothing
  (state, push) <- if tdState td
    then do
      o <- cOracle ctx v'
      case o of
        Nothing -> prismError "UnsuppliedState" ("element=" ++ showValue v')
        Just s -> do
          p <- newExternal rt ("push " ++ showPath path)
          d <- mkDyn rt s p
          return (Just d, Just p)
    else return (Nothing, Nothing)
  addChild sc (CSlot (Slot path v' responds reqE state push))
  return $ case (responds, state) of
    (Just e, Just d) -> VTuple [VEvent e, VDyn d]
    (Just e, _) -> VEvent e
    (_, Just d) -> VDyn d
    _ -> VUnit
