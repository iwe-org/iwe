use std::default;

use itertools::Itertools;

use super::GraphContext;
use super::{graph_node_visitor::GraphNodeVisitor, Graph, NodeIter};
use crate::model::document::OrderedList;
use crate::model::graph::{self, Node};
use crate::model::NodeId;

pub struct ChangeKeyVisitor<'a> {
    id: NodeId,
    graph: &'a Graph,
    target_key: String,
    updated_key: String,
}

impl<'a> ChangeKeyVisitor<'a> {
    pub fn new(graph: &'a Graph, key: &str, target_key: &str, updated_key: &str) -> Self {
        let start_id = graph.visit_key(key).unwrap().id();
        Self {
            id: start_id,
            graph,
            target_key: target_key.to_string(),
            updated_key: updated_key.to_string(),
        }
    }

    fn current(&self) -> GraphNodeVisitor {
        self.graph.visit_node(self.id)
    }
}

impl<'a> NodeIter<'a> for ChangeKeyVisitor<'a> {
    fn next(&self) -> Option<impl NodeIter> {
        return self.current().to_next().map(|child| Self {
            id: child.id(),
            graph: self.graph,
            target_key: self.target_key.clone(),
            updated_key: self.updated_key.clone(),
        });
    }

    fn child(&self) -> Option<impl NodeIter> {
        return self.current().to_child().map(|child| Self {
            id: child.id(),
            graph: self.graph,
            target_key: self.target_key.clone(),
            updated_key: self.updated_key.clone(),
        });
    }

    fn node(&self) -> Option<Node> {
        self.graph
            .node(self.id)
            .map(|node| match node.clone() {
                Node::Section(inlines) => Node::Section(
                    inlines
                        .clone()
                        .iter()
                        .map(|inline| {
                            inline.change_key(&self.target_key, &self.updated_key, self.graph)
                        })
                        .collect_vec(),
                ),
                Node::Leaf(inlines) => Node::Leaf(
                    inlines
                        .clone()
                        .iter()
                        .map(|inline| {
                            inline.change_key(&self.target_key, &self.updated_key, self.graph)
                        })
                        .collect_vec(),
                ),
                Node::Reference(key, title) => {
                    if key == self.target_key {
                        Node::Reference(
                            self.updated_key.clone(),
                            self.graph.get_ref_text(&self.target_key).unwrap_or(title),
                        )
                    } else {
                        node
                    }
                }
                default => node,
            })
            .or_else(|| self.graph.node(self.id))
    }
}
