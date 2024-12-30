use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
};

use change_list_type_visitor::ChangeListTypeVisitor;
use extract_visitor::ExtractVisitor;
use graph_line::Line;
use index::RefIndex;
use inline_visitor::InlineVisitor;
use node_visitor::NodeVisitor;
use projector::Projector;
use rand::distributions::{Alphanumeric, DistString};
use sections_builder::SectionsBuilder;
use squash_visitor::SquashVisitor;

use crate::{
    key::{with_extension, without_extension},
    markdown::MarkdownReader,
    model::graph::MarkdownOptions,
};
use arena::Arena;
use builder::GraphBuilder;
use futures::StreamExt;
use graph_node_visitor::GraphNodeVisitor;
use itertools::Itertools;
use path::{graph_to_paths, NodePath};
use rayon::prelude::*;
use unwrap_visitor::UnwrapVisitor;
use wrap_visitor::WrapVisitor;

use crate::graph::graph_node::GraphNode;
use crate::model::document::DocumentBlocks;
use crate::model::graph::{blocks_to_markdown_sparce, Block, Inlines, Node};
use crate::model::InlinesContext;
use crate::model::{Key, LineId, LineNumber, LineRange, NodeId, NodesMap, State};

mod arena;
pub mod builder;
mod change_list_type_visitor;
mod extract_visitor;
mod graph_line;
pub mod graph_node;
pub mod graph_node_visitor;
mod index;
mod inline_visitor;
mod node_visitor;
mod projector;
mod source_map;
mod squash_visitor;
mod unwrap_visitor;
mod wrap_visitor;

pub mod path;
pub mod sections_builder;

#[derive(Clone, Default)]
pub struct Graph {
    arena: Arena,
    index: RefIndex,
    keys: HashMap<Key, NodeId>,
    nodes_map: HashMap<Key, NodesMap>,
    global_nodes_map: HashMap<NodeId, LineRange>,
    sequential_keys: bool,
    keys_to_ref_text: HashMap<Key, String>,
    markdown_options: MarkdownOptions,
}

pub trait NodeIter<'a> {
    fn next(&self) -> Option<impl NodeIter>;
    fn child(&self) -> Option<impl NodeIter>;
    fn node(&self) -> Option<Node>;
}

pub trait Reader {
    fn blocks<'a>(&self, content: &str) -> DocumentBlocks;
}

