-- | Running every test in a program and reporting.
module Prism.Test (runFile) where

import Prism.Syntax
import Prism.Parser
import Prism.World
import Control.Monad
import System.Exit

runFile :: Bool -> [FilePath] -> IO ()
runFile verbose paths = do
  srcs <- mapM readFile paths
  prog <- case mapM parseProgram srcs of
    Left err -> do putStrLn ("parse error: " ++ err); exitFailure
    Right ps -> return (concat ps)
  let tests = [t | t@(ITest {}) <- prog]
  results <- forM tests $ \t@(ITest name _ _) -> do
    o <- runTest prog t
    case toVerdict o of
      Right () -> putStrLn ("PASS " ++ name)
      Left err -> putStrLn ("FAIL " ++ name ++ ": " ++ err)
    when (verbose || either (const True) (const False) (toVerdict o)) $
      mapM_ (putStrLn . ("    " ++)) (toTranscript o)
    return (toVerdict o)
  let failed = length [() | Left _ <- results]
  putStrLn (show (length results - failed) ++ " passed, " ++ show failed ++ " failed")
  when (failed > 0) exitFailure
