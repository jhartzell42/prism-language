-- | The Test World: runs a test's turns against a blueprint, prints the transcript, checks.
module Prism.World where

import Prism.Syntax
import Prism.Value
import Prism.Runtime
import Prism.Eval
import Data.IORef
import qualified Data.Map.Strict as M
import Data.List (intercalate)
import Data.Maybe (catMaybes)
import Control.Monad
import Control.Exception

data TurnRecord = TurnRecord
  { trIndex     :: Int
  , trResponses :: [([Seg], Value)]
  , trPushes    :: [(Value, Value)]
  , trList      :: [Slot]
  , trRequests  :: M.Map String [Value]        -- by path spelling
  , trResult    :: Maybe (Either (Maybe Value) Value)   -- Left: event (fired/quiet); Right: value
  , trError     :: Maybe PrismError
  , trDeparted  :: [Slot]                       -- slots torn down in the turn that fired a request
  }

data TestOutcome = TestOutcome
  { toName       :: String
  , toTranscript :: [String]
  , toVerdict    :: Either String ()
  }

-- | Build the top-level environment: tags, types, functions and blueprints.
topLevel :: Program -> IO (M.Map String TagDecl, M.Map String TypeDecl, RT -> IORef (M.Map Value Value) -> IO (Ctx, Env))
topLevel prog = do
  let tags = M.fromList [(tdName t, t) | ITag t <- prog]
      types = M.fromList [(n, t) | IType t <- prog, let n = typeName t]
      typeName (TDRecord n _) = n
      typeName (TDChoice n _) = n
      typeName (TDOther n) = n
      mk rt snapRef = do
        let ctx = Ctx rt tags types (\e -> M.lookup e <$> readIORef snapRef)
        -- every top-level definition is a thunk over the final environment, so functions can
        -- refer to each other and to themselves in any order
        envRef <- newIORef M.empty
        entries <- forM prog $ \item -> case item of
          IFun (Name s n) e -> do
            ref <- newIORef (Left (readIORef envRef >>= \env -> eval ctx env e))
            return [(s ++ n, VThunk ref), (n, VThunk ref)]
          IBlueprint (Name s n) stmts -> do
            ref <- newIORef (Left (readIORef envRef >>= \env -> return (VBlueprint (Blueprint (\sc -> buildBlock ctx env sc stmts) False))))
            return [(s ++ n, VThunk ref), (n, VThunk ref)]
          _ -> return []
        let env = M.fromList (concat entries)
        writeIORef envRef env
        return (ctx, env)
  return (tags, types, mk)

