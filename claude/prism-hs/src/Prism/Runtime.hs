-- | The push graph: events, dynamics, the height-ordered scheduler, holds that commit at the
-- end of an occurrence, forwards for cyclic bindings, switch and join.
module Prism.Runtime where

import Prism.Value
import Data.IORef
import qualified Data.Map.Strict as M
import qualified Data.Set as S
import Control.Monad
import Control.Exception
import System.Environment (lookupEnv)

data PrismError = PrismError { peKind :: String, peDetail :: String }
  deriving Show
instance Exception PrismError

data RT = RT
  { rtIds     :: IORef Int
  , rtOcc     :: IORef Int                 -- current occurrence id
  , rtQueue   :: IORef (M.Map Int [Node])
  , rtHeight  :: IORef (Maybe Int)         -- height being run, Nothing between occurrences
  , rtCommits :: IORef [IO ()]             -- run at the end of the occurrence, in order
  , rtPending :: IORef [(Ev, Value)]        -- fires requested while not propagating (a build)
  }

newRT :: IO RT
newRT = RT <$> newIORef 0 <*> newIORef 0 <*> newIORef M.empty <*> newIORef Nothing <*> newIORef [] <*> newIORef []

propagating :: RT -> IO Bool
propagating rt = maybe False (const True) <$> readIORef (rtHeight rt)

freshId :: RT -> IO Int
freshId rt = atomicModifyIORef' (rtIds rt) (\n -> (n + 1, n))

currentOcc :: RT -> IO Int
currentOcc rt = readIORef (rtOcc rt)

-- ---------------------------------------------------------------------------
-- Events

newEv :: RT -> String -> IO [Ev] -> IO Ev
newEv rt label sources = do
  i <- freshId rt
  Ev i <$> newIORef [] <*> newIORef Nothing <*> newIORef sources <*> pure label

newExternal :: RT -> String -> IO Ev
newExternal rt label = newEv rt label (return [])

-- | The value an event fired with in the current occurrence, if any.
firingOf :: RT -> Ev -> IO (Maybe Value)
firingOf rt ev = do
  occ <- currentOcc rt
  f <- readIORef (evFiring ev)
  return $ case f of
    Just (o, v) | o == occ -> Just v
    _ -> Nothing

-- | Height: 1 + the max of the prompt sources; an external event is 0. Cycles are an error.
heightOf :: RT -> Ev -> IO Int
heightOf rt = go S.empty
  where
    go seen ev
      | evId ev `S.member` seen = throwIO (PrismError "Cycle" ("through " ++ evLabel ev))
      | otherwise = do
          srcs <- join (readIORef (evSources ev))
          if null srcs then return 0 else do
            hs <- mapM (go (S.insert (evId ev) seen)) srcs
            return (1 + maximum hs)

subscribe :: RT -> Ev -> Node -> IO ()
subscribe rt ev node = do
  modifyIORef' (evSubs ev) (node :)
  -- catch up: the event may already have fired in this occurrence
  f <- firingOf rt ev
  case f of
    Just _ -> schedule rt node
    Nothing -> return ()

unsubscribe :: Ev -> Node -> IO ()
unsubscribe ev node = modifyIORef' (evSubs ev) (filter ((/= nodeId node) . nodeId))

-- | Fire an event. While nothing is propagating (a build, or the World delivering inputs), the
-- firing waits until propagation starts, so that everything built meanwhile is subscribed.
fire :: RT -> Ev -> Value -> IO ()
fire rt ev v = do
  p <- propagating rt
  if p then fireNow rt ev v else modifyIORef' (rtPending rt) ((ev, v) :)

fireNow :: RT -> Ev -> Value -> IO ()
fireNow rt ev v = do
  occ <- currentOcc rt
  tr <- lookupEnv "PRISM_TRACE"
  when (tr == Just "1") $ putStrLn ("  fire " ++ evLabel ev ++ "#" ++ show (evId ev) ++ " = " ++ showValue v)
  writeIORef (evFiring ev) (Just (occ, v))
  subs <- readIORef (evSubs ev)
  mapM_ (schedule rt) (reverse subs)

-- | Put a node on the queue at its height; if the scheduler is already past that height
-- (a node built in the middle of an occurrence), run it now.
schedule :: RT -> Node -> IO ()
schedule rt node = do
  occ <- currentOcc rt
  ran <- readIORef (nodeRan node)
  when (ran /= occ) $ do
    h <- heightOf rt (nodeEvent node)
    cur <- readIORef (rtHeight rt)
    case cur of
      Just c | h <= c -> runNode rt node
      _ -> modifyIORef' (rtQueue rt) (M.insertWith (flip (++)) h [node])

