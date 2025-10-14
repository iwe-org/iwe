use liwe::graph::basic_iter::GraphNodePointer;
use rayon::prelude::*;
use serde::Serialize;

use liwe::model::node::Node;

use liwe::graph::{Graph, GraphContext};
use liwe::model::node::{NodeIter, NodePointer};

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

    pub fn count_node(&mut self, node: &liwe::model::node::Node) {
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

                let mut stats = KeyStatistics::default();
                stats.key = key_str;
                stats.title = title;

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
