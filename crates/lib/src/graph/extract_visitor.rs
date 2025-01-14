use std::collections::HashMap;

use super::{Graph, NodeIter};
use crate::model::graph::Node;
use crate::model::{Key, NodeId};

pub struct ExtractVisitor<'a> {
    id: NodeId,
    graph: &'a Graph,
    keys: HashMap<NodeId, Key>,
}

impl<'a> ExtractVisitor<'a> {
    pub fn new(graph: &'a Graph, id: NodeId, keys: HashMap<NodeId, Key>) -> Self {
        ExtractVisitor { id, keys, graph }
    }
}

impl<'a> NodeIter<'a> for ExtractVisitor<'a> {
    fn next(&self) -> Option<Self> {
        self.graph
            .graph_node(self.id)
            .next_id()
            .map(|id| ExtractVisitor {
                id,
                keys: self.keys.clone(),
                graph: self.graph,
            })
    }

    fn child(&self) -> Option<Self> {
        if self.keys.contains_key(&self.id) {
            return None;
        }
        self.graph
            .graph_node(self.id)
            .child_id()
            .map(|id| ExtractVisitor {
                id,
                keys: self.keys.clone(),
                graph: self.graph,
            })
    }

    fn node(&self) -> Option<Node> {
        if self.keys.contains_key(&self.id) {
            return Some(Node::Reference(
                self.keys.get(&self.id).expect("to have key").clone(),
                "".to_string(),
            ));
        }

        self.graph.node(self.id)
    }
}
