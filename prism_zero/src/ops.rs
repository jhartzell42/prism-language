//! Operations on plain values: `prism_dyn::ops`, which compiled code shares, with the
//! operator spellings of `prism_syntax`'s tree mapped onto it.

use std::cmp::Ordering;
use std::rc::Rc;

use prism_rt::Value;
use prism_syntax::ast::BinOp;

pub use prism_dyn::ops::{as_float, equal, float, index_of, int_of, negate, num_f64, ordering_value, show_plain, Op, R};

use crate::load::Loaded;

/// The tree's operator as `prism_dyn`'s.
pub fn op_of(op: BinOp) -> Op {
    match op {
        BinOp::Eq => Op::Eq,
        BinOp::NotEq => Op::NotEq,
        BinOp::Lt => Op::Lt,
        BinOp::Le => Op::Le,
        BinOp::Gt => Op::Gt,
        BinOp::Ge => Op::Ge,
        BinOp::Add => Op::Add,
        BinOp::Sub => Op::Sub,
        BinOp::Mul => Op::Mul,
        BinOp::Div => Op::Div,
        BinOp::Mod => Op::Mod,
        BinOp::AddQ => Op::AddQ,
        BinOp::SubQ => Op::SubQ,
        BinOp::MulQ => Op::MulQ,
        BinOp::And => Op::And,
        BinOp::Or => Op::Or,
        BinOp::On => Op::On,
        BinOp::Merge => Op::Merge,
    }
}

/// A binary operator on plain values, with no user `impl`.
pub fn binop(_l: &Loaded, op: BinOp, a: &Value, b: &Value) -> R<Value> {
    prism_dyn::ops::binop(op_of(op), a, b)
}

/// `^Ord`, derived. The constructors' declaration order is in `prism_dyn`'s tables.
pub fn compare(_l: &Loaded, a: &Value, b: &Value) -> Ordering {
    prism_dyn::ops::compare(a, b)
}

/// The runtime type of a value, as an `impl` header names it.
pub fn type_name(_l: &Loaded, v: &Value) -> Rc<str> {
    prism_dyn::ops::type_name(v)
}
