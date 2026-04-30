use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use serde::Serialize;
use crate::graph::{Graph, GraphContext};
use crate::model::node::{NodeIter, NodePointer};
use crate::model::{Key, NodeId};
use crate::query::{
    self, CountArg, Filter, InclusionAnchor, NumExpr, ReferenceAnchor,
};

#[derive(Debug, Clone, Serialize)]
pub struct ParentDocumentInfo {
    pub key: String,
    pub title: String,
    pub section_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindResult {
    pub key: String,
    pub title: String,
    pub display_title: String,
    pub is_root: bool,
    pub incoming_refs: usize,
    pub outgoing_refs: usize,
    pub parent_documents: Vec<ParentDocumentInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindOutput {
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub total: usize,
    pub results: Vec<FindResult>,
}

#[derive(Debug, Clone, Default)]
pub struct FindOptions {
    pub query: Option<String>,
    pub roots: bool,
    pub refs_to: Option<Key>,
    pub refs_from: Option<Key>,
    pub filter: Option<Filter>,
    pub limit: Option<usize>,
}

pub struct DocumentFinder<'a> {
    graph: &'a Graph,
}

enum Order<'a> {
    Fuzzy(&'a str),
    Rank,
}

impl<'a> Order<'a> {
    fn from_options(options: &'a FindOptions) -> Order<'a> {
        match &options.query {
            Some(q) => Order::Fuzzy(q),
            None => Order::Rank,
        }
    }
}

impl<'a> DocumentFinder<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    pub fn find(&self, options: &FindOptions) -> FindOutput {
        let candidates = self.candidates(options);
        let ordered = self.order(candidates, Order::from_options(options));

        let total = ordered.len();
        let take = options.limit.unwrap_or(total);
        let results: Vec<FindResult> = ordered
            .into_iter()
            .take(take)
            .map(|key| self.build_result(&key))
            .collect();
        let limit = options.limit.filter(|&l| l < total);

        FindOutput {
            query: options.query.clone(),
            limit,
            total,
            results,
        }
    }

    fn candidates(&self, options: &FindOptions) -> Vec<Key> {
        match build_filter(options) {
            None => self.graph.keys(),
            Some(f) => query::evaluate(&f, self.graph),
        }
    }

    fn order(&self, candidates: Vec<Key>, order: Order<'_>) -> Vec<Key> {
        let mut scored: Vec<(Key, i64)> = match order {
            Order::Fuzzy(q) => {
                let matcher = SkimMatcherV2::default();
                candidates
                    .into_iter()
                    .filter_map(|key| {
                        let title = self.graph.get_key_title(&key).unwrap_or_default();
                        let text = format!("{} {}", key, title);
                        let score = matcher.fuzzy_match(&text, q).unwrap_or(0);
                        (score > 0).then_some((key, score))
                    })
                    .collect()
            }
            Order::Rank => candidates
                .into_iter()
                .map(|key| {
                    let rank = self.node_rank(&key) as i64;
                    (key, rank)
                })
                .collect(),
        };
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        scored.into_iter().map(|(k, _)| k).collect()
    }

    fn build_result(&self, key: &Key) -> FindResult {
        let title = self.graph.get_key_title(key).unwrap_or_default();
        let parent_documents = self.get_parent_documents(key);
        let display_title = Self::render_display_title(&title, &parent_documents);

        FindResult {
            key: key.to_string(),
            title,
            display_title,
            is_root: self.is_root(key),
            incoming_refs: self.graph.get_inclusion_edges_to(key).len()
                + self.graph.get_reference_edges_to(key).len(),
            outgoing_refs: self.graph.get_inclusion_edges_in(key).len(),
            parent_documents,
        }
    }

    fn render_display_title(title: &str, parent_documents: &[ParentDocumentInfo]) -> String {
        if parent_documents.is_empty() {
            title.to_string()
        } else {
            let parents = parent_documents
                .iter()
                .map(|p| format!("↖{}", p.title))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} {}", title, parents)
        }
    }

    fn is_root(&self, key: &Key) -> bool {
        self.graph.get_inclusion_edges_to(key).is_empty()
    }

    fn node_rank(&self, key: &Key) -> usize {
        self.graph.get_reference_edges_to(key).len()
            + self.graph.get_inclusion_edges_to(key).len()
    }

    fn get_parent_documents(&self, key: &Key) -> Vec<ParentDocumentInfo> {
        let refs = self.graph.get_inclusion_edges_to(key);
        let mut parents = Vec::new();

        for ref_id in refs {
            let node = self.graph.node(ref_id);

            if let Some(doc_node) = node.to_document() {
                if let Some(doc_key) = doc_node.document_key() {
                    let title = self
                        .graph
                        .get_key_title(&doc_key)
                        .unwrap_or_else(|| doc_key.to_string());

                    let section_path = self.get_section_path(ref_id);

                    parents.push(ParentDocumentInfo {
                        key: doc_key.to_string(),
                        title,
                        section_path,
                    });
                }
            }
        }

        parents.into_iter().unique_by(|p| p.key.clone()).collect()
    }

    fn get_section_path(&self, node_id: NodeId) -> Vec<String> {
        let mut path = Vec::new();
        let mut current = self.graph.node(node_id);

        while let Some(parent) = current.to_parent() {
            if parent.is_section()
                && parent
                    .to_parent()
                    .map(|p| p.is_document())
                    .unwrap_or(true)
            {
                let text = parent.plain_text().trim().to_string();
                path.push(text);
            }
            if parent.is_document() {
                break;
            }
            current = parent;
        }
        path
    }
}

fn build_filter(options: &FindOptions) -> Option<Filter> {
    let mut conjuncts: Vec<Filter> = options.filter.clone().into_iter().collect();
    if options.roots {
        conjuncts.push(Filter::IncludedByCount(CountArg::direct(NumExpr::eq(0))));
    }
    if let Some(target) = &options.refs_to {
        conjuncts.push(Filter::Or(vec![
            Filter::Includes(vec![InclusionAnchor::with_max(target.to_string(), 1)]),
            Filter::References(vec![ReferenceAnchor::with_max(target.to_string(), 1)]),
        ]));
    }
    if let Some(source) = &options.refs_from {
        conjuncts.push(Filter::Or(vec![
            Filter::IncludedBy(vec![InclusionAnchor::with_max(source.to_string(), 1)]),
            Filter::ReferencedBy(vec![ReferenceAnchor::with_max(source.to_string(), 1)]),
        ]));
    }
    if conjuncts.is_empty() {
        None
    } else {
        Some(Filter::And(conjuncts))
    }
}
