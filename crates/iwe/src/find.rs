pub mod output;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use liwe::graph::{Graph, GraphContext};
use liwe::model::node::{NodeIter, NodePointer};
use liwe::model::{Key, NodeId};

use output::{FindOutput, FindResult, ParentDocumentInfo};

#[derive(Debug, Clone, Default)]
pub struct FindOptions {
    pub query: Option<String>,
    pub roots: bool,
    pub refs_to: Option<Key>,
    pub refs_from: Option<Key>,
    pub limit: Option<usize>,
}

pub struct DocumentFinder<'a> {
    graph: &'a Graph,
}

impl<'a> DocumentFinder<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    pub fn find(&self, options: &FindOptions) -> FindOutput {
        let matcher = SkimMatcherV2::default();

        let mut results: Vec<(Key, i64)> = self
            .graph
            .keys()
            .into_iter()
            .filter_map(|key| {
                if options.roots && !self.is_root(&key) {
                    return None;
                }
                if let Some(ref target) = options.refs_to {
                    if !self.references(&key, target) {
                        return None;
                    }
                }
                if let Some(ref source) = options.refs_from {
                    if !self.references(source, &key) {
                        return None;
                    }
                }

                let title = self.graph.get_key_title(&key).unwrap_or_default();
                let search_text = format!("{} {}", key, title);

                let score = options
                    .query
                    .as_ref()
                    .map(|q| matcher.fuzzy_match(&search_text, q).unwrap_or(0))
                    .unwrap_or(self.node_rank(&key) as i64);

                if options.query.is_some() && score == 0 {
                    return None;
                }

                Some((key, score))
            })
            .collect();

        results.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let total = results.len();
        let results: Vec<FindResult> = if let Some(limit) = options.limit {
            results
                .into_iter()
                .take(limit)
                .map(|(key, _)| self.build_result(&key))
                .collect()
        } else {
            results
                .into_iter()
                .map(|(key, _)| self.build_result(&key))
                .collect()
        };

        let limit = options.limit.filter(|&l| l < total);

        FindOutput {
            query: options.query.clone(),
            limit,
            total,
            results,
        }
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
            incoming_refs: self.graph.get_block_references_to(key).len()
                + self.graph.get_inline_references_to(key).len(),
            outgoing_refs: self.graph.get_block_references_in(key).len(),
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
        self.graph.get_block_references_to(key).is_empty()
    }

    fn node_rank(&self, key: &Key) -> usize {
        self.graph.get_inline_references_to(key).len()
            + self.graph.get_block_references_to(key).len()
    }

    fn references(&self, source: &Key, target: &Key) -> bool {
        let block_refs = self.graph.get_block_references_in(source);
        for ref_id in block_refs {
            if let Some(ref_key) = self.graph.graph_node(ref_id).ref_key() {
                if &ref_key == target {
                    return true;
                }
            }
        }

        if let Some(node_id) = self.graph.get_node_id(source) {
            let sub_nodes = self.graph.node(node_id).get_all_sub_nodes();
            for sub_node_id in sub_nodes {
                if let Some(line_id) = self.graph.graph_node(sub_node_id).line_id() {
                    let line = self.graph.get_line(line_id);
                    for ref_key in line.ref_keys() {
                        if &ref_key == target {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn get_parent_documents(&self, key: &Key) -> Vec<ParentDocumentInfo> {
        let refs = self.graph.get_block_references_to(key);
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
