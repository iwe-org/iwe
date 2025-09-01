use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Formatter},
};

use basic_iter::GraphNodePointer;
use graph_line::Line;
use index::RefIndex;
use rand::distr::{Alphanumeric, SampleString};
use sections_builder::SectionsBuilder;

use crate::{
    markdown::MarkdownReader,
    model::{document::Document, rank::node_rank, tree::Tree},
};
use arena::Arena;
use builder::GraphBuilder;
use itertools::Itertools;
use path::{graph_to_paths, NodePath};
use rayon::prelude::*;

use crate::graph::graph_node::GraphNode;
use crate::model::config::MarkdownOptions;
use crate::model::graph::GraphInlines;
use crate::model::node::{NodeIter, NodePointer};
use crate::model::InlinesContext;
use crate::model::{Key, LineId, LineNumber, LineRange, NodeId, NodesMap, State};

mod arena;
pub mod basic_iter;
pub mod builder;
mod graph_line;
pub mod graph_node;
mod index;

pub mod path;
pub mod sections_builder;
mod squash_iter;

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
    metadata: HashMap<Key, String>,
}

#[derive(Clone, Debug, Default)]
pub struct SearchPath {
    pub search_text: String,
    pub node_rank: usize,
    pub key: Key,
    pub root: bool,
    pub line: u32,
    pub path: NodePath,
}

