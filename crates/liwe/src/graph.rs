use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Formatter},
};

use basic_iter::GraphNodePointer;
use graph_line::Line;
use index::RefIndex;
use log::debug;
use rand::distr::{Alphanumeric, SampleString};
use sections_builder::SectionsBuilder;
use serde_yaml::Mapping;

use crate::model::{document::Document, tree::Tree};
use arena::{finalize_build, Arena, BuildArena, BuildIds, LineMap, NodeMap, NodeStore};
use builder::GraphBuilder;
use itertools::Itertools;
use path::{graph_to_paths, NodePath};
use rayon::prelude::*;

use crate::parser::Parser;

use crate::graph::graph_node::GraphNode;
use crate::model::config::{Format, FormatOptions, MarkdownOptions, WikiLinkPath};
use crate::model::frontmatter_to_string;
use crate::model::inline::Inlines;
use crate::model::key_index::KeyIndex;
use crate::model::node::Node;
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
pub mod walk;

type Documents = HashMap<Key, Content>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentReference {
    pub source_key: Key,
    pub source_title: Option<String>,
}

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

    pub fn normalize_ref_text(&self) -> bool {
        self.format_options.refs_text().normalize()
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
    }

    pub fn update_document(&mut self, key: Key, content: Content) {
        self.update_key(key.clone(), &content);
        self.content.insert(key.clone(), content);
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

    pub fn has_key(&self, key: &Key) -> bool {
        self.keys.contains_key(key)
    }

    pub fn refresh_ref_text(&mut self, key: &Key) {
        if !self.keys.contains_key(key) {
            self.keys_to_ref_text.remove(key);
            return;
        }
        match self.extract_ref_text(key) {
            Some(text) => {
                self.keys_to_ref_text.insert(key.clone(), text);
            }
            None => {
                self.keys_to_ref_text.remove(key);
            }
        }
    }

    pub fn reindex_keys(&mut self, keys: &[Key]) {
        let sources: HashSet<Key> = keys.iter().cloned().collect();
        let mut index = std::mem::take(&mut self.index);
        index.remove_edges_from_sources(self, &sources);
        self.index = index;

        let mut fresh = RefIndex::new();
        for key in keys {
            if let Some(root_id) = self.keys.get(key).copied() {
                fresh.index_node(self, root_id);
            }
        }
        self.index.merge(fresh);
    }

    pub fn rebuild_indexes(&mut self) {
        build_index_and_titles(self);
    }

    pub fn raw_metadata(&self, key: &Key) -> Option<String> {
        self.frontmatter.get(key).map(frontmatter_to_string)
    }

    pub fn get_document_references_to(&self, key: &Key) -> Vec<DocumentReference> {
        let mut seen = HashSet::new();
        self.index
            .get_inclusion_edges_to(key)
            .into_iter()
            .chain(self.index.get_reference_edges_to(key))
            .filter_map(|node_id| {
                let source_key = self.node_key(node_id);
                if source_key == *key || !seen.insert(source_key.clone()) {
                    return None;
                }
                Some(DocumentReference {
                    source_title: self.get_key_title(&source_key),
                    source_key,
                })
            })
            .collect()
    }

    pub fn root_section_keys(&self) -> Vec<Key> {
        self.keys
            .iter()
            .filter_map(|(key, &doc_id)| {
                let doc_node = self.graph_node(doc_id);
                let mut child_id = doc_node.child_id();
                while let Some(id) = child_id {
                    let child = self.graph_node(id);
                    if matches!(child, GraphNode::Section(_))
                        && self
                            .index
                            .get_inclusion_edges_to(&self.node_key(id))
                            .is_empty()
                    {
                        return Some(key.clone());
                    }
                    child_id = child.next_id();
                }
                None
            })
            .collect()
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

    pub fn node_count(&self) -> usize {
        self.arena.node_count()
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.arena.node_ids()
    }

    pub fn section_ids(&self) -> Vec<NodeId> {
        self.arena.section_ids()
    }

    pub fn new_node_id(&mut self) -> NodeId {
        self.arena.new_node_id()
    }

    pub fn get_line(&self, id: LineId) -> &Line {
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
        if let Some(Node::Document(_, frontmatter)) = iter.node() {
            match frontmatter {
                Some(mapping) => {
                    self.frontmatter.insert(key.clone(), mapping);
                }
                None => {
                    self.frontmatter.remove(key);
                }
            }
        }
        let map = self.build_key(key).insert_from_iter(iter);
        self.nodes_map.insert(key.clone(), map.clone());
        self.global_nodes_map.extend(map);
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
        Self::from_state(content, false, format_options, frontmatter_document_title)
    }

    pub fn from_state(
        state: &State,
        sequential_ids: bool,
        format_options: impl Into<FormatOptions>,
        frontmatter_document_title: Option<String>,
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

        merge_outputs(&mut graph, outputs);
        graph.key_index = key_index;
        build_index_and_titles(&mut graph);
        graph
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
        if self.keys.len() != other.keys.len() {
            return false;
        }
        if !self.keys.keys().all(|key| other.keys.contains_key(key)) {
            return false;
        }
        self.keys
            .keys()
            .all(|key| GraphContext::collect(&self, key) == GraphContext::collect(&other, key))
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
    fn normalize_ref_text(&self) -> bool {
        Graph::normalize_ref_text(self)
    }
}

struct DocBuildOutput {
    key: Key,
    root_id: NodeId,
    frontmatter: Option<Mapping>,
    nodes_map: crate::model::NodesMap,
    nodes: NodeMap,
    lines: LineMap,
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

fn merge_outputs(graph: &mut Graph, outputs: Vec<DocBuildOutput>) {
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
    graph.arena = finalize_build(all_parts);
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
    let root_ids: Vec<NodeId> = graph.keys.values().copied().collect();
    graph.index = if root_ids.len() < PARALLEL_BUILD_THRESHOLD {
        let mut idx = RefIndex::new();
        for id in &root_ids {
            idx.index_node(graph, *id);
        }
        idx
    } else {
        root_ids
            .par_iter()
            .fold(RefIndex::new, |mut local, id| {
                local.index_node(graph, *id);
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
    graph.keys_to_ref_text = extractions.into_iter().collect();
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
        let map = if iter.is_document() {
            self.build_key(key)
                .insert_from_iter(iter.child().expect("to have child in document iter"))
        } else {
            self.build_key(key).insert_from_iter(iter)
        };

        self.nodes_map.insert(key.clone(), map.clone());
        self.global_nodes_map.extend(map);

        self.extract_ref_text(key)
            .map(|text| self.keys_to_ref_text.insert(key.clone(), text));

        let id = self.get_document_id(key);
        let mut index = RefIndex::new();
        index.index_node(self, id);
        self.index.merge(index);
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

        let nodes_len = graph.node_count();
        let lines_len = graph.arena.lines_len();
        let global_len = graph.global_nodes_map.len();
        let edge_counts = graph.index.edge_counts();
        let key_counts = graph.index.key_counts();
        let markdown = graph.to_markdown(&"source".into());

        for _ in 0..100 {
            graph.update_document("source".into(), linking_document());
        }

        assert_eq!(graph.node_count(), nodes_len);
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
    fn shrink_then_grow_keeps_nodes_bounded() {
        let small = "# One\n".to_string();
        let large = "# One\n\n# Two\n\n# Three\n".to_string();

        let mut graph = Graph::new();
        graph.insert_document("source".into(), small.clone());
        let small_nodes = graph.node_count();

        graph.update_document("source".into(), large.clone());
        let large_nodes = graph.node_count();
        let large_markdown = graph.to_markdown(&"source".into());

        graph.update_document("source".into(), small.clone());
        assert_eq!(graph.node_count(), small_nodes);
        assert_eq!(graph.to_markdown(&"source".into()), small);

        graph.update_document("source".into(), large.clone());
        assert_eq!(graph.node_count(), large_nodes);
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

#[cfg(test)]
mod new_api_tests {
    use super::*;

    #[test]
    fn has_key_reflects_presence() {
        let mut graph = Graph::new();
        graph.insert_document("a".into(), "# A\n".to_string());
        assert!(graph.has_key(&"a".into()));
        assert!(!graph.has_key(&"missing".into()));
    }

    #[test]
    fn raw_metadata_returns_stripped_frontmatter() {
        let mut graph = Graph::new();
        graph.insert_document("a".into(), "---\ntitle: Hello\n---\n\n# A\n".to_string());
        assert_eq!(
            graph.raw_metadata(&"a".into()),
            Some("title: Hello".to_string())
        );
        assert_eq!(graph.raw_metadata(&"no-front".into()), None);
    }

    #[test]
    fn refresh_ref_text_missing_key_does_not_panic() {
        let mut graph = Graph::new();
        graph
            .keys_to_ref_text
            .insert("ghost".into(), "Stale".to_string());
        graph.refresh_ref_text(&"ghost".into());
        assert_eq!(graph.get_key_title(&"ghost".into()), None);
    }

    #[test]
    fn refresh_ref_text_updates_and_clears() {
        let mut graph = Graph::new();
        graph.insert_document("titled".into(), "# Fresh Title\n".to_string());
        graph.keys_to_ref_text.remove(&"titled".into());
        graph.refresh_ref_text(&"titled".into());
        assert_eq!(
            graph.get_key_title(&"titled".into()),
            Some("Fresh Title".to_string())
        );

        graph.insert_document("untitled".into(), "A plain paragraph.\n".to_string());
        graph
            .keys_to_ref_text
            .insert("untitled".into(), "Stale".to_string());
        graph.refresh_ref_text(&"untitled".into());
        assert_eq!(graph.get_key_title(&"untitled".into()), None);
    }

    #[test]
    fn rebuild_indexes_drops_stale_titles() {
        let mut graph = Graph::new();
        graph.insert_document("a".into(), "# A Title\n".to_string());
        graph
            .keys_to_ref_text
            .insert("ghost".into(), "Stale".to_string());
        graph.rebuild_indexes();
        assert_eq!(graph.get_key_title(&"ghost".into()), None);
        assert_eq!(
            graph.get_key_title(&"a".into()),
            Some("A Title".to_string())
        );
    }

    #[test]
    fn document_references_cover_inclusion_and_reference_edges() {
        let mut graph = Graph::new();
        graph.insert_document("target".into(), "# Target\n".to_string());
        graph.insert_document(
            "via-link".into(),
            "See [Target](target) here.\n".to_string(),
        );
        graph.insert_document(
            "via-include".into(),
            "# Src\n\n[Target](target)\n".to_string(),
        );

        let keys: Vec<String> = graph
            .get_document_references_to(&"target".into())
            .iter()
            .map(|reference| reference.source_key.to_string())
            .sorted()
            .collect();
        assert_eq!(
            keys,
            vec!["via-include".to_string(), "via-link".to_string()]
        );
    }

    #[test]
    fn document_references_dedup_and_carry_title() {
        let mut graph = Graph::new();
        graph.insert_document("target".into(), "# Target\n".to_string());
        graph.insert_document(
            "src".into(),
            "# Source Title\n\nSee [Target](target) here.\n\n[Target](target)\n".to_string(),
        );
        assert_eq!(
            graph.get_document_references_to(&"target".into()),
            vec![DocumentReference {
                source_key: "src".into(),
                source_title: Some("Source Title".to_string()),
            }]
        );
    }

    #[test]
    fn document_references_exclude_self() {
        let mut graph = Graph::new();
        graph.insert_document("self".into(), "# Self\n\n[Self](self)\n".to_string());
        assert_eq!(
            graph.get_document_references_to(&"self".into()),
            Vec::<DocumentReference>::new()
        );
    }

    #[test]
    fn reindex_keys_replaces_stale_edges() {
        let mut graph = Graph::new();
        graph.insert_document("target-a".into(), "# A\n".to_string());
        graph.insert_document("target-b".into(), "# B\n".to_string());
        graph.insert_document("source".into(), "Link to [A](target-a) here.\n".to_string());
        assert_eq!(graph.get_reference_edges_to(&"target-a".into()).len(), 1);
        assert_eq!(graph.get_reference_edges_to(&"target-b".into()).len(), 0);

        let mut other = Graph::new();
        other.insert_document("source".into(), "Link to [B](target-b) here.\n".to_string());
        let relinked = (&other).collect(&"source".into()).with_new_ids();
        graph.build_key_from_iter(&"source".into(), relinked.iter());

        graph.reindex_keys(&["source".into()]);

        assert_eq!(graph.get_reference_edges_to(&"target-a".into()).len(), 0);
        assert_eq!(graph.get_reference_edges_to(&"target-b".into()).len(), 1);
    }

    #[test]
    fn root_section_keys_excludes_transcluded_documents() {
        let mut graph = Graph::new();
        graph.insert_document("child".into(), "# Child\n".to_string());
        graph.insert_document("root".into(), "# Root\n\n[Child](child)\n".to_string());

        let keys: Vec<String> = graph
            .root_section_keys()
            .iter()
            .map(|key| key.to_string())
            .sorted()
            .collect();
        assert_eq!(keys, vec!["root".to_string()]);
    }
}
