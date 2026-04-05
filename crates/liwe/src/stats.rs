use std::collections::HashSet;
use std::io::Write;

use itertools::Itertools;
use rayon::prelude::*;
use serde::Serialize;

use crate::graph::basic_iter::GraphNodePointer;
use crate::graph::{Graph, GraphContext};
use crate::model::is_ref_url;
use crate::model::node::{Node, NodeIter, NodePointer};
use crate::model::Key;

#[derive(Debug, Clone, Serialize)]
pub struct BrokenLink {
    #[serde(serialize_with = "serialize_key")]
    pub source_key: Key,
    #[serde(serialize_with = "serialize_key")]
    pub target_key: Key,
}

fn serialize_key<S: serde::Serializer>(key: &Key, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&key.to_string())
}

fn broken_links(graph: &Graph) -> Vec<BrokenLink> {
    let existing_keys: HashSet<Key> = graph.keys().into_iter().collect();
    let mut seen = HashSet::new();
    let mut broken = Vec::new();

    for target_key in graph.block_reference_target_keys() {
        if !existing_keys.contains(&target_key) {
            for node_id in graph.get_block_references_to(&target_key) {
                let source_key = graph.key_of(node_id);
                if seen.insert((source_key.clone(), target_key.clone())) {
                    broken.push(BrokenLink {
                        source_key,
                        target_key: target_key.clone(),
                    });
                }
            }
        }
    }

    for target_key in graph.inline_reference_target_keys() {
        if !existing_keys.contains(&target_key) && is_ref_url(&target_key.to_string()) {
            for node_id in graph.get_inline_references_to(&target_key) {
                let source_key = graph.key_of(node_id);
                if seen.insert((source_key.clone(), target_key.clone())) {
                    broken.push(BrokenLink {
                        source_key,
                        target_key: target_key.clone(),
                    });
                }
            }
        }
    }

    broken.sort_by(|a, b| (&a.source_key, &a.target_key).cmp(&(&b.source_key, &b.target_key)));
    broken
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct KeyStatistics {
    pub key: String,
    pub title: String,
    pub sections: usize,
    pub paragraphs: usize,
    pub lines: usize,
    pub words: usize,
    pub incoming_block_refs: usize,
    pub incoming_inline_refs: usize,
    pub total_incoming_refs: usize,
    pub outgoing_block_refs: usize,
    pub outgoing_inline_refs: usize,
    pub total_connections: usize,
    pub bullet_lists: usize,
    pub ordered_lists: usize,
    pub code_blocks: usize,
    pub tables: usize,
    pub quotes: usize,
}

impl KeyStatistics {
    pub fn new(key: String, title: String) -> Self {
        Self {
            key,
            title,
            ..Default::default()
        }
    }

    pub fn count_node(&mut self, node: &Node) {
        match node {
            Node::Section(_) => self.sections += 1,
            Node::Leaf(_) => self.paragraphs += 1,
            Node::BulletList() => self.bullet_lists += 1,
            Node::OrderedList() => self.ordered_lists += 1,
            Node::Raw(_, _) => self.code_blocks += 1,
            Node::Table(_) => self.tables += 1,
            Node::Quote() => self.quotes += 1,
            _ => {}
        }
    }

    pub fn merge(mut self, other: Self) -> Self {
        self.sections += other.sections;
        self.paragraphs += other.paragraphs;
        self.lines += other.lines;
        self.words += other.words;
        self.incoming_block_refs += other.incoming_block_refs;
        self.incoming_inline_refs += other.incoming_inline_refs;
        self.total_incoming_refs += other.total_incoming_refs;
        self.outgoing_block_refs += other.outgoing_block_refs;
        self.outgoing_inline_refs += other.outgoing_inline_refs;
        self.total_connections += other.total_connections;
        self.bullet_lists += other.bullet_lists;
        self.ordered_lists += other.ordered_lists;
        self.code_blocks += other.code_blocks;
        self.tables += other.tables;
        self.quotes += other.quotes;
        self
    }

    fn count_nodes_recursive<'a, T: NodeIter<'a> + NodePointer<'a>>(
        iter: &T,
        stats: &mut KeyStatistics,
        graph: &'a Graph,
    ) {
        if let Some(node) = iter.node() {
            stats.count_node(&node);

            if let Some(id) = iter.id() {
                if let Some(line_id) = graph.graph_node(id).line_id() {
                    stats.outgoing_inline_refs += graph.get_line(line_id).ref_keys().len();
                }
            }
        }

        if let Some(child) = iter.child() {
            Self::count_nodes_recursive(&child, stats, graph);

            let mut current = child;
            while let Some(next) = current.next() {
                Self::count_nodes_recursive(&next, stats, graph);
                current = next;
            }
        }
    }

    pub fn from_graph(graph: &Graph) -> Vec<KeyStatistics> {
        let mut key_stats: Vec<KeyStatistics> = graph
            .keys()
            .par_iter()
            .map(|key| {
                let key_str = key.to_string();
                let title = graph.get_ref_text(key).unwrap_or_else(|| key.to_string());

                let mut stats = KeyStatistics {
                    key: key_str,
                    title,
                    ..Default::default()
                };

                if let Some(node_id) = graph.get_node_id(key) {
                    let root = GraphNodePointer::new(graph, node_id);
                    Self::count_nodes_recursive(&root, &mut stats, graph);
                }

                if let Some(content) = graph.get_document(key) {
                    stats.lines = content.lines().count();
                    stats.words = content.split_whitespace().count();
                }

                stats.incoming_block_refs = graph.get_block_references_to(key).len();
                stats.incoming_inline_refs = graph.get_inline_references_to(key).len();
                stats.total_incoming_refs = stats.incoming_block_refs + stats.incoming_inline_refs;
                stats.outgoing_block_refs = graph.get_block_references_in(key).len();
                stats.total_connections = stats.incoming_block_refs
                    + stats.incoming_inline_refs
                    + stats.outgoing_block_refs
                    + stats.outgoing_inline_refs;

                stats
            })
            .collect();

        key_stats.par_sort_by(|a, b| a.key.cmp(&b.key));
        key_stats
    }
}