pub trait Reader {
    fn document<'a>(&self, content: &str, markdown_options: &MarkdownOptions) -> Document;
}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            ..Default::default()
        }
    }

    pub fn markdown_options(&self) -> MarkdownOptions {
        self.markdown_options.clone()
    }

    pub fn search_paths(&self) -> Vec<SearchPath> {
        self.paths()
            .par_iter()
            .map(|path| SearchPath {
                search_text: render_search_text(path, self),
                node_rank: node_rank(self, path.last_id()),
                key: self.node_key(path.target()),
                root: path.ids().len() == 1,
                line: self.node_line_number(path.target()).unwrap_or(0) as u32,
                path: path.clone(),
            })
            .collect::<Vec<_>>()
            .into_iter()
            .sorted_by(|a, b| {
                let primary = b.node_rank.cmp(&a.node_rank);
                if primary == Ordering::Equal {
                    a.key.cmp(&b.key)
                } else {
                    primary
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn new_patch(&self) -> Graph {
        Graph {
            markdown_options: self.markdown_options.clone(),
            metadata: self.metadata.clone(),
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

    fn add_line(&mut self, inlines: GraphInlines) -> LineId {
        self.arena.add_line(inlines)
    }

    pub fn build_key(&mut self, key: &Key) -> GraphBuilder<'_> {
        let id = self.arena.new_node_id();
        self.keys.insert(key.clone(), id);
        self.arena
            .set_node(id, GraphNode::new_root(key.clone(), id, None));
        GraphBuilder::new(self, id)
    }

    pub fn build_key_from_iter<'b>(&mut self, key: &Key, iter: impl NodeIter<'b>) {
        self.build_key(key).insert_from_iter(iter);
    }

    pub fn builder(&mut self, id: NodeId) -> GraphBuilder<'_> {
        GraphBuilder::new(self, id)
    }

    pub fn get_key_title(&self, key: &Key) -> Option<String> {
        self.keys_to_ref_text.get(key).cloned()
    }

    pub fn build_key_and<F>(&mut self, key: &Key, f: F) -> &mut Self
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let id = self.arena.new_node_id();
        self.keys.insert(key.clone(), id);
        self.arena
            .set_node(id, GraphNode::new_root(key.clone(), id, None));
        f(&mut GraphBuilder::new(self, id));

        self.extract_ref_text(key)
            .map(|text| self.keys_to_ref_text.insert(key.clone(), text));

        let mut index = RefIndex::new();
        index.index_node(self, id);
        self.index.merge(index);

        self
    }

    fn extract_ref_text(&self, key: &Key) -> Option<String> {
        self.graph_node(self.get_document_id(key))
            .child_id()
            .map(|id| self.graph_node(id))
            .filter(|node| node.is_section())
            .and_then(|node| node.line_id())
            .map(|line_id| self.arena.get_line(line_id).to_plain_text())
    }

    pub fn maybe_key(&self, key: &Key) -> Option<impl NodePointer<'_>> {
        self.keys
            .get(key)
            .map(|id| GraphNodePointer::new(self, *id))
    }

    pub fn get_document_id(&self, key: &Key) -> NodeId {
        *self
            .keys
            .get(key)
            .expect(format!("to have key, {}", key).as_str())
    }

    pub fn from_markdown(&mut self, key: Key, content: &str, reader: impl Reader) {
        let document = reader.document(content, &self.markdown_options);

        if let Some(meta) = document.metadata {
            self.metadata.insert(key.clone(), meta);
        } else {
            self.metadata.remove(&key);
        }

        let mut build_key = self.build_key(&key);
        let id = build_key.id();

        let nodes_map = SectionsBuilder::new(&mut build_key, &document.blocks, &key).nodes_map();

        self.nodes_map.insert(key.clone(), nodes_map.clone());
        self.global_nodes_map.extend(nodes_map);

        let mut index = RefIndex::new();
        index.index_node(self, id);
        self.index.merge(index);

        self.extract_ref_text(&key)
            .map(|text| self.keys_to_ref_text.insert(key, text));
    }

    pub fn to_markdown(&self, key: &Key) -> String {
        let markdown = self
            .collect(key)
            .iter()
            .to_markdown(&key.parent(), &self.markdown_options);

        format!("{}", markdown)
    }

    pub fn paths(&self) -> Vec<NodePath> {
        graph_to_paths(self)
    }

    pub fn update_key(&mut self, key: Key, content: &str) -> &mut Graph {
        let id = self.keys.get(&key);
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
        let mut graph = Graph::new_with_options(markdown_options.clone());

        let reader = MarkdownReader::new();

        let blocks = content
            .iter()
            .sorted_by(|a, b| a.0.cmp(&b.0))
            .collect_vec()
            .par_iter()
            .map(|(k, v)| (Key::name(k), reader.document(v, &markdown_options)))
            .collect::<Vec<_>>();

        for (key, document) in blocks.into_iter() {
            if let Some(meta) = document.metadata.clone() {
                graph.metadata.insert(key.clone(), meta);
            } else {
                graph.metadata.remove(&key);
            }

            let nodes_map =
                SectionsBuilder::new(&mut graph.build_key(&key), &document.blocks, &key)
                    .nodes_map();
            graph.nodes_map.insert(key.clone(), nodes_map.clone());
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
                .map(|text| graph.keys_to_ref_text.insert(key.clone(), text));
        }
        graph
    }

    pub fn export_key(&self, key: &Key) -> Option<String> {
        Some(self.to_markdown(key))
    }

    pub fn export(&self) -> State {
        self.keys
            .par_iter()
            .map(|(k, _)| (k.to_string(), self.to_markdown(k)))
            .collect()
    }

    fn node_fmt(&self, id: NodeId, depth: usize, f: &mut Formatter<'_>) {
        let line = self
            .graph_node(id)
            .line_id()
            .map(|id| self.get_line(id).to_plain_text())
            .unwrap_or(String::default());

        let ref_key = self.graph_node(id).ref_key().unwrap_or(Key::default());

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

    pub fn get_block_references_to(&self, key: &Key) -> Vec<NodeId> {
        // remove empty node ids
        self.index
            .get_block_references_to(key)
            .iter()
            .filter(|id| !self.graph_node(**id).is_empty())
            .cloned()
            .collect()
    }

    pub fn get_block_references_in(&self, key: &Key) -> Vec<NodeId> {
        self.maybe_key(key)
            .expect("to have key")
            .get_all_sub_nodes()
            .into_iter()
            .filter(|id| !self.graph_node(*id).is_empty())
            .filter(|id| self.graph_node(*id).is_ref())
            .collect()
    }

    pub fn get_inline_references_to(&self, key: &Key) -> Vec<NodeId> {
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
            .for_each(|(_, id)| self.node_fmt(*id, 0, f));
        write!(f, "")
    }
}

impl PartialEq for Graph {
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys && self.arena == other.arena
    }
}

impl InlinesContext for &Graph {
    fn get_ref_title(&self, key: &Key) -> Option<String> {
        self.get_key_title(&key)
    }
}

