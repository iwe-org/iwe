use rayon::prelude::*;
use serde_yaml::Mapping;

use crate::graph::{Graph, GraphContext};
use crate::model::node::{Node, NodeIter};
use crate::model::Key;
use crate::query::document::{
    CountOp, DeleteOp, Filter, FindOp, Limit, Operation, Projection, Sort, UpdateOp,
};
use crate::query::eval;
use crate::query::frontmatter::strip_reserved;
use crate::query::project::shape;
use crate::query::sort::sort_in_place;
use crate::query::update::{self, UpdateError};

#[derive(Debug)]
pub enum Outcome {
    Find {
        matches: Vec<FindMatch>,
    },
    Count(usize),
    Update {
        changes: Vec<(Key, String)>,
        failed: Vec<(Key, UpdateError)>,
    },
    Delete {
        removed: Vec<Key>,
    },
}

#[derive(Debug, Clone)]
pub struct FindMatch {
    pub key: Key,
    pub document: Mapping,
}

pub fn execute(op: &Operation, graph: &Graph) -> Outcome {
    match op {
        Operation::Find(find) => execute_find(find, graph),
        Operation::Count(count) => execute_count(count, graph),
        Operation::Update(upd) => execute_update(upd, graph),
        Operation::Delete(del) => execute_delete(del, graph),
    }
}

fn select(filter: Option<&Filter>, graph: &Graph) -> Vec<(Key, Mapping)> {
    let keys: Vec<Key> = match filter {
        None => {
            let mut k = graph.keys();
            k.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
            k
        }
        Some(f) => eval::evaluate(f, graph),
    };
    keys.into_par_iter()
        .map(|key| {
            let mapping = graph.frontmatter(&key).cloned().unwrap_or_default();
            (key, mapping)
        })
        .collect()
}

fn apply_sort_and_limit(
    mut rows: Vec<(Key, Mapping)>,
    sort: Option<&Sort>,
    limit: Option<&Limit>,
) -> Vec<(Key, Mapping)> {
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

fn execute_find(op: &FindOp, graph: &Graph) -> Outcome {
    let rows = select(op.filter.as_ref(), graph);
    let rows = apply_sort_and_limit(rows, op.sort.as_ref(), op.limit.as_ref());
    let matches: Vec<FindMatch> = rows
        .into_iter()
        .map(|(key, mut m)| {
            let document = match &op.project {
                Some(p) => project_doc(p, &m),
                None => {
                    strip_reserved(&mut m);
                    m
                }
            };
            FindMatch { key, document }
        })
        .collect();
    Outcome::Find { matches }
}

fn project_doc(projection: &Projection, m: &Mapping) -> Mapping {
    shape(projection, m)
}

fn execute_count(op: &CountOp, graph: &Graph) -> Outcome {
    let rows = select(op.filter.as_ref(), graph);
    let rows = apply_sort_and_limit(rows, op.sort.as_ref(), op.limit.as_ref());
    Outcome::Count(rows.len())
}

fn execute_update(op: &UpdateOp, graph: &Graph) -> Outcome {
    let rows = select(Some(&op.filter), graph);
    let rows = apply_sort_and_limit(rows, op.sort.as_ref(), op.limit.as_ref());
    let mut changes = Vec::new();
    let mut failed = Vec::new();
    for (key, mut mapping) in rows {
        if let Err(e) = update::apply(&op.update, &mut mapping) {
            failed.push((key, e));
            continue;
        }
        strip_reserved(&mut mapping);
        let markdown = render_with_frontmatter(graph, &key, mapping);
        changes.push((key, markdown));
    }
    Outcome::Update { changes, failed }
}

fn execute_delete(op: &DeleteOp, graph: &Graph) -> Outcome {
    let rows = select(Some(&op.filter), graph);
    let rows = apply_sort_and_limit(rows, op.sort.as_ref(), op.limit.as_ref());
    let removed = rows.into_iter().map(|(k, _)| k).collect();
    Outcome::Delete { removed }
}


fn render_with_frontmatter(graph: &Graph, key: &Key, mapping: Mapping) -> String {
    let mut tree = graph.collect(key);
    let frontmatter = if mapping.is_empty() {
        None
    } else {
        Some(mapping)
    };
    tree.node = Node::Document(key.clone(), frontmatter);
    tree.iter()
        .to_markdown(&key.parent(), &graph.markdown_options())
}