pub trait Converter {}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            ..Default::default()
        }
    }

    pub fn new_with_options(markdown_options: MarkdownOptions) -> Graph {
        Graph {
            markdown_options,
            ..Default::default()
        }
    }

    pub fn set_sequential_keys(&mut self, sequential_keys: bool) {
        self.sequential_keys = sequential_keys;
    }

    fn node_key(&self, id: NodeId) -> Key {
        match self.graph_node(id).key() {
            Some(key) => key.clone(),
            None => self.node_key(self.graph_node(id).prev_id().expect("to have a prev_id")),
        }
    }

    pub fn keys(&self) -> Vec<Key> {
        self.keys.keys().cloned().collect()
    }

    pub fn with<F>(f: F) -> Graph
    where
        F: FnOnce(&mut Self),
    {
        let mut graph = Graph::new();
        f(&mut graph);
        graph
    }

    pub fn graph_node(&self, id: NodeId) -> GraphNode {
        self.arena.node(id)
    }

    pub fn node(&self, id: NodeId) -> Option<Node> {
        match self.graph_node(id) {
            GraphNode::Empty => None,
            GraphNode::Document(document) => Some(Node::Document(document.key().to_string())),
            GraphNode::Section(section) => Some(Node::Section(
                self.get_line(section.line_id()).normalize(self),
            )),
            GraphNode::Quote(_) => Some(Node::Quote()),
            GraphNode::BulletList(_) => Some(Node::BulletList()),
            GraphNode::OrderedList(_) => Some(Node::OrderedList()),
            GraphNode::Leaf(leaf) => {
                Some(Node::Leaf(self.get_line(leaf.line_id()).normalize(self)))
            }
            GraphNode::Raw(raw) => Some(Node::Raw(raw.lang(), raw.content().to_string())),
            GraphNode::HorizontalRule(_) => Some(Node::HorizontalRule()),
            GraphNode::Reference(reference) => Some(Node::Reference(
                reference.key().to_string(),
                self.get_ref_text(reference.key())
                    .unwrap_or(reference.title().to_string()),
            )),
        }
    }

    pub fn nodes(&self) -> &Vec<GraphNode> {
        self.arena.nodes()
    }

    fn node_mut(&mut self, id: NodeId) -> &mut GraphNode {
        self.arena.node_mut(id)
    }

    pub fn new_node_id(&mut self) -> NodeId {
        self.arena.new_node_id()
    }

    fn add_graph_node(&mut self, node: GraphNode) -> NodeId {
        let id = self.arena.new_node_id();
        self.arena.set_node(id, node);
        id
    }

    pub fn get_line(&self, id: LineId) -> Line {
        self.arena.get_line(id)
    }

    fn add_line(&mut self, inlines: Inlines) -> LineId {
        self.arena.add_line(inlines)
    }

    pub fn build_key(&mut self, key: &str) -> GraphBuilder {
        let id = self.arena.new_node_id();
        self.keys.insert(key.to_string(), id);
        self.arena
            .set_node(id, GraphNode::new_root(key.to_string(), id));
        GraphBuilder::new(self, id)
    }

    pub fn build_key_from_iter<'b>(&mut self, key: &str, iter: impl NodeIter<'b>) {
        self.build_key(key).insert_from_iter(iter.child().unwrap());
    }

    pub fn builder(&mut self, id: NodeId) -> GraphBuilder {
        GraphBuilder::new(self, id)
    }

    pub fn get_key_title(&self, key: &Key) -> Option<String> {
        self.keys_to_ref_text.get(key).cloned()
    }

    pub fn build_key_and<F>(&mut self, key: &str, f: F) -> &mut Self
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let id = self.arena.new_node_id();
        self.keys.insert(key.to_string(), id);
        self.arena
            .set_node(id, GraphNode::new_root(key.to_string(), id));
        f(&mut GraphBuilder::new(self, id));

        self.extract_ref_text(key)
            .map(|text| self.keys_to_ref_text.insert(key.to_string(), text));

        let mut index = RefIndex::new();
        index.index_node(self, id);
        self.index.merge(index);

        self
    }

    fn extract_ref_text(&self, key: &str) -> Option<String> {
        self.graph_node(self.get_document_id(key))
            .child_id()
            .map(|id| self.graph_node(id))
            .filter(|node| node.is_section())
            .and_then(|node| node.line_id())
            .map(|line_id| self.arena.get_line(line_id).to_plain_text())
    }

    pub fn visit_key(&self, key: &str) -> Option<GraphNodeVisitor> {
        self.keys
            .get(key)
            .map(|id| GraphNodeVisitor::new(self, *id))
    }

    pub fn get_document_id(&self, key: &str) -> NodeId {
        *self
            .keys
            .get(key)
            .expect(format!("to have key, {}", key).as_str())
    }

    pub fn visit_node(&self, id: NodeId) -> GraphNodeVisitor {
        GraphNodeVisitor::new(self, id)
    }

    pub fn from_markdown(&mut self, key: &str, content: &str, reader: impl Reader) {
        let mut build_key = self.build_key(key);
        let id = build_key.id();
        let blocks = reader.blocks(content);
        let nodes_map = SectionsBuilder::new(&mut build_key, &blocks).nodes_map();

        self.nodes_map.insert(key.to_string(), nodes_map.clone());
        self.global_nodes_map.extend(nodes_map);

        let mut index = RefIndex::new();
        index.index_node(self, id);
        self.index.merge(index);

        self.extract_ref_text(key)
            .map(|text| self.keys_to_ref_text.insert(key.to_string(), text));
    }

    pub fn project(&self, key: &str) -> Vec<Block> {
        let id = self.keys.get(key).unwrap();
        Projector::new(self, *id, 0, 0).project()
    }

    pub fn to_markdown(&self, key: &str) -> String {
        let id = self.keys.get(key).unwrap();
        let blocks = Projector::new(self, *id, 0, 0).project();
        blocks_to_markdown_sparce(&blocks, &self.markdown_options)
    }

    pub fn paths(&self) -> Vec<NodePath> {
        graph_to_paths(self)
    }

    pub fn update_key(&mut self, key: &str, content: &str) -> &mut Graph {
        if key.ends_with(".md") {
            panic!();
        }

        let id = self.keys.get(key);
        if id.is_some() {
            self.arena.delete_branch(*id.unwrap());
        }

        self.from_markdown(key, content, MarkdownReader::new());

        self
    }

    pub fn node_line_range(&self, id: NodeId) -> Option<LineRange> {
        self.global_nodes_map.get(&id).cloned()
    }

    pub fn import(content: &State, markdown_options: MarkdownOptions) -> Graph {
        let mut graph = Graph::new_with_options(markdown_options);

        let reader = MarkdownReader::new();

        let blocks = content
            .iter()
            .sorted_by(|a, b| a.0.cmp(&b.0))
            .collect_vec()
            .par_iter()
            .map(|(k, v)| (without_extension(k).clone(), reader.blocks(v)))
            .collect::<Vec<_>>();

        for (key, block_vec) in blocks {
            let nodes_map =
                SectionsBuilder::new(&mut graph.build_key(&key), &block_vec).nodes_map();
            graph.nodes_map.insert(key.to_string(), nodes_map.clone());
            graph.global_nodes_map.extend(nodes_map);
        }

        let mut index = RefIndex::new();
        for node in graph.arena.nodes() {
            index.index_node(&graph, node.id());
        }
        graph.index = index;

        for (key, _) in graph.keys.clone() {
            graph
                .extract_ref_text(&key)
                .map(|text| graph.keys_to_ref_text.insert(key.to_string(), text));
        }
        graph
    }

    pub fn export_key(&self, key: &str) -> Option<String> {
        Some(blocks_to_markdown_sparce(
            &self.project(key),
            &self.markdown_options,
        ))
    }

    pub fn export(&self) -> State {
        self.keys
            .par_iter()
            .map(|(k, v)| {
                (
                    with_extension(k),
                    blocks_to_markdown_sparce(&self.project(k), &self.markdown_options),
                )
            })
            .collect()
    }

    fn node_fmt(&self, id: NodeId, depth: usize, f: &mut Formatter<'_>) {
        let line = self
            .graph_node(id)
            .line_id()
            .map(|id| self.get_line(id).to_plain_text())
            .unwrap_or("".to_string());

        let ref_key = self.graph_node(id).ref_key().unwrap_or("".to_string());

        let _ = write!(
            f,
            "{:indent$} • {}{}: {}{}\n",
            "",
            self.graph_node(id).to_symbol(),
            id,
            line,
            ref_key,
            indent = depth * 2
        );
        let node = self.graph_node(id);

        node.child_id().map(|id| self.node_fmt(id, depth + 1, f));
        node.next_id().map(|id| self.node_fmt(id, depth, f));
    }

    pub fn get_block_references_to(&self, key: &str) -> Vec<NodeId> {
        // remove empty node ids
        self.index
            .get_block_references_to(key)
            .iter()
            .filter(|id| !self.graph_node(**id).is_empty())
            .cloned()
            .collect()
    }

    pub fn get_inline_references_to(&self, key: &str) -> Vec<NodeId> {
        // remove empty node ids
        self.index
            .get_inline_references_to(key)
            .iter()
            .filter(|id| !self.graph_node(**id).is_empty())
            .cloned()
            .collect()
    }
}

