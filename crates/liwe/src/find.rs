use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use serde::Serialize;
use serde_yaml::Mapping;
use crate::graph::{Graph, GraphContext};
use crate::model::node::{NodeIter, NodePointer};
use crate::model::{Key, NodeId};
use crate::query::{self, Filter, InclusionAnchor, Projection, ReferenceAnchor, Sort};
use crate::query::frontmatter::strip_reserved;
use crate::query::project::shape;
use crate::query::sort::sort_in_place;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentDocumentInfo {
    pub key: String,
    pub title: String,
    pub section_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindResult {
    pub key: String,
    pub title: String,
    pub includes_count: usize,
    pub included_by_count: usize,
    pub references_count: usize,
    pub referenced_by_count: usize,
    pub included_by: Vec<ParentDocumentInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<Mapping>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindOutput {
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub total: usize,
    pub results: Vec<FindResult>,
}

#[derive(Debug, Clone, Default)]
pub struct FindOptions {
    pub query: Option<String>,
    pub refs_to: Option<Key>,
    pub refs_from: Option<Key>,
    pub filter: Option<Filter>,
    pub limit: Option<usize>,
    pub sort: Option<Sort>,
    pub project: Option<Projection>,
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

        let candidates = match (&options.sort, &options.query) {
            (Some(_), _) => self.fuzzy_filter_only(candidates, options.query.as_deref()),
            (None, _) => candidates,
        };

        let ordered = if let Some(s) = &options.sort {
            self.sort_by_frontmatter(candidates, s)
        } else {
            self.order(candidates, Order::from_options(options))
        };

        let total = ordered.len();
        let take = options.limit.filter(|&l| l > 0).unwrap_or(total);
        let results: Vec<FindResult> = ordered
            .into_iter()
            .take(take)
            .map(|key| self.build_result(&key, options.project.as_ref()))
            .collect();
        let limit = options.limit.filter(|&l| l > 0 && l < total);

        FindOutput {
            query: options.query.clone(),
            limit,
            total,
            results,
        }
    }

    fn fuzzy_filter_only(&self, candidates: Vec<Key>, query: Option<&str>) -> Vec<Key> {
        let Some(q) = query else {
            return candidates;
        };
        let matcher = SkimMatcherV2::default();
        candidates
            .into_iter()
            .filter(|key| {
                let title = self.graph.get_key_title(key).unwrap_or_default();
                let text = format!("{} {}", key, title);
                matcher.fuzzy_match(&text, q).unwrap_or(0) > 0
            })
            .collect()
    }

    fn sort_by_frontmatter(&self, candidates: Vec<Key>, sort: &Sort) -> Vec<Key> {
        let mut rows: Vec<(Key, Mapping)> = candidates
            .into_iter()
            .map(|k| {
                let m = self.graph.frontmatter(&k).cloned().unwrap_or_default();
                (k, m)
            })
            .collect();
        sort_in_place(&mut rows, sort);
        rows.into_iter().map(|(k, _)| k).collect()
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

    fn build_result(&self, key: &Key, project: Option<&Projection>) -> FindResult {
        let title = self.graph.get_key_title(key).unwrap_or_default();
        let included_by = self.get_parent_documents(key);
        let frontmatter = project.map(|p| {
            let m = self.graph.frontmatter(key).cloned().unwrap_or_default();
            shape(p, &m)
        }).or_else(|| {
            let mut m = self.graph.frontmatter(key).cloned()?;
            strip_reserved(&mut m);
            if m.is_empty() {
                None
            } else {
                Some(m)
            }
        });

        FindResult {
            key: key.to_string(),
            title,
            includes_count: self.graph.get_inclusion_edges_in(key).len(),
            included_by_count: self.graph.get_inclusion_edges_to(key).len(),
            references_count: self.graph.get_reference_edges_in(key).len(),
            referenced_by_count: self.graph.get_reference_edges_to(key).len(),
            included_by,
            frontmatter,
        }
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

        let mut parents: Vec<ParentDocumentInfo> =
            parents.into_iter().unique_by(|p| p.key.clone()).collect();
        parents.sort_by(|a, b| a.key.cmp(&b.key));
        parents
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
    if let Some(target) = &options.refs_to {
        conjuncts.push(Filter::Or(vec![
            Filter::Includes(Box::new(InclusionAnchor::with_max(target.to_string(), 1))),
            Filter::References(Box::new(ReferenceAnchor::with_max(target.to_string(), 1))),
        ]));
    }
    if let Some(source) = &options.refs_from {
        conjuncts.push(Filter::Or(vec![
            Filter::IncludedBy(Box::new(InclusionAnchor::with_max(source.to_string(), 1))),
            Filter::ReferencedBy(Box::new(ReferenceAnchor::with_max(source.to_string(), 1))),
        ]));
    }
    if conjuncts.is_empty() {
        None
    } else {
        Some(Filter::And(conjuncts))
    }
}
