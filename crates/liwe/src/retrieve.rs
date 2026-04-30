use std::collections::HashSet;

use itertools::Itertools;
use serde::Serialize;
use crate::graph::walk::{
    ancestors_inclusion, descendants_inclusion, outbound_reference,
};
use crate::graph::{Graph, GraphContext};
use crate::model::node::{NodeIter, NodePointer};
use crate::model::{Key, NodeId};
use crate::query::{self, Filter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentDocumentInfo {
    pub key: String,
    pub title: String,
    pub section_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkInfo {
    pub key: String,
    pub title: String,
    pub section_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildDocumentInfo {
    pub key: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentOutput {
    pub key: String,
    pub title: String,
    pub content: String,
    pub included_by: Vec<ParentDocumentInfo>,
    pub includes: Vec<ChildDocumentInfo>,
    pub referenced_by: Vec<BacklinkInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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
    pub filter: Option<Filter>,
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
        let effective_keys: Vec<Key> = match (&options.filter, keys.is_empty()) {
            (Some(f), true) => query::evaluate(f, self.graph),
            (Some(f), false) => {
                let set: HashSet<Key> =
                    query::evaluate(f, self.graph).into_iter().collect();
                keys.iter().filter(|k| set.contains(k)).cloned().collect()
            }
            (None, _) => keys.to_vec(),
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
        let mut result: Vec<Key> = vec![key.clone()];
        let mut seen: HashSet<Key> = HashSet::from([key.clone()]);

        let push = |k: Key, result: &mut Vec<Key>, seen: &mut HashSet<Key>| {
            if seen.insert(k.clone()) {
                result.push(k);
            }
        };

        if options.depth > 0 {
            let mut desc: Vec<(Key, u32)> =
                descendants_inclusion(self.graph, key, options.depth as u32)
                    .into_iter()
                    .collect();
            desc.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            for (k, _) in desc {
                push(k, &mut result, &mut seen);
            }
        }

        if options.context > 0 {
            let mut anc: Vec<(Key, u32)> =
                ancestors_inclusion(self.graph, key, options.context as u32)
                    .into_iter()
                    .collect();
            anc.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            for (k, _) in anc {
                push(k, &mut result, &mut seen);
            }

            if options.depth > 0 {
                let mut sub_doc_keys: Vec<Key> =
                    descendants_inclusion(self.graph, key, 1)
                        .into_keys()
                        .collect();
                sub_doc_keys.sort();
                for sub_key in sub_doc_keys {
                    let mut sub_anc: Vec<(Key, u32)> =
                        ancestors_inclusion(self.graph, &sub_key, options.context as u32)
                            .into_iter()
                            .collect();
                    sub_anc.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
                    for (k, _) in sub_anc {
                        push(k, &mut result, &mut seen);
                    }
                }
            }
        }

        if options.links {
            let mut links: Vec<(Key, u32)> = outbound_reference(self.graph, key, 1)
                .into_iter()
                .collect();
            links.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            for (k, _) in links {
                push(k, &mut result, &mut seen);
            }
        }

        result
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
        let included_by = self.get_parent_documents(key);

        let includes = if options.no_content {
            self.get_child_documents(key)
        } else {
            Vec::new()
        };

        let referenced_by = if options.backlinks {
            self.get_backlinks(key)
        } else {
            Vec::new()
        };

        DocumentOutput {
            key: key.to_string(),
            title,
            content,
            included_by,
            includes,
            referenced_by,
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

        let mut parents: Vec<ParentDocumentInfo> =
            parents.into_iter().unique_by(|p| p.key.clone()).collect();
        parents.sort_by(|a, b| a.key.cmp(&b.key));
        parents
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

        let mut children: Vec<ChildDocumentInfo> =
            children.into_iter().unique_by(|c| c.key.clone()).collect();
        children.sort_by(|a, b| a.key.cmp(&b.key));
        children
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

        backlinks.sort_by(|a, b| a.key.cmp(&b.key));
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
