use crate::model::Key;
use crate::query::document::{KeyOp, NumExpr, NumOp};

pub fn match_key_op(op: &KeyOp, key: &Key) -> bool {
    match op {
        KeyOp::Eq(target) => key == target,
        KeyOp::Ne(target) => key != target,
        KeyOp::In(targets) => targets.iter().any(|t| t == key),
        KeyOp::Nin(targets) => !targets.iter().any(|t| t == key),
    }
}

pub fn eval_num_expr(expr: &NumExpr, value: u64) -> bool {
    expr.0.iter().all(|op| eval_num_op(op, value))
}

fn eval_num_op(op: &NumOp, value: u64) -> bool {
    match op {
        NumOp::Eq(n) => value == *n,
        NumOp::Ne(n) => value != *n,
        NumOp::Gt(n) => value > *n,
        NumOp::Gte(n) => value >= *n,
        NumOp::Lt(n) => value < *n,
        NumOp::Lte(n) => value <= *n,
        NumOp::In(ns) => ns.contains(&value),
        NumOp::Nin(ns) => !ns.contains(&value),
    }
}
