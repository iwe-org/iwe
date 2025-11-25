use std::cmp::Ordering;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use liwe::{
    graph::{path::NodePath, Graph, GraphContext},
    model::{Key, NodeId},
};
use rayon::prelude::*;

#[derive(Clone, Debug, Default)]
pub struct SearchPath {
    pub search_text: String,
    pub node_rank: usize,
    pub key: Key,
    pub root: bool,
    pub line: u32,
    pub path: NodePath,
}

#[derive(Clone, Default)]
pub struct SearchIndex {
    paths: Vec<SearchPath>,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, graph: &Graph) {
        let graph_ctx: &Graph = graph;
        self.paths = graph
            .paths()
            .par_iter()
            .map(|path| SearchPath {
                search_text: render_search_text(path, graph_ctx),
                node_rank: node_rank(graph_ctx, path.last_id()),
                key: graph_ctx.get_node_key(path.target()),
                root: path.ids().len() == 1,
                line: graph_ctx.node_line_number(path.target()).unwrap_or(0) as u32,
                path: path.clone(),
            })
            .collect::<Vec<_>>()
            .into_iter()
            .sorted_by(|a, b| {
                let primary = b.node_rank.cmp(&a.node_rank);
                if primary == Ordering::Equal {
                    a.key.cmp(&b.key)
                } else {
                    primary
                }
            })
            .collect::<Vec<_>>();
    }

    pub fn search(&self, query: &str) -> Vec<SearchPath> {
        let matcher = SkimMatcherV2::default();
        assert_eq!(None, matcher.fuzzy_match("abc", "abx"));

        self.paths
            .par_iter()
            .map(|path| {
                (
                    path,
                    matcher.fuzzy_match(&path.search_text, query).unwrap_or(0),
                )
            })
            .collect::<Vec<_>>()
            .into_iter()
            .sorted_by(|(path_a, rank_a), (path_b, rank_b)| {
                if query.is_empty() {
                    path_b
                        .node_rank
                        .cmp(&path_a.node_rank)
                        .then_with(|| path_a.search_text.len().cmp(&path_b.search_text.len()))
                } else {
                    rank_b
                        .cmp(rank_a)
                        .then_with(|| path_a.search_text.len().cmp(&path_b.search_text.len()))
                        .then_with(|| path_b.node_rank.cmp(&path_a.node_rank))
                }
            })
            .map(|(path, _)| path)
            .take(100)
            .cloned()
            .collect_vec()
    }

    pub fn paths(&self) -> Vec<SearchPath> {
        self.paths.clone()
    }
}

fn render_search_text(path: &NodePath, context: impl GraphContext) -> String {
    path.ids()
        .iter()
        .map(|id| context.get_text(*id).trim().to_string())
        .collect_vec()
        .join(" ")
        .chars()
        .filter(|c| c.is_alphabetic() || c.is_numeric())
        .collect::<String>()
}

fn node_rank(graph: &Graph, id: NodeId) -> usize {
    use liwe::model::node::NodePointer;

    if !graph.node(id).is_primary_section() {
        return 0;
    }

    let inline_refs_count = graph
        .node(id)
        .to_document()
        .and_then(|doc| doc.document_key())
        .map(|key| graph.get_inline_references_to(&key).len())
        .unwrap_or(0);

    let block_refs_count = graph
        .node(id)
        .to_document()
        .and_then(|doc| doc.document_key())
        .map(|key| graph.get_block_references_to(&key).len())
        .unwrap_or(0);

    inline_refs_count + block_refs_count
}