runTest :: Program -> Item -> IO TestOutcome
runTest prog (ITest name wExpr turns) = do
  (_, _, mk) <- topLevel prog
  rt <- newRT
  snapRef <- newIORef M.empty
  (ctx, env) <- mk rt snapRef
  root <- newScope []
  resultRef <- newIORef VUnit
  wv <- eval ctx env wExpr
  bp <- case wv of
    VBlueprint b -> return b
    _ -> throwIO (PrismError "Build" "the test's subject is not a blueprint")
  let loop _ [] acc = return (reverse acc, Right ())
      loop i (t : rest) acc = do
        r <- try (runTurn ctx env rt root resultRef snapRef bp i t)
        case r of
          Left (PrismError "Script" d) -> return (reverse acc, Left ("turn " ++ show i ++ ": script error: " ++ d))
          Left e -> throwIO e
          Right rec -> step i t rest acc rec
      step i t rest acc rec = do
        let acc' = rec : acc
        case checkTurn ctx env rec t of
          Left err -> return (reverse acc', Left ("turn " ++ show i ++ ": " ++ err))
          Right () -> case (trError rec, any isErrorsCheck (tChecks t)) of
            (Just e, False) -> return (reverse acc', Left ("turn " ++ show i ++ " ended in an error with no `errors` check: " ++ peKind e ++ " " ++ peDetail e))
            _ -> loop (i + 1) rest acc'
  (recs, verdict) <- loop 0 turns []
  return (TestOutcome name (concatMap printTurn recs) verdict)
  where
    isErrorsCheck (CErrors _) = True
    isErrorsCheck _ = False
runTest _ _ = error "not a test"

runTurn :: Ctx -> Env -> RT -> Scope -> IORef Value -> IORef (M.Map Value Value) -> Blueprint -> Int -> Turn -> IO TurnRecord
runTurn ctx env rt root resultRef snapRef bp i (Turn withs inputs _) = do
  list0 <- slotsOf root
  -- withs: snapshot entries for this turn; a changed entry is this turn's push for it
  withPushes <- fmap catMaybes $ forM withs $ \(p, e) -> do
    v <- eval ctx env e
    let k = patValue p
    old <- M.lookup k <$> readIORef snapRef
    modifyIORef' snapRef (M.insert k v)
    return (if old == Just v then Nothing else Just (k, v))
  -- resolve the inputs against the list as the turn opens
  resolved <- forM inputs $ \inp -> case inp of
    IBuild -> return (Right (Left ()))
    IIdle -> return (Right (Right Nothing))
    IClick p -> do
      s <- findOne ctx list0 p
      return (Right (Right (Just (s, VCons "click" []))))
    ISend p e -> do
      s <- findOne ctx list0 p
      v <- eval ctx env e
      return (Right (Right (Just (s, v))))
    IPush p e -> do
      v <- eval ctx env e
      let elemV = patValue p
      modifyIORef' snapRef (M.insert elemV v)
      return (Left (elemV, v))
  let isBuild = not (null [() | Right (Left ()) <- resolved])
      deliveries = [sv | Right (Right (Just sv)) <- resolved]
      pushes = withPushes ++ [ev | Left ev <- resolved]
  outcome <- try $ runOccurrence rt $ do
    when isBuild $ do
      r <- bpBuild bp root
      writeIORef resultRef r
    forM_ deliveries $ \(s, v) -> case slotResponds s of
      Just e -> fire rt e v
      Nothing -> throwIO (PrismError "Build" ("slot " ++ showPath (slotPath s) ++ " has no responds"))
    forM_ pushes $ \(elemV, v) ->
      forM_ list0 $ \s -> when (slotElement s == elemV) $
        case slotPush s of
          Just p -> fire rt p v
          Nothing -> return ()
  case outcome of
    Left err -> return (TurnRecord i [(slotPath s, v) | (s, v) <- deliveries] pushes list0 M.empty Nothing (Just err) [])
    Right () -> do
      list1 <- slotsOf root
      -- requests are collected from the slots that existed as the turn opened as well as those
      -- that exist as it closes: a slot torn down in the turn still fired what it fired
      let paths1 = map (showPath . slotPath) list1
          departed = [s | s <- list0, showPath (slotPath s) `notElem` paths1]
      reqs <- fmap (M.fromList . catMaybes) $ forM (list1 ++ departed) $ \s -> case slotRequests s of
        Just e -> do
          f <- firingOf rt e
          return (fmap (\v -> (showPath (slotPath s), [v])) f)
        Nothing -> return Nothing
      r <- readIORef resultRef
      res <- case r of
        VDyn d -> Right <$> beforeOf d
        VEvent e -> Left <$> firingOf rt e
        v -> return (Right v)
      return (TurnRecord i [(slotPath s, v) | (s, v) <- deliveries] pushes list1 reqs (Just res) Nothing [s | s <- departed, M.member (showPath (slotPath s)) reqs])

findOne :: Ctx -> [Slot] -> Pat -> IO Slot
findOne ctx list p = case filter (matches ctx p) list of
  [s] -> return s
  ss -> throwIO (PrismError "Script" ("pattern matches " ++ show (length ss) ++ " slots, expected 1"))

matches :: Ctx -> Pat -> Slot -> Bool
matches ctx p s = case matchPat ctx p (slotElement s) of
  Just _ -> True
  Nothing -> False

-- | A snapshot key from a pattern: the whole element value it names.
patValue :: Pat -> Value
patValue p = case p of
  PCons n [] -> VRec n []
  PCons n ps -> VCons n (map patValue ps)
  PRec n fs _ -> VRec n [(f, patValue q) | (f, q) <- fs]
  PLit l -> litValue l
  PText s -> VText s
  PTuple ps -> VTuple (map patValue ps)
  _ -> VUnit

-- ---------------------------------------------------------------------------
-- Checks

checkTurn :: Ctx -> Env -> TurnRecord -> Turn -> Either String ()
checkTurn ctx env rec (Turn _ _ checks) = mapM_ one checks
  where
    list = trList rec
    one c = case c of
      CExpect p -> require (any (matches ctx p) list) ("expect: nothing matches " ++ show p)
      CExpectTop p -> require (any (matches ctx p) list) ("expect_top: nothing matches " ++ show p)
      CExpectNone p -> require (not (any (matches ctx p) list)) ("expect_none: something matches " ++ show p)
      CExpectTopNone p -> require (not (any (matches ctx p) list)) ("expect_top_none: something matches " ++ show p)
      CExpectCount p n -> let k = length (filter (matches ctx p) list) in
        require (fromIntegral k == n) ("expect_count: " ++ show k ++ " match, expected " ++ show n)
      CReturns p -> case trResult rec of
        Just (Right v) -> require (matchesV p v) ("returns: result is " ++ showValue v)
        Just (Left _) -> Left "returns: the result is an event (use fired/quiet)"
        Nothing -> Left "returns: no result (the turn failed)"
      CFired p -> case trResult rec of
        Just (Left (Just v)) -> require (matchesV p v) ("fired: fired with " ++ showValue v)
        Just (Left Nothing) -> Left "fired: the result event was quiet"
        _ -> Left "fired: the result is not an event"
      CQuiet -> case trResult rec of
        Just (Left Nothing) -> Right ()
        Just (Left (Just v)) -> Left ("quiet: fired with " ++ showValue v)
        _ -> Left "quiet: the result is not an event"
      CRequested p vp ->
        let hits = [v | s <- list ++ trDeparted rec, matches ctx p s, Just vs <- [M.lookup (showPath (slotPath s)) (trRequests rec)], v <- vs, matchesV vp v]
        in require (not (null hits)) ("requested: no matching slot fired a matching request")
      CUnrequested p ->
        let hits = [() | s <- list, matches ctx p s, M.member (showPath (slotPath s)) (trRequests rec)]
        in require (null hits) "unrequested: a matching slot fired its request"
      CErrors p -> case trError rec of
        Just (PrismError k d) -> require (matchesV p (VRec "Error" [("kind", VCons k []), ("detail", VText d)])) ("errors: the error was " ++ k ++ " " ++ d)
        Nothing -> Left "errors: the turn did not end in an error"
      CSettles -> case trError rec of
        Nothing -> Right ()
        Just (PrismError k d) -> Left ("settles: the turn ended in " ++ k ++ ": " ++ d)
    matchesV p v = case matchPat ctx p v of { Just _ -> True; Nothing -> False }
    require True _ = Right ()
    require False msg = Left msg

-- ---------------------------------------------------------------------------
-- Transcript

printTurn :: TurnRecord -> [String]
printTurn rec =
  [ "turn " ++ show (trIndex rec) ++ inputs ] ++
  case trError rec of
    Just (PrismError k d) -> ["  error: Error(kind=" ++ k ++ ", detail=" ++ show d ++ ")"]
    Nothing ->
      [ "  list:" ] ++
      [ "    " ++ showPath (slotPath s) ++ ": " ++ showValue (slotElement s) ++ reqs s | s <- trList rec ] ++
      [ "    " ++ showPath (slotPath s) ++ ": " ++ showValue (slotElement s) ++ reqs s ++ " (torn down)" | s <- trDeparted rec ] ++
      [ "  result: " ++ result ]
  where
    inputs =
      (if null (trResponses rec) then "" else
        "  responses: {" ++ intercalate ", " [showPath p ++ ": " ++ showValue v | (p, v) <- trResponses rec] ++ "}") ++
      (if null (trPushes rec) then "" else
        "  pushes: {" ++ intercalate ", " [showValue e ++ ": " ++ showValue v | (e, v) <- trPushes rec] ++ "}")
    reqs s = case M.lookup (showPath (slotPath s)) (trRequests rec) of
      Just vs -> " requested: " ++ intercalate "; " (map showValue vs)
      Nothing -> ""
    result = case trResult rec of
      Just (Right v) -> showValue v
      Just (Left (Just v)) -> "fired: " ++ showValue v
      Just (Left Nothing) -> "quiet"
      Nothing -> "?"
