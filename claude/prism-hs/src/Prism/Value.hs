{-# LANGUAGE RankNTypes #-}
-- | Runtime values over Reflex: a plain value, or an `Event t`, a `Dynamic t`, or a blueprint
-- that builds in the push monad. Also paths, slots and the scope a block builds into.
module Prism.Value where

import Reflex
import Data.IORef
import qualified Data.Map as M
import Control.Exception (Exception)
import Prism.Syntax (Name, Expr)
import Data.List (intercalate)

data Value t
  = VInt Integer
  | VFloat Double
  | VBool Bool
  | VText String
  | VUnit
  | VCons String [Value t]               -- choice constructors and positional records: Instant 5000
  | VRec String [(String, Value t)]      -- named records and elements: Ui.Button(label="x")
  | VArray [Value t]
  | VMap (M.Map (Value t) (Value t))
  | VSome (Value t)
  | VNil
  | VTuple [Value t]
  | VFn (Closure t)
  | VPrim String                         -- a prelude function by name
  | VEvent (Event t (Value t))           -- `!A`
  | VDyn (Dynamic t (Value t))           -- `$A`
  | VBlueprint (Blueprint t)             -- `@A`

data Closure t = Closure { cParams :: [Name], cBody :: Expr, cEnv :: Env t }

-- | Environments are lazy maps: a block's forward references are knots tied with `mdo`, and a
-- strict map would force them while the block is still being built.
type Env t = M.Map String (Value t)

-- | A blueprint builds into a scope, in the push monad, and returns the block's result.
-- `bpOwnPos` says whether binding it takes a position of its own in the parent's path (a
-- widget-function body does; rebuild, for and @Element allocate their own positions while
-- building).
data Blueprint t = Blueprint { bpBuild :: Scope t -> PushM t (Value t), bpOwnPos :: Bool }

data PrismError = PrismError { peKind :: String, peDetail :: String }
  deriving Show
instance Exception PrismError

-- | Path segments as chapter 2 spells them.
data Seg t = SPos Int | SGen Int | SKey (Value t)

instance Show (Seg t) where
  show (SPos i) = show i
  show (SGen g) = '#' : show g
  show (SKey k) = "[" ++ showValue k ++ "]"

instance Eq (Seg t) where
  a == b = compare a b == EQ

-- | Lexicographic order on paths is tree order: a slot's path extends its parent's, positions
-- are numbered in bind order, and the keys of a `for` sort by value.
instance Ord (Seg t) where
  compare (SPos i) (SPos j) = compare i j
  compare (SGen i) (SGen j) = compare i j
  compare (SKey a) (SKey b) = compare a b
  compare a b = compare (segRank a) (segRank b)
    where segRank :: Seg t -> Int
          segRank (SPos _) = 0
          segRank (SGen _) = 1
          segRank (SKey _) = 2

type Path t = [Seg t]

showPath :: Path t -> String
showPath = intercalate "/" . map show

-- | A slot: an element in the list. Its response event is selected from the root fan by path,
-- its request event is told to the enclosing scope, its state is a cell keyed by element value.
data Slot t = Slot
  { slotPath     :: Path t
  , slotElement  :: Value t
  , slotResponds :: Bool
  }

type ListMap t = M.Map (Path t) (Slot t)
type Requests t = [(Path t, Value t)]

-- | A scope: where a block's slots and child blocks register. The list is the union of the
-- children's list dynamics; the requests are the merge of the children's request events.
data Scope t = Scope
  { scPath  :: Path t
  , scPos   :: IORef Int
  , scLists :: IORef [Dynamic t (ListMap t)]
  , scReqs  :: IORef [Event t (Requests t)]
  }

-- ---------------------------------------------------------------------------
-- Equality and ordering on plain values (functor values compare by rank only)

instance Eq (Value t) where
  a == b = compare a b == EQ

instance Ord (Value t) where
  compare = cmpV

rank :: Value t -> Int
rank v = case v of
  VInt _ -> 0; VFloat _ -> 1; VBool _ -> 2; VText _ -> 3; VUnit -> 4; VCons _ _ -> 5
  VRec _ _ -> 6; VArray _ -> 7; VMap _ -> 8; VSome _ -> 9; VNil -> 10; VTuple _ -> 11
  VFn _ -> 12; VPrim _ -> 13; VEvent _ -> 14; VDyn _ -> 15; VBlueprint _ -> 16

cmpV :: Value t -> Value t -> Ordering
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
  (VPrim x, VPrim y) -> compare x y
  _ -> compare (rank a) (rank b)

-- | Force a plain value fully, so that an error inside it surfaces where the value is made (in
-- the occurrence) and not where the transcript prints it. Functor values are left alone.
forceValue :: Value t -> ()
forceValue v = case v of
  VInt n -> n `seq` ()
  VFloat d -> d `seq` ()
  VBool b -> b `seq` ()
  VText s -> length s `seq` ()
  VCons n xs -> length n `seq` foldr (seq . forceValue) () xs
  VRec n fs -> length n `seq` foldr (\(f, x) r -> length f `seq` forceValue x `seq` r) () fs
  VArray xs -> foldr (seq . forceValue) () xs
  VMap m -> foldr (\(k, x) r -> forceValue k `seq` forceValue x `seq` r) () (M.toList m)
  VSome x -> forceValue x
  VTuple xs -> foldr (seq . forceValue) () xs
  _ -> ()

-- | Print a plain value as a Prism literal (chapter 10's transcript spelling).
showValue :: Value t -> String
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
  VEvent _ -> "<event>"
  VDyn _ -> "<dynamic>"
  VBlueprint _ -> "<blueprint>"
  where
    showArg x = case x of
      VCons _ (_:_) -> "(" ++ showValue x ++ ")"
      VSome _ -> "(" ++ showValue x ++ ")"
      _ -> showValue x

-- | Text of a value inside an interpolation: text is inserted raw.
textOf :: Value t -> String
textOf (VText s) = s
textOf v = showValue v
