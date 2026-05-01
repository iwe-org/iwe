use crate::model::Key;
use crate::query::document::KeyOp;

pub fn match_key_op(op: &KeyOp, key: &Key) -> bool {
    match op {
        KeyOp::Eq(target) => key == target,
        KeyOp::Ne(target) => key != target,
        KeyOp::In(targets) => targets.iter().any(|t| t == key),
        KeyOp::Nin(targets) => !targets.iter().any(|t| t == key),
    }
}
