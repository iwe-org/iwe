use std::collections::HashSet;

use crate::tokens::{
    apply_budget, count_tokens, truncate_to_tokens, truncation_marker, Budget, Truncation,
};
use itertools::Itertools;
use liwe::graph::walk::{
    ancestors_inclusion, descendants_inclusion, inbound_reference, outbound_reference,
};
use liwe::graph::{Graph, GraphContext};
use liwe::model::node::{NodeIter, NodePointer};
use liwe::model::{Key, NodeId};
use liwe::query::{self, Filter};
use serde::Serialize;
use serde_yaml::Mapping;

pub use liwe::query::edges::EdgeRef;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentOutput {
    pub key: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<Mapping>,
    pub content: String,
    pub references: Vec<EdgeRef>,
    pub includes: Vec<EdgeRef>,
    pub referenced_by: Vec<EdgeRef>,
    pub included_by: Vec<EdgeRef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrieveOutput {
    pub documents: Vec<DocumentOutput>,
    #[serde(skip)]
    pub truncation: Truncation,
}

/// Depths for the four expansion directions. Each follows one edge kind out from every seed and
/// pulls the reached documents into the result set. `0` = do not follow; [`UNBOUNDED`] = follow to
/// the graph's edge; any other `N` = follow `N` levels. The edge-list output toggles (`backlinks`,
/// `children`) are independent — they populate output arrays without adding documents.
///
/// `limit` caps the seed set before expansion (the first `N` resolved seeds, `None`/`Some(0)` =
/// unlimited); `max_documents` caps the document count after expansion, trimming periphery
/// documents first (`None`/`Some(0)` = unlimited).
#[derive(Debug, Clone, Default)]
pub struct RetrieveOptions {
    pub includes: u32,
    pub included_by: u32,
    pub references: u32,
    pub referenced_by: u32,
    pub backlinks: bool,
    pub exclude: HashSet<Key>,
    pub children: bool,
    pub frontmatter: bool,
    pub filter: Option<Filter>,
    pub limit: Option<usize>,
    pub max_documents: Option<usize>,
    pub max_tokens: Option<usize>,
    pub max_document_tokens: Option<usize>,
}

/// Sentinel expansion depth meaning "follow this direction with no depth limit".
pub const UNBOUNDED: u32 = u32::MAX;

/// Map an `--expand` / `expand` depth value to an internal expansion depth: `0` is the unbounded
/// sentinel, any other value is that many levels. (The deprecated `-d` / `-c` / `-l` aliases keep
/// their legacy `0` = off meaning and do not go through this mapping.)
pub fn expand_depth(value: u64) -> u32 {
    if value == 0 {
        UNBOUNDED
    } else {
        value.min(u32::MAX as u64) as u32
    }
}

pub struct DocumentReader<'a> {
    graph: &'a Graph,
}