impl Debug for Graph {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.keys
            .iter()
            .for_each(|(key, id)| self.node_fmt(*id, 0, f));
        write!(f, "")
    }
}

impl PartialEq for Graph {
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys && self.arena == other.arena
    }
}

impl InlinesContext for &Graph {
    fn get_ref_title(&self, key: Key) -> Option<String> {
        self.get_key_title(&key)
    }
}

pub trait GraphContext: Copy {
    fn extract_vistior(&self, key: &str, keys: HashMap<NodeId, Key>) -> impl NodeIter;
    fn get_container_doucment_ref_text(&self, id: NodeId) -> String;
    fn get_container_key(&self, id: NodeId) -> Key;
    fn get_key(&self, id: NodeId) -> String;
    fn get_node_id(&self, key: &str) -> Option<NodeId>;
    fn get_node_id_at(&self, key: &str, line: LineNumber) -> Option<NodeId>;
    fn get_ref_text(&self, key: &str) -> Option<String>;
    fn get_reference_key(&self, id: NodeId) -> Key;
    fn get_sub_sections(&self, node_id: NodeId) -> Vec<NodeId>;
    fn get_surrounding_list_id(&self, id: NodeId) -> Option<NodeId>;
    fn get_text(&self, id: NodeId) -> String;
    fn inline_vistior(&self, key: &str, inline_id: NodeId) -> impl NodeIter;
    fn is_header(&self, id: NodeId) -> bool;
    fn is_list(&self, id: NodeId) -> bool;
    fn is_ordered_list(&self, id: NodeId) -> bool;
    fn is_bullet_list(&self, id: NodeId) -> bool;
    fn is_reference(&self, id: NodeId) -> bool;
    fn node_line_number(&self, id: NodeId) -> Option<LineNumber>;
    fn node_visitor(&self, id: NodeId) -> impl NodeIter;
    fn node_visitor_cut(&self, id: NodeId) -> impl NodeIter;
    fn random_key(&self) -> String;
    fn squash_vistior(&self, key: &str, depth: u8) -> impl NodeIter;
    fn unwrap_vistior(&self, key: &str, target_id: NodeId) -> impl NodeIter;
    fn change_list_type_visiton(&self, key: &str, target_id: NodeId) -> impl NodeIter;
    fn visitor(&self, key: &str) -> impl NodeIter;
    fn wrap_vistior(&self, target_id: NodeId) -> impl NodeIter;
}

