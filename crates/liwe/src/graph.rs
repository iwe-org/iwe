use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    path::Path,
};

use basic_iter::GraphNodePointer;
use graph_line::Line;
use index::RefIndex;
use log::debug;
use rand::distr::{Alphanumeric, SampleString};
use sections_builder::SectionsBuilder;
use serde_yaml::Mapping;

use crate::model::{document::Document, tree::Tree};
use arena::{finalize_build, Arena, BuildArena, BuildIds, NodeStore};
use builder::GraphBuilder;
use itertools::Itertools;
use path::{graph_to_paths, NodePath};
use rayon::prelude::*;

use crate::parser::Parser;

use crate::graph::graph_node::GraphNode;
use crate::model::config::{Format, FormatOptions, MarkdownOptions, WikiLinkPath};
use crate::model::inline::Inlines;
use crate::model::key_index::KeyIndex;
use crate::model::node::{NodeIter, NodePointer};
use crate::model::InlinesContext;
use crate::model::{Content, Key, LineId, LineNumber, LineRange, NodeId, NodesMap, State};
use crate::search::{Bm25Index, Language, ScoredDocument};

mod arena;
pub mod basic_iter;
pub mod builder;
mod graph_line;
pub mod graph_node;
mod index;

pub mod path;
pub mod sections_builder;
mod squash_iter;
pub mod walk;

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
    format_options: FormatOptions,
    frontmatter: HashMap<Key, Mapping>,
    content: Documents,
    frontmatter_document_title: Option<String>,
    key_index: KeyIndex,
    bm25: Option<Bm25Index>,
}

pub trait Reader {
    fn document(&self, content: &str, markdown_options: &MarkdownOptions) -> Document;
}

pub trait DatabaseContext {
    fn lines(&self, key: &Key) -> u32;
    fn parser(&self, key: &Key) -> Option<Parser>;
}

