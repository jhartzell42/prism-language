module Main where

import Prism.Test
import System.Environment
import System.IO

main :: IO ()
main = do
  hSetBuffering stdout LineBuffering
  args <- getArgs
  case args of
    ("-v" : files) -> runFile True files
    files@(_:_) -> runFile False files
    _ -> putStrLn "usage: prism-hs [-v] prelude.prism file.prism ..."