#[derive(Debug, Serialize)]
pub struct GraphStatistics {
    pub total_documents: usize,
    pub total_nodes: usize,
    pub total_paths: usize,

    pub total_sections: usize,
    pub avg_sections_per_doc: f64,
    pub top_docs_by_sections: Vec<KeyStatistics>,
    pub total_paragraphs: usize,
    pub avg_paragraphs_per_doc: f64,

    pub block_references: usize,
    pub inline_references: usize,
    pub total_references: usize,
    pub orphaned_documents: usize,
    pub orphaned_percentage: f64,
    pub leaf_documents: usize,
    pub leaf_percentage: f64,
    pub top_referenced: Vec<KeyStatistics>,

    pub total_lines: usize,
    pub avg_lines_per_doc: f64,
    pub top_docs_by_lines: Vec<KeyStatistics>,

    pub total_words: usize,
    pub avg_words_per_doc: f64,
    pub top_docs_by_words: Vec<KeyStatistics>,

    pub root_sections: usize,
    pub max_path_depth: usize,
    pub avg_path_depth: f64,
    pub bullet_lists: usize,
    pub ordered_lists: usize,
    pub code_blocks: usize,
    pub tables: usize,
    pub quotes: usize,

    pub avg_refs_per_doc: f64,
    pub most_connected: Vec<KeyStatistics>,

    pub broken_link_count: usize,
    pub broken_links: Vec<BrokenLink>,
}

impl GraphStatistics {
    pub fn export_csv<W: Write>(graph: &Graph, writer: W) -> Result<(), csv::Error> {
        let mut csv_writer = csv::Writer::from_writer(writer);
        let key_stats = KeyStatistics::from_graph(graph);

        for stat in key_stats {
            csv_writer.serialize(stat)?;
        }

        csv_writer.flush()?;
        Ok(())
    }

    pub fn from_graph(graph: &Graph) -> Self {
        let key_stats = KeyStatistics::from_graph(graph);
        let broken_links = broken_links(graph);

        let all_nodes = graph.nodes();
        let paths = graph.paths();
        let total_nodes = all_nodes.len();
        let total_paths = paths.len();
        let root_sections = paths.iter().filter(|p| p.ids().len() == 1).count();
        let max_path_depth = paths.iter().map(|p| p.ids().len()).max().unwrap_or(0);
        let avg_path_depth = if total_paths > 0 {
            paths.iter().map(|p| p.ids().len()).sum::<usize>() as f64 / total_paths as f64
        } else {
            0.0
        };

        Self::aggregate_statistics(
            key_stats,
            total_nodes,
            total_paths,
            root_sections,
            max_path_depth,
            avg_path_depth,
            broken_links,
        )
    }

