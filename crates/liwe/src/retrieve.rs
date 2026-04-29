use std::collections::HashSet;

use itertools::Itertools;
use serde::Serialize;
use crate::graph::{Graph, GraphContext};
use crate::model::node::{NodeIter, NodePointer};
use crate::model::{Key, NodeId};
use crate::selector::Selector;

#[derive(Debug, Clone, Serialize)]
pub struct ParentDocumentInfo {
    pub key: String,
    pub title: String,
    pub section_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacklinkInfo {
    pub key: String,
    pub title: String,
    pub section_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChildDocumentInfo {
    pub key: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentOutput {
    pub key: String,
    pub title: String,
    pub content: String,
    pub parent_documents: Vec<ParentDocumentInfo>,
    pub child_documents: Vec<ChildDocumentInfo>,
    pub backlinks: Vec<BacklinkInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetrieveOutput {
    pub documents: Vec<DocumentOutput>,
}

#[derive(Debug, Clone, Default)]
pub struct RetrieveOptions {
    pub depth: u8,
    pub context: u8,
    pub links: bool,
    pub backlinks: bool,
    pub exclude: HashSet<Key>,
    pub no_content: bool,
    pub selector: Selector,
}

pub struct DocumentReader<'a> {
    graph: &'a Graph,
}

impl<'a> DocumentReader<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    pub fn retrieve(&self, key: &Key, options: &RetrieveOptions) -> RetrieveOutput {
        let mut documents = Vec::new();
        let mut seen_keys = HashSet::new();

        let keys_to_process = self.collect_document_keys(key, options);

        for doc_key in keys_to_process {
            if seen_keys.contains(&doc_key) || options.exclude.contains(&doc_key) {
                continue;
            }
            seen_keys.insert(doc_key.clone());

            let doc_output = self.build_document_output(&doc_key, options);
            documents.push(doc_output);
        }

        RetrieveOutput { documents }
    }

    pub fn retrieve_many(&self, keys: &[Key], options: &RetrieveOptions) -> RetrieveOutput {
        let effective_keys: Vec<Key> = if options.selector.is_empty() {
            keys.to_vec()
        } else {
            let selector_set = options.selector.resolve(self.graph);
            if keys.is_empty() {
                let mut v: Vec<Key> = selector_set.into_iter().collect();
                v.sort();
                v
            } else {
                keys.iter()
                    .filter(|k| selector_set.contains(k))
                    .cloned()
                    .collect()
            }
        };

        let mut documents = Vec::new();
        let mut seen_keys = HashSet::new();

        for key in &effective_keys {
            let keys_to_process = self.collect_document_keys(key, options);

            for doc_key in keys_to_process {
                if seen_keys.contains(&doc_key) || options.exclude.contains(&doc_key) {
                    continue;
                }
                seen_keys.insert(doc_key.clone());

                let doc_output = self.build_document_output(&doc_key, options);
                documents.push(doc_output);
            }
        }

        RetrieveOutput { documents }
    }

    fn collect_document_keys(&self, key: &Key, options: &RetrieveOptions) -> Vec<Key> {
        let mut result = vec![key.clone()];

        if options.depth > 0 {
            let expanded_keys = self.get_expanded_keys(key, options.depth);
            for expanded_key in expanded_keys {
                if expanded_key != *key {
                    result.push(expanded_key);
                }
            }
        }

        if options.context > 0 {
            let context_keys = self.collect_context(key, options.context);
            for context_key in context_keys {
                if context_key != *key && !result.contains(&context_key) {
                    result.push(context_key);
                }
            }

            if options.depth > 0 {
                let sub_doc_parents = self.collect_sub_document_parents(key, options.context);
                for parent_key in sub_doc_parents {
                    if parent_key != *key && !result.contains(&parent_key) {
                        result.push(parent_key);
                    }
                }
            }
        }

        if options.links {
            let linked_keys = self.get_inline_referenced_keys(key);
            for linked_key in linked_keys {
                if linked_key != *key && !result.contains(&linked_key) {
                    result.push(linked_key);
                }
            }
        }

        result
    }

    fn collect_context(&self, key: &Key, levels: u8) -> Vec<Key> {
        if levels == 0 {
            return vec![];
        }

        let mut parents = Vec::new();
        let refs = self.graph.get_inclusion_edges_to(key);

        for ref_id in refs {
            let node = self.graph.node(ref_id);

            if let Some(doc_node) = node.to_document() {
                if let Some(parent_key) = doc_node.document_key() {
                    if !parents.contains(&parent_key) {
                        parents.push(parent_key.clone());

                        if levels > 1 {
                            let grandparents = self.collect_context(&parent_key, levels - 1);
                            for grandparent_key in grandparents {
                                if !parents.contains(&grandparent_key) {
                                    parents.push(grandparent_key);
                                }
                            }
                        }
                    }
                }
            }
        }

        parents
    }

    fn collect_sub_document_parents(&self, key: &Key, levels: u8) -> Vec<Key> {
        let mut parents = Vec::new();
        let sub_docs = self.graph.get_inclusion_edges_in(key);

        for ref_id in sub_docs {
            if let Some(sub_key) = self.graph.graph_node(ref_id).ref_key() {
                let sub_parents = self.collect_context(&sub_key, levels);
                for parent_key in sub_parents {
                    if parent_key != *key && !parents.contains(&parent_key) {
                        parents.push(parent_key);
                    }
                }
            }
        }

        parents
    }

    fn get_inline_referenced_keys(&self, key: &Key) -> Vec<Key> {
        let mut keys = Vec::new();

        if let Some(node_id) = self.graph.get_node_id(key) {
            let sub_nodes = self.graph.node(node_id).get_all_sub_nodes();

            for sub_node_id in sub_nodes {
                if let Some(line_id) = self.graph.graph_node(sub_node_id).line_id() {
                    let line = self.graph.get_line(line_id);
                    for ref_key in line.ref_keys() {
                        if !keys.contains(&ref_key) {
                            keys.push(ref_key);
                        }
                    }
                }
            }
        }

        keys
    }

    fn get_expanded_keys(&self, key: &Key, depth: u8) -> Vec<Key> {
        let mut keys = Vec::new();

        let refs = self.graph.get_inclusion_edges_in(key);
        for ref_id in refs {
            if let Some(ref_key) = self.graph.graph_node(ref_id).ref_key() {
                if !keys.contains(&ref_key) {
                    keys.push(ref_key.clone());

                    if depth > 1 {
                        let sub_keys = self.get_expanded_keys(&ref_key, depth - 1);
                        for sub_key in sub_keys {
                            if !keys.contains(&sub_key) {
                                keys.push(sub_key);
                            }
                        }
                    }
                }
            }
        }

        keys
    }

    fn build_document_output(&self, key: &Key, options: &RetrieveOptions) -> DocumentOutput {
        let title = self
            .graph
            .get_key_title(key)
            .unwrap_or_else(|| key.to_string());

        let content = if options.no_content {
            String::new()
        } else {
            self.get_document_content(key)
        };
        let parent_documents = self.get_parent_documents(key);

        let child_documents = if options.no_content {
            self.get_child_documents(key)
        } else {
            Vec::new()
        };

        let backlinks = if options.backlinks {
            self.get_backlinks(key)
        } else {
            Vec::new()
        };

        DocumentOutput {
            key: key.to_string(),
            title,
            content,
            parent_documents,
            child_documents,
            backlinks,
        }
    }

    fn get_document_content(&self, key: &Key) -> String {
        self.graph.to_markdown_skip_frontmatter(key)
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

    fn get_child_documents(&self, key: &Key) -> Vec<ChildDocumentInfo> {
        let refs = self.graph.get_inclusion_edges_in(key);
        let mut children = Vec::new();

        for ref_id in refs {
            if let Some(ref_key) = self.graph.graph_node(ref_id).ref_key() {
                let title = self
                    .graph
                    .get_key_title(&ref_key)
                    .unwrap_or_else(|| ref_key.to_string());

                children.push(ChildDocumentInfo {
                    key: ref_key.to_string(),
                    title,
                });
            }
        }

        children.into_iter().unique_by(|c| c.key.clone()).collect()
    }

    fn get_backlinks(&self, key: &Key) -> Vec<BacklinkInfo> {
        let inline_refs = self.graph.get_reference_edges_to(key);

        let mut backlinks = Vec::new();
        let mut seen_keys = HashSet::new();

        for ref_id in inline_refs {
            let node = self.graph.node(ref_id);

            if let Some(doc_node) = node.to_document() {
                if let Some(doc_key) = doc_node.document_key() {
                    if seen_keys.contains(&doc_key) {
                        continue;
                    }
                    seen_keys.insert(doc_key.clone());

                    let title = self
                        .graph
                        .get_key_title(&doc_key)
                        .unwrap_or_else(|| doc_key.to_string());

                    let section_path = self.get_section_path(ref_id);

                    backlinks.push(BacklinkInfo {
                        key: doc_key.to_string(),
                        title,
                        section_path,
                    });
                }
            }
        }

        backlinks
    }

    fn get_section_path(&self, node_id: NodeId) -> Vec<String> {
        let mut path = Vec::new();
        let mut current = self.graph.node(node_id);

        while let Some(parent) = current.to_parent() {
            if parent.is_section() && parent.is_header() {
                if let Some(grandparent) = parent.to_parent() {
                    if !grandparent.is_document() {
                        let text = parent.plain_text().trim().to_string();
                        path.push(text);
                    }
                }
            }
            if parent.is_document() {
                break;
            }
            current = parent;
        }

        path.reverse();
        path
    }
}
