{-# LANGUAGE RankNTypes, ScopedTypeVariables, GADTs, FlexibleContexts, TypeFamilies #-}
-- | The Test World: runs a test's turns against a blueprint on a Spider timeline of its own,
-- prints the transcript, checks. Each turn is one frame: responses by path and pushes by
-- element value go in as trigger firings, the request firings and the result come out of the
-- read phase, and the list is read after the frame.
module Prism.World where

import Reflex hiding (apply)
import Reflex.Spider
import Reflex.Host.Class
import Prism.Syntax
import Prism.Value
import Prism.Eval
import Data.IORef
import qualified Data.Map as M
import Data.List (intercalate)
import Data.Maybe (catMaybes, fromMaybe)
import Data.Functor.Identity
import Data.Dependent.Sum (DSum(..))
import Control.Monad
import Control.Exception
import Reflex.Spider.Internal (EventLoopException(..), HasSpiderTimeline, SpiderTimelineEnv)
import GHC.IO.Exception (FixIOException(..))

data TurnRecord t = TurnRecord
  { trIndex     :: Int
  , trResponses :: [(Path t, Value t)]
  , trPushes    :: [(Value t, Value t)]
  , trList      :: [Slot t]
  , trRequests  :: M.Map String [Value t]        -- by path spelling
  , trResult    :: Maybe (Either (Maybe (Value t)) (Value t))   -- Left: event (fired/quiet); Right: value
  , trError     :: Maybe PrismError
  , trDeparted  :: [Slot t]                       -- slots torn down in the turn that fired a request
  }

data TestOutcome = TestOutcome
  { toName       :: String
  , toTranscript :: [String]
  , toVerdict    :: Either String ()
  }

-- | The top-level environment: a lazy knot, so functions can refer to each other and to
-- themselves in any order.
topEnv :: Rx t => Ctx t -> Program -> Env t
topEnv ctx prog = env
  where
    env = M.fromList (concatMap item prog)
    item (IFun (Name s n) e) = let v = evalPure ctx env e in [(s ++ n, v), (n, v)]
    item (IBlueprint (Name s n) stmts) =
      let v = VBlueprint (Blueprint (\sc -> buildBlock ctx env sc stmts) False) in [(s ++ n, v), (n, v)]
    item _ = []

runTest :: Program -> Item -> IO TestOutcome
runTest prog t = withSpiderTimeline (runTestOn prog t)

-- | Everything the turn loop needs from the host side of one test.
data Host x = Host
  { hRun       :: forall a. SpiderHost x a -> IO a
  , hCtx       :: Ctx (SpiderTimeline x)
  , hEnv       :: Env (SpiderTimeline x)
  , hSnap      :: IORef (M.Map (Value (SpiderTimeline x)) (Value (SpiderTimeline x)))
  , hList      :: Dynamic (SpiderTimeline x) (ListMap (SpiderTimeline x))
  , hResult    :: IORef (Value (SpiderTimeline x))
  , hRespRef   :: IORef (Maybe (EventTrigger (SpiderTimeline x) (M.Map (Path (SpiderTimeline x)) (Value (SpiderTimeline x)))))
  , hPushRef   :: IORef (Maybe (EventTrigger (SpiderTimeline x) (M.Map (Value (SpiderTimeline x)) (Value (SpiderTimeline x)))))
  , hBuildRef  :: IORef (Maybe (EventTrigger (SpiderTimeline x) ()))
  , hReqH      :: EventHandle (SpiderTimeline x) (Requests (SpiderTimeline x))
  , hResH      :: EventHandle (SpiderTimeline x) (Value (SpiderTimeline x))
  , hRootH     :: EventHandle (SpiderTimeline x) (Value (SpiderTimeline x))
  }

runTestOn :: forall x. HasSpiderTimeline x => Program -> Item -> SpiderTimelineEnv x -> IO TestOutcome
runTestOn prog (ITest name wExpr turns) tl = do
  let host :: forall a. SpiderHost x a -> IO a
      host act = runSpiderHostForTimeline act tl
      tags = M.fromList [(tdName t, t) | ITag t <- prog]
      types = M.fromList [(typeName t, t) | IType t <- prog]
      typeName (TDRecord n _) = n
      typeName (TDChoice n _) = n
      typeName (TDOther n) = n
  snapRef <- newIORef M.empty
  cells <- newIORef M.empty
  (respE, respRef) <- host newEventWithTriggerRef
  (pushE, pushRef) <- host newEventWithTriggerRef
  (buildE, buildRef) <- host newEventWithTriggerRef
  let ctx = Ctx tags types (\e -> M.lookup e <$> readIORef snapRef) (fanMap respE) pushE cells
      env = topEnv ctx prog
      -- the root is a generation built when the build trigger fires: occurrence zero
      rootE :: Event (SpiderTimeline x) (Gen (SpiderTimeline x))
      rootE = flip push buildE $ \() -> do
        root <- newScope []
        wv <- eval atBuild ctx env wExpr
        r <- case wv of
          VBlueprint bp -> bpBuild bp root
          _ -> prismError "Build" "the test's subject is not a blueprint"
        (l, q) <- closeScope root
        return (Just (Gen r l q))
      resultEvent (VEvent e) = e
      resultEvent _ = never
  listH <- host (holdDyn (constDyn M.empty) (genList <$> rootE))
  reqH <- host (holdDyn never (genReqs <$> rootE))
  resH <- host (holdDyn never (resultEvent . genResult <$> rootE))
  let listD = join listH
      reqE = mergeWith (++) [switch (current reqH), registerBirth (genReqs <$> rootE)]
      resE = leftmost [switch (current resH), registerBirth (resultEvent . genResult <$> rootE)]
  reqHandle <- host (subscribeEvent reqE)
  resHandle <- host (subscribeEvent resE)
  rootHandle <- host (subscribeEvent (genResult <$> rootE))
  resultRef <- newIORef VUnit
  let h = Host host ctx env snapRef listD resultRef respRef pushRef buildRef reqHandle resHandle rootHandle
      loop _ [] acc = return (reverse acc, Right ())
      loop i (t : rest) acc = do
        r <- try (runTurn h i t)
        case r of
          Left (PrismError "Script" d) -> return (reverse acc, Left ("turn " ++ show i ++ ": script error: " ++ d))
          Left e -> throwIO e
          Right rec -> step i t rest acc rec
      step i t rest acc rec = do
        let acc' = rec : acc
        case checkTurn ctx rec t of
          Left err -> return (reverse acc', Left ("turn " ++ show i ++ ": " ++ err))
          Right () -> case (trError rec, any isErrorsCheck (tChecks t)) of
            (Just e, False) -> return (reverse acc', Left ("turn " ++ show i ++ " ended in an error with no `errors` check: " ++ peKind e ++ " " ++ peDetail e))
            _ -> loop (i + 1) rest acc'
  (recs, verdict) <- loop 0 turns []
  return (TestOutcome name (concatMap printTurn recs) verdict)
  where
    isErrorsCheck (CErrors _) = True
    isErrorsCheck _ = False
runTestOn _ _ _ = error "not a test"

runTurn :: forall x. HasSpiderTimeline x => Host x -> Int -> Turn -> IO (TurnRecord (SpiderTimeline x))
runTurn h i (Turn withs inputs _) = do
  let ctx = hCtx h
      env = hEnv h
      host :: forall a. SpiderHost x a -> IO a
      host = hRun h
  list0 <- M.elems <$> host (sample (current (hList h)))
  -- withs: snapshot entries for this turn; a changed entry is this turn's push for it
  withPushes <- fmap catMaybes $ forM withs $ \(p, e) -> do
    let v = evalPure ctx env e
        k = patValue p
    old <- M.lookup k <$> readIORef (hSnap h)
    modifyIORef' (hSnap h) (M.insert k v)
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
      return (Right (Right (Just (s, evalPure ctx env e))))
    IPush p e -> do
      let v = evalPure ctx env e
          elemV = patValue p
      modifyIORef' (hSnap h) (M.insert elemV v)
      return (Left (elemV, v))
  let isBuild = not (null [() | Right (Left ()) <- resolved])
      deliveries = [sv | Right (Right (Just sv)) <- resolved]
      pushes = withPushes ++ [ev | Left ev <- resolved]
      responses = [(slotPath s, v) | (s, v) <- deliveries]
      failed err = return (TurnRecord i responses pushes list0 M.empty Nothing (Just err) [])
  case [s | (s, _) <- deliveries, not (slotResponds s)] of
    (s : _) -> failed (PrismError "Build" ("slot " ++ showPath (slotPath s) ++ " has no responds"))
    [] -> do
      mBuild <- readIORef (hBuildRef h)
      mResp <- readIORef (hRespRef h)
      mPush <- readIORef (hPushRef h)
      let triggers = catMaybes
            [ if isBuild then fmap (\t -> t :=> Identity ()) mBuild else Nothing
            , if null deliveries then Nothing else fmap (\t -> t :=> Identity (M.fromList responses)) mResp
            , if null pushes then Nothing else fmap (\t -> t :=> Identity (M.fromList pushes)) mPush ]
      outcome <- try $ host $ fireEventsAndRead triggers $ do
        reqs <- readEvent (hReqH h) >>= sequence
        res <- readEvent (hResH h) >>= sequence
        root <- readEvent (hRootH h) >>= sequence
        return (fromMaybe [] reqs, res, root)
      case outcome of
        Left err -> failed (classify err)
        Right (reqs, resFired, root) -> do
          forM_ root (writeIORef (hResult h))
          list1 <- M.elems <$> host (sample (current (hList h)))
          r <- readIORef (hResult h)
          res <- case r of
            VDyn d -> Right <$> host (sample (current d))
            VEvent _ -> return (Left resFired)
            v -> return (Right v)
          let reqMap = M.fromListWith (flip (++)) [(showPath p, [v]) | (p, v) <- reqs]
              paths1 = map (showPath . slotPath) list1
              -- requests are collected from the slots that existed as the turn opened as well
              -- as those that exist as it closes: a slot torn down in the turn still fired
              departed = [s | s <- list0, showPath (slotPath s) `notElem` paths1]
          return (TurnRecord i responses pushes list1 reqMap (Just res) Nothing
                    [s | s <- departed, M.member (showPath (slotPath s)) reqMap])

-- | An exception that escaped the frame, as the error kind the transcript prints.
classify :: SomeException -> PrismError
classify e
  | Just pe <- fromException e = pe
  | Just (le :: EventLoopException) <- fromException e = PrismError "Cycle" (takeWhile (/= '\n') (show le))
  | Just StackOverflow <- fromException e = PrismError "Cycle" "Spider recursed without bound while subscribing: a prompt cycle"
  | Just FixIOException <- fromException e = PrismError "Cycle" "a binding was used at build time before its line"
  | Just BlockedIndefinitelyOnMVar <- fromException e = PrismError "Cycle" "a binding was used at build time before its line"
  | Just (ErrorCall m) <- fromException e = PrismError "Build" m
  | otherwise = PrismError "Build" (show e)

findOne :: Ctx t -> [Slot t] -> Pat -> IO (Slot t)
findOne ctx list p = case filter (matches ctx p) list of
  [s] -> return s
  ss -> throwIO (PrismError "Script" ("pattern matches " ++ show (length ss) ++ " slots, expected 1"))

matches :: Ctx t -> Pat -> Slot t -> Bool
matches ctx p s = case matchPat ctx p (slotElement s) of
  Just _ -> True
  Nothing -> False

-- | A snapshot key from a pattern: the whole element value it names.
patValue :: Pat -> Value t
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

checkTurn :: Ctx t -> TurnRecord t -> Turn -> Either String ()
checkTurn ctx rec (Turn _ _ checks) = mapM_ one checks
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
        in require (not (null hits)) "requested: no matching slot fired a matching request"
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

printTurn :: TurnRecord t -> [String]
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
