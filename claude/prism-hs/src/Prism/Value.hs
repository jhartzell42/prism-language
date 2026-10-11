-- | Runtime values and the graph's node types.
module Prism.Value where

import Data.IORef
import qualified Data.Map.Strict as M
import Prism.Syntax (Name, Expr, Block)
import Data.List (intercalate)

data Value
  = VInt Integer
  | VFloat Double
  | VBool Bool
  | VText String
  | VUnit
  | VCons String [Value]               -- choice constructors and positional records: Instant 5000
  | VRec String [(String, Value)]      -- named records and elements: Ui.Button(label="x")
  | VArray [Value]
  | VMap (M.Map Value Value)
  | VSome Value
  | VNil
  | VTuple [Value]
  | VFn Closure
  | VPrim String                       -- a prelude function by name
  | VEvent Ev
  | VDyn Dyn
  | VBlueprint Blueprint
  | VThunk (IORef (Either (IO Value) Value))   -- a later `let`, forced on first use

data Closure = Closure { cParams :: [Name], cBody :: Expr, cEnv :: Env }

type Env = M.Map String Value

-- | A blueprint: builds into a scope and returns the block's result. `bpOwnPos` says whether
-- binding it takes a position of its own in the parent's path (a widget-function body does;
-- rebuild, for and @Element allocate their own positions while building).
data Blueprint = Blueprint { bpBuild :: Scope -> IO Value, bpOwnPos :: Bool }

-- | An event node. `evFiring` holds (occurrence id, value) of the last firing.
data Ev = Ev
  { evId      :: Int
  , evSubs    :: IORef [Node]
  , evFiring  :: IORef (Maybe (Int, Value))
  , evSources :: IORef (IO [Ev])        -- prompt sources, for the height (may change: switch)
  , evLabel   :: String
  }

-- | A node is the thing the scheduler runs: it recomputes an event from its sources.
data Node = Node
  { nodeId      :: Int
  , nodeEvent   :: Ev                   -- the event this node fires
  , nodeCompute :: IO (Maybe Value)
  , nodeRan     :: IORef Int            -- the occurrence id in which it last ran
  }

-- | A dynamic: a committed current value and its change event.
data Dyn = Dyn
  { dynId     :: Int
  , dynCur    :: IORef Value
  , dynChange :: Ev
  }

-- | Path segments as chapter 2 spells them.
data Seg = SPos Int | SGen Int | SKey Value

instance Show Seg where
  show (SPos i) = show i
  show (SGen g) = '#' : show g
  show (SKey k) = "[" ++ showValue k ++ "]"

showPath :: [Seg] -> String
showPath = intercalate "/" . map show

-- | A slot: an element in the list with its three possible parts.
data Slot = Slot
  { slotPath     :: [Seg]
  , slotElement  :: Value
  , slotResponds :: Maybe Ev            -- fired by the World
  , slotRequests :: Maybe Ev            -- the program's request event
  , slotState    :: Maybe Dyn
  , slotPush     :: Maybe Ev            -- fired by the World when the snapshot changes
  }

data Child = CSlot Slot | CScope (IORef (Maybe Scope)) | CKeyed (IORef (M.Map Value Scope))

-- | A scope: where slots and child scopes are created, with the path they get.
data Scope = Scope
  { scPath     :: [Seg]
  , scPos      :: IORef Int
  , scChildren :: IORef [Child]
  }

-- ---------------------------------------------------------------------------
-- Equality and ordering on plain values (functor values compare by id)

instance Eq Value where
  a == b = compare a b == EQ

instance Ord Value where
  compare = cmpV

rank :: Value -> Int
rank v = case v of
  VInt _ -> 0; VFloat _ -> 1; VBool _ -> 2; VText _ -> 3; VUnit -> 4; VCons _ _ -> 5
  VRec _ _ -> 6; VArray _ -> 7; VMap _ -> 8; VSome _ -> 9; VNil -> 10; VTuple _ -> 11
  VFn _ -> 12; VPrim _ -> 13; VEvent _ -> 14; VDyn _ -> 15; VBlueprint _ -> 16; VThunk _ -> 17

cmpV :: Value -> Value -> Ordering
cmpV a b = case (a, b) of
  (VInt x, VInt y) -> compare x y
  (VInt x, VFloat y) -> compare (fromIntegral x) y
  (VFloat x, VInt y) -> compare x (fromIntegral y)
  (VFloat x, VFloat y) -> compare x y
  (VBool x, VBool y) -> compare x y
  (VText x, VText y) -> compare x y
  (VUnit, VUnit) -> EQ
  (VCons n xs, VCons m ys) -> compare n m <> compare xs ys
  (VRec n xs, VRec m ys) -> compare n m <> compare (map snd xs) (map snd ys)
  (VArray xs, VArray ys) -> compare xs ys
  (VMap x, VMap y) -> compare (M.toList x) (M.toList y)
  (VSome x, VSome y) -> compare x y
  (VNil, VNil) -> EQ
  (VTuple xs, VTuple ys) -> compare xs ys
  (VEvent x, VEvent y) -> compare (evId x) (evId y)
  (VDyn x, VDyn y) -> compare (dynId x) (dynId y)
  (VPrim x, VPrim y) -> compare x y
  _ -> compare (rank a) (rank b)

-- | Print a plain value as a Prism literal (chapter 10's transcript spelling).
showValue :: Value -> String
showValue v = case v of
  VInt n -> show n
  VFloat d -> show d
  VBool True -> "True"
  VBool False -> "False"
  VText s -> show s
  VUnit -> "unit"
  VCons n [] -> n
  VCons n xs -> n ++ " " ++ intercalate ", " (map showArg xs)
  VRec n [] -> n
  VRec n fs -> n ++ "(" ++ intercalate ", " [f ++ "=" ++ showValue x | (f, x) <- fs] ++ ")"
  VArray xs -> "[[" ++ intercalate ", " (map showValue xs) ++ "]]"
  VMap m -> "{" ++ intercalate ", " [showValue k ++ ": " ++ showValue x | (k, x) <- M.toList m] ++ "}"
  VSome x -> "Some " ++ showArg x
  VNil -> "Nil"
  VTuple xs -> "((" ++ intercalate ", " (map showValue xs) ++ "))"
  VFn _ -> "<fn>"
  VPrim n -> "<" ++ n ++ ">"
  VEvent e -> "<event " ++ evLabel e ++ ">"
  VDyn _ -> "<dynamic>"
  VBlueprint _ -> "<blueprint>"
  VThunk _ -> "<thunk>"
  where
    showArg x = case x of
      VCons _ (_:_) -> "(" ++ showValue x ++ ")"
      VSome _ -> "(" ++ showValue x ++ ")"
      _ -> showValue x

-- | Text of a value inside an interpolation: text is inserted raw.
textOf :: Value -> String
textOf (VText s) = s
textOf v = showValue v
