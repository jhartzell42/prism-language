{-# LANGUAGE RecursiveDo, ScopedTypeVariables, GADTs, FlexibleContexts #-}
-- | Two claims chapter 14 makes about Reflex, run against reflex 0.9.4.1, plus the control
-- that the port relies on. The first two fail; that is the point of the suite. Run with
-- `cabal test chapter14-claims`; `test.sh` does not run it.
--
--   1. "PerformEventT is in the stack for its Adjustable instance", and a block built by
--      runWithReplace in occurrence n sees n. Measured: the block is built in a follow-up
--      frame and sees nothing of n.
--   2. "If a prompt cycle were built, Spider throws an EventLoopException". Measured: Spider
--      recurses while subscribing until the stack is exhausted (the suite runs with -K16m).
--   3. Control: the same law as 1 with the block built inside a push closure, one frame.
module Main where

import Reflex
import Reflex.Spider
import Reflex.Host.Class
import Reflex.PerformEvent.Base (hostPerformEventT, FireCommand(..))
import Reflex.Spider.Internal (EventLoopException)
import Control.Exception
import Control.Monad
import Control.Monad.IO.Class
import Data.Dependent.Sum (DSum(..))
import Data.Functor.Identity
import Data.IORef
import System.Exit

type T = SpiderTimeline Global

-- | The law under test, as the laws10 script a_new_block_sees_the_event_that_made_it has it:
-- a click adds a key; the key's block holds `$hits <- 0 then $hits + 1 on !click`; after the
-- click the new block's hits is 1, and the turn was one occurrence.
claim1 :: IO (Either String ())
claim1 = runSpiderHost $ do
  (clickE, clickRef) <- newEventWithTriggerRef
  (newbornD, FireCommand fire) <- hostPerformEventT $ do
    keys <- foldDyn (+) (0 :: Int) (1 <$ clickE)
    (_, newbornE) <- runWithReplace (return ()) $ ffor (updated keys) $ \_ ->
      foldDyn (+) (0 :: Int) (1 <$ clickE)                 -- the newborn's hold over the click
    holdDyn Nothing (Just <$> newbornE)
  Just t <- liftIO (readIORef clickRef)
  frames <- fire [t :=> Identity ()] (return ())           -- one run call: the click
  mHits <- sample (current newbornD)
  hits <- maybe (return Nothing) (fmap Just . sample . current) mHits
  return $ case (length frames, hits) of
    (1, Just 1) -> Right ()
    (n, h) -> Left ("the run call took " ++ show n ++ " frames and the newborn's hits is " ++ show h
                    ++ " (built in a follow-up frame, after the click had passed)")

-- | A prompt cycle: `!out <- @Echo !out` with Echo's body `!in + 1 | 0 on !started`.
claim2 :: IO (Either String ())
claim2 = do
  r <- try $ runSpiderHost $ do
    (buildE, buildRef) <- newEventWithTriggerRef
    let rootE = flip push buildE $ \() -> mdo
          nowE <- now
          let fwd = switch (constant real)                 -- the forward reference, as the port spells it
              real = leftmost [fmap (+ 1) fwd, 0 <$ nowE] :: Event T Int
          Just <$> holdDyn (-1) real                       -- subscribing this is what loops
    h <- holdDyn (constDyn (-1)) rootE
    Just t <- liftIO (readIORef buildRef)
    fireEventsAndRead [t :=> Identity ()] (return ())
    sample (current (join h))
  return $ case r of
    Left (e :: SomeException)
      | Just (_ :: EventLoopException) <- fromException e -> Right ()
      | otherwise -> Left ("Spider raised " ++ takeWhile (/= '\n') (show e) ++ ", not EventLoopException")
    Right v -> Left ("no exception; the hold read " ++ show v)

-- | Control: the law of claim 1 with the block built inside the push closure.
control :: IO (Either String ())
control = runSpiderHost $ do
  (clickE, clickRef) <- newEventWithTriggerRef
  keys <- foldDyn (+) (0 :: Int) (1 <$ clickE)
  let newbornE = flip push (updated keys) $ \_ -> Just <$> foldDyn (+) (0 :: Int) (1 <$ clickE)
  newbornD <- holdDyn Nothing (Just <$> newbornE)
  Just t <- liftIO (readIORef clickRef)
  fireEventsAndRead [t :=> Identity ()] (return ())        -- one frame by construction
  mHits <- sample (current newbornD)
  hits <- maybe (return Nothing) (fmap Just . sample . current) mHits
  return $ case hits of
    Just 1 -> Right ()
    h -> Left ("the newborn's hits is " ++ show h)

main :: IO ()
main = do
  results <- forM
    [ ("claim 1: runWithReplace under PerformEventT builds in the frame of the change", claim1)
    , ("claim 2: Spider throws EventLoopException on a prompt cycle", claim2)
    , ("control: a block built inside push sees the frame of the change", control) ]
    $ \(name, act) -> do
      r <- act
      putStrLn $ case r of
        Right () -> "PASS " ++ name
        Left why -> "FAIL " ++ name ++ "\n     " ++ why
      return r
  let failed = length [() | Left _ <- results]
  putStrLn (show (length results - failed) ++ " passed, " ++ show failed ++ " failed")
  when (failed > 0) exitFailure
