use crate::model::NodeId;
use crate::model::graph::Node;
use super::{
    graph_node_visitor::GraphNodeVisitor, Graph, NodeIter,
};

pub struct UnnestVisitor<'a> {
    id: NodeId,
    target_id: NodeId,
    resume_id: Option<NodeId>,
    graph: &'a Graph,
}

impl<'a> UnnestVisitor<'a> {
    pub fn new(graph: &'a Graph, key: &str, target_id: NodeId) -> Self {
        let start_id = graph.visit_key(key).unwrap().id();
        Self {
            id: start_id,
            target_id,
            resume_id: None,
            graph,
        }
    }

    fn next_is_target(&self) -> bool {
        self.graph
            .visit_node(self.id)
            .to_next()
            .map(|next| next.id() == self.target_id)
            .unwrap_or(false)
    }

    fn child_is_target(&self) -> bool {
        self.graph
            .visit_node(self.id)
            .to_child()
            .map(|child| child.id() == self.target_id)
            .unwrap_or(false)
    }

    fn current(&self) -> GraphNodeVisitor {
        self.graph.visit_node(self.target_id)
    }
}

impl<'a> NodeIter<'a> for UnnestVisitor<'a> {
    fn next(&self) -> Option<impl NodeIter> {
        if self.next_is_target() {
            return Some(UnnestVisitor {
                id: self.current().to_child().expect("target has child").id(),
                target_id: self.target_id,
                resume_id: self.graph.graph_node(self.id).next_id(),
                graph: self.graph,
            });
        }

        if self.resume_id.is_some()
            && self.graph.graph_node(self.id).next_id().is_none()
            && self.graph.visit_node(self.id).to_parent().unwrap().id() == self.target_id
        {
            return self
                .resume_id
                .and_then(|id| self.graph.visit_node(id).to_next())
                .map(|next| next.id())
                .map(|resume_next_id| UnnestVisitor {
                    id: resume_next_id,
                    target_id: self.target_id,
                    resume_id: None,
                    graph: self.graph,
                });
        }

        self.graph
            .graph_node(self.id)
            .next_id()
            .map(|id| UnnestVisitor {
                id,
                target_id: self.target_id,
                resume_id: self.resume_id,
                graph: self.graph,
            })
    }

    fn child(&self) -> Option<impl NodeIter> {
        if self.child_is_target() {
            return self.current().to_child().map(|child| UnnestVisitor {
                id: child.id(),
                target_id: self.target_id,
                resume_id: Some(self.target_id),
                graph: self.graph,
            });
        }

        self.graph
            .graph_node(self.id)
            .child_id()
            .map(|id| UnnestVisitor {
                id,
                target_id: self.target_id,
                resume_id: None,
                graph: self.graph,
            })
    }

    fn node(&self) -> Option<Node> {
        self.graph.node(self.id)
    }
}