runNode :: RT -> Node -> IO ()
runNode rt node = do
  occ <- currentOcc rt
  ran <- readIORef (nodeRan node)
  when (ran /= occ) $ do
    writeIORef (nodeRan node) occ
    r <- nodeCompute node
    case r of
      Just v -> fire rt (nodeEvent node) v
      Nothing -> return ()

newNode :: RT -> Ev -> IO (Maybe Value) -> IO Node
newNode rt ev compute = do
  i <- freshId rt
  Node i ev compute <$> newIORef (-1)

-- | A derived event: a node subscribed to its sources, computing from their firings.
derived :: RT -> String -> [Ev] -> IO (Maybe Value) -> IO Ev
derived rt label sources compute = do
  ev <- newEv rt label (return sources)
  node <- newNode rt ev compute
  mapM_ (\s -> subscribe rt s node) sources
  return ev

-- | A watcher runs when its sources fire and fires nothing itself (commits, teardowns).
watcher :: RT -> String -> [Ev] -> IO () -> IO ()
watcher rt label sources action = do
  ev <- newEv rt label (return sources)
  node <- newNode rt ev (action >> return Nothing)
  mapM_ (\s -> subscribe rt s node) sources

-- | A watcher for something built in the middle of an occurrence: if a source already fired in
-- this occurrence, the builder has seen it (it used the now-value), so the watcher does not run
-- again for it.
watcherFresh :: RT -> String -> [Ev] -> IO () -> IO ()
watcherFresh rt label sources action = do
  ev <- newEv rt label (return sources)
  node <- newNode rt ev (action >> return Nothing)
  fired <- mapM (firingOf rt) sources
  when (any (maybe False (const True)) fired) $ currentOcc rt >>= writeIORef (nodeRan node)
  mapM_ (\s -> subscribe rt s node) sources

commitLater :: RT -> IO () -> IO ()
commitLater rt act = modifyIORef' (rtCommits rt) (act :)

-- ---------------------------------------------------------------------------
-- Propagation

-- | Run one occurrence: the action fires inputs or builds; then everything propagates in
-- height order; then the commits run.
runOccurrence :: RT -> IO a -> IO a
runOccurrence rt action = do
  modifyIORef' (rtOcc rt) (+ 1)
  writeIORef (rtQueue rt) M.empty
  writeIORef (rtCommits rt) []
  writeIORef (rtPending rt) []
  r <- (do a <- action; startPropagation rt; propagate rt; return a) `onException` do
    writeIORef (rtQueue rt) M.empty
    writeIORef (rtCommits rt) []
    writeIORef (rtHeight rt) Nothing
  commits <- readIORef (rtCommits rt)
  writeIORef (rtCommits rt) []
  sequence_ (reverse commits)
  writeIORef (rtHeight rt) Nothing
  return r

-- | Apply the pending fires: the inputs and everything a build asked to fire.
startPropagation :: RT -> IO ()
startPropagation rt = do
  writeIORef (rtHeight rt) (Just (-1))
  pending <- readIORef (rtPending rt)
  writeIORef (rtPending rt) []
  forM_ (reverse pending) $ \(ev, v) -> fireNow rt ev v

propagate :: RT -> IO ()
propagate rt = do
  q <- readIORef (rtQueue rt)
  case M.minViewWithKey q of
    Nothing -> writeIORef (rtHeight rt) Nothing
    Just ((h, nodes), rest) -> do
      writeIORef (rtQueue rt) rest
      writeIORef (rtHeight rt) (Just h)
      mapM_ (runNode rt) nodes
      propagate rt

-- ---------------------------------------------------------------------------
-- Dynamics

-- | The value of a dynamic as this occurrence sees it after its change: the change's firing
-- if it fired, else the committed value.
nowOf :: RT -> Dyn -> IO Value
nowOf rt d = do
  f <- firingOf rt (dynChange d)
  case f of
    Just v -> return v
    Nothing -> readIORef (dynCur d)

-- | The value before the occurrence: what a tag reads.
beforeOf :: Dyn -> IO Value
beforeOf d = readIORef (dynCur d)

-- | Make a dynamic from a current value and a change event; its commit is registered here.
mkDyn :: RT -> Value -> Ev -> IO Dyn
mkDyn rt cur change = do
  i <- freshId rt
  ref <- newIORef cur
  let d = Dyn i ref change
  watcher rt ("commit " ++ evLabel change) [change] $ do
    f <- firingOf rt change
    case f of
      Just v -> commitLater rt (writeIORef ref v)
      Nothing -> return ()
  return d

constDyn :: RT -> Value -> IO Dyn
constDyn rt v = do
  never <- newExternal rt "never"
  i <- freshId rt
  ref <- newIORef v
  return (Dyn i ref never)

