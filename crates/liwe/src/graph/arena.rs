use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::graph::{
    LineId, NodeId,
    {graph_line::Line, graph_node::GraphNode},
};
use crate::model::inline::Inlines;

#[derive(Clone, Default)]
pub struct Arena {
    nodes: Vec<GraphNode>,
    lines: Vec<Line>,
}

impl Arena {
    pub fn from_parts(nodes: Vec<GraphNode>, lines: Vec<Line>) -> Self {
        Arena { nodes, lines }
    }

    pub fn node(&self, id: NodeId) -> GraphNode {
        self.nodes[id as usize].clone()
    }

    pub fn get_line(&self, id: LineId) -> Line {
        self.lines[id].clone()
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
            self.lines[line_id] = Line::new(line_id, Inlines::new());
        }

        if let Some(id) = self.node(from_id).child_id() {
            self.delete_branch(id)
        }

        if let Some(id) = self.node(from_id).next_id() {
            self.delete_branch(id)
        }

        self.set_node(from_id, GraphNode::Empty);
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

    pub fn update_node<F>(&mut self, id: NodeId, f: F)
    where
        F: FnOnce(&mut GraphNode),
    {
        if matches!(self.nodes[id as usize], GraphNode::Empty) {
            panic!("Node {} is empty", id);
        }
        f(&mut self.nodes[id as usize]);
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

#[derive(Default)]
pub struct BuildIds {
    next_node_id: AtomicU64,
    next_line_id: AtomicUsize,
}

impl BuildIds {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc_node_id(&self) -> NodeId {
        self.next_node_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn alloc_line_id(&self) -> LineId {
        self.next_line_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn total_nodes(&self) -> usize {
        self.next_node_id.load(Ordering::Relaxed) as usize
    }

    pub fn total_lines(&self) -> usize {
        self.next_line_id.load(Ordering::Relaxed)
    }
}

#[derive(Default)]
pub struct BuildArena<'a> {
    nodes: HashMap<NodeId, GraphNode>,
    lines: HashMap<LineId, Line>,
    ids: Option<&'a BuildIds>,
}

impl<'a> BuildArena<'a> {
    pub fn new(ids: &'a BuildIds) -> Self {
        Self {
            nodes: HashMap::new(),
            lines: HashMap::new(),
            ids: Some(ids),
        }
    }

    fn ids(&self) -> &BuildIds {
        self.ids.expect("BuildArena was constructed without ids")
    }

    pub fn new_node_id(&self) -> NodeId {
        self.ids().alloc_node_id()
    }

    pub fn new_line_id(&self) -> LineId {
        self.ids().alloc_line_id()
    }

    pub fn set_node(&mut self, id: NodeId, node: GraphNode) {
        self.nodes.insert(id, node);
    }

    pub fn add_line(&mut self, inlines: Inlines) -> LineId {
        let id = self.new_line_id();
        self.lines.insert(id, Line::new(id, inlines));
        id
    }

    pub fn node(&self, id: NodeId) -> GraphNode {
        self.nodes
            .get(&id)
            .expect("BuildArena::node: id not present")
            .clone()
    }

    pub fn update_node<F>(&mut self, id: NodeId, f: F)
    where
        F: FnOnce(&mut GraphNode),
    {
        let entry = self
            .nodes
            .get_mut(&id)
            .expect("BuildArena::update_node: id not present");
        f(entry);
    }

    pub fn into_parts(self) -> (HashMap<NodeId, GraphNode>, HashMap<LineId, Line>) {
        (self.nodes, self.lines)
    }
}

pub trait NodeStore {
    fn new_node_id(&mut self) -> NodeId;
    fn add_line(&mut self, inlines: Inlines) -> LineId;
    fn add_graph_node(&mut self, node: GraphNode) -> NodeId;
    fn update_node(&mut self, id: NodeId, f: &mut dyn FnMut(&mut GraphNode));
    fn graph_node(&self, id: NodeId) -> GraphNode;
}

impl<'a> NodeStore for BuildArena<'a> {
    fn new_node_id(&mut self) -> NodeId {
        BuildArena::new_node_id(self)
    }

    fn add_line(&mut self, inlines: Inlines) -> LineId {
        BuildArena::add_line(self, inlines)
    }

    fn add_graph_node(&mut self, node: GraphNode) -> NodeId {
        let id = node.id();
        self.set_node(id, node);
        id
    }

    fn update_node(&mut self, id: NodeId, f: &mut dyn FnMut(&mut GraphNode)) {
        BuildArena::update_node(self, id, f);
    }

    fn graph_node(&self, id: NodeId) -> GraphNode {
        self.node(id)
    }
}

pub fn finalize_build(
    ids: &BuildIds,
    parts: Vec<(HashMap<NodeId, GraphNode>, HashMap<LineId, Line>)>,
) -> Arena {
    let total_nodes = ids.total_nodes();
    let total_lines = ids.total_lines();

    let mut nodes = vec![GraphNode::Empty; total_nodes];
    let mut lines: Vec<Line> = (0..total_lines)
        .map(|i| Line::new(i, Inlines::new()))
        .collect();

    for (doc_nodes, doc_lines) in parts {
        for (id, node) in doc_nodes {
            nodes[id as usize] = node;
        }
        for (id, line) in doc_lines {
            lines[id] = line;
        }
    }

    Arena::from_parts(nodes, lines)
}
