-- | Lexer and layout for the Prism subset: tokens plus the offside rule.
module Prism.Lexer (Tok(..), Token(..), lexProgram) where

import Data.Char
import Data.List (isPrefixOf)
import qualified Data.Map.Strict as M

data Tok
  = TName String String          -- sigil, base (base may be dotted: Ui.Button, Ui.*)
  | TNat Integer
  | TFloat Double
  | TText [Either String String] -- Left literal piece, Right expression source
  | TSym String                  -- operators and punctuation, including free sigils "!" "$" "@"
  | TKw String
  | TOpen                        -- layout block open
  | TClose                       -- layout block close
  | TSep                         -- statement separator (new line at block column)
  | TEOF
  deriving (Eq, Show)

data Token = Token { tokLine :: Int, tokCol :: Int, tok :: Tok }
  deriving (Show)

keywords :: [String]
keywords = ["let","type","tag","of","fn","match","if","else","for","rebuild","intercept","widget"
           ,"record","choice","map","array","unit","test","ignore","on","then","not","and","or"
           ,"key","empty","build","idle","click","send","push","with","expect","expect_top"
           ,"expect_none","expect_top_none","expect_count","returns","fired","quiet","requested"
           ,"unrequested","errors","settles"]

-- Test-step words are keywords only at the start of a turn line or after =>; the parser
-- decides, so here they are TKw and the parser accepts TKw as a name where a name is wanted.

isNameStart, isNameChar :: Char -> Bool
isNameStart c = isAlpha c || c == '_'
isNameChar c = isAlphaNum c || c == '_'

