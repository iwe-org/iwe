use crate::graph::Graph;
use crate::model::Key;
use crate::query::document::{Filter, InclusionAnchor, KeyOp, ReferenceAnchor};

pub fn match_key_op(op: &KeyOp, key: &Key) -> bool {
    match op {
        KeyOp::Eq(target) => key == target,
        KeyOp::Ne(target) => key != target,
        KeyOp::In(targets) => targets.iter().any(|t| t == key),
        KeyOp::Nin(targets) => !targets.iter().any(|t| t == key),
        KeyOp::StartsWith(prefix) => key.as_str().starts_with(prefix.as_str()),
    }
}

pub fn resolve_filter_keys(filter: &Filter, graph: &Graph) -> Filter {
    let resolve_all = |filters: &[Filter]| {
        filters
            .iter()
            .map(|f| resolve_filter_keys(f, graph))
            .collect()
    };
    match filter {
        Filter::And(children) => Filter::And(resolve_all(children)),
        Filter::Or(children) => Filter::Or(resolve_all(children)),
        Filter::Nor(children) => Filter::Nor(resolve_all(children)),
        Filter::Key(op) => Filter::Key(resolve_key_op(op, graph)),
        Filter::Includes(anchor) => Filter::Includes(Box::new(resolve_inclusion(anchor, graph))),
        Filter::IncludedBy(anchor) => {
            Filter::IncludedBy(Box::new(resolve_inclusion(anchor, graph)))
        }
        Filter::References(anchor) => {
            Filter::References(Box::new(resolve_reference(anchor, graph)))
        }
        Filter::ReferencedBy(anchor) => {
            Filter::ReferencedBy(Box::new(resolve_reference(anchor, graph)))
        }
        Filter::Field { .. } | Filter::Content(_) => filter.clone(),
    }
}

fn resolve_key_op(op: &KeyOp, graph: &Graph) -> KeyOp {
    let resolve = |key: &Key| graph.resolve_existing_key(key.as_str());
    match op {
        KeyOp::Eq(key) => KeyOp::Eq(resolve(key)),
        KeyOp::Ne(key) => KeyOp::Ne(resolve(key)),
        KeyOp::In(keys) => KeyOp::In(keys.iter().map(resolve).collect()),
        KeyOp::Nin(keys) => KeyOp::Nin(keys.iter().map(resolve).collect()),
        KeyOp::StartsWith(_) => op.clone(),
    }
}

fn resolve_inclusion(anchor: &InclusionAnchor, graph: &Graph) -> InclusionAnchor {
    InclusionAnchor {
        match_filter: resolve_filter_keys(&anchor.match_filter, graph),
        ..anchor.clone()
    }
}

fn resolve_reference(anchor: &ReferenceAnchor, graph: &Graph) -> ReferenceAnchor {
    ReferenceAnchor {
        match_filter: resolve_filter_keys(&anchor.match_filter, graph),
        ..anchor.clone()
    }
}
