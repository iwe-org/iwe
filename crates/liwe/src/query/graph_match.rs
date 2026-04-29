use crate::graph::Graph;
use crate::model::Key;
use crate::query::document::{
    CountArg, InclusionAnchor, KeyOp, MaxDepth, NumExpr, NumOp, ReferenceAnchor,
};
use crate::query::graph_walk::{
    ancestors_inclusion, descendants_inclusion, inbound_reference, outbound_reference,
};

pub(crate) fn match_key_op(op: &KeyOp, key: &Key) -> bool {
    match op {
        KeyOp::Eq(target) => key == target,
        KeyOp::Ne(target) => key != target,
        KeyOp::In(targets) => targets.iter().any(|t| t == key),
        KeyOp::Nin(targets) => !targets.iter().any(|t| t == key),
    }
}

pub(crate) fn match_inclusion_count(arg: &CountArg, key: &Key, graph: &Graph, outbound: bool) -> bool {
    let max = match arg.max_depth {
        MaxDepth::Bounded(n) => n,
        MaxDepth::Any => u32::MAX,
    };
    let walk = if outbound {
        descendants_inclusion(graph, key, max)
    } else {
        ancestors_inclusion(graph, key, max)
    };
    let count = walk
        .values()
        .filter(|&&d| d >= arg.min_depth && d <= max)
        .count() as u64;
    eval_num_expr(&arg.count, count)
}

pub(crate) fn match_inclusion_walk(
    anchors: &[InclusionAnchor],
    key: &Key,
    graph: &Graph,
    outbound: bool,
) -> bool {
    if anchors.is_empty() {
        return false;
    }
    anchors.iter().all(|anchor| {
        if &anchor.key == key {
            return false;
        }
        let walk = if outbound {
            ancestors_inclusion(graph, &anchor.key, anchor.max_depth)
        } else {
            descendants_inclusion(graph, &anchor.key, anchor.max_depth)
        };
        match walk.get(key) {
            Some(&depth) => depth >= anchor.min_depth && depth <= anchor.max_depth,
            None => false,
        }
    })
}

pub(crate) fn match_reference_walk(
    anchors: &[ReferenceAnchor],
    key: &Key,
    graph: &Graph,
    outbound: bool,
) -> bool {
    if anchors.is_empty() {
        return false;
    }
    anchors.iter().all(|anchor| {
        if &anchor.key == key {
            return false;
        }
        let walk = if outbound {
            inbound_reference(graph, &anchor.key, anchor.max_distance)
        } else {
            outbound_reference(graph, &anchor.key, anchor.max_distance)
        };
        match walk.get(key) {
            Some(&distance) => {
                distance >= anchor.min_distance && distance <= anchor.max_distance
            }
            None => false,
        }
    })
}

fn eval_num_expr(expr: &NumExpr, value: u64) -> bool {
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
