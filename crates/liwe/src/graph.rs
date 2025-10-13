use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Formatter},
};

use basic_iter::GraphNodePointer;
use graph_line::Line;
use index::RefIndex;
use log::debug;
use rand::distr::{Alphanumeric, SampleString};
use sections_builder::SectionsBuilder;

use crate::{
    markdown::MarkdownReader,
    model::{document::Document, rank::node_rank, tree::Tree},
};
use arena::Arena;
use builder::GraphBuilder;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use path::{graph_to_paths, NodePath};
use rayon::prelude::*;

use crate::parser::Parser;

use crate::graph::graph_node::GraphNode;
use crate::model::config::MarkdownOptions;
use crate::model::graph::GraphInlines;
use crate::model::node::{NodeIter, NodePointer};
use crate::model::InlinesContext;
use crate::model::{Content, Key, LineId, LineNumber, LineRange, NodeId, NodesMap, State};

mod arena;
pub mod basic_iter;
pub mod builder;
mod graph_line;
pub mod graph_node;
mod index;

pub mod path;
pub mod sections_builder;
mod squash_iter;

type Documents = HashMap<Key, Content>;

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
    content: Documents,
    paths: Vec<SearchPath>,
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

pub trait DatabaseContext {
    fn lines(&self, key: &Key) -> u32;
    fn parser(&self, key: &Key) -> Option<Parser>;
}

impl DatabaseContext for &Graph {
    fn parser(&self, key: &Key) -> Option<Parser> {
        self.content
            .get(key)
            .map(|content| Parser::new(&content, &self.markdown_options(), MarkdownReader::new()))
    }

    fn lines(&self, key: &Key) -> u32 {
        self.content
            .get(key)
            .map(|content| content.lines().count() as u32)
            .unwrap_or(0)
    }
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
        self.paths.clone()
    }

    fn update_search_paths(&mut self) {
        let graph_ref = &*self;
        self.paths = self
            .paths()
            .par_iter()
            .map(|path| SearchPath {
                search_text: render_search_text(path, graph_ref),
                node_rank: node_rank(graph_ref, path.last_id()),
                key: graph_ref.node_key(path.target()),
                root: path.ids().len() == 1,
                line: graph_ref.node_line_number(path.target()).unwrap_or(0) as u32,
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
            .collect::<Vec<_>>();
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

    pub fn global_search(&self, query: &str) -> Vec<SearchPath> {
        let matcher = SkimMatcherV2::default();
        assert_eq!(None, matcher.fuzzy_match("abc", "abx"));

        self.paths
            .par_iter()
            .map(|path| {
                (
                    path,
                    matcher.fuzzy_match(&path.search_text, query).unwrap_or(0),
                )
            })
            .collect::<Vec<_>>()
            .into_iter()
            .sorted_by(|(path_a, rank_a), (path_b, rank_b)| {
                if query.is_empty() {
                    path_b
                        .node_rank
                        .cmp(&path_a.node_rank)
                        .then_with(|| path_a.search_text.len().cmp(&path_b.search_text.len()))
                } else {
                    rank_b
                        .cmp(&rank_a)
                        .then_with(|| path_a.search_text.len().cmp(&path_b.search_text.len()))
                        .then_with(|| path_b.node_rank.cmp(&path_a.node_rank))
                }
            })
            .map(|(path, _)| path)
            .take(100)
            .cloned()
            .collect_vec()
    }

    pub fn get_document(&self, key: &Key) -> Option<Content> {
        self.content.get(key).cloned()
    }

    pub fn insert_document(&mut self, key: Key, content: Content) -> () {
        self.update_key(key.clone(), &content);
        self.content.insert(key.clone(), content);
        self.update_search_paths();
    }

    pub fn update_document(&mut self, key: Key, content: Content) -> () {
        self.update_key(key.clone(), &content);
        self.content.insert(key.clone(), content);
        self.update_search_paths();
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
        Self::from_state(content.clone(), false, markdown_options)
    }

    pub fn from_state(
        state: State,
        sequential_ids: bool,
        markdown_options: MarkdownOptions,
    ) -> Self {
        let mut graph = Graph::new_with_options(markdown_options.clone());
        graph.set_sequential_keys(sequential_ids);

        let reader = MarkdownReader::new();

        let blocks = state
            .iter()
            .sorted_by(|a, b| a.0.cmp(&b.0))
            .collect_vec()
            .par_iter()
            .map(|(k, v)| {
                debug!("parsing content, key={}", k);
                (Key::name(k), reader.document(v, &markdown_options))
            })
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

        graph.content = state
            .iter()
            .map(|(k, v)| (Key::name(k), v.clone()))
            .collect();

        graph.update_search_paths();

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
            .map(|node| {
                node.get_all_sub_nodes()
                    .into_iter()
                    .filter(|id| !self.graph_node(*id).is_empty())
                    .filter(|id| self.graph_node(*id).is_ref())
                    .collect()
            })
            .unwrap_or_default()
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
    fn unique_ids(&self, relative_to: &str, number: usize) -> Vec<String>;
    fn get_node_key(&self, id: NodeId) -> Key;
    fn get_node_id(&self, key: &Key) -> Option<NodeId>;
    fn get_node_id_at(&self, key: &Key, line: LineNumber) -> Option<NodeId>;
    fn node_line_number(&self, id: NodeId) -> Option<LineNumber>;

    fn random_key(&self, relative_to: &str) -> Key;
    fn random_keys(&self, relative_to: &str, number: usize) -> Vec<Key>;

    fn key_of(&self, id: NodeId) -> Key;
    fn collect(&self, key: &Key) -> Tree;
    fn squash(&self, key: &Key, depth: u8) -> Tree;

    fn get_ref_text(&self, key: &Key) -> Option<String>;
    fn get_container_document_ref_text(&self, id: NodeId) -> Option<String>;

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
            .get(key)?
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

    fn get_container_document_ref_text(&self, id: NodeId) -> Option<String> {
        let container_key = self.node(id).to_document().unwrap().document_key().unwrap();
        self.get_key_title(&container_key).map(|s| s.to_string())
    }

    fn get_node_key(&self, id: NodeId) -> Key {
        self.node(id).to_document().unwrap().document_key().unwrap()
    }

    fn random_key(&self, relative_to: &str) -> Key {
        self.random_keys(relative_to, 1)
            .first()
            .expect("to have one")
            .clone()
    }

    fn random_keys(&self, relative_to: &str, number: usize) -> Vec<Key> {
        self.unique_ids(relative_to, number)
            .iter()
            .map(|k| Key::from_rel_link_url(&k, relative_to))
            .collect_vec()
    }

    fn unique_ids(&self, relative_to: &str, number: usize) -> Vec<String> {
        let mut keys = vec![];

        if self.sequential_keys {
            for i in 0..number {
                let key_num = self.keys.len() + 1 + i as usize;
                keys.push(key_num.to_string());
            }
            return keys;
        } else {
            for _ in 0..number {
                loop {
                    let key = Alphanumeric
                        .sample_string(&mut rand::rng(), 8)
                        .to_lowercase();
                    if !self
                        .keys
                        .contains_key(&Key::from_rel_link_url(&key, relative_to))
                        && !keys.contains(&key)
                    {
                        keys.push(key.to_string());
                        break;
                    }
                }
            }
        }
        keys
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