pub trait GraphContext: Copy {
    fn get_node_key(&self, id: NodeId) -> Key;
    fn get_node_id(&self, key: &Key) -> Option<NodeId>;
    fn get_node_id_at(&self, key: &Key, line: LineNumber) -> Option<NodeId>;
    fn node_line_number(&self, id: NodeId) -> Option<LineNumber>;

    fn random_key(&self, relative_to: &str) -> Key;

    fn key_of(&self, id: NodeId) -> Key;
    fn collect(&self, key: &Key) -> Tree;
    fn squash(&self, key: &Key, depth: u8) -> Tree;

    fn get_ref_text(&self, key: &Key) -> Option<String>;

    fn get_container_document_ref_text(&self, id: NodeId) -> String;
    fn get_text(&self, id: NodeId) -> String;

    fn node(&self, id: NodeId) -> impl NodePointer<'_>;

    fn markdown_options(&self) -> &MarkdownOptions;
}

pub trait GraphPatch<'a> {
    fn markdown(&self, key: &Key) -> Option<String>;
    fn add_key(&mut self, key: &Key, iter: impl NodeIter<'a>);
}

impl<'a> GraphPatch<'a> for Graph {
    fn add_key(&mut self, key: &Key, iter: impl NodeIter<'a>) {
        if iter.is_document() {
            self.build_key_and(key, |doc| {
                doc.insert_from_iter(iter.child().expect("to have child in document iter"))
            });
        } else {
            self.build_key_and(key, |doc| doc.insert_from_iter(iter));
        }
    }

    fn markdown(&self, key: &Key) -> Option<String> {
        self.export_key(key)
    }
}

impl GraphContext for &Graph {
    fn node(&self, id: NodeId) -> impl NodePointer<'_> {
        GraphNodePointer::new(self, id)
    }

    fn collect(&self, key: &Key) -> Tree {
        GraphNodePointer::new(self, *self.keys.get(key).expect("to have key")).collect_tree()
    }

    fn squash(&self, key: &Key, depth: u8) -> Tree {
        GraphNodePointer::new(self, self.get_node_id(&key).expect("to have key")).squash_tree(depth)
    }

    fn get_node_id_at(&self, key: &Key, line: LineNumber) -> Option<NodeId> {
        self.nodes_map
            .get(key)
            .expect(&format!("to have key, {}", key))
            .iter()
            .rev()
            .find(|(_, v)| (*v).contains(&line))
            .map(|(k, _)| k.clone())
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

    fn get_ref_text(&self, key: &Key) -> Option<String> {
        self.get_key_title(key)
    }

    fn get_container_document_ref_text(&self, id: NodeId) -> String {
        let container_key = self.node(id).to_document().unwrap().document_key().unwrap();
        self.get_key_title(&container_key)
            .unwrap_or_default()
            .to_string()
    }

    fn get_node_key(&self, id: NodeId) -> Key {
        self.node(id).to_document().unwrap().document_key().unwrap()
    }

    fn random_key(&self, relative_to: &str) -> Key {
        if self.sequential_keys {
            let key = self.keys().len() + 1;
            return Key::from_rel_link_url(&key.to_string(), relative_to);
        }

        loop {
            let key = Alphanumeric
                .sample_string(&mut rand::rng(), 8)
                .to_lowercase();
            if !self
                .keys
                .contains_key(&Key::from_rel_link_url(&key, relative_to))
            {
                return Key::from_rel_link_url(&key, relative_to);
            }
        }
    }

    fn get_node_id(&self, key: &Key) -> Option<NodeId> {
        self.keys.get(key).map(|v| *v)
    }

    fn key_of(&self, id: NodeId) -> Key {
        self.node_key(id)
    }

    fn markdown_options(&self) -> &MarkdownOptions {
        &self.markdown_options
    }
}

fn render_search_text(path: &NodePath, context: impl GraphContext) -> String {
    path.ids()
        .iter()
        .map(|id| context.get_text(*id).trim().to_string())
        .collect_vec()
        .join(" ")
        .chars()
        .filter(|c| c.is_alphabetic() || c.is_numeric())
        .collect::<String>()
}
