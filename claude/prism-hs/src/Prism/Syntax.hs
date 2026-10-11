-- | Surface syntax of the Prism subset this prototype understands.
module Prism.Syntax where

-- | A name with its sigil: sigil is "" "!" "$" "@" "?" "!!" "$$" "!$" etc.
data Name = Name { nSigil :: String, nBase :: String }
  deriving (Eq, Ord, Show)

data Lit
  = LNat Integer
  | LInt Integer          -- written with a sign
  | LFloat Double
  | LBool Bool
  | LUnit
  deriving (Eq, Show)

data TextPart = TLit String | TExpr Expr
  deriving (Show)

data Expr
  = EVar Name
  | ELit Lit
  | EText [TextPart]
  | EApp Expr [Expr]                      -- F a, b   (head is a name or expression)
  | ECons String [(Maybe String, Expr)]   -- Ui.Button(label=x) / Todo a, b
  | EField Expr String
  | EIndex Expr Expr
  | EBin String Expr Expr                 -- + - * / == != < <= > >= and or | on
  | ENot Expr
  | ENeg Expr
  | EThen Expr Expr                       -- init then event   (hold)
  | EIf Expr Expr Expr
  | EMatch Expr [(Pat, Maybe Expr, Expr)]        -- arms with an optional guard
  | ELam [Name] Expr                      -- \|a, b| body
  | EBlock Block                          -- @widget: block  (a blueprint)
  | ERebuild Expr Block                   -- rebuild $x: block   (pronoun bound inside)
  | EFor (Maybe (Name, Name)) Expr Block  -- for k, $v <- $m: block   /  for $m: block
  | EReturn Expr                          -- @ v   (free widget sigil = return)
  | EFree String Expr                     -- $ v  (constant dynamic) / ! v
  | EElement Expr                         -- @Element e
  | EStarted
  | EArray [Expr]
  | EMap [(Expr, Expr)]
  | ETuple [Expr]
  | ELet Name Expr Expr                   -- let inside an expression? (unused)
  deriving (Show)

data Pat
  = PWild                                  -- ignore / _
  | PVar Name
  | PLit Lit
  | PText String
  | PCons String [Pat]                     -- Some x, Running from, Add label, Stopped
  | PRec String [(String, Pat)] Bool       -- Ui.Button(label="OK", ..)  (Bool: has ..)
  | PTuple [Pat]
  deriving (Show)

data Stmt
  = SBind Name Expr                        -- name <- expr   (hold when expr is EThen)
  | SBindPat Pat Expr                      -- ((!a, $b)) <- expr
  | SLet Name Expr                         -- let name = expr
  | SExpr Expr                             -- a bare expression statement (bind-less widget, or the result)
  deriving (Show)

type Block = [Stmt]

-- | Tag declaration: field names in order and which special parts it has.
data TagDecl = TagDecl
  { tdName     :: String
  , tdFields   :: [String]
  , tdRequests :: Bool
  , tdResponds :: Bool
  , tdState    :: Bool
  } deriving (Show)

data TypeDecl
  = TDRecord String [String]               -- type X = record: f1; f2
  | TDChoice String [(String, Int)]        -- constructors with payload arity
  | TDOther String
  deriving (Show)

-- | One line of a test: inputs, then checks.
data Turn = Turn
  { tWiths  :: [(Pat, Expr)]
  , tInputs :: [Input]
  , tChecks :: [Check]
  } deriving (Show)

data Input
  = IBuild
  | IIdle
  | IClick Pat
  | ISend Pat Expr
  | IPush Pat Expr
  deriving (Show)

data Check
  = CExpect Pat
  | CExpectTop Pat
  | CExpectNone Pat
  | CExpectTopNone Pat
  | CExpectCount Pat Integer
  | CReturns Pat
  | CFired Pat
  | CQuiet
  | CRequested Pat Pat
  | CUnrequested Pat
  | CErrors Pat
  | CSettles
  deriving (Show)

data Item
  = ITag TagDecl
  | IType TypeDecl
  | IBlueprint Name Block                  -- @counter: block
  | IFun Name Expr                         -- @Button: fn .. = \|..| ...  /  let F = \|..| ...
  | ITest String Expr [Turn]
  deriving (Show)

type Program = [Item]
