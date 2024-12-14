use itertools::Itertools;

use crate::graph::{
    LineId, NodeId,
    {graph_line::Line, graph_node::GraphNode},
};
use crate::model::graph::Inlines;

#[derive(Clone, Default)]
pub struct Arena {
    nodes: Vec<GraphNode>,
    lines: Vec<Line>,
}

impl Arena {
    fn new() -> Arena {
        Arena {
            ..Default::default()
        }
    }

    pub fn node(&self, id: NodeId) -> GraphNode {
        let node = self.nodes[id as usize].clone();
        node
    }

    pub fn get_line(&self, id: LineId) -> Line {
        self.lines[id as usize].clone()
    }

    pub fn add_line(&mut self, inlines: Inlines) -> LineId {
        let id = self.new_line_id();
        self.lines.push(Line::new(id, inlines));
        id
    }

    pub fn new_node_id(&mut self) -> NodeId {
        self.nodes.len() as NodeId
    }

    fn new_line_id(&mut self) -> LineId {
        self.lines.len() as LineId
    }

    pub fn delete_branch(&mut self, from_id: NodeId) {
        if let Some(line_id) = self.node(from_id).line_id() {
            self.lines[line_id as usize] = Line::new(line_id, Inlines::new());
        }

        self.node(from_id)
            .child_id()
            .map(|id| self.delete_branch(id));

        self.node(from_id)
            .next_id()
            .map(|id| self.delete_branch(id));

        self.set_node(from_id, GraphNode::Empty);
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        (0..self.nodes.len() as NodeId).collect_vec()
    }

    pub fn nodes(&self) -> &Vec<GraphNode> {
        &self.nodes
    }

    pub fn set_node(&mut self, id: NodeId, node: GraphNode) {
        if id as usize >= self.nodes.len() {
            self.nodes.push(node)
        } else {
            self.nodes[id as usize] = node;
        }
    }

    pub fn node_mut(&mut self, id: NodeId) -> &mut GraphNode {
        let node = self.nodes[id as usize].clone();

        if matches!(node, GraphNode::Empty) {
            panic!("Node {} is empty", id);
        }

        &mut self.nodes[id as usize]
    }
}

impl PartialEq for Arena {
    fn eq(&self, other: &Self) -> bool {
        if self.nodes.len() != other.nodes.len() {
            return false;
        }
        for (i, node) in self.nodes.iter().enumerate() {
            if node != &other.nodes[i] {
                return false;
            }
        }
        if self.lines.len() != other.lines.len() {
            return false;
        }
        for (i, line) in self.lines.iter().enumerate() {
            if line != &other.lines[i] {
                return false;
            }
        }
        true
    }
}