impl GraphContext for &Graph {
    fn node_visitor(&self, id: NodeId) -> impl NodeIter {
        NodeVisitor::new(self, id)
    }

    fn node_visitor_cut(&self, id: NodeId) -> impl NodeIter {
        NodeVisitor::new_cut(self, id)
    }

    fn inline_vistior(&self, key: &str, inline_id: NodeId) -> impl NodeIter {
        let id = self
            .visit_key(key)
            .expect("to have key")
            .to_child()
            .expect("to have child")
            .id();
        InlineVisitor::new(self, id, inline_id)
    }

    fn squash_vistior(&self, key: &str, depth: u8) -> impl NodeIter {
        let id = self.get_document_id(key);
        SquashVisitor::new(self, id, depth)
    }

    fn visitor(&self, key: &str) -> impl NodeIter {
        let doc = *self.keys.get(key).expect("to have key");

        NodeVisitor::new(
            self,
            self.graph_node(doc)
                .child_id()
                .expect("to have child")
                .clone(),
        )
    }

    fn extract_vistior(&self, key: &str, keys: HashMap<NodeId, Key>) -> impl NodeIter {
        let doc = *self.keys.get(key).expect("to have key");
        ExtractVisitor::new(
            self,
            self.graph_node(doc)
                .child_id()
                .expect("to have child")
                .clone(),
            keys,
        )
    }

    fn wrap_vistior(&self, target_id: NodeId) -> impl NodeIter {
        WrapVisitor::new(self, target_id)
    }

    fn unwrap_vistior(&self, key: &str, target_id: NodeId) -> impl NodeIter {
        UnwrapVisitor::new(self, key, target_id)
    }

    fn change_list_type_visiton(&self, key: &str, target_id: NodeId) -> impl NodeIter {
        ChangeListTypeVisitor::new(self, key, target_id)
    }

    fn get_node_id_at(&self, key: &str, line: LineNumber) -> Option<NodeId> {
        self.nodes_map
            .get(key)
            .expect(&format!("to have key, {}", key))
            .iter()
            .rev()
            .find(|(k, v)| (*v).contains(&line))
            .map(|(k, v)| k.clone())
    }

    fn node_line_number(&self, id: NodeId) -> Option<LineNumber> {
        Some(self.global_nodes_map.get(&id)?.start)
    }

    fn get_text(&self, id: NodeId) -> String {
        self.graph_node(id)
            .line_id()
            .map(|id| self.get_line(id).to_plain_text())
            .unwrap_or_default()
            .to_string()
    }

    fn get_ref_text(&self, key: &str) -> Option<String> {
        self.get_key_title(&key.to_string())
    }

    fn get_container_doucment_ref_text(&self, id: NodeId) -> String {
        let container_key = self
            .visit_node(id)
            .to_document()
            .unwrap()
            .doucment_key()
            .unwrap();
        self.get_key_title(&container_key)
            .unwrap_or_default()
            .to_string()
    }

    fn get_container_key(&self, id: NodeId) -> Key {
        self.visit_node(id)
            .to_document()
            .unwrap()
            .doucment_key()
            .unwrap()
    }

    fn get_key(&self, id: NodeId) -> Key {
        self.node_key(id)
    }

    fn random_key(&self) -> String {
        if self.sequential_keys {
            return (self.keys().len() + 1).to_string();
        }

        loop {
            let key = format!("{}", Alphanumeric.sample_string(&mut rand::thread_rng(), 8))
                .to_lowercase();
            if self.keys.get(&key).is_none() {
                return key;
            }
        }
    }

    fn is_header(&self, id: NodeId) -> bool {
        !self.visit_node(id).is_in_list() && self.graph_node(id).is_section()
    }

    fn is_list(&self, id: NodeId) -> bool {
        self.visit_node(id).is_in_list()
    }

    fn is_ordered_list(&self, id: NodeId) -> bool {
        self.visit_node(id).is_ordered_list()
    }

    fn is_bullet_list(&self, id: NodeId) -> bool {
        self.visit_node(id).is_bullet_list()
    }

    fn get_surrounding_list_id(&self, id: NodeId) -> Option<NodeId> {
        self.visit_node(id).get_list().map(|v| v.id())
    }

    fn is_reference(&self, id: NodeId) -> bool {
        self.graph_node(id).is_ref()
    }

    fn get_reference_key(&self, id: NodeId) -> Key {
        self.graph_node(id).ref_key().unwrap().clone()
    }

    fn get_node_id(&self, key: &str) -> Option<NodeId> {
        self.keys.get(key).map(|v| *v)
    }

    fn get_sub_sections(&self, node_id: NodeId) -> Vec<NodeId> {
        self.visit_node(node_id).get_sub_sections()
    }
}
