use rayon::prelude::*;
use serde_yaml::Mapping;

use crate::graph::{Graph, GraphContext};
use crate::model::node::{Node, NodeIter};
use crate::model::tree::Tree;
use crate::model::Key;
use crate::query::block_update::{self, DocRef, EvalError};
use crate::query::document::{CountOp, DeleteOp, Filter, FindOp, Limit, Operation, Sort, UpdateOp};
use crate::query::eval;
use crate::query::frontmatter::strip_reserved;
use crate::query::project::{apply_projection, ProjectionContext};
use crate::query::search;
use crate::query::sort::sort_in_place;
use crate::query::update;

#[derive(Debug)]
pub enum Outcome {
    Find { matches: Vec<FindMatch> },
    Count(usize),
    Update { changes: Vec<(Key, String)> },
    Delete { removed: Vec<Key> },
}

#[derive(Debug, Clone)]
pub struct FindMatch {
    pub key: Key,
    pub document: Mapping,
}

pub fn execute(op: &Operation, graph: &Graph) -> Result<Outcome, EvalError> {
    match op {
        Operation::Find(find) => execute_find(find, graph),
        Operation::Count(count) => Ok(execute_count(count, graph)),
        Operation::Update(upd) => execute_update(upd, graph),
        Operation::Delete(del) => execute_delete(del, graph),
    }
}

/// Names the mutating applications in `op` that lack an `expect` guard.
///
/// Strict surfaces (the `--strict` CLI flag, the always-strict MCP tool, §9.4) refuse to run a
/// mutation while this returns anything: every mutating application — the operation's document-level
/// `expect` and each block operator's `expect` — must carry a guard. Reads (`find` / `count`) never
/// have anything to guard, so they always return empty.
pub fn strict_guard_violations(op: &Operation) -> Vec<String> {
    let mut missing = Vec::new();
    match op {
        Operation::Update(upd) => {
            if upd.expect.is_none() {
                missing.push("document-level expect".to_string());
            }
            for block_op in &upd.update.block_ops {
                if block_op.expect.is_none() {
                    missing.push(format!("{} expect", block_op.op.name()));
                }
            }
        }
        Operation::Delete(del) => {
            if del.expect.is_none() {
                missing.push("document-level expect".to_string());
            }
        }
        Operation::Find(_) | Operation::Count(_) => {}
    }
    missing
}

fn select_keys(filter: Option<&Filter>, graph: &Graph) -> Vec<Key> {
    match filter {
        None => {
            let mut k = graph.keys();
            k.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
            k
        }
        Some(f) => eval::evaluate(f, graph),
    }
}

fn rows_for(keys: Vec<Key>, graph: &Graph) -> Vec<(Key, Mapping)> {
    keys.into_par_iter()
        .map(|key| {
            let mapping = graph.frontmatter(&key).cloned().unwrap_or_default();
            (key, mapping)
        })
        .collect()
}

fn select(filter: Option<&Filter>, graph: &Graph) -> Vec<(Key, Mapping)> {
    rows_for(select_keys(filter, graph), graph)
}

/// Order and cap `rows`. With `preserve_order`, the incoming order is kept (the search stage already
/// ranked by relevance); otherwise rows sort by key ascending first. An explicit `sort` then reorders
/// with a stable algorithm, so key order remains the tie-break.
fn apply_sort_and_limit(
    mut rows: Vec<(Key, Mapping)>,
    sort: Option<&Sort>,
    limit: Option<&Limit>,
    preserve_order: bool,
) -> Vec<(Key, Mapping)> {
    if !preserve_order {
        rows.sort_by(|a, b| a.0.to_string().cmp(&b.0.to_string()));
    }
    if let Some(s) = sort {
        sort_in_place(&mut rows, s);
    }
    if let Some(l) = limit {
        if !l.is_unbounded() {
            rows.truncate(l.0 as usize);
        }
    }
    rows
}

fn execute_find(op: &FindOp, graph: &Graph) -> Result<Outcome, EvalError> {
    let candidates = select_keys(op.filter.as_ref(), graph);

    let (keys, preserve_order) = match &op.search {
        None => (candidates, false),
        Some(spec) => {
            if !graph.has_search_index() {
                return Err(EvalError::SearchIndexMissing);
            }
            if op.sort.is_some() {
                (search::matched(graph, candidates, spec), false)
            } else {
                (search::ranked(graph, &candidates, spec), true)
            }
        }
    };

    let rows = rows_for(keys, graph);
    let rows = apply_sort_and_limit(rows, op.sort.as_ref(), op.limit.as_ref(), preserve_order);
    let matches: Vec<FindMatch> = rows
        .into_iter()
        .map(|(key, _)| {
            let ctx = ProjectionContext::new(graph, &key);
            let document = apply_projection(&ctx, &op.project);
            FindMatch { key, document }
        })
        .collect();
    Ok(Outcome::Find { matches })
}

fn execute_count(op: &CountOp, graph: &Graph) -> Outcome {
    let rows = select(op.filter.as_ref(), graph);
    let rows = apply_sort_and_limit(rows, op.sort.as_ref(), op.limit.as_ref(), false);
    Outcome::Count(rows.len())
}

fn execute_update(op: &UpdateOp, graph: &Graph) -> Result<Outcome, EvalError> {
    let rows = select(Some(&op.filter), graph);
    let rows = apply_sort_and_limit(rows, op.sort.as_ref(), op.limit.as_ref(), false);
    let mut bodies = if op.update.block_ops.is_empty() {
        None
    } else {
        let keys: Vec<Key> = rows.iter().map(|(key, _)| key.clone()).collect();
        Some(block_update::plan_and_apply(
            graph,
            &keys,
            &op.update.block_ops,
        )?)
    };
    let documents: Vec<DocRef> = rows.iter().map(|(key, _)| doc_ref(graph, key)).collect();
    block_update::check_document_expect("update", op.expect, &documents)?;
    let mut changes = Vec::new();
    for (key, mut mapping) in rows {
        update::apply(&op.update, &mut mapping);
        strip_reserved(&mut mapping);
        let body = bodies.as_mut().and_then(|map| map.remove(&key));
        let markdown = render_with_frontmatter(graph, &key, body, mapping);
        changes.push((key, markdown));
    }
    Ok(Outcome::Update { changes })
}

fn execute_delete(op: &DeleteOp, graph: &Graph) -> Result<Outcome, EvalError> {
    let rows = select(Some(&op.filter), graph);
    let rows = apply_sort_and_limit(rows, op.sort.as_ref(), op.limit.as_ref(), false);
    let documents: Vec<DocRef> = rows.iter().map(|(key, _)| doc_ref(graph, key)).collect();
    block_update::check_document_expect("delete", op.expect, &documents)?;
    let removed = rows.into_iter().map(|(k, _)| k).collect();
    Ok(Outcome::Delete { removed })
}

fn doc_ref(graph: &Graph, key: &Key) -> DocRef {
    DocRef {
        key: key.to_string(),
        title: graph.get_key_title(key).unwrap_or_else(|| key.to_string()),
    }
}

fn render_with_frontmatter(
    graph: &Graph,
    key: &Key,
    body: Option<Tree>,
    mapping: Mapping,
) -> String {
    let mut tree = body.unwrap_or_else(|| graph.collect(key));
    let frontmatter = if mapping.is_empty() {
        None
    } else {
        Some(mapping)
    };
    tree.node = Node::Document(key.clone(), frontmatter);
    tree.iter().to_text(&key.parent(), graph.format_options())
}
