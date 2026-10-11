-- | Recursive-descent parser over the layout token stream.
module Prism.Parser (parseProgram, parseExprSource) where

import Prism.Syntax
import Prism.Lexer
import Data.Char (isUpper, isLower)
import Control.Monad

-- parser state: the tokens, and whether application may take comma-separated arguments
-- (off while parsing a test line's values, where commas separate inputs)
data PS = PS { psToks :: [Token], psCommas :: Bool }

newtype P a = P { runP' :: PS -> Either String (a, PS) }

runP :: P a -> [Token] -> Either String (a, [Token])
runP (P p) ts = fmap (\(a, st) -> (a, psToks st)) (p (PS ts True))

instance Functor P where fmap f (P p) = P $ \st -> fmap (\(a, r) -> (f a, r)) (p st)
instance Applicative P where
  pure a = P $ \st -> Right (a, st)
  (<*>) = ap
instance Monad P where
  P p >>= f = P $ \st -> case p st of
    Left e -> Left e
    Right (a, r) -> runP' (f a) r

failP :: String -> P a
failP msg = P $ \st -> Left (msg ++ " at " ++ describe (psToks st))
  where describe (Token l c t : _) = "line " ++ show l ++ ":" ++ show c ++ " (" ++ show t ++ ")"
        describe [] = "end of input"

commasAllowed :: P Bool
commasAllowed = P $ \st -> Right (psCommas st, st)

withCommas :: Bool -> P a -> P a
withCommas b (P p) = P $ \st -> case p st { psCommas = b } of
  Left e -> Left e
  Right (a, st') -> Right (a, st' { psCommas = psCommas st })

peek :: P Tok
peek = P $ \st -> case psToks st of
  (t:_) -> Right (tok t, st)
  [] -> Right (TEOF, st)

peek2 :: P Tok
peek2 = P $ \st -> case psToks st of
  (_:t:_) -> Right (tok t, st)
  _ -> Right (TEOF, st)

peekTok :: P (Maybe Token)
peekTok = P $ \st -> case psToks st of
  (t:_) -> Right (Just t, st)
  [] -> Right (Nothing, st)

advance :: P Tok
advance = P $ \st -> case psToks st of
  (t:r) -> Right (tok t, st { psToks = r })
  [] -> Right (TEOF, st)

expect :: Tok -> P ()
expect t = do
  x <- peek
  if x == t then void advance else failP ("expected " ++ show t)

optional' :: Tok -> P Bool
optional' t = do
  x <- peek
  if x == t then advance >> return True else return False

skipSeps :: P ()
skipSeps = do
  x <- peek
  when (x == TSep) (advance >> skipSeps)

-- | Skip tokens until a token satisfying the predicate at bracket depth 0 (not consumed).
skipUntil :: (Tok -> Bool) -> P ()
skipUntil stop = go (0 :: Int)
  where
    go d = do
      x <- peek
      case x of
        TEOF -> return ()
        _ | d == 0 && stop x -> return ()
        TSym s | s `elem` ["(", "((", "[", "[[", "{"] -> advance >> go (d + 1)
               | s `elem` [")", "))", "]", "]]", "}"] -> advance >> go (max 0 (d - 1))
        TOpen -> advance >> go (d + 1)
        TClose -> if d == 0 then return () else advance >> go (d - 1)
        _ -> advance >> go d

-- ---------------------------------------------------------------------------
-- Program

parseProgram :: String -> Either String Program
parseProgram src = do
  toks <- lexProgram src
  (items, _) <- runP (skipSeps >> items) toks
  return items
  where
    items = do
      skipSeps
      x <- peek
      case x of
        TEOF -> return []
        _ -> do
          i <- item
          rest <- items
          return (i : rest)

item :: P Item
item = do
  x <- peek
  case x of
    TKw "type" -> advance >> typeDecl
    TKw "tag" -> advance >> tagDecl
    TKw "let" -> advance >> topLet
    TKw "test" -> advance >> testDecl
    TName "@" nm -> do
      advance
      expect (TSym ":")
      y <- peek
      case y of
        TOpen -> IBlueprint (Name "@" nm) <$> block
        _ -> do
          skipUntil (== TSym "=")
          expect (TSym "=")
          IFun (Name "@" nm) <$> expr
    _ -> failP "expected a top-level item"

topLet :: P Item
topLet = do
  n <- name
  y <- peek
  when (y == TSym ":") $ skipUntil (== TSym "=")
  expect (TSym "=")
  IFun n <$> expr

typeDecl :: P Item
typeDecl = do
  n <- plainName
  skipUntil (== TSym "=")
  expect (TSym "=")
  x <- peek
  case x of
    TKw "record" -> do
      advance
      expect (TSym ":")
      fs <- fieldBlock
      return (IType (TDRecord n (map fst fs)))
    TKw "choice" -> do
      advance
      y <- peek
      case y of
        TSym ":" -> do
          advance
          z <- peek
          case z of
            TOpen -> do
              advance
              cs <- consLines
              expect TClose
              return (IType (TDChoice n cs))
            _ -> do
              cs <- consInline
              return (IType (TDChoice n cs))
        TSym ".." -> advance >> return (IType (TDOther n))
        _ -> IType . TDChoice n <$> consInline
    _ -> do
      -- type Instant = Instant Int : a one-constructor type; record it as a choice
      y <- peek
      case y of
        TName "" c | isUpper (head c) -> do
          advance
          k <- countTypeArgs
          return (IType (TDChoice n [(c, k)]))
        _ -> skipUntil (\t -> t == TSep || t == TClose) >> return (IType (TDOther n))
  where
    consLines = do
      skipSeps
      x <- peek
      case x of
        TClose -> return []
        TName "" c -> do
          advance
          k <- countTypeArgs
          rest <- consLines
          return ((c, k) : rest)
        _ -> failP "expected a constructor"
    consInline = do
      c <- plainName
      k <- countTypeArgs
      more <- optional' (TSym "|")
      if more then ((c, k) :) <$> consInline else return [(c, k)]
    countTypeArgs = do
      -- count type arguments up to | , newline or close: each top-level type is one arg
      let go d k = do
            x <- peek
            case x of
              TSep -> return k
              TClose -> return k
              TEOF -> return k
              TSym "|" | d == 0 -> return k
              TSym "#" -> return k
              TSym s | s `elem` ["(", "((", "[", "[["] -> advance >> go (d + 1) k
                     | s `elem` [")", "))", "]", "]]"] -> advance >> go (d - 1) k
                     | s == "," && d == 0 -> advance >> go d k
              TName _ _ | d == 0 -> advance >> go d (k + 1)
              _ -> advance >> go d k
      go (0 :: Int) 0

fieldBlock :: P [(String, Bool)]
fieldBlock = do
  x <- peek
  case x of
    TOpen -> do
      advance
      fs <- fieldLines
      expect TClose
      return fs
    _ -> inlineFields
  where
    fieldLines = do
      skipSeps
      x <- peek
      case x of
        TClose -> return []
        _ -> do
          f <- field
          rest <- fieldLines
          return (f : rest)
    inlineFields = do
      f <- field
      more <- optional' (TSym ";")
      if more then (f :) <$> inlineFields else return [f]
    field = do
      x <- advance
      n <- case x of
        TName _ n -> return n
        TKw n -> return n
        _ -> failP "expected a field name"
      expect (TSym ":")
      skipUntil (\t -> t == TSep || t == TClose || t == TSym ";")
      return (n, True)

tagDecl :: P Item
tagDecl = do
  n <- plainName
  expect (TKw "of")
  _ <- plainName
  x <- peek
  if x == TSym ":"
    then do
      advance
      fs <- fieldBlock
      let names = map fst fs
          plain = filter (`notElem` ["requests", "responds", "state"]) names
      return (ITag (TagDecl n plain ("requests" `elem` names) ("responds" `elem` names) ("state" `elem` names)))
    else return (ITag (TagDecl n [] False False False))

plainName :: P String
plainName = do
  x <- advance
  case x of
    TName _ n -> return n
    TKw n -> return n
    _ -> failP "expected a name"

name :: P Name
name = do
  x <- advance
  case x of
    TName s n -> return (Name s n)
    TKw n -> return (Name "" n)
    _ -> failP "expected a name"

-- ---------------------------------------------------------------------------
-- Blocks and statements

block :: P Block
block = do
  x <- peek
  case x of
    TOpen -> do
      advance
      ss <- stmts
      expect TClose
      return ss
    _ -> inlineStmts
  where
    stmts = do
      skipSeps
      x <- peek
      case x of
        TClose -> return []
        TEOF -> return []
        _ -> do
          s <- stmt
          rest <- stmts
          return (s : rest)
    inlineStmts = do
      s <- stmt
      more <- optional' (TSym ";")
      if more then (s :) <$> inlineStmts else return [s]

stmt :: P Stmt
stmt = do
  x <- peek
  y <- peek2
  case (x, y) of
    (TKw "let", _) -> do
      advance
      n <- name
      z <- peek
      when (z == TSym ":") $ skipUntil (== TSym "=")
      expect (TSym "=")
      SLet n <$> expr
    (TName s n, TSym "<-") -> do
      advance
      advance
      SBind (Name s n) <$> expr
    (TSym o, _) | o `elem` ["(", "(("] -> do
      -- a tuple-pattern bind: ((!rx, $link)) <- expr ; otherwise an expression statement
      isBind <- lookaheadBind
      if isBind
        then do
          p <- pat
          expect (TSym "<-")
          SBindPat p <$> expr
        else SExpr <$> expr
    (TKw n, TSym "<-") -> do
      advance
      advance
      SBind (Name "" n) <$> expr
    _ -> SExpr <$> expr

-- ---------------------------------------------------------------------------
-- Expressions

expr :: P Expr
expr = do
  e <- exprMerge
  x <- peek
  case x of
    TKw "then" -> advance >> EThen e <$> exprMerge
    _ -> return e

-- paren-less application: an applicable head followed by comma-separated args, each taking
-- the rest of its expression (down to `then`)
application :: Expr -> P Expr
application hd = do
  starts <- startsExpr
  if starts then EApp hd <$> args else return hd
  where
    args = do
      a <- exprMerge
      commas <- commasAllowed
      more <- if commas then optional' (TSym ",") else return False
      if more then (a :) <$> args else return [a]

applicableName :: String -> Bool
applicableName n = not (null n) && isUpper (head n)

startsExpr :: P Bool
startsExpr = do
  x <- peek
  return $ case x of
    TName _ _ -> True
    TNat _ -> True
    TFloat _ -> True
    TText _ -> True
    TKw k -> k `elem` ["not", "if", "match", "rebuild", "for", "unit", "empty", "key"]
    TSym s -> s `elem` ["(", "((", "[[", "{", "-", "+", "!", "$", "@", "?", "\\|", "\\"]
    _ -> False

binLeft :: P Expr -> [String] -> (Tok -> Maybe String) -> P Expr
binLeft next ops asOp = do
  l <- next
  go l
  where
    go l = do
      x <- peek
      case asOp x of
        Just op | op `elem` ops -> do
          advance
          r <- next
          go (EBin op l r)
        _ -> return l

symOp :: Tok -> Maybe String
symOp (TSym s) = Just s
symOp (TKw k) = Just k
symOp _ = Nothing

exprMerge, exprOn, exprOr, exprAnd, exprNot, exprCmp, exprAdd, exprMul, exprPrefix, exprPostfix :: P Expr
exprMerge = binLeft exprOn ["|"] symOp
exprOn = binLeft exprOr ["on"] symOp
exprOr = binLeft exprAnd ["or"] symOp
exprAnd = binLeft exprNot ["and"] symOp
exprNot = do
  x <- peek
  case x of
    TKw "not" -> advance >> ENot <$> exprNot
    _ -> exprCmp
exprCmp = do
  l <- exprAdd
  x <- peek
  case symOp x of
    Just op | op `elem` ["==", "!=", "<", "<=", ">", ">="] -> do
      advance
      r <- exprAdd
      return (EBin op l r)
    _ -> return l
exprAdd = binLeft exprMul ["+", "-"] symOp
exprMul = binLeft exprPrefix ["*", "/"] symOp
exprPrefix = do
  x <- peek
  case x of
    TSym "-" -> advance >> ENeg <$> exprPrefix
    TSym "+" -> advance >> exprPrefix
    TSym "@" -> advance >> EReturn <$> exprMerge
    TSym "$" -> advance >> EFree "$" <$> exprMerge
    TSym "!" -> advance >> EFree "!" <$> exprMerge
    TSym "?" -> advance >> EFree "?" <$> exprMerge
    _ -> exprPostfix
exprPostfix = do
  e <- primary
  e' <- post e
  case e' of
    EVar (Name _ n) | applicableName n -> application e'
    _ -> return e'
  where
    post e = do
      x <- peek
      case x of
        TSym "." -> do
          y <- peek2
          case y of
            TName "" f | not (null f) && isLower (head f) -> advance >> advance >> post (EField e f)
            _ -> return e
        TSym "[" -> do
          advance
          i <- expr
          expect (TSym "]")
          post (EIndex e i)
        _ -> return e

primary :: P Expr
primary = do
  x <- advance
  case x of
    TNat n -> return (ELit (LNat n))
    TFloat d -> return (ELit (LFloat d))
    TText parts -> EText <$> mapM part parts
    TKw "unit" -> return (ELit LUnit)
    TKw "empty" -> return (EArray [])
    TKw "key" -> return (EVar (Name "" "key"))
    TKw "ignore" -> return (EVar (Name "" "ignore"))
    TName "@" "started" -> return EStarted
    TName "@" "Element" -> EElement <$> exprMerge
    TName "@" "widget" -> do
      expect (TSym ":")
      EBlock <$> block
    TName s n
      | n == "True" -> return (ELit (LBool True))
      | n == "False" -> return (ELit (LBool False))
      | applicableName n -> do
          y <- peekTok
          case y of
            Just t | tok t == TSym "(" -> do
              named <- isNamedCons
              if named then advance >> ECons n <$> namedFields
                       else return (EVar (Name s n))
            _ -> return (EVar (Name s n))
      | otherwise -> return (EVar (Name s n))
    TSym "((" -> withCommas True $ do
      e <- expr
      more <- optional' (TSym ",")
      rest <- if more then tupleRest else return []
      expect (TSym "))")
      return (ETuple (e : rest))
    TSym "(" -> withCommas True $ do
      e <- expr
      more <- optional' (TSym ",")
      if more
        then do
          rest <- tupleRest
          expect (TSym ")")
          return (ETuple (e : rest))
        else expect (TSym ")") >> return e
    TSym "[[" -> withCommas True $ do
      y <- peek
      if y == TSym "]]" then advance >> return (EArray [])
        else do
          es <- commaList
          expect (TSym "]]")
          return (EArray es)
    TSym "{" -> withCommas True $ do
      y <- peek
      if y == TSym "}" then advance >> return (EMap [])
        else do
          ps <- mapPairs
          expect (TSym "}")
          return (EMap ps)
    TKw "if" -> do
      c <- expr
      expect (TSym ":")
      t <- bodyExpr
      skipSeps
      expect (TKw "else")
      expect (TSym ":")
      e <- bodyExpr
      return (EIf c t e)
    TKw "match" -> do
      s <- expr
      expect (TSym ":")
      arms <- matchArms
      return (EMatch s arms)
    TKw "rebuild" -> do
      d <- expr
      expect (TSym ":")
      ERebuild d <$> block
    TKw "for" -> forExpr
    TSym "\\|" -> do
      ps <- params
      body <- lambdaBody
      return (ELam ps body)
    TSym "\\" -> do
      y <- advance
      case y of
        TName "@" "widget" -> do
          expect (TSym ":")
          EBlock <$> block >>= \b -> return (ELam [Name "" "_"] b)
        _ -> failP "expected \\@widget:"
    _ -> failP "unexpected token in expression"
  where
    part (Left s) = return (TLit s)
    part (Right src) = case parseExprSource src of
      Left e -> failP ("in interpolation: " ++ e)
      Right e -> return (TExpr e)
    tupleRest = do
      e <- expr
      more <- optional' (TSym ",")
      if more then (e :) <$> tupleRest else return [e]
    commaList = do
      e <- expr
      more <- optional' (TSym ",")
      if more then (e :) <$> commaList else return [e]
    mapPairs = do
      k <- expr
      expect (TSym ":")
      v <- expr
      more <- optional' (TSym ",")
      if more then ((k, v) :) <$> mapPairs else return [(k, v)]
    params = do
      x <- peek
      if x == TSym "|" then advance >> return [] else do
        n <- name
        more <- optional' (TSym ",")
        if more then (n :) <$> params else expect (TSym "|") >> return [n]
    lambdaBody = do
      x <- peek
      case x of
        TName "@" "widget" -> do
          advance
          expect (TSym ":")
          EBlock <$> block
        _ -> expr

-- does a `<-` follow the balanced parenthesised group at the head of this statement?
lookaheadBind :: P Bool
lookaheadBind = P $ \st -> Right (scan (0 :: Int) (psToks st), st)
  where
    scan _ [] = False
    scan d (Token _ _ t : rest) = case t of
      TSym "(" -> scan (d + 1) rest
      TSym "((" -> scan (d + 1) rest
      TSym ")" -> if d - 1 == 0 then startsArrow rest else scan (d - 1) rest
      TSym "))" -> if d - 1 == 0 then startsArrow rest else scan (d - 1) rest
      TSep -> False
      TClose -> False
      TOpen -> False
      _ -> scan d rest
    startsArrow (Token _ _ (TSym "<-") : _) = True
    startsArrow _ = False

-- is the parenthesis after an uppercase name a named-field construction?
isNamedCons :: P Bool
isNamedCons = P $ \st -> case psToks st of
  (Token _ _ (TSym "(") : Token _ _ (TSym ")") : _) -> Right (True, st)
  (Token _ _ (TSym "(") : Token _ _ (TName "" _) : Token _ _ (TSym "=") : _) -> Right (True, st)
  (Token _ _ (TSym "(") : Token _ _ (TKw _) : Token _ _ (TSym "=") : _) -> Right (True, st)
  (Token _ _ (TSym "(") : Token _ _ (TSym "..") : _) -> Right (True, st)
  _ -> Right (False, st)

namedFields :: P [(Maybe String, Expr)]
namedFields = withCommas True $ do
  x <- peek
  if x == TSym ")" then advance >> return [] else go
  where
    go = do
      f <- plainName
      expect (TSym "=")
      v <- expr
      more <- optional' (TSym ",")
      if more then ((Just f, v) :) <$> go else expect (TSym ")") >> return [(Just f, v)]

-- a body after `if c:` / `else:` / a match arm: inline expression or an indented block of
-- lets ending in an expression
bodyExpr :: P Expr
bodyExpr = do
  x <- peek
  case x of
    TOpen -> do
      advance
      ss <- stmtsUntilClose
      expect TClose
      foldStmts ss
    _ -> expr
  where
    stmtsUntilClose = do
      skipSeps
      x <- peek
      case x of
        TClose -> return []
        _ -> do
          s <- stmt
          rest <- stmtsUntilClose
          return (s : rest)
    foldStmts [] = failP "empty block"
    foldStmts [SExpr e] = return e
    foldStmts (SLet n v : rest) = ELet n v <$> foldStmts rest
    foldStmts (SBind n v : rest) = ELet n v <$> foldStmts rest
    foldStmts (SExpr _ : rest) = foldStmts rest

matchArms :: P [(Pat, Maybe Expr, Expr)]
matchArms = do
  x <- peek
  case x of
    TOpen -> do
      advance
      as <- arms
      expect TClose
      return as
    _ -> do
      a <- arm
      more <- optional' (TSym ";")
      if more then (a :) <$> matchArms else return [a]
  where
    arms = do
      skipSeps
      x <- peek
      case x of
        TClose -> return []
        _ -> do
          a <- arm
          rest <- arms
          return (a : rest)
    arm = do
      p <- pat
      g <- do
        x <- peek
        if x == TKw "if" then advance >> Just <$> expr else return Nothing
      expect (TSym ":")
      b <- bodyExpr
      return (p, g, b)

forExpr :: P Expr
forExpr = do
  x <- peek
  y <- peek2
  case (x, y) of
    (TName _ _, TSym ",") -> do
      k <- name
      expect (TSym ",")
      v <- name
      expect (TSym "<-")
      m <- expr
      expect (TSym ":")
      EFor (Just (k, v)) m <$> block
    (TKw "key", TSym ",") -> do
      advance
      expect (TSym ",")
      v <- name
      expect (TSym "<-")
      m <- expr
      expect (TSym ":")
      EFor (Just (Name "" "key", v)) m <$> block
    _ -> do
      m <- expr
      expect (TSym ":")
      EFor Nothing m <$> block

-- ---------------------------------------------------------------------------
-- Patterns

pat :: P Pat
pat = do
  x <- advance
  case x of
    TKw "ignore" -> return PWild
    TName "" "_" -> return PWild
    TName _ "_" -> return PWild
    TNat n -> return (PLit (LNat n))
    TText parts -> return (PText (concatMap lit parts))
    TKw "unit" -> return (PLit LUnit)
    TName s n
      | n == "True" -> return (PLit (LBool True))
      | n == "False" -> return (PLit (LBool False))
      | applicableName n -> do
          y <- peek
          case y of
            TSym "(" -> do
              named <- isNamedCons
              if named then advance >> recFields n else PCons n <$> subPats
            _ -> PCons n <$> subPats
      | otherwise -> return (PVar (Name s n))
    TSym "-" -> do
      y <- advance
      case y of
        TNat n -> return (PLit (LInt (negate n)))
        _ -> failP "expected a number"
    TSym "((" -> withCommas True $ do
      p <- pat
      more <- optional' (TSym ",")
      ps <- if more then tuplePats else return []
      expect (TSym "))")
      return (PTuple (p : ps))
    TSym "(" -> withCommas True $ do
      p <- pat
      more <- optional' (TSym ",")
      if more
        then do
          ps <- tuplePats
          expect (TSym ")")
          return (PTuple (p : ps))
        else expect (TSym ")") >> return p
    _ -> failP "unexpected token in pattern"
  where
    lit (Left s) = s
    lit (Right _) = "{}"
    -- sub-patterns of a constructor: primaries until a token that cannot start a pattern
    subPats = do
      y <- peek
      case y of
        TName _ _ -> (:) <$> pat <*> subPats
        TNat _ -> (:) <$> pat <*> subPats
        TText _ -> (:) <$> pat <*> subPats
        TKw "ignore" -> (:) <$> pat <*> subPats
        TKw "unit" -> (:) <$> pat <*> subPats
        TSym "(" -> (:) <$> pat <*> subPats
        TSym "," -> do
          -- `Todo a, b`: commas separate constructor args in patterns, except inside the
          -- fields of a record pattern, where they separate fields
          commas <- commasAllowed
          y2 <- peek2
          if commas && y2 /= TSym ".." then advance >> ((:) <$> pat <*> subPats) else return []
        _ -> return []
    tuplePats = do
      p <- pat
      more <- optional' (TSym ",")
      if more then (p :) <$> tuplePats else return [p]
    recFields n = withCommas False $ do
      y <- peek
      if y == TSym ")" then advance >> return (PRec n [] False) else go n [] False
    go n acc dots = do
      y <- peek
      case y of
        TSym ".." -> do
          advance
          z <- peek
          if z == TSym "," then advance >> go n acc True else expect (TSym ")") >> return (PRec n (reverse acc) True)
        _ -> do
          f <- plainName
          expect (TSym "=")
          p <- pat
          more <- optional' (TSym ",")
          if more then go n ((f, p) : acc) dots
            else expect (TSym ")") >> return (PRec n (reverse ((f, p) : acc)) dots)

-- ---------------------------------------------------------------------------
-- Tests

testDecl :: P Item
testDecl = do
  n <- plainName
  expect (TSym ",")
  w <- expr
  expect (TSym ":")
  expect TOpen
  ts <- turns
  expect TClose
  return (ITest n w ts)
  where
    turns = do
      skipSeps
      x <- peek
      case x of
        TClose -> return []
        _ -> do
          t <- turn
          rest <- turns
          return (t : rest)

turn :: P Turn
turn = withCommas False $ do
  (ws, ins) <- inputs [] []
  cs <- checks
  return (Turn (reverse ws) (reverse ins) cs)
  where
    inputs ws ins = do
      x <- advance
      item' <- case x of
        TKw "build" -> return (Right IBuild)
        TKw "idle" -> return (Right IIdle)
        TKw "click" -> Right . IClick <$> pat
        TKw "send" -> do
          p <- pat
          expect (TSym ",")
          Right . ISend p <$> withCommas False expr
        TKw "push" -> do
          p <- pat
          expect (TSym "=")
          Right . IPush p <$> withCommas False expr
        TKw "with" -> do
          p <- pat
          expect (TSym "=")
          Left . (,) p <$> withCommas False expr
        _ -> failP "expected an input (build, idle, click, send, push, with)"
      let (ws', ins') = case item' of
            Left w -> (w : ws, ins)
            Right i -> (ws, i : ins)
      more <- optional' (TSym ",")
      if more then inputs ws' ins' else return (ws', ins')
    checks = do
      x <- peek
      case x of
        TSym "=>" -> do
          advance
          c <- check
          rest <- checks
          return (c : rest)
        _ -> return []
    check = do
      x <- advance
      case x of
        TKw "expect" -> CExpect <$> pat
        TKw "expect_top" -> CExpectTop <$> pat
        TKw "expect_none" -> CExpectNone <$> pat
        TKw "expect_top_none" -> CExpectTopNone <$> pat
        TKw "expect_count" -> do
          p <- pat
          expect (TSym ",")
          y <- advance
          case y of
            TNat n -> return (CExpectCount p n)
            _ -> failP "expected a count"
        TKw "returns" -> CReturns <$> pat
        TKw "fired" -> CFired <$> pat
        TKw "quiet" -> return CQuiet
        TKw "requested" -> do
          p <- pat
          expect (TSym ",")
          CRequested p <$> pat
        TKw "unrequested" -> CUnrequested <$> pat
        TKw "errors" -> CErrors <$> pat
        TKw "settles" -> return CSettles
        _ -> failP "expected a check"

-- | Parse an expression from a source fragment (text interpolation).
parseExprSource :: String -> Either String Expr
parseExprSource src = do
  toks <- lexProgram src
  let toks' = filter (\t -> tok t `notElem` [TSep, TOpen, TClose]) toks
  (e, rest) <- runP expr toks'
  case rest of
    (Token _ _ TEOF : _) -> return e
    [] -> return e
    (t:_) -> Left ("trailing tokens in interpolation: " ++ show (tok t))
