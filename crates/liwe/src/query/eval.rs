use std::collections::HashSet;

use rayon::prelude::*;

use crate::graph::Graph;
use crate::model::Key;
use crate::query::document::{
    CountArg, FieldOp, FieldPath, Filter, InclusionAnchor, KeyOp, MaxDepth, ReferenceAnchor,
};
use crate::query::filter::{match_field_op, resolve_path, Resolution};
use crate::query::graph_match::{eval_num_expr, match_key_op};
use crate::graph::walk::{
    ancestors_inclusion, descendants_inclusion, inbound_reference, outbound_reference,
};

const PARALLEL_THRESHOLD: usize = 64;

pub fn evaluate(filter: &Filter, graph: &Graph) -> Vec<Key> {
    let set = eval(filter, graph, None);
    let mut keys: Vec<Key> = set.into_iter().collect();
    keys.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
    keys
}

fn all_keys(graph: &Graph) -> HashSet<Key> {
    graph.keys().into_iter().collect()
}

fn eval(filter: &Filter, graph: &Graph, scope: Option<&HashSet<Key>>) -> HashSet<Key> {
    match filter {
        Filter::And(children) => eval_and(children, graph, scope),
        Filter::Or(children) => eval_or(children, graph, scope),
        Filter::Not(inner) => eval_not(inner, graph, scope),
        Filter::Field { path, op } => eval_field(path, op, graph, scope),
        Filter::Key(op) => eval_key(op, graph, scope),
        Filter::Includes(anchors) => eval_inclusion(anchors, graph, scope, true),
        Filter::IncludedBy(anchors) => eval_inclusion(anchors, graph, scope, false),
        Filter::References(anchors) => eval_reference(anchors, graph, scope, true),
        Filter::ReferencedBy(anchors) => eval_reference(anchors, graph, scope, false),
        Filter::IncludesCount(arg) => eval_count(arg, graph, scope, true),
        Filter::IncludedByCount(arg) => eval_count(arg, graph, scope, false),
    }
}

fn is_predicate(filter: &Filter) -> bool {
    matches!(
        filter,
        Filter::Field { .. } | Filter::IncludesCount(_) | Filter::IncludedByCount(_)
    )
}

fn eval_and(children: &[Filter], graph: &Graph, scope: Option<&HashSet<Key>>) -> HashSet<Key> {
    if children.is_empty() {
        return scope.cloned().unwrap_or_else(|| all_keys(graph));
    }

    let (predicates, generators): (Vec<&Filter>, Vec<&Filter>) =
        children.iter().partition(|f| is_predicate(f));

    let candidate: HashSet<Key> = if generators.is_empty() {
        scope.cloned().unwrap_or_else(|| all_keys(graph))
    } else {
        let sets: Vec<HashSet<Key>> = generators
            .par_iter()
            .map(|f| eval(f, graph, scope))
            .collect();
        intersect_sets(sets)
    };

    if predicates.is_empty() || candidate.is_empty() {
        return candidate;
    }

    apply_predicates(&predicates, candidate, graph)
}

fn eval_or(children: &[Filter], graph: &Graph, scope: Option<&HashSet<Key>>) -> HashSet<Key> {
    if children.is_empty() {
        return HashSet::new();
    }
    children
        .par_iter()
        .map(|f| eval(f, graph, scope))
        .reduce(HashSet::new, |mut a, b| {
            if a.is_empty() {
                b
            } else if b.is_empty() {
                a
            } else {
                a.extend(b);
                a
            }
        })
}

fn eval_not(inner: &Filter, graph: &Graph, scope: Option<&HashSet<Key>>) -> HashSet<Key> {
    let universe = scope.cloned().unwrap_or_else(|| all_keys(graph));
    let inner_set = eval(inner, graph, Some(&universe));
    universe.into_iter().filter(|k| !inner_set.contains(k)).collect()
}

fn eval_field(
    path: &FieldPath,
    op: &FieldOp,
    graph: &Graph,
    scope: Option<&HashSet<Key>>,
) -> HashSet<Key> {
    let candidate = scope.cloned().unwrap_or_else(|| all_keys(graph));
    filter_by_field(candidate, path, op, graph)
}

fn eval_key(op: &KeyOp, graph: &Graph, scope: Option<&HashSet<Key>>) -> HashSet<Key> {
    let universe = scope.cloned().unwrap_or_else(|| all_keys(graph));
    universe.into_iter().filter(|k| match_key_op(op, k)).collect()
}

