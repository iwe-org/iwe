use std::collections::HashSet;

use itertools::Itertools;

use crate::graph::{Graph, GraphContext};
use crate::model::node::{NodeIter, NodePointer};
use crate::model::{Key, NodeId};
use crate::retrieve::EdgeRef;

pub fn included_by(graph: &Graph, key: &Key) -> Vec<EdgeRef> {
    let refs = graph.get_inclusion_edges_to(key);
    let mut out = Vec::new();
    for ref_id in refs {
        let node = graph.node(ref_id);
        if let Some(doc_node) = node.to_document() {
            if let Some(doc_key) = doc_node.document_key() {
                let title = graph
                    .get_key_title(&doc_key)
                    .unwrap_or_else(|| doc_key.to_string());
                let section_path = section_path(graph, ref_id);
                out.push(EdgeRef {
                    key: doc_key.to_string(),
                    title,
                    section_path,
                });
            }
        }
    }
    let mut out: Vec<EdgeRef> = out.into_iter().unique_by(|p| p.key.clone()).collect();
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

pub fn includes(graph: &Graph, key: &Key) -> Vec<EdgeRef> {
    let refs = graph.get_inclusion_edges_in(key);
    let mut out = Vec::new();
    for ref_id in refs {
        if let Some(ref_key) = graph.graph_node(ref_id).ref_key() {
            let title = graph
                .get_key_title(&ref_key)
                .unwrap_or_else(|| ref_key.to_string());
            let section_path = section_path(graph, ref_id);
            out.push(EdgeRef {
                key: ref_key.to_string(),
                title,
                section_path,
            });
        }
    }
    let mut out: Vec<EdgeRef> = out.into_iter().unique_by(|c| c.key.clone()).collect();
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

pub fn referenced_by(graph: &Graph, key: &Key) -> Vec<EdgeRef> {
    let inline_refs = graph.get_reference_edges_to(key);
    let mut out = Vec::new();
    let mut seen_keys = HashSet::new();
    for ref_id in inline_refs {
        let node = graph.node(ref_id);
        if let Some(doc_node) = node.to_document() {
            if let Some(doc_key) = doc_node.document_key() {
                if seen_keys.contains(&doc_key) {
                    continue;
                }
                seen_keys.insert(doc_key.clone());
                let title = graph
                    .get_key_title(&doc_key)
                    .unwrap_or_else(|| doc_key.to_string());
                let section_path = section_path(graph, ref_id);
                out.push(EdgeRef {
                    key: doc_key.to_string(),
                    title,
                    section_path,
                });
            }
        }
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

pub fn references(graph: &Graph, key: &Key) -> Vec<EdgeRef> {
    let target_keys = graph.get_reference_edges_in(key);
    let mut out = Vec::new();
    let mut seen_keys = HashSet::new();
    for target_key in target_keys {
        if seen_keys.contains(&target_key) {
            continue;
        }
        seen_keys.insert(target_key.clone());
        let title = graph
            .get_key_title(&target_key)
            .unwrap_or_else(|| target_key.to_string());
        out.push(EdgeRef {
            key: target_key.to_string(),
            title,
            section_path: Vec::new(),
        });
    }
    out.sort_by(|a, b| a.key.cmp(&b.key));
    out
}

pub fn section_path(graph: &Graph, node_id: NodeId) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = graph.node(node_id);

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
