use std::collections::HashMap;

use itertools::Itertools;
use minijinja::Environment;
use serde::Serialize;

use liwe::graph::{Graph, GraphContext};
use liwe::model::node::NodePointer;
use liwe::model::Key;

const STATS_TEMPLATE: &str = r#"# Graph Statistics

## Overview

- **Total documents:** {{ total_documents }}
- **Total nodes:** {{ total_nodes }}
- **Total paths:** {{ total_paths }}

## Document Statistics

- **Total sections:** {{ total_sections }}
- **Average sections/doc:** {{ avg_sections_per_doc|round(2) }}

{%- if top_docs_by_sections %}

### Top Documents by Sections

{% for item in top_docs_by_sections -%}
{{ loop.index }}. **{{ item.name }}** ({{ item.count }} sections)
{% endfor %}
{%- endif %}

## Reference Statistics

- **Block references:** {{ block_references }}
- **Inline references:** {{ inline_references }}
- **Total references:** {{ total_references }}
- **Orphaned documents:** {{ orphaned_documents }} ({{ orphaned_percentage|round(1) }}%)
- **Leaf documents:** {{ leaf_documents }} ({{ leaf_percentage|round(1) }}%)

{%- if top_referenced %}

### Top Referenced Documents

{% for item in top_referenced -%}
{{ loop.index }}. **{{ item.name }}** ({{ item.count }})
{% endfor %}
{%- endif %}

## Lines Statistics

- **Total lines:** {{ total_lines }}
- **Average lines/doc:** {{ avg_lines_per_doc|round(2) }}

{%- if top_docs_by_lines %}

### Top Documents by Lines

{% for item in top_docs_by_lines -%}
{{ loop.index }}. **{{ item.name }}** ({{ item.count }} lines)
{% endfor %}
{%- endif %}

## Words Statistics

- **Total words:** {{ total_words }}
- **Average words/doc:** {{ avg_words_per_doc|round(2) }}

{%- if top_docs_by_words %}

### Top Documents by Words

{% for item in top_docs_by_words -%}
{{ loop.index }}. **{{ item.name }}** ({{ item.count }} words)
{% endfor %}
{%- endif %}

## Structure Statistics

- **Root sections:** {{ root_sections }}
- **Maximum path depth:** {{ max_path_depth }}
- **Average path depth:** {{ avg_path_depth|round(2) }}
- **Bullet lists:** {{ bullet_lists }}
- **Ordered lists:** {{ ordered_lists }}
- **Code blocks:** {{ code_blocks }}
- **Tables:** {{ tables }}
- **Quotes:** {{ quotes }}

## Network Analysis

- **Average references/doc:** {{ avg_refs_per_doc|round(2) }}

{%- if most_connected %}

### Most Connected Documents

{% for item in most_connected -%}
{{ loop.index }}. **{{ item.name }}** ({{ item.count }} connections)
{% endfor %}
{%- endif %}
"#;

#[derive(Debug, Serialize)]
pub struct DocumentStat {
    name: String,
    count: usize,
}

#[derive(Debug, Serialize)]
pub struct GraphStatistics {
    // Overview
    total_documents: usize,
    total_nodes: usize,
    total_paths: usize,

    // Document Statistics
    total_sections: usize,
    avg_sections_per_doc: f64,
    top_docs_by_sections: Vec<DocumentStat>,

    // Reference Statistics
    block_references: usize,
    inline_references: usize,
    total_references: usize,
    orphaned_documents: usize,
    orphaned_percentage: f64,
    leaf_documents: usize,
    leaf_percentage: f64,
    top_referenced: Vec<DocumentStat>,

    // Content Statistics - Lines
    total_lines: usize,
    avg_lines_per_doc: f64,
    top_docs_by_lines: Vec<DocumentStat>,

    // Content Statistics - Words
    total_words: usize,
    avg_words_per_doc: f64,
    top_docs_by_words: Vec<DocumentStat>,

    // Structure Statistics
    root_sections: usize,
    max_path_depth: usize,
    avg_path_depth: f64,
    bullet_lists: usize,
    ordered_lists: usize,
    code_blocks: usize,
    tables: usize,
    quotes: usize,

    // Network Analysis
    avg_refs_per_doc: f64,
    most_connected: Vec<DocumentStat>,
}