fn eval_inclusion(
    anchors: &[InclusionAnchor],
    graph: &Graph,
    scope: Option<&HashSet<Key>>,
    outbound: bool,
) -> HashSet<Key> {
    if anchors.is_empty() {
        return HashSet::new();
    }
    let sets: Vec<HashSet<Key>> = anchors
        .par_iter()
        .map(|anchor| {
            let walk = if outbound {
                ancestors_inclusion(graph, &anchor.key, anchor.max_depth)
            } else {
                descendants_inclusion(graph, &anchor.key, anchor.max_depth)
            };
            let mut set: HashSet<Key> = walk
                .into_iter()
                .filter(|(_, d)| *d >= anchor.min_depth && *d <= anchor.max_depth)
                .map(|(k, _)| k)
                .collect();
            set.remove(&anchor.key);
            set
        })
        .collect();
    let mut result = intersect_sets(sets);
    if let Some(s) = scope {
        result.retain(|k| s.contains(k));
    }
    result
}

fn eval_reference(
    anchors: &[ReferenceAnchor],
    graph: &Graph,
    scope: Option<&HashSet<Key>>,
    outbound: bool,
) -> HashSet<Key> {
    if anchors.is_empty() {
        return HashSet::new();
    }
    let sets: Vec<HashSet<Key>> = anchors
        .par_iter()
        .map(|anchor| {
            let walk = if outbound {
                inbound_reference(graph, &anchor.key, anchor.max_distance)
            } else {
                outbound_reference(graph, &anchor.key, anchor.max_distance)
            };
            let mut set: HashSet<Key> = walk
                .into_iter()
                .filter(|(_, d)| *d >= anchor.min_distance && *d <= anchor.max_distance)
                .map(|(k, _)| k)
                .collect();
            set.remove(&anchor.key);
            set
        })
        .collect();
    let mut result = intersect_sets(sets);
    if let Some(s) = scope {
        result.retain(|k| s.contains(k));
    }
    result
}

fn eval_count(
    arg: &CountArg,
    graph: &Graph,
    scope: Option<&HashSet<Key>>,
    outbound: bool,
) -> HashSet<Key> {
    let candidate = scope.cloned().unwrap_or_else(|| all_keys(graph));
    let keys: Vec<Key> = candidate.into_iter().collect();
    if keys.len() >= PARALLEL_THRESHOLD {
        keys.into_par_iter()
            .filter(|k| count_matches(arg, k, graph, outbound))
            .collect()
    } else {
        keys.into_iter()
            .filter(|k| count_matches(arg, k, graph, outbound))
            .collect()
    }
}

fn count_matches(arg: &CountArg, key: &Key, graph: &Graph, outbound: bool) -> bool {
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

fn intersect_sets(mut sets: Vec<HashSet<Key>>) -> HashSet<Key> {
    if sets.is_empty() {
        return HashSet::new();
    }
    sets.sort_by_key(|s| s.len());
    let mut iter = sets.into_iter();
    let mut acc = iter.next().unwrap();
    for s in iter {
        acc.retain(|k| s.contains(k));
        if acc.is_empty() {
            break;
        }
    }
    acc
}

fn apply_predicates(
    predicates: &[&Filter],
    candidate: HashSet<Key>,
    graph: &Graph,
) -> HashSet<Key> {
    let keys: Vec<Key> = candidate.into_iter().collect();
    let pred_fn = |k: &Key| predicates.iter().all(|p| run_predicate(p, k, graph));
    if keys.len() >= PARALLEL_THRESHOLD {
        keys.into_par_iter().filter(|k| pred_fn(k)).collect()
    } else {
        keys.into_iter().filter(|k| pred_fn(k)).collect()
    }
}

fn run_predicate(filter: &Filter, key: &Key, graph: &Graph) -> bool {
    match filter {
        Filter::Field { path, op } => match_field_at(graph, key, path, op),
        Filter::IncludesCount(arg) => count_matches(arg, key, graph, true),
        Filter::IncludedByCount(arg) => count_matches(arg, key, graph, false),
        _ => unreachable!("non-predicate filter passed to run_predicate"),
    }
}

fn filter_by_field(
    candidate: HashSet<Key>,
    path: &FieldPath,
    op: &FieldOp,
    graph: &Graph,
) -> HashSet<Key> {
    let keys: Vec<Key> = candidate.into_iter().collect();
    if keys.len() >= PARALLEL_THRESHOLD {
        keys.into_par_iter()
            .filter(|k| match_field_at(graph, k, path, op))
            .collect()
    } else {
        keys.into_iter()
            .filter(|k| match_field_at(graph, k, path, op))
            .collect()
    }
}

fn match_field_at(graph: &Graph, key: &Key, path: &FieldPath, op: &FieldOp) -> bool {
    let mapping = graph.frontmatter(key).cloned().unwrap_or_default();
    match resolve_path(&mapping, path) {
        Resolution::Present(value) => match_field_op(op, Some(value)),
        Resolution::Missing => match_field_op(op, None),
    }
}
