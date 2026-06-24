use std::collections::{HashMap, HashSet};

use crate::model::{Key, NodeId};

use super::{graph_node::GraphNode, Graph};

#[derive(Default, Clone)]
pub struct RefIndex {
    inclusion_edges: HashMap<Key, HashSet<NodeId>>,
    reference_edges: HashMap<Key, HashSet<NodeId>>,
}

impl RefIndex {
    pub fn new() -> RefIndex {
        RefIndex::default()
    }

    pub fn merge(&mut self, other: RefIndex) {
        for (key, set) in other.inclusion_edges {
            self.inclusion_edges.entry(key).or_default().extend(set);
        }
        for (key, set) in other.reference_edges {
            self.reference_edges.entry(key).or_default().extend(set);
        }
    }

    pub fn get_inclusion_edges_to(&self, key: &Key) -> Vec<NodeId> {
        self.inclusion_edges
            .get(key)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_reference_edges_to(&self, key: &Key) -> Vec<NodeId> {
        self.reference_edges
            .get(key)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn inclusion_edge_target_keys(&self) -> impl Iterator<Item = &Key> {
        self.inclusion_edges.keys()
    }

    pub fn reference_edge_target_keys(&self) -> impl Iterator<Item = &Key> {
        self.reference_edges.keys()
    }

    pub fn index_node(&mut self, graph: &Graph, root_id: NodeId) {
        Self::walk_edges(graph, root_id, |inclusion, key, node_id| {
            let edges = if inclusion {
                &mut self.inclusion_edges
            } else {
                &mut self.reference_edges
            };
            edges.entry(key).or_default().insert(node_id);
        });
    }

    pub fn unindex_node(&mut self, graph: &Graph, root_id: NodeId) {
        Self::walk_edges(graph, root_id, |inclusion, key, node_id| {
            let edges = if inclusion {
                &mut self.inclusion_edges
            } else {
                &mut self.reference_edges
            };
            if let Some(set) = edges.get_mut(&key) {
                set.remove(&node_id);
                if set.is_empty() {
                    edges.remove(&key);
                }
            }
        });
    }

    fn walk_edges<F>(graph: &Graph, root_id: NodeId, mut visit: F)
    where
        F: FnMut(bool, Key, NodeId),
    {
        let mut stack: Vec<NodeId> = vec![root_id];
        while let Some(node_id) = stack.pop() {
            match graph.graph_node(node_id) {
                GraphNode::Reference(reference) => {
                    visit(true, reference.key().clone(), reference.id());

                    if let Some(child_id) = reference.next_id() {
                        stack.push(child_id);
                    }
                }
                GraphNode::Section(section) => {
                    for key in graph.get_line(section.line_id()).ref_keys() {
                        visit(false, key.clone(), section.id());
                    }
                    if let Some(child_id) = section.child_id() {
                        stack.push(child_id);
                    }
                    if let Some(child_id) = section.next_id() {
                        stack.push(child_id);
                    }
                }
                GraphNode::Leaf(leaf) => {
                    for key in graph.get_line(leaf.line_id()).ref_keys() {
                        visit(false, key.clone(), leaf.id());
                    }

                    if let Some(child_id) = leaf.next_id() {
                        stack.push(child_id);
                    }
                }
                GraphNode::Document(document) => {
                    if let Some(child_id) = document.child_id() {
                        stack.push(child_id);
                    }
                }
                GraphNode::Quote(quote) => {
                    if let Some(child_id) = quote.child_id() {
                        stack.push(child_id);
                    }
                    if let Some(child_id) = quote.next_id() {
                        stack.push(child_id);
                    }
                }
                GraphNode::BulletList(bullet_list) => {
                    if let Some(child_id) = bullet_list.child_id() {
                        stack.push(child_id);
                    }
                    if let Some(child_id) = bullet_list.next_id() {
                        stack.push(child_id);
                    }
                }
                GraphNode::OrderedList(ordered_list) => {
                    if let Some(child_id) = ordered_list.child_id() {
                        stack.push(child_id);
                    }
                    if let Some(child_id) = ordered_list.next_id() {
                        stack.push(child_id);
                    }
                }
                GraphNode::Empty => {}
                GraphNode::Raw(raw_leaf) => {
                    if let Some(child_id) = raw_leaf.next_id() {
                        stack.push(child_id);
                    }
                }
                GraphNode::HorizontalRule(horizontal_rule) => {
                    if let Some(child_id) = horizontal_rule.next_id() {
                        stack.push(child_id);
                    }
                }
                GraphNode::Table(table) => {
                    for line_id in table.header() {
                        for key in graph.get_line(*line_id).ref_keys() {
                            visit(false, key.clone(), table.id());
                        }
                    }
                    for row in table.rows() {
                        for line_id in row {
                            for key in graph.get_line(*line_id).ref_keys() {
                                visit(false, key.clone(), table.id());
                            }
                        }
                    }
                    if let Some(next_id) = table.next_id() {
                        stack.push(next_id);
                    }
                }
            }
        }
    }

    #[cfg(test)]
    pub fn edge_counts(&self) -> (usize, usize) {
        (
            self.inclusion_edges.values().map(|set| set.len()).sum(),
            self.reference_edges.values().map(|set| set.len()).sum(),
        )
    }

    #[cfg(test)]
    pub fn key_counts(&self) -> (usize, usize) {
        (self.inclusion_edges.len(), self.reference_edges.len())
    }
}