impl GraphStatistics {
    pub fn from_graph(graph: &Graph) -> Self {
        let all_nodes = graph.nodes();
        let paths = graph.paths();
        let total_documents = graph.keys().len();
        let total_nodes = all_nodes.len();
        let total_paths = paths.len();

        // Document Statistics
        let total_sections = all_nodes.iter().filter(|n| n.is_section()).count();
        let avg_sections_per_doc = if total_documents > 0 {
            total_sections as f64 / total_documents as f64
        } else {
            0.0
        };

        let mut doc_sections: HashMap<Key, usize> = HashMap::new();
        for node in all_nodes.iter() {
            if node.is_section() {
                let doc_key = graph.node(node.id()).node_key();
                *doc_sections.entry(doc_key).or_insert(0) += 1;
            }
        }
        let top_docs_by_sections: Vec<DocumentStat> = doc_sections
            .iter()
            .sorted_by(|a, b| b.1.cmp(a.1))
            .take(10)
            .map(|(key, count)| DocumentStat {
                name: graph.get_ref_text(key).unwrap_or_else(|| key.to_string()),
                count: *count,
            })
            .collect();

        // Reference Statistics
        let block_references = all_nodes.iter().filter(|n| n.is_reference()).count();

        let mut inline_references = 0;
        for node in all_nodes.iter() {
            if let Some(line_id) = node.line_id() {
                let line = graph.get_line(line_id);
                inline_references += line.ref_keys().len();
            }
        }

        let mut incoming_refs: HashMap<Key, usize> = HashMap::new();
        let mut outgoing_refs: HashMap<Key, usize> = HashMap::new();

        for key in graph.keys() {
            let block_refs = graph.get_block_references_to(&key).len();
            let inline_refs = graph.get_inline_references_to(&key).len();
            let total_incoming = block_refs + inline_refs;
            if total_incoming > 0 {
                incoming_refs.insert(key.clone(), total_incoming);
            }

            let refs_in = graph.get_block_references_in(&key).len();
            if refs_in > 0 {
                outgoing_refs.insert(key.clone(), refs_in);
            }
        }

        let orphaned_documents = total_documents - incoming_refs.len();
        let orphaned_percentage = if total_documents > 0 {
            (orphaned_documents as f64 / total_documents as f64) * 100.0
        } else {
            0.0
        };

        let leaf_documents = total_documents - outgoing_refs.len();
        let leaf_percentage = if total_documents > 0 {
            (leaf_documents as f64 / total_documents as f64) * 100.0
        } else {
            0.0
        };

        let top_referenced: Vec<DocumentStat> = incoming_refs
            .iter()
            .sorted_by(|a, b| b.1.cmp(a.1))
            .take(10)
            .map(|(key, count)| DocumentStat {
                name: graph.get_ref_text(key).unwrap_or_else(|| key.to_string()),
                count: *count,
            })
            .collect();

        // Content Statistics
        let mut total_lines = 0;
        let mut total_words = 0;
        let mut doc_lines: HashMap<Key, usize> = HashMap::new();
        let mut doc_words: HashMap<Key, usize> = HashMap::new();

        for key in graph.keys() {
            if let Some(content) = graph.get_document(&key) {
                let lines = content.lines().count();
                total_lines += lines;
                doc_lines.insert(key.clone(), lines);

                // Count words by splitting on whitespace
                let words = content.split_whitespace().count();
                total_words += words;
                doc_words.insert(key, words);
            }
        }

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

        let top_docs_by_lines: Vec<DocumentStat> = doc_lines
            .iter()
            .sorted_by(|a, b| b.1.cmp(a.1))
            .take(10)
            .map(|(key, lines)| DocumentStat {
                name: graph.get_ref_text(key).unwrap_or_else(|| key.to_string()),
                count: *lines,
            })
            .collect();

        let top_docs_by_words: Vec<DocumentStat> = doc_words
            .iter()
            .sorted_by(|a, b| b.1.cmp(a.1))
            .take(10)
            .map(|(key, words)| DocumentStat {
                name: graph.get_ref_text(key).unwrap_or_else(|| key.to_string()),
                count: *words,
            })
            .collect();

        // Structure Statistics
        let root_sections = paths.iter().filter(|p| p.ids().len() == 1).count();
        let max_path_depth = paths.iter().map(|p| p.ids().len()).max().unwrap_or(0);
        let avg_path_depth = if total_paths > 0 {
            paths.iter().map(|p| p.ids().len()).sum::<usize>() as f64 / total_paths as f64
        } else {
            0.0
        };

        let bullet_lists = all_nodes.iter().filter(|n| n.is_bullet_list()).count();
        let ordered_lists = all_nodes.iter().filter(|n| n.is_ordered_list()).count();
        let code_blocks = all_nodes.iter().filter(|n| n.is_raw()).count();
        let tables = all_nodes.iter().filter(|n| n.is_table()).count();
        let quotes = all_nodes.iter().filter(|n| n.is_quote()).count();

        // Network Analysis
        let mut total_refs: HashMap<Key, usize> = HashMap::new();
        for key in graph.keys() {
            let incoming = incoming_refs.get(&key).unwrap_or(&0);
            let outgoing = outgoing_refs.get(&key).unwrap_or(&0);
            let total = incoming + outgoing;
            if total > 0 {
                total_refs.insert(key, total);
            }
        }

        let most_connected: Vec<DocumentStat> = total_refs
            .iter()
            .sorted_by(|a, b| b.1.cmp(a.1))
            .take(10)
            .map(|(key, count)| DocumentStat {
                name: graph.get_ref_text(key).unwrap_or_else(|| key.to_string()),
                count: *count,
            })
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
