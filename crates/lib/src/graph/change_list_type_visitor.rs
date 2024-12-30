use std::default;

use super::{graph_node_visitor::GraphNodeVisitor, Graph, NodeIter};
use crate::model::document::OrderedList;
use crate::model::graph::Node;
use crate::model::NodeId;

pub struct ChangeListTypeVisitor<'a> {
    id: NodeId,
    target_id: NodeId,
    graph: &'a Graph,
}

impl<'a> ChangeListTypeVisitor<'a> {
    pub fn new(graph: &'a Graph, key: &str, target_id: NodeId) -> Self {
        let start_id = graph.visit_key(key).unwrap().id();
        Self {
            id: start_id,
            target_id,
            graph,
        }
    }

    fn current(&self) -> GraphNodeVisitor {
        self.graph.visit_node(self.id)
    }
}

impl<'a> NodeIter<'a> for ChangeListTypeVisitor<'a> {
    fn next(&self) -> Option<impl NodeIter> {
        return self.current().to_next().map(|child| Self {
            id: child.id(),
            target_id: self.target_id,
            graph: self.graph,
        });
    }

    fn child(&self) -> Option<impl NodeIter> {
        return self.current().to_child().map(|child| Self {
            id: child.id(),
            target_id: self.target_id,
            graph: self.graph,
        });
    }

    fn node(&self) -> Option<Node> {
        self.graph
            .node(self.id)
            .filter(|node| self.target_id == self.id)
            .map(|node| match node {
                Node::BulletList() => Node::OrderedList(),
                Node::OrderedList() => Node::BulletList(),
                _ => panic!("Unexpected node type"),
            })
            .or_else(|| self.graph.node(self.id))
    }
}
