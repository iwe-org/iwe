use super::Graph;
use crate::model::node::Node;
use crate::model::NodeId;
use crate::model::node::{NodeIter, NodePointer};

pub struct SquashIter<'a> {
    id: NodeId,
    depth: u8,
    graph: &'a Graph,
    resume_id: Option<NodeId>,
}

impl<'a> SquashIter<'a> {
    pub fn new(graph: &'a Graph, id: NodeId, depth: u8) -> Self {
        Self {
            id,
            depth,
            graph,
            resume_id: None,
        }
    }

    fn next_referenced_id(&self) -> Option<NodeId> {
        self.graph
            .visit_node(self.id)
            .to_next()
            .and_then(|n| n.ref_key())
            .and_then(|key| self.graph.visit_key(&key))
            .and_then(|doc| doc.to_child())
            .and_then(|node| node.id())
    }

    fn resume_next_referenced_id(&self) -> Option<NodeId> {
        self.resume_id
            .map(|id| self.graph.visit_node(id))
            .and_then(|n| n.to_next())
            .and_then(|n| n.ref_key())
            .and_then(|key| self.graph.visit_key(&key))
            .and_then(|doc| doc.to_child())
            .and_then(|node| node.id())
    }

    fn resume_next_id(&self) -> Option<NodeId> {
        self.resume_id
            .and_then(|resume_id| self.graph.visit_node(resume_id).to_next())
            .and_then(|resume_next| resume_next.id())
    }

    fn child_referenced_id(&self) -> Option<NodeId> {
        self.graph
            .visit_node(self.id)
            .to_child()
            .and_then(|n| n.ref_key())
            .and_then(|key| self.graph.visit_key(&key))
            .and_then(|doc| doc.to_child())
            .and_then(|node| node.id())
    }

    fn next_id(&self) -> Option<NodeId> {
        self.graph.graph_node(self.id).next_id()
    }

    fn child_id(&self) -> Option<NodeId> {
        self.graph.graph_node(self.id).child_id()
    }
}

impl<'a> NodeIter<'a> for SquashIter<'a> {
    fn next(&self) -> Option<Self> {
        self.next_referenced_id()
            .filter(|_| self.depth > 0)
            .map(|id| SquashIter {
                id,
                depth: self.depth - 1,
                resume_id: self.next_id(),
                graph: self.graph,
            })
            .or(self
                .resume_id
                .filter(|_| self.next_id().is_none())
                .and_then(|_| {
                    self.resume_next_referenced_id()
                        .filter(|_| self.depth > 0)
                        .map(|id| SquashIter {
                            id,
                            depth: self.depth + 1,
                            resume_id: self.resume_next_id(),
                            graph: self.graph,
                        })
                        .or(self.resume_next_id().map(|id| SquashIter {
                            id,
                            depth: self.depth + 1,
                            resume_id: None,
                            graph: self.graph,
                        }))
                }))
            .or(self.next_id().map(|id| SquashIter {
                id,
                depth: self.depth,
                resume_id: self.resume_id,
                graph: self.graph,
            }))
    }

    fn child(&self) -> Option<Self> {
        self.child_referenced_id()
            .filter(|_| self.depth > 0)
            .map(|id| SquashIter {
                id,
                depth: self.depth - 1,
                resume_id: self.child_id(),
                graph: self.graph,
            })
            .or(self.child_id().map(|id| SquashIter {
                id,
                depth: self.depth,
                resume_id: None,
                graph: self.graph,
            }))
    }

    fn node(&self) -> Option<Node> {
        self.graph.node(self.id)
    }
}