-- | Lex a whole program: tokens with layout tokens inserted.
lexProgram :: String -> Either String [Token]
lexProgram src = mergeDoubles <$> go (zip [1..] (lines src)) [0] [] False []
  where
    -- go lines stack bracketDepth pendingOpen acc
    go [] stack _ _ acc = Right (reverse (closes (length stack - 1) ++ acc) ++ [Token 0 0 TEOF])
      where closes n = replicate n (Token 0 0 TClose)
    go ((ln, line):rest) stack depth pendingOpen acc
      | blank line = go rest stack depth pendingOpen acc
      | not (null depth) = do
          (toks, depth', endsColon) <- lexLine ln line depth
          go rest stack depth' endsColon (reverse toks ++ acc)
      | otherwise = do
          let col = length (takeWhile (== ' ') line)
          (toks, depth', endsColon) <- lexLine ln line depth
          if pendingOpen
            then if col > head stack
                   then go rest (col : stack) depth' endsColon (reverse toks ++ (Token ln col TOpen : acc))
                   else Left ("line " ++ show ln ++ ": expected an indented block after ':'")
            else if col == head stack
              then go rest stack depth' endsColon (reverse toks ++ (Token ln col TSep : acc))
              else if col > head stack
                then go rest stack depth' endsColon (reverse toks ++ acc)   -- continuation
                else
                  let (popped, stack') = span (> col) stack
                  in if null stack' || head stack' /= col
                       then Left ("line " ++ show ln ++ ": unindent does not match any open block")
                       else go rest stack' depth' endsColon
                              (reverse toks ++ (Token ln col TSep : replicate (length popped) (Token ln col TClose) ++ acc))
    blank l = let s = dropWhile isSpace l in null s || "#" `isPrefixOf` s

-- | Lex one line. Returns tokens, bracket depth after the line, and whether the last token is a
-- block-opening colon (a ':' that is last on the line, outside brackets).
lexLine :: Int -> String -> [String] -> Either String ([Token], [String], Bool)
lexLine ln line depth0 = loop 1 line depth0 []
  where
    loop _ [] depth acc = done depth acc
    loop col s@(c:cs) depth acc
      | c == '#' = done depth acc
      | isSpace c = loop (col + 1) cs depth acc
      | c == '"' = do
          (parts, rest, used) <- lexText cs
          loop (col + 1 + used) rest depth (Token ln col (TText parts) : acc)
      | c == '\'' = do
          -- a char literal is a one-character text
          (ch, rest, used) <- lexChar cs
          loop (col + 1 + used) rest depth (Token ln col (TText [Left [ch]]) : acc)
      | isDigit c =
          let (num, rest) = span (\x -> isDigit x || x == '.' || x == '_') s
              num' = filter (/= '_') num
          in if '.' `elem` num' && not (".." `isPrefixOf` dropWhile isDigit num')
               then loop (col + length num) rest depth (Token ln col (TFloat (read num')) : acc)
               else let (ds, rest') = span isDigit s
                    in loop (col + length ds) rest' depth (Token ln col (TNat (read ds)) : acc)
      | c `elem` "!$@?" && sigilAttached s =
          let (sig, rest) = span (`elem` "!$@?") s
              (nm, rest') = lexName rest
          in loop (col + length sig + length nm) rest' depth (Token ln col (TName sig nm) : acc)
      | isNameStart c =
          let (nm, rest) = lexName s
              t = if nm `elem` keywords then TKw nm else TName "" nm
          in loop (col + length nm) rest depth (Token ln col t : acc)
      | otherwise =
          case symbol s of
            Just sym0 ->
              let sym = sym0
                  rest = drop (length sym) s
                  depth' = case sym of
                    "(" -> "(" : depth
                    "[" -> "[" : depth
                    "{" -> "{" : depth
                    ")" -> drop 1 depth
                    "]" -> drop 1 depth
                    "}" -> drop 1 depth
                    _ -> depth
              in loop (col + length sym) rest depth' (Token ln col (TSym sym) : acc)
            Nothing -> Left ("line " ++ show ln ++ ": unexpected character " ++ show c)
    done depth acc =
      let toks = reverse acc
          endsColon = null depth && not (null toks) && tok (last toks) == TSym ":"
      in Right (toks, depth, endsColon)

sigilAttached :: String -> Bool
sigilAttached s =
  let rest = dropWhile (`elem` "!$@?") s
  in case rest of
       (c:_) -> isNameStart c
       _ -> False

lexName :: String -> (String, String)
lexName s =
  let (a, rest) = span isNameChar s
  in if not (null a) && isUpper (head a)
       then dotted a rest
       else (a, rest)
  where
    dotted acc ('.':'*':rest) = (acc ++ ".*", rest)
    dotted acc ('.':c:rest) | isUpper c =
      let (b, rest') = span isNameChar (c:rest) in dotted (acc ++ "." ++ b) rest'
    dotted acc rest = (acc, rest)

symbols :: [String]
symbols = ["==>","=>","<-","->","==","!=","<=",">=","\\|","..",
           "<",">","=","|","(",")","[","]","{","}",",",".",":",";","+","-","*","/","\\","!","$","@","?"]

symbol :: String -> Maybe String
symbol s = case filter (`isPrefixOf` s) symbols of
  [] -> Nothing
  xs -> Just (longest xs)
  where longest = foldr1 (\a b -> if length a >= length b then a else b)

lexChar :: String -> Either String (Char, String, Int)
lexChar ('\\':c:'\'':rest) = Right (case c of { 'n' -> '\n'; 't' -> '\t'; x -> x }, rest, 3)
lexChar (c:'\'':rest) = Right (c, rest, 2)
lexChar _ = Left "bad char literal"

-- | Lex a text literal body after the opening quote. Returns parts, the rest of the line, and
-- the number of characters consumed (including the closing quote).
lexText :: String -> Either String ([Either String String], String, Int)
lexText = go "" [] 0
  where
    go lit parts n [] = Left "unterminated text literal"
    go lit parts n ('"':rest) = Right (reverse (flushLit lit parts), rest, n + 1)
    go lit parts n ('\\':c:rest) =
      let ch = case c of { 'n' -> '\n'; 't' -> '\t'; '{' -> '{'; '}' -> '}'; '"' -> '"'; '\\' -> '\\'; x -> x }
      in go (ch:lit) parts (n + 2) rest
    go lit parts n ('{':rest) =
      let (expr, rest') = matchBrace 0 "" rest
      in go "" (Right expr : flushLit lit parts) (n + 2 + length expr) rest'
    go lit parts n (c:rest) = go (c:lit) parts (n + 1) rest
    flushLit "" parts = parts
    flushLit lit parts = Left (reverse lit) : parts
    matchBrace :: Int -> String -> String -> (String, String)
    matchBrace _ acc [] = (reverse acc, [])
    matchBrace d acc ('}':rest) | d == 0 = (reverse acc, rest)
                                | otherwise = matchBrace (d - 1) ('}':acc) rest
    matchBrace d acc ('{':rest) = matchBrace (d + 1) ('{':acc) rest
    matchBrace d acc (c:rest) = matchBrace d (c:acc) rest

-- | Chapter 3's double-bracket rule: two opens written with nothing between them, whose
-- matching closes are also written with nothing between them, are one double bracket. Brackets
-- are paired first, outermost first; so `((Insert m, "c") on !add)` is two single parentheses
-- (the closes do not touch) and `((a, b))` is a tuple.
mergeDoubles :: [Token] -> [Token]
mergeDoubles toks = go 0 [] M.empty
  where
    arr = zip [0 :: Int ..] toks
    n = length toks
    at i = toks !! i
    isOpen t = tok t `elem` [TSym "(", TSym "["]
    isClose t = tok t `elem` [TSym ")", TSym "]"]
    touches a b = tokLine a == tokLine b && tokCol b == tokCol a + 1
    -- pair every bracket
    pairs = pairUp arr [] M.empty
    pairUp [] _ m = m
    pairUp ((i, t):rest) st m
      | isOpen t = pairUp rest (i : st) m
      | isClose t = case st of
          (o:st') -> pairUp rest st' (M.insert o i (M.insert i o m))
          [] -> pairUp rest st m
      | otherwise = pairUp rest st m
    -- a double is an open o with a touching open o+1 whose closes are c+1 and c, touching
    doubles = M.fromList
      [ (o, c) | (o, c) <- M.toList pairs, o < c, o + 1 < n, isOpen (at (o + 1)), touches (at o) (at (o + 1))
      , Just c' <- [M.lookup (o + 1) pairs], c' + 1 == c, touches (at c') (at c)
      , sameKind (at o) (at (o + 1)) ]
    sameKind a b = tok a == tok b
    dropped = M.fromList ([(o + 1, ()) | (o, _) <- M.toList doubles] ++ [(c - 1, ()) | (_, c) <- M.toList doubles])
    go i _ _ | i >= n = []
    go i st m
      | M.member i dropped = go (i + 1) st m
      | Just c <- M.lookup i doubles = retag (at i) True : go (i + 1) st m
      | i `elem` M.elems doubles = retag (at i) False : go (i + 1) st m
      | otherwise = at i : go (i + 1) st m
    retag t _ = case tok t of
      TSym "(" -> t { tok = TSym "((" }
      TSym ")" -> t { tok = TSym "))" }
      TSym "[" -> t { tok = TSym "[[" }
      TSym "]" -> t { tok = TSym "]]" }
      _ -> t
