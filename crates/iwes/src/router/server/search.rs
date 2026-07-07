use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use liwe::{
    graph::{Graph, GraphContext},
    model::{
        node::{NodeIter, NodePointer},
        Key, NodeId,
    },
};
use rayon::prelude::*;

#[derive(Clone, Debug, Default)]
pub struct SearchPath {
    pub search_text: String,
    pub node_rank: usize,
    pub key: Key,
    pub root: bool,
    pub line: u32,
    pub title: String,
    pub parent_titles: Vec<String>,
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
            .nodes()
            .par_iter()
            .filter(|graph_node| graph_node.is_section())
            .filter_map(|graph_node| {
                let node_id = graph_node.id();
                let node = graph_ctx.node(node_id);

                let parent_is_document = node.to_parent().map(|p| p.is_document()).unwrap_or(false);
                if !parent_is_document {
                    return None;
                }

                let key = graph_ctx.get_node_key(node_id)?;

                let title = graph_ctx.get_text(node_id).trim().to_string();

                if title.is_empty() {
                    return None;
                }

                let parent_titles: Vec<String> = graph_ctx
                    .get_inclusion_edges_to(&key)
                    .iter()
                    .filter_map(|ref_id| {
                        let parent_key = graph_ctx.node(*ref_id).to_document()?.document_key()?;
                        graph_ctx.get_ref_text(&parent_key)
                    })
                    .sorted()
                    .collect();

                let has_parents = !parent_titles.is_empty();

                Some(SearchPath {
                    search_text: render_search_text(&title, &parent_titles, &key),
                    node_rank: node_rank(graph_ctx, node_id),
                    key,
                    root: !has_parents,
                    line: graph_ctx.node_line_number(node_id).unwrap_or(0) as u32,
                    title,
                    parent_titles,
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .sorted_by(|a, b| {
                b.node_rank
                    .cmp(&a.node_rank)
                    .then_with(|| a.key.cmp(&b.key))
                    .then_with(|| a.line.cmp(&b.line))
            })
            .unique_by(|p| (p.key.clone(), p.line))
            .collect::<Vec<_>>();
    }

    pub fn search(&self, query: &str, graph: &Graph) -> Vec<SearchPath> {
        if query.is_empty() {
            return self
                .paths
                .iter()
                .sorted_by(|path_a, path_b| {
                    path_b
                        .node_rank
                        .cmp(&path_a.node_rank)
                        .then_with(|| path_a.search_text.len().cmp(&path_b.search_text.len()))
                        .then_with(|| path_a.key.cmp(&path_b.key))
                        .then_with(|| path_a.line.cmp(&path_b.line))
                })
                .take(100)
                .cloned()
                .collect_vec();
        }

        let matcher = SkimMatcherV2::default();
        let bm25_scores = graph.search_scores(query);

        let scored: Vec<(&SearchPath, f64, f64)> = self
            .paths
            .par_iter()
            .map(|path| {
                let fuzzy = matcher.fuzzy_match(&path.search_text, query).unwrap_or(0) as f64;
                let bm25 = bm25_scores.get(&path.key).copied().unwrap_or(0.0) as f64;
                (path, fuzzy, bm25)
            })
            .collect();

        let fuzzy_range = min_max(scored.iter().map(|(_, fuzzy, _)| *fuzzy));
        let bm25_range = min_max(scored.iter().map(|(_, _, bm25)| *bm25));

        scored
            .into_iter()
            .map(|(path, fuzzy, bm25)| {
                let fuzzy_n = normalize(fuzzy, fuzzy_range);
                let bm25_n = normalize(bm25, bm25_range);
                let combined = BM25_BLEND_WEIGHT * bm25_n + (1.0 - BM25_BLEND_WEIGHT) * fuzzy_n;
                (path, combined)
            })
            .sorted_by(|(path_a, combined_a), (path_b, combined_b)| {
                combined_b
                    .partial_cmp(combined_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| path_a.search_text.len().cmp(&path_b.search_text.len()))
                    .then_with(|| path_b.node_rank.cmp(&path_a.node_rank))
                    .then_with(|| path_a.key.cmp(&path_b.key))
                    .then_with(|| path_a.line.cmp(&path_b.line))
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

const BM25_BLEND_WEIGHT: f64 = 0.5;

fn min_max(values: impl Iterator<Item = f64>) -> (f64, f64) {
    values.fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
        (min.min(value), max.max(value))
    })
}

fn normalize(value: f64, (min, max): (f64, f64)) -> f64 {
    if max > min {
        (value - min) / (max - min)
    } else {
        0.0
    }
}

fn render_search_text(title: &str, parent_titles: &[String], key: &Key) -> String {
    let mut all_titles = vec![title.to_string()];
    all_titles.extend(parent_titles.iter().cloned());
    all_titles.push(key.relative_path.to_string());
    all_titles
        .join(" ")
        .chars()
        .filter(|c| c.is_alphabetic() || c.is_numeric() || c.is_whitespace() || *c == '/')
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
        .map(|key| graph.get_reference_edges_to(&key).len())
        .unwrap_or(0);

    let block_refs_count = graph
        .node(id)
        .to_document()
        .and_then(|doc| doc.document_key())
        .map(|key| graph.get_inclusion_edges_to(&key).len())
        .unwrap_or(0);

    inline_refs_count + block_refs_count
}