impl DatabaseContext for &Graph {
    fn parser(&self, key: &Key) -> Option<Parser> {
        self.content
            .get(key)
            .map(|content| Parser::new(content, &self.format_options))
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

    pub fn format(&self) -> Format {
        self.format_options.format()
    }

    pub fn format_options(&self) -> &FormatOptions {
        &self.format_options
    }

    pub fn wiki_display(&self, key: &Key, original_url: &str) -> String {
        match self.format_options.markdown_options().wiki_link_path {
            WikiLinkPath::Full => key.to_library_url(),
            WikiLinkPath::Short => self.key_index.shorten_wiki(key),
            WikiLinkPath::Preserve => original_url.to_string(),
        }
    }

    pub fn new_patch(&self) -> Graph {
        Graph {
            format_options: self.format_options.clone(),
            frontmatter: self.frontmatter.clone(),
            frontmatter_document_title: self.frontmatter_document_title.clone(),
            key_index: self.key_index.clone(),
            ..Default::default()
        }
    }

    pub fn new_with_options(format_options: impl Into<FormatOptions>) -> Graph {
        Graph {
            format_options: format_options.into(),
            ..Default::default()
        }
    }

    pub fn set_sequential_keys(&mut self, sequential_keys: bool) {
        self.sequential_keys = sequential_keys;
    }

    pub fn get_document(&self, key: &Key) -> Option<Content> {
        self.content.get(key).cloned()
    }

    pub fn insert_document(&mut self, key: Key, content: Content) {
        self.update_key(key.clone(), &content);
        self.content.insert(key.clone(), content);
        self.reindex_search(&key);
    }

    pub fn update_document(&mut self, key: Key, content: Content) {
        self.update_key(key.clone(), &content);
        self.content.insert(key.clone(), content);
        self.reindex_search(&key);
    }

    pub fn remove_document(&mut self, key: Key) {
        if let Some(id) = self.keys.get(&key).copied() {
            self.unindex_key(&key, id);
            self.arena.delete_branch(id);
        }

        self.keys.remove(&key);
        self.key_index.remove(&key);
        self.nodes_map.remove(&key);
        self.keys_to_ref_text.remove(&key);
        self.frontmatter.remove(&key);
        self.content.remove(&key);

        if let Some(index) = self.bm25.as_mut() {
            index.remove(&key);
        }
    }

    fn reindex_search(&mut self, key: &Key) {
        if self.bm25.is_none() {
            return;
        }
        let text = self.corpus_text(key);
        if let Some(index) = self.bm25.as_mut() {
            index.upsert(key.clone(), text);
        }
    }

    fn corpus_text(&self, key: &Key) -> String {
        let title = self.get_key_title(key).unwrap_or_default();
        format!("{}\n{}", title, self.to_plain_text(key))
    }

    pub fn search(&self, query: &str) -> Vec<ScoredDocument<Key>> {
        self.bm25
            .as_ref()
            .map(|index| index.search(query))
            .unwrap_or_default()
    }

    pub fn has_search_index(&self) -> bool {
        self.bm25.is_some()
    }

    pub fn search_scores(&self, query: &str) -> HashMap<Key, f32> {
        self.bm25
            .as_ref()
            .map(|index| index.scores(query))
            .unwrap_or_default()
    }

    pub fn lexical_query_has_terms(&self, query: &str) -> bool {
        self.bm25
            .as_ref()
            .map(|index| index.has_query_terms(query))
            .unwrap_or(true)
    }

    fn unindex_key(&mut self, key: &Key, root_id: NodeId) {
        let mut index = std::mem::take(&mut self.index);
        index.unindex_node(self, root_id);
        self.index = index;

        let old_ids: Vec<NodeId> = self
            .nodes_map
            .get(key)
            .map(|map| map.iter().map(|(id, _)| *id).collect())
            .unwrap_or_default();
        for id in old_ids {
            self.global_nodes_map.remove(&id);
        }
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

    pub fn inclusion_edge_target_keys(&self) -> Vec<Key> {
        self.index.inclusion_edge_target_keys().cloned().collect()
    }

    pub fn reference_edge_target_keys(&self) -> Vec<Key> {
        self.index.reference_edge_target_keys().cloned().collect()
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

    pub fn new_node_id(&mut self) -> NodeId {
        self.arena.new_node_id()
    }

    pub fn get_line(&self, id: LineId) -> Line {
        self.arena.get_line(id)
    }

    pub fn build_key(&mut self, key: &Key) -> GraphBuilder<'_> {
        let id = self.arena.new_node_id();
        self.keys.insert(key.clone(), id);
        self.key_index.insert(key);
        self.arena
            .set_node(id, GraphNode::new_root(key.clone(), id));
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
        F: FnOnce(&mut GraphBuilder),
    {
        let id = self.arena.new_node_id();
        self.keys.insert(key.clone(), id);
        self.key_index.insert(key);
        self.arena
            .set_node(id, GraphNode::new_root(key.clone(), id));
        f(&mut GraphBuilder::new(self, id));

        self.extract_ref_text(key)
            .map(|text| self.keys_to_ref_text.insert(key.clone(), text));

        let mut index = RefIndex::new();
        index.index_node(self, id);
        self.index.merge(index);

        self
    }

    fn extract_ref_text(&self, key: &Key) -> Option<String> {
        if let Some(title) = self.extract_frontmatter_title(key) {
            return Some(title);
        }

        self.graph_node(self.get_document_id(key))
            .child_id()
            .map(|id| self.graph_node(id))
            .filter(|node| node.is_section())
            .and_then(|node| node.line_id())
            .map(|line_id| self.arena.get_line(line_id).to_plain_text())
    }

    fn extract_frontmatter_title(&self, key: &Key) -> Option<String> {
        let title_key = self.frontmatter_document_title.as_ref()?;
        let mapping = self.frontmatter.get(key)?;
        mapping
            .get(serde_yaml::Value::String(title_key.clone()))?
            .as_str()
            .map(|s| s.to_string())
    }

    pub fn frontmatter(&self, key: &Key) -> Option<&Mapping> {
        self.frontmatter.get(key)
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
            .unwrap_or_else(|| panic!("to have key, {}", key))
    }

    pub fn from_markdown(&mut self, key: Key, content: &str, reader: impl Reader) {
        let document = reader.document(content, &self.format_options.markdown_options());
        self.ingest_document(key, document);
    }

    fn ingest_document(&mut self, key: Key, document: Document) {
        if let Some(fm) = document.frontmatter {
            self.frontmatter.insert(key.clone(), fm);
        } else {
            self.frontmatter.remove(&key);
        }

        let mut key_index = std::mem::take(&mut self.key_index);
        key_index.insert(&key);

        let mut build_key = self.build_key(&key);
        let id = build_key.id();

        let nodes_map =
            SectionsBuilder::new(&mut build_key, &document.blocks, &key, &key_index).nodes_map();

        self.nodes_map.insert(key.clone(), nodes_map.clone());
        self.global_nodes_map.extend(nodes_map);

        let mut index = RefIndex::new();
        index.index_node(self, id);
        self.index.merge(index);

        self.key_index = key_index;

        self.extract_ref_text(&key)
            .map(|text| self.keys_to_ref_text.insert(key, text));
    }

    pub fn key_index(&self) -> &KeyIndex {
        &self.key_index
    }

    pub fn to_markdown(&self, key: &Key) -> String {
        if !self.keys.contains_key(key) {
            return String::new();
        }
        let markdown = self
            .collect(key)
            .iter()
            .to_text(&key.parent(), &self.format_options);

        markdown
    }

    pub fn to_markdown_skip_frontmatter(&self, key: &Key) -> String {
        if !self.keys.contains_key(key) {
            return String::new();
        }
        let markdown = self
            .collect(key)
            .iter()
            .to_text_skip_frontmatter(&key.parent(), &self.format_options);

        markdown
    }

    pub fn to_plain_text(&self, key: &Key) -> String {
        if !self.keys.contains_key(key) {
            return String::new();
        }
        let mut lines = Vec::new();
        collect_plain_text(&self.collect(key), &mut lines);
        lines.join("\n")
    }

    pub fn paths(&self) -> Vec<NodePath> {
        graph_to_paths(self)
    }

    pub fn update_key(&mut self, key: Key, content: &str) -> &mut Graph {
        if let Some(id) = self.keys.get(&key).copied() {
            self.unindex_key(&key, id);
            self.arena.delete_branch(id);
        }

        let document = crate::format::read_document(content, &self.format_options);
        self.ingest_document(key, document);

        self
    }

    pub fn node_line_range(&self, id: NodeId) -> Option<LineRange> {
        self.global_nodes_map.get(&id).cloned()
    }

    pub fn import(
        content: &State,
        format_options: impl Into<FormatOptions>,
        frontmatter_document_title: Option<String>,
    ) -> Graph {
        Self::from_state(
            content,
            false,
            format_options,
            frontmatter_document_title,
            None,
        )
    }

    pub fn from_state(
        state: &State,
        sequential_ids: bool,
        format_options: impl Into<FormatOptions>,
        frontmatter_document_title: Option<String>,
        search_language: Option<Language>,
    ) -> Self {
        let format_options = format_options.into();
        let mut graph = Graph::new_with_options(format_options.clone());
        graph.set_sequential_keys(sequential_ids);
        graph.frontmatter_document_title = frontmatter_document_title;

        let keys: Vec<Key> = state.iter().map(|(k, _)| Key::from_stripped(k)).collect();
        let key_index = KeyIndex::build(keys.iter());

        let ids = BuildIds::new();
        let outputs: Vec<DocBuildOutput> = if state.len() < PARALLEL_BUILD_THRESHOLD {
            state
                .iter()
                .map(|(k, v)| {
                    build_doc(
                        &ids,
                        Key::from_stripped(k),
                        v.clone(),
                        &format_options,
                        &key_index,
                    )
                })
                .collect()
        } else {
            state
                .par_iter()
                .map(|(k, v)| {
                    debug!("building doc, key={}", k);
                    build_doc(
                        &ids,
                        Key::from_stripped(k),
                        v.clone(),
                        &format_options,
                        &key_index,
                    )
                })
                .collect()
        };

        merge_outputs(&mut graph, outputs, &ids);
        graph.key_index = key_index;
        build_index_and_titles(&mut graph);
        graph.build_search_index(search_language);
        graph
    }

    pub fn from_path(
        base_path: &Path,
        sequential_ids: bool,
        format_options: impl Into<FormatOptions>,
        frontmatter_document_title: Option<String>,
        search_language: Option<Language>,
    ) -> Self {
        let format_options = format_options.into();
        let mut graph = Graph::new_with_options(format_options.clone());
        graph.set_sequential_keys(sequential_ids);
        graph.frontmatter_document_title = frontmatter_document_title;

        let entries = crate::fs::walk_md_paths(base_path, format_options.format());

        let keys: Vec<Key> = entries.iter().map(|(k, _)| Key::from_stripped(k)).collect();
        let key_index = KeyIndex::build(keys.iter());

        let ids = BuildIds::new();
        let outputs: Vec<DocBuildOutput> = if entries.len() < PARALLEL_BUILD_THRESHOLD {
            entries
                .into_iter()
                .filter_map(|(key, path)| {
                    crate::fs::read_md_file(&path).map(|content| {
                        build_doc(
                            &ids,
                            Key::from_stripped(&key),
                            content,
                            &format_options,
                            &key_index,
                        )
                    })
                })
                .collect()
        } else {
            entries
                .into_par_iter()
                .filter_map(|(key, path)| {
                    debug!("building doc, key={}", key);
                    crate::fs::read_md_file(&path).map(|content| {
                        build_doc(
                            &ids,
                            Key::from_stripped(&key),
                            content,
                            &format_options,
                            &key_index,
                        )
                    })
                })
                .collect()
        };

        merge_outputs(&mut graph, outputs, &ids);
        graph.key_index = key_index;
        build_index_and_titles(&mut graph);
        graph.build_search_index(search_language);
        graph
    }

    fn build_search_index(&mut self, search_language: Option<Language>) {
        let Some(language) = search_language else {
            return;
        };
        let graph = &*self;
        let docs: Vec<(Key, String)> = if graph.keys.len() < PARALLEL_BUILD_THRESHOLD {
            graph
                .keys
                .keys()
                .map(|key| (key.clone(), graph.corpus_text(key)))
                .collect()
        } else {
            graph
                .keys
                .par_iter()
                .map(|(key, _)| (key.clone(), graph.corpus_text(key)))
                .collect()
        };
        self.bm25 = Some(Bm25Index::build(docs, language));
    }

    pub fn export_key(&self, key: &Key) -> Option<String> {
        Some(self.to_markdown(key))
    }

    pub fn export(&self) -> State {
        self.keys
            .par_iter()
            .map(|(k, _)| {
                let markdown = self
                    .collect(k)
                    .iter()
                    .to_text(&k.parent(), &self.format_options);
                (k.to_string(), markdown)
            })
            .collect()
    }

    fn node_fmt(&self, id: NodeId, depth: usize, f: &mut Formatter<'_>) {
        let line = self
            .graph_node(id)
            .line_id()
            .map(|id| self.get_line(id).to_plain_text())
            .unwrap_or_default();

        let ref_key = self.graph_node(id).ref_key().unwrap_or_default();

        let _ = writeln!(
            f,
            "{:indent$} • {}{}: {}{}",
            "",
            self.graph_node(id).to_symbol(),
            id,
            line,
            ref_key,
            indent = depth * 2
        );
        let node = self.graph_node(id);

        if let Some(id) = node.child_id() {
            self.node_fmt(id, depth + 1, f)
        }
        if let Some(id) = node.next_id() {
            self.node_fmt(id, depth, f)
        }
    }

    pub fn get_inclusion_edges_to(&self, key: &Key) -> Vec<NodeId> {
        self.index
            .get_inclusion_edges_to(key)
            .iter()
            .filter(|id| !self.graph_node(**id).is_empty())
            .cloned()
            .collect()
    }

    pub fn get_inclusion_edges_in(&self, key: &Key) -> Vec<NodeId> {
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

    pub fn get_reference_edges_to(&self, key: &Key) -> Vec<NodeId> {
        self.index
            .get_reference_edges_to(key)
            .iter()
            .filter(|id| !self.graph_node(**id).is_empty())
            .cloned()
            .collect()
    }

    pub fn get_reference_edges_in(&self, key: &Key) -> Vec<Key> {
        let Some(pointer) = self.maybe_key(key) else {
            return Vec::new();
        };
        let mut keys = Vec::new();
        for node_id in pointer.get_all_sub_nodes() {
            match self.graph_node(node_id) {
                GraphNode::Section(section) => {
                    keys.extend(self.get_line(section.line_id()).ref_keys());
                }
                GraphNode::Leaf(leaf) => {
                    keys.extend(self.get_line(leaf.line_id()).ref_keys());
                }
                GraphNode::Table(table) => {
                    for line_id in table.header() {
                        keys.extend(self.get_line(*line_id).ref_keys());
                    }
                    for row in table.rows() {
                        for line_id in row {
                            keys.extend(self.get_line(*line_id).ref_keys());
                        }
                    }
                }
                _ => {}
            }
        }
        keys
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

impl NodeStore for Graph {
    fn new_node_id(&mut self) -> NodeId {
        self.arena.new_node_id()
    }

    fn add_line(&mut self, inlines: Inlines) -> LineId {
        self.arena.add_line(inlines)
    }

    fn add_graph_node(&mut self, node: GraphNode) -> NodeId {
        let id = node.id();
        self.arena.set_node(id, node);
        id
    }

    fn update_node(&mut self, id: NodeId, f: &mut dyn FnMut(&mut GraphNode)) {
        self.arena.update_node(id, f);
    }

    fn graph_node(&self, id: NodeId) -> GraphNode {
        self.arena.node(id)
    }
}

impl InlinesContext for &Graph {
    fn get_ref_title(&self, key: &Key) -> Option<String> {
        self.get_key_title(key)
    }
    fn wiki_display(&self, key: &Key, original_url: &str) -> String {
        Graph::wiki_display(self, key, original_url)
    }
}

struct DocBuildOutput {
    key: Key,
    root_id: NodeId,
    frontmatter: Option<Mapping>,
    nodes_map: crate::model::NodesMap,
    nodes: HashMap<NodeId, GraphNode>,
    lines: HashMap<crate::graph::LineId, graph_line::Line>,
    content: String,
}

fn build_doc(
    ids: &BuildIds,
    key: Key,
    content: String,
    format_options: &FormatOptions,
    key_index: &KeyIndex,
) -> DocBuildOutput {
    let document = crate::format::read_document(&content, format_options);

    let mut arena = BuildArena::new(ids);
    let root_id = arena.new_node_id();
    arena.set_node(root_id, GraphNode::new_root(key.clone(), root_id));

    let nodes_map = SectionsBuilder::new(
        &mut GraphBuilder::new(&mut arena, root_id),
        &document.blocks,
        &key,
        key_index,
    )
    .nodes_map();

    let (nodes, lines) = arena.into_parts();

    DocBuildOutput {
        key,
        root_id,
        frontmatter: document.frontmatter,
        nodes_map,
        nodes,
        lines,
        content,
    }
}

fn merge_outputs(graph: &mut Graph, outputs: Vec<DocBuildOutput>, ids: &BuildIds) {
    let mut all_parts = Vec::with_capacity(outputs.len());
    for output in outputs {
        if let Some(fm) = output.frontmatter {
            graph.frontmatter.insert(output.key.clone(), fm);
        } else {
            graph.frontmatter.remove(&output.key);
        }
        graph.keys.insert(output.key.clone(), output.root_id);
        graph
            .nodes_map
            .insert(output.key.clone(), output.nodes_map.clone());
        graph.global_nodes_map.extend(output.nodes_map);
        graph.content.insert(output.key.clone(), output.content);
        all_parts.push((output.nodes, output.lines));
    }
    graph.arena = finalize_build(ids, all_parts);
}

fn collect_plain_text(tree: &Tree, lines: &mut Vec<String>) {
    let text = tree.node.plain_text();
    if !text.is_empty() {
        lines.push(text);
    }
    for child in &tree.children {
        collect_plain_text(child, lines);
    }
}

fn build_index_and_titles(graph: &mut Graph) {
    let nodes = graph.arena.nodes();
    graph.index = if nodes.len() < PARALLEL_BUILD_THRESHOLD {
        let mut idx = RefIndex::new();
        for node in nodes {
            idx.index_node(graph, node.id());
        }
        idx
    } else {
        nodes
            .par_chunks(4096)
            .map(|chunk| {
                let mut local = RefIndex::new();
                for node in chunk {
                    local.index_node(graph, node.id());
                }
                local
            })
            .reduce(RefIndex::new, |mut acc, other| {
                acc.merge(other);
                acc
            })
    };

    let extractions: Vec<(Key, String)> = if graph.keys.len() < PARALLEL_BUILD_THRESHOLD {
        graph
            .keys
            .iter()
            .filter_map(|(key, _)| graph.extract_ref_text(key).map(|text| (key.clone(), text)))
            .collect()
    } else {
        graph
            .keys
            .par_iter()
            .filter_map(|(key, _)| graph.extract_ref_text(key).map(|text| (key.clone(), text)))
            .collect()
    };
    graph.keys_to_ref_text.extend(extractions);
}

const PARALLEL_BUILD_THRESHOLD: usize = 128;

pub trait GraphContext: Copy {
    fn unique_ids(&self, relative_to: &str, number: usize) -> Vec<String>;
    fn get_node_key(&self, id: NodeId) -> Option<Key>;
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

    fn markdown_options(&self) -> MarkdownOptions;
    fn format_options(&self) -> FormatOptions;
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
        GraphNodePointer::new(self, self.get_node_id(key).expect("to have key")).squash_tree(depth)
    }

    fn get_node_id_at(&self, key: &Key, line: LineNumber) -> Option<NodeId> {
        self.nodes_map
            .get(key)?
            .iter()
            .rev()
            .find(|(_, v)| (*v).contains(&line))
            .map(|(k, _)| *k)
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
        let container_key = self.node(id).to_document()?.document_key()?;
        self.get_key_title(&container_key)
    }

    fn get_node_key(&self, id: NodeId) -> Option<Key> {
        self.node(id).to_document()?.document_key()
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
            .map(|k| Key::from_rel_link_url(k, relative_to))
            .collect_vec()
    }

    fn unique_ids(&self, relative_to: &str, number: usize) -> Vec<String> {
        let mut keys = vec![];

        if self.sequential_keys {
            for i in 0..number {
                let key_num = self.keys.len() + 1 + i;
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
        self.keys.get(key).copied()
    }

    fn key_of(&self, id: NodeId) -> Key {
        self.node_key(id)
    }

    fn markdown_options(&self) -> MarkdownOptions {
        self.format_options.markdown_options()
    }

    fn format_options(&self) -> FormatOptions {
        self.format_options.clone()
    }
}

#[cfg(test)]
mod plain_text_tests {
    use super::*;

    fn plain_text_of(content: &str) -> String {
        let mut graph = Graph::new();
        graph.insert_document("doc".into(), content.to_string());
        graph.to_plain_text(&"doc".into())
    }

    #[test]
    fn strips_markup_keeps_link_text_and_code_drops_frontmatter() {
        let content = "---\ntitle: Front Title\n---\n\n# Heading One\n\nA paragraph with a [display text](http://example.com/page) link.\n\n```rust\nlet answer = 42;\n```\n";
        assert_eq!(
            plain_text_of(content),
            "Heading One\nA paragraph with a display text link.\nlet answer = 42;\n"
        );
    }

    #[test]
    fn includes_table_header_and_cell_text() {
        let content = "# Numbers\n\n| Name | Value |\n| ---- | ----- |\n| foo | bar |\n";
        assert_eq!(plain_text_of(content), "Numbers\nName Value foo bar");
    }

    #[test]
    fn block_reference_is_not_expanded() {
        let mut graph = Graph::new();
        graph.insert_document(
            "target".into(),
            "# Target Title\n\nSecret target body.\n".to_string(),
        );
        graph.insert_document(
            "source".into(),
            "# Source\n\n[Target Title](target)\n".to_string(),
        );
        assert_eq!(
            graph.to_plain_text(&"source".into()),
            "Source\nTarget Title"
        );
    }
}

#[cfg(test)]
mod retention_tests {
    use super::*;

    fn linking_document() -> String {
        "# Title\n\nA paragraph linking to [target](target).\n\n## Section\n\nAnother paragraph.\n"
            .to_string()
    }

    #[test]
    fn repeated_updates_keep_graph_bounded() {
        let mut graph = Graph::new();
        graph.insert_document("target".into(), "# Target\n".to_string());
        graph.insert_document("source".into(), linking_document());

        graph.update_document("source".into(), linking_document());

        let nodes_len = graph.nodes().len();
        let lines_len = graph.arena.lines_len();
        let global_len = graph.global_nodes_map.len();
        let edge_counts = graph.index.edge_counts();
        let key_counts = graph.index.key_counts();
        let markdown = graph.to_markdown(&"source".into());

        for _ in 0..100 {
            graph.update_document("source".into(), linking_document());
        }

        assert_eq!(graph.nodes().len(), nodes_len);
        assert_eq!(graph.arena.lines_len(), lines_len);
        assert_eq!(graph.global_nodes_map.len(), global_len);
        assert_eq!(graph.index.edge_counts(), edge_counts);
        assert_eq!(graph.index.key_counts(), key_counts);
        assert_eq!(graph.to_markdown(&"source".into()), markdown);
        assert_eq!(graph.get_reference_edges_to(&"target".into()).len(), 1);
    }

    fn table_document() -> String {
        "# Title\n\n| a | b |\n| - | - |\n| c | d |\n".to_string()
    }

    #[test]
    fn repeated_table_updates_keep_lines_bounded() {
        let mut graph = Graph::new();
        graph.insert_document("source".into(), table_document());

        let lines_len = graph.arena.lines_len();
        let markdown = graph.to_markdown(&"source".into());

        for _ in 0..100 {
            graph.update_document("source".into(), table_document());
        }

        assert_eq!(graph.arena.lines_len(), lines_len);
        assert_eq!(graph.to_markdown(&"source".into()), markdown);
    }

    #[test]
    fn shrink_then_grow_reuses_slots() {
        let small = "# One\n".to_string();
        let large = "# One\n\n# Two\n\n# Three\n".to_string();

        let mut graph = Graph::new();
        graph.insert_document("source".into(), small.clone());

        graph.update_document("source".into(), large.clone());
        let large_nodes = graph.nodes().len();
        let large_markdown = graph.to_markdown(&"source".into());

        graph.update_document("source".into(), small.clone());
        assert_eq!(graph.nodes().len(), large_nodes);

        graph.update_document("source".into(), large.clone());
        assert_eq!(graph.nodes().len(), large_nodes);
        assert_eq!(graph.to_markdown(&"source".into()), large_markdown);
    }

    #[test]
    fn remove_document_clears_edges() {
        let mut graph = Graph::new();
        graph.insert_document("target".into(), "# Target\n".to_string());

        let baseline_global = graph.global_nodes_map.len();
        let baseline_edges = graph.index.edge_counts();
        let baseline_keys = graph.index.key_counts();

        graph.insert_document("source".into(), linking_document());
        assert_eq!(graph.get_reference_edges_to(&"target".into()).len(), 1);

        graph.remove_document("source".into());

        assert_eq!(graph.get_reference_edges_to(&"target".into()).len(), 0);
        assert_eq!(graph.global_nodes_map.len(), baseline_global);
        assert_eq!(graph.index.edge_counts(), baseline_edges);
        assert_eq!(graph.index.key_counts(), baseline_keys);
    }
}
