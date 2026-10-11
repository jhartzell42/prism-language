module Main where

import Prism.Test
import System.Environment

main :: IO ()
main = do
  args <- getArgs
  case args of
    ("-v" : files) -> runFile True files
    files@(_:_) -> runFile False files
    _ -> putStrLn "usage: prism-hs [-v] prelude.prism file.prism ..."
