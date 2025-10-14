mod key_statistics;

use std::io::Write;

use itertools::Itertools;
use minijinja::Environment;
use serde::Serialize;

use liwe::graph::Graph;

pub use key_statistics::KeyStatistics;

const STATS_TEMPLATE: &str = include_str!("../templates/stats.md.jinja");

#[derive(Debug, Serialize)]
pub struct GraphStatistics {
    total_documents: usize,
    total_nodes: usize,
    total_paths: usize,

    total_sections: usize,
    avg_sections_per_doc: f64,
    top_docs_by_sections: Vec<KeyStatistics>,
    total_paragraphs: usize,
    avg_paragraphs_per_doc: f64,

    block_references: usize,
    inline_references: usize,
    total_references: usize,
    orphaned_documents: usize,
    orphaned_percentage: f64,
    leaf_documents: usize,
    leaf_percentage: f64,
    top_referenced: Vec<KeyStatistics>,

    total_lines: usize,
    avg_lines_per_doc: f64,
    top_docs_by_lines: Vec<KeyStatistics>,

    total_words: usize,
    avg_words_per_doc: f64,
    top_docs_by_words: Vec<KeyStatistics>,

    root_sections: usize,
    max_path_depth: usize,
    avg_path_depth: f64,
    bullet_lists: usize,
    ordered_lists: usize,
    code_blocks: usize,
    tables: usize,
    quotes: usize,

    avg_refs_per_doc: f64,
    most_connected: Vec<KeyStatistics>,
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
        )
    }

    fn aggregate_statistics(
        key_stats: Vec<KeyStatistics>,
        total_nodes: usize,
        total_paths: usize,
        root_sections: usize,
        max_path_depth: usize,
        avg_path_depth: f64,
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
        }
    }

    pub fn render(&self) -> String {
        let mut env = Environment::new();
        env.add_template("stats", STATS_TEMPLATE)
            .expect("Failed to add template");

        let template = env.get_template("stats").expect("Failed to get template");
        template.render(self).expect("Failed to render template")
    }
}
