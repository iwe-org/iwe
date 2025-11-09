use std::collections::{HashMap, HashSet};

use crate::model::{Key, NodeId};

use super::{graph_node::GraphNode, Graph};

#[derive(Default, Clone)]
pub struct RefIndex {
    block_references: HashMap<Key, HashSet<NodeId>>,
    inline_references: HashMap<Key, HashSet<NodeId>>,
}

impl RefIndex {
    pub fn new() -> RefIndex {
        RefIndex::default()
    }

    pub fn merge(&mut self, other: RefIndex) {
        for (key, set) in other.block_references {
            self.block_references
                .entry(key)
                .or_default()
                .extend(set);
        }
        for (key, set) in other.inline_references {
            self.inline_references
                .entry(key)
                .or_default()
                .extend(set);
        }
    }

    pub fn get_block_references_to(&self, key: &Key) -> Vec<NodeId> {
        self.block_references
            .get(key)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_inline_references_to(&self, key: &Key) -> Vec<NodeId> {
        self.inline_references
            .get(key)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn index_node(&mut self, graph: &Graph, node_id: NodeId) {
        match graph.graph_node(node_id) {
            GraphNode::Reference(reference) => {
                self.block_references
                    .entry(reference.key().clone())
                    .or_default()
                    .insert(reference.id());

                if let Some(child_id) = reference.next_id() { self.index_node(graph, child_id); }
            }
            GraphNode::Section(section) => {
                for key in graph.get_line(section.line_id()).ref_keys() {
                    self.inline_references
                        .entry(key.clone())
                        .or_default()
                        .insert(section.id());
                }
                if let Some(child_id) = section.child_id() { self.index_node(graph, child_id); }

                if let Some(child_id) = section.next_id() { self.index_node(graph, child_id); }
            }
            GraphNode::Leaf(leaf) => {
                for key in graph.get_line(leaf.line_id()).ref_keys() {
                    self.inline_references
                        .entry(key.clone())
                        .or_default()
                        .insert(leaf.id());
                }

                if let Some(child_id) = leaf.next_id() { self.index_node(graph, child_id); }
            }
            GraphNode::Document(document) => {
                if let Some(child_id) = document.child_id() { self.index_node(graph, child_id); }
            }
            GraphNode::Quote(quote) => {
                if let Some(child_id) = quote.child_id() { self.index_node(graph, child_id); }
                if let Some(child_id) = quote.next_id() { self.index_node(graph, child_id); }
            }
            GraphNode::BulletList(bullet_list) => {
                if let Some(child_id) = bullet_list.child_id() { self.index_node(graph, child_id); }
                if let Some(child_id) = bullet_list.next_id() { self.index_node(graph, child_id); }
            }
            GraphNode::OrderedList(ordered_list) => {
                if let Some(child_id) = ordered_list.child_id() { self.index_node(graph, child_id); }
                if let Some(child_id) = ordered_list.next_id() { self.index_node(graph, child_id); }
            }
            GraphNode::Empty => {}
            GraphNode::Raw(raw_leaf) => {
                if let Some(child_id) = raw_leaf.next_id() { self.index_node(graph, child_id); }
            }
            GraphNode::HorizontalRule(horizontal_rule) => {
                if let Some(child_id) = horizontal_rule.next_id() { self.index_node(graph, child_id); }
            }
            GraphNode::Table(_) => {}
        }
    }
}
