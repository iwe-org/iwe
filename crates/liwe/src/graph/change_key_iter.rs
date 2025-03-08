use itertools::Itertools;

use super::Graph;
use super::GraphContext;
use crate::model::node::Reference;
use crate::model::{Key, NodeId};
use crate::model::node::{Node, NodeIter, NodePointer};

pub struct ChangeKeyVisitor<'a> {
    id: NodeId,
    graph: &'a Graph,
    target_key: Key,
    updated_key: Key,
}

impl<'a> ChangeKeyVisitor<'a> {
    pub fn new(graph: &'a Graph, key: &Key, target_key: &Key, updated_key: &Key) -> Self {
        let start_id = graph.visit_key(key).unwrap().id().unwrap();
        Self {
            id: start_id,
            graph,
            target_key: target_key.clone(),
            updated_key: updated_key.clone(),
        }
    }

    fn current(&self) -> impl NodePointer {
        self.graph.visit_node(self.id)
    }
}

impl<'a> NodeIter<'a> for ChangeKeyVisitor<'a> {
    fn next(&self) -> Option<Self> {
        return self.current().to_next().map(|child| Self {
            id: child.id().unwrap(),
            graph: self.graph,
            target_key: self.target_key.clone(),
            updated_key: self.updated_key.clone(),
        });
    }

    fn child(&self) -> Option<Self> {
        return self.current().to_child().map(|child| Self {
            id: child.id().unwrap(),
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
                Node::Reference(Reference {
                    key,
                    text,
                    reference_type,
                }) => {
                    if key == self.target_key {
                        Node::Reference(Reference {
                            key: self.updated_key.clone(),
                            text: self.graph.get_ref_text(&self.target_key).unwrap_or(text),
                            reference_type,
                        })
                    } else {
                        node
                    }
                }
                _ => node,
            })
            .or_else(|| self.graph.node(self.id))
    }
}