-- | A derived dynamic: f over the sources, promptly. Made in the middle of an occurrence in
-- which a source already changed, it is born with the new value and does not fire its change
-- again in that occurrence (a widget built in n sees all of n, once).
derivedDyn :: RT -> String -> [Dyn] -> ([Value] -> IO Value) -> IO Dyn
derivedDyn rt label ds f = do
  fired <- mapM (firingOf rt . dynChange) ds
  let mid = any (maybe False (const True)) fired
  cur <- if mid then mapM (nowOf rt) ds >>= f else mapM beforeOf ds >>= f
  ev <- newEv rt label (return (map dynChange ds))
  node <- newNode rt ev $ do
    nows <- mapM (nowOf rt) ds
    Just <$> f nows
  when mid $ currentOcc rt >>= writeIORef (nodeRan node)
  mapM_ (\d -> subscribe rt (dynChange d) node) ds
  mkDyn rt cur ev

-- | Lift f over one event and some dynamics read through a tag (before-values).
tagMap :: RT -> String -> Ev -> [Dyn] -> (Value -> [Value] -> IO Value) -> IO Ev
tagMap rt label ev ds f = derived rt label [ev] $ do
  m <- firingOf rt ev
  case m of
    Nothing -> return Nothing
    Just v -> do
      befores <- mapM beforeOf ds
      Just <$> f v befores

leftmost :: RT -> [Ev] -> IO Ev
leftmost rt evs = derived rt "leftmost" evs $ do
  fs <- mapM (firingOf rt) evs
  return (case [v | Just v <- fs] of { (v:_) -> Just v; [] -> Nothing })

-- | A forward: an event connected to its source later (cyclic bindings).
newForward :: RT -> String -> IO (Ev, Ev -> IO ())
newForward rt label = do
  ev <- newEv rt label (return [])
  let connect target = do
        writeIORef (evSources ev) (return [target])
        node <- newNode rt ev (firingOf rt target)
        subscribe rt target node
  return (ev, connect)

-- | !Switch: an event of events; listens to the inner chosen before the occurrence, and
-- adopts a new inner at commit.
switchE :: RT -> Ev -> IO Ev
switchE rt outer = do
  innerRef <- newIORef Nothing
  ev <- newEv rt "switch" (maybe [] (: []) <$> readIORef innerRef)
  node <- newNode rt ev $ do
    inner <- readIORef innerRef
    case inner of
      Nothing -> return Nothing
      Just e -> firingOf rt e
  watcher rt "switch-outer" [outer] $ do
    f <- firingOf rt outer
    case f of
      Just (VEvent e') -> commitLater rt $ do
        old <- readIORef innerRef
        maybe (return ()) (`unsubscribe` node) old
        writeIORef innerRef (Just e')
        modifyIORef' (evSubs e') (node :)
      _ -> return ()
  return ev

-- | $Join: a dynamic of dynamics, prompt on the outer and on the inner.
joinD :: RT -> Dyn -> IO Dyn
joinD rt outer = do
  -- built in the middle of an occurrence in which the outer already changed: born with the new
  -- inner, and the change does not fire again for it
  outerFired <- firingOf rt (dynChange outer)
  VDyn inner0 <- case outerFired of
    Just v -> return v
    Nothing -> beforeOf outer
  innerRef <- newIORef inner0
  cur <- case outerFired of
    Just _ -> nowOf rt inner0
    Nothing -> beforeOf inner0
  ev <- newEv rt "join" (do i <- readIORef innerRef; return [dynChange outer, dynChange i])
  node <- newNode rt ev $ do
    o <- firingOf rt (dynChange outer)
    case o of
      Just (VDyn i') -> Just <$> nowOf rt i'
      _ -> do
        i <- readIORef innerRef
        firingOf rt (dynChange i)
  case outerFired of
    Just _ -> currentOcc rt >>= writeIORef (nodeRan node)
    Nothing -> return ()
  subscribe rt (dynChange outer) node
  subscribe rt (dynChange inner0) node
  watcher rt "join-outer" [dynChange outer] $ do
    f <- firingOf rt (dynChange outer)
    case f of
      Just (VDyn i') -> commitLater rt $ do
        old <- readIORef innerRef
        unsubscribe (dynChange old) node
        writeIORef innerRef i'
        modifyIORef' (evSubs (dynChange i')) (node :)
      _ -> return ()
  mkDyn rt cur ev

-- | @started: fires once, in the occurrence it is made.
started :: RT -> IO Ev
started rt = do
  ev <- newExternal rt "started"
  fire rt ev VUnit
  return ev

-- | The change event of a dynamic (!Updated).
updatedE :: Dyn -> Ev
updatedE = dynChange
