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
                .or_insert_with(HashSet::new)
                .extend(set);
        }
        for (key, set) in other.inline_references {
            self.inline_references
                .entry(key)
                .or_insert_with(HashSet::new)
                .extend(set);
        }
    }

    pub fn get_block_references_to(&self, key: &str) -> Vec<NodeId> {
        self.block_references
            .get(key)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or(Vec::new())
    }

    pub fn get_inline_references_to(&self, key: &str) -> Vec<NodeId> {
        self.inline_references
            .get(key)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or(Vec::new())
    }

    pub fn index_node(&mut self, graph: &Graph, node_id: NodeId) {
        match graph.graph_node(node_id) {
            GraphNode::Reference(reference) => {
                self.block_references
                    .entry(reference.key().to_string())
                    .or_insert_with(HashSet::new)
                    .insert(reference.id());

                reference.next_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
            }
            GraphNode::Section(section) => {
                for key in graph.get_line(section.line_id()).ref_keys() {
                    self.inline_references
                        .entry(key.to_string())
                        .or_insert_with(HashSet::new)
                        .insert(section.id());
                }
                section.child_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });

                section.next_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
            }
            GraphNode::Leaf(leaf) => {
                for key in graph.get_line(leaf.line_id()).ref_keys() {
                    self.inline_references
                        .entry(key.to_string())
                        .or_insert_with(HashSet::new)
                        .insert(leaf.id());
                }

                leaf.next_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
            }
            GraphNode::Document(document) => {
                document.child_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
            }
            GraphNode::Quote(quote) => {
                quote.child_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
                quote.next_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
            }
            GraphNode::BulletList(bullet_list) => {
                bullet_list.child_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
                bullet_list.next_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
            }
            GraphNode::OrderedList(ordered_list) => {
                ordered_list.child_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
                ordered_list.next_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
            }
            GraphNode::Empty => {}
            GraphNode::Raw(raw_leaf) => {
                raw_leaf.next_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
            }
            GraphNode::HorizontalRule(horizontal_rule) => {
                horizontal_rule.next_id().map(|child_id| {
                    self.index_node(graph, child_id);
                });
            }
        }
    }
}