impl<'a> DocumentReader<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    pub fn retrieve(&self, key: &Key, options: &RetrieveOptions) -> RetrieveOutput {
        self.retrieve_seeds(std::slice::from_ref(key), options)
    }

    pub fn retrieve_many(&self, keys: &[Key], options: &RetrieveOptions) -> RetrieveOutput {
        let mut effective_keys: Vec<Key> = match (&options.filter, keys.is_empty()) {
            (Some(f), true) => query::evaluate(f, self.graph),
            (Some(f), false) => {
                let set: HashSet<Key> = query::evaluate(f, self.graph).into_iter().collect();
                keys.iter().filter(|k| set.contains(k)).cloned().collect()
            }
            (None, _) => keys.to_vec(),
        };

        if let Some(cap) = options.limit.filter(|&n| n > 0) {
            effective_keys.truncate(cap);
        }

        self.retrieve_seeds(&effective_keys, options)
    }

    fn retrieve_seeds(&self, seeds: &[Key], options: &RetrieveOptions) -> RetrieveOutput {
        let mut documents = Vec::new();
        let mut seen_keys = HashSet::new();

        for seed in seeds {
            for doc_key in self.collect_document_keys(seed, options) {
                if seen_keys.contains(&doc_key) || options.exclude.contains(&doc_key) {
                    continue;
                }
                seen_keys.insert(doc_key.clone());

                let doc_output = self.build_document_output(&doc_key, options);
                documents.push(doc_output);
            }
        }

        let truncation = apply_document_budget(&mut documents, options);
        RetrieveOutput {
            documents,
            truncation,
        }
    }

    fn collect_document_keys(&self, key: &Key, options: &RetrieveOptions) -> Vec<Key> {
        let mut result: Vec<Key> = vec![key.clone()];
        let mut seen: HashSet<Key> = HashSet::from([key.clone()]);

        let push = |k: Key, result: &mut Vec<Key>, seen: &mut HashSet<Key>| {
            if seen.insert(k.clone()) {
                result.push(k);
            }
        };

        if options.includes > 0 {
            let mut desc: Vec<(Key, u32)> =
                descendants_inclusion(self.graph, key, options.includes)
                    .into_iter()
                    .collect();
            desc.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            for (k, _) in desc {
                push(k, &mut result, &mut seen);
            }
        }

        if options.included_by > 0 {
            let mut anc: Vec<(Key, u32)> =
                ancestors_inclusion(self.graph, key, options.included_by)
                    .into_iter()
                    .collect();
            anc.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            for (k, _) in anc {
                push(k, &mut result, &mut seen);
            }

            if options.includes > 0 {
                let mut sub_doc_keys: Vec<Key> = descendants_inclusion(self.graph, key, 1)
                    .into_keys()
                    .collect();
                sub_doc_keys.sort();
                for sub_key in sub_doc_keys {
                    let mut sub_anc: Vec<(Key, u32)> =
                        ancestors_inclusion(self.graph, &sub_key, options.included_by)
                            .into_iter()
                            .collect();
                    sub_anc.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
                    for (k, _) in sub_anc {
                        push(k, &mut result, &mut seen);
                    }
                }
            }
        }

        if options.references > 0 {
            let mut links: Vec<(Key, u32)> =
                outbound_reference(self.graph, key, options.references)
                    .into_iter()
                    .collect();
            links.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            for (k, _) in links {
                push(k, &mut result, &mut seen);
            }
        }

        if options.referenced_by > 0 {
            let mut back: Vec<(Key, u32)> =
                inbound_reference(self.graph, key, options.referenced_by)
                    .into_iter()
                    .collect();
            back.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            for (k, _) in back {
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

        let content = self.get_document_content(key);
        let included_by = self.get_parent_documents(key);

        let includes = if options.children {
            self.get_child_documents(key)
        } else {
            Vec::new()
        };

        let referenced_by = if options.backlinks {
            self.get_backlinks(key)
        } else {
            Vec::new()
        };

        let references = if options.references > 0 {
            liwe::query::edges::references(self.graph, key)
        } else {
            Vec::new()
        };

        DocumentOutput {
            key: key.to_string(),
            title,
            frontmatter: options
                .frontmatter
                .then(|| self.graph.frontmatter(key).cloned())
                .flatten()
                .filter(|fields| !fields.is_empty()),
            content,
            references: without_excluded(references, &options.exclude),
            includes: without_excluded(includes, &options.exclude),
            referenced_by: without_excluded(referenced_by, &options.exclude),
            included_by: without_excluded(included_by, &options.exclude),
        }
    }

    fn get_document_content(&self, key: &Key) -> String {
        self.graph.to_markdown_skip_frontmatter(key)
    }

    fn get_parent_documents(&self, key: &Key) -> Vec<EdgeRef> {
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

                    parents.push(EdgeRef {
                        key: doc_key.to_string(),
                        title,
                        section_path,
                    });
                }
            }
        }

        let mut parents: Vec<EdgeRef> = parents.into_iter().unique_by(|p| p.key.clone()).collect();
        parents.sort_by(|a, b| a.key.cmp(&b.key));
        parents
    }

    fn get_child_documents(&self, key: &Key) -> Vec<EdgeRef> {
        let refs = self.graph.get_inclusion_edges_in(key);
        let mut children = Vec::new();

        for ref_id in refs {
            if let Some(ref_key) = self.graph.reference_target_key(ref_id) {
                let title = self
                    .graph
                    .get_key_title(&ref_key)
                    .unwrap_or_else(|| ref_key.to_string());

                let section_path = self.get_section_path(ref_id);

                children.push(EdgeRef {
                    key: ref_key.to_string(),
                    title,
                    section_path,
                });
            }
        }

        let mut children: Vec<EdgeRef> =
            children.into_iter().unique_by(|c| c.key.clone()).collect();
        children.sort_by(|a, b| a.key.cmp(&b.key));
        children
    }

    fn get_backlinks(&self, key: &Key) -> Vec<EdgeRef> {
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

                    backlinks.push(EdgeRef {
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

fn apply_document_budget(
    documents: &mut Vec<DocumentOutput>,
    options: &RetrieveOptions,
) -> Truncation {
    let matched = documents.len();
    let budget = Budget {
        limit: options.max_documents,
        max_tokens: options.max_tokens,
        max_document_tokens: options.max_document_tokens,
    };

    apply_budget(
        documents,
        &budget,
        matched,
        |doc| doc.key.clone(),
        document_payload_tokens,
        |doc, max| {
            let (head, omitted) = truncate_to_tokens(&doc.content, max);
            if omitted > 0 {
                doc.content = format!("{}{}", head, truncation_marker(omitted));
                Some(omitted)
            } else {
                None
            }
        },
    )
}

fn document_payload_tokens(doc: &DocumentOutput) -> usize {
    count_tokens(&doc.content) + frontmatter_tokens(doc) + edge_tokens(doc)
}

fn frontmatter_tokens(doc: &DocumentOutput) -> usize {
    doc.frontmatter
        .as_ref()
        .and_then(|fields| serde_yaml::to_string(fields).ok())
        .map(|text| count_tokens(&text))
        .unwrap_or(0)
}

fn edge_tokens(doc: &DocumentOutput) -> usize {
    [
        &doc.references,
        &doc.includes,
        &doc.referenced_by,
        &doc.included_by,
    ]
    .into_iter()
    .filter(|edges| !edges.is_empty())
    .filter_map(|edges| serde_yaml::to_string(edges).ok())
    .map(|s| count_tokens(&s))
    .sum()
}

fn without_excluded(edges: Vec<EdgeRef>, exclude: &HashSet<Key>) -> Vec<EdgeRef> {
    edges
        .into_iter()
        .filter(|edge| !exclude.contains(&Key::name(&edge.key)))
        .collect()
}