    fn aggregate_statistics(
        key_stats: Vec<KeyStatistics>,
        total_nodes: usize,
        total_paths: usize,
        root_sections: usize,
        max_path_depth: usize,
        avg_path_depth: f64,
        broken_links: Vec<BrokenLink>,
    ) -> Self {
        let total_documents = key_stats.len();

        let total_sections: usize = key_stats.iter().map(|ks| ks.sections).sum();
        let avg_sections_per_doc = if total_documents > 0 {
            total_sections as f64 / total_documents as f64
        } else {
            0.0
        };

        let top_docs_by_sections: Vec<KeyStatistics> = key_stats
            .iter()
            .sorted_by(|a, b| b.sections.cmp(&a.sections))
            .take(10)
            .cloned()
            .collect();

        let total_paragraphs: usize = key_stats.iter().map(|ks| ks.paragraphs).sum();
        let avg_paragraphs_per_doc = if total_documents > 0 {
            total_paragraphs as f64 / total_documents as f64
        } else {
            0.0
        };

        let total_incoming_block: usize = key_stats.iter().map(|ks| ks.incoming_block_refs).sum();
        let total_incoming_inline: usize = key_stats.iter().map(|ks| ks.incoming_inline_refs).sum();
        let block_references = total_incoming_block;
        let inline_references = total_incoming_inline;

        let orphaned_documents = key_stats
            .iter()
            .filter(|ks| ks.total_incoming_refs == 0)
            .count();
        let orphaned_percentage = if total_documents > 0 {
            (orphaned_documents as f64 / total_documents as f64) * 100.0
        } else {
            0.0
        };

        let leaf_documents = key_stats
            .iter()
            .filter(|ks| ks.outgoing_block_refs == 0)
            .count();
        let leaf_percentage = if total_documents > 0 {
            (leaf_documents as f64 / total_documents as f64) * 100.0
        } else {
            0.0
        };

        let top_referenced: Vec<KeyStatistics> = key_stats
            .iter()
            .filter(|ks| ks.total_incoming_refs > 0)
            .sorted_by(|a, b| b.total_incoming_refs.cmp(&a.total_incoming_refs))
            .take(10)
            .cloned()
            .collect();

        let total_lines: usize = key_stats.iter().map(|ks| ks.lines).sum();
        let total_words: usize = key_stats.iter().map(|ks| ks.words).sum();

        let avg_lines_per_doc = if total_documents > 0 {
            total_lines as f64 / total_documents as f64
        } else {
            0.0
        };

        let avg_words_per_doc = if total_documents > 0 {
            total_words as f64 / total_documents as f64
        } else {
            0.0
        };

        let top_docs_by_lines: Vec<KeyStatistics> = key_stats
            .iter()
            .sorted_by(|a, b| b.lines.cmp(&a.lines))
            .take(10)
            .cloned()
            .collect();

        let top_docs_by_words: Vec<KeyStatistics> = key_stats
            .iter()
            .sorted_by(|a, b| b.words.cmp(&a.words))
            .take(10)
            .cloned()
            .collect();

        let bullet_lists: usize = key_stats.iter().map(|ks| ks.bullet_lists).sum();
        let ordered_lists: usize = key_stats.iter().map(|ks| ks.ordered_lists).sum();
        let code_blocks: usize = key_stats.iter().map(|ks| ks.code_blocks).sum();
        let tables: usize = key_stats.iter().map(|ks| ks.tables).sum();
        let quotes: usize = key_stats.iter().map(|ks| ks.quotes).sum();

        let most_connected: Vec<KeyStatistics> = key_stats
            .iter()
            .filter(|ks| ks.total_connections > 0)
            .sorted_by(|a, b| b.total_connections.cmp(&a.total_connections))
            .take(10)
            .cloned()
            .collect();

        let avg_refs_per_doc = if total_documents > 0 {
            (block_references + inline_references) as f64 / total_documents as f64
        } else {
            0.0
        };

        GraphStatistics {
            total_documents,
            total_nodes,
            total_paths,
            total_sections,
            avg_sections_per_doc,
            top_docs_by_sections,
            total_paragraphs,
            avg_paragraphs_per_doc,
            block_references,
            inline_references,
            total_references: block_references + inline_references,
            orphaned_documents,
            orphaned_percentage,
            leaf_documents,
            leaf_percentage,
            top_referenced,
            total_lines,
            avg_lines_per_doc,
            top_docs_by_lines,
            total_words,
            avg_words_per_doc,
            top_docs_by_words,
            root_sections,
            max_path_depth,
            avg_path_depth,
            bullet_lists,
            ordered_lists,
            code_blocks,
            tables,
            quotes,
            avg_refs_per_doc,
            most_connected,
            broken_link_count: broken_links.len(),
            broken_links,
        }
    }
}
