use std::sync::OnceLock;

use rustc_hash::FxHashMap;

use crate::graph::{
    LineId, NodeId,
    {graph_line::Line, graph_node::GraphNode},
};
use crate::model::ids::{alloc_line_id, alloc_node_id};
use crate::model::inline::Inlines;

pub type NodeMap = FxHashMap<NodeId, GraphNode>;
pub type LineMap = FxHashMap<LineId, Line>;

fn empty_line() -> &'static Line {
    static EMPTY: OnceLock<Line> = OnceLock::new();
    EMPTY.get_or_init(|| Line::new(0, Inlines::new()))
}

#[derive(Clone, Default)]
pub struct Arena {
    nodes: NodeMap,
    lines: LineMap,
}

impl Arena {
    pub fn from_parts(nodes: NodeMap, lines: LineMap) -> Self {
        Arena { nodes, lines }
    }

    pub fn node(&self, id: NodeId) -> GraphNode {
        self.nodes.get(&id).cloned().unwrap_or(GraphNode::Empty)
    }

    pub fn get_line(&self, id: LineId) -> &Line {
        self.lines.get(&id).unwrap_or_else(|| empty_line())
    }

    pub fn add_line(&mut self, inlines: Inlines) -> LineId {
        let id = alloc_line_id();
        self.lines.insert(id, Line::new(id, inlines));
        id
    }

    pub fn new_node_id(&mut self) -> NodeId {
        alloc_node_id()
    }

    pub fn delete_branch(&mut self, from_id: NodeId) {
        for line_id in self.node(from_id).line_ids() {
            self.lines.remove(&line_id);
        }

        if let Some(id) = self.node(from_id).child_id() {
            self.delete_branch(id)
        }

        if let Some(id) = self.node(from_id).next_id() {
            self.delete_branch(id)
        }

        self.nodes.remove(&from_id);
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }

    pub fn section_ids(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.is_section())
            .map(|(id, _)| *id)
            .collect()
    }

    #[cfg(test)]
    pub fn lines_len(&self) -> usize {
        self.lines.len()
    }

    pub fn set_node(&mut self, id: NodeId, node: GraphNode) {
        self.nodes.insert(id, node);
    }

    pub fn update_node<F>(&mut self, id: NodeId, f: F)
    where
        F: FnOnce(&mut GraphNode),
    {
        let node = self
            .nodes
            .get_mut(&id)
            .unwrap_or_else(|| panic!("Node {} is missing", id));
        if matches!(node, GraphNode::Empty) {
            panic!("Node {} is empty", id);
        }
        f(node);
    }
}

#[derive(Default)]
pub struct BuildIds;

impl BuildIds {
    pub fn new() -> Self {
        BuildIds
    }

    pub fn alloc_node_id(&self) -> NodeId {
        alloc_node_id()
    }

    pub fn alloc_line_id(&self) -> LineId {
        alloc_line_id()
    }
}

#[derive(Default)]
pub struct BuildArena<'a> {
    nodes: NodeMap,
    lines: LineMap,
    ids: Option<&'a BuildIds>,
}

impl<'a> BuildArena<'a> {
    pub fn new(ids: &'a BuildIds) -> Self {
        Self {
            nodes: NodeMap::default(),
            lines: LineMap::default(),
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

    pub fn into_parts(self) -> (NodeMap, LineMap) {
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

pub fn finalize_build(parts: Vec<(NodeMap, LineMap)>) -> Arena {
    let total_nodes: usize = parts.iter().map(|(nodes, _)| nodes.len()).sum();
    let total_lines: usize = parts.iter().map(|(_, lines)| lines.len()).sum();

    let mut nodes = NodeMap::default();
    let mut lines = LineMap::default();
    nodes.reserve(total_nodes);
    lines.reserve(total_lines);

    for (doc_nodes, doc_lines) in parts {
        nodes.extend(doc_nodes);
        lines.extend(doc_lines);
    }

    Arena::from_parts(nodes, lines)
}
