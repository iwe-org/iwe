use std::collections::{HashMap, HashSet};
use std::io::Write;

use itertools::Itertools;
use rayon::prelude::*;
use serde::Serialize;

use liwe::graph::basic_iter::GraphNodePointer;
use liwe::graph::{Graph, GraphContext};
use liwe::model::is_ref_url;
use liwe::model::node::{Node, NodeIter, NodePointer};
use liwe::model::Key;

use crate::search::{Bm25Index, Language};
use crate::search_query::corpus_text;
use crate::tokens::count_tokens;

/// The self-normalized BM25 ratio a pair must clear in both directions to count as near-identical.
/// Callers can override it per index with [`SimilarityIndex::with_threshold`].
pub const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.85;
const SIMILAR_PAGES_CAP: usize = 3;
const SIMILARITY_MIN_TOKENS: usize = 50;
const SIMILARITY_MAX_LENGTH_RATIO: f32 = 2.0;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrokenLink {
    #[serde(serialize_with = "serialize_key")]
    pub source_key: Key,
    #[serde(serialize_with = "serialize_key")]
    pub target_key: Key,
}

fn serialize_key<S: serde::Serializer>(key: &Key, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&key.to_string())
}

fn serialize_keys<S: serde::Serializer>(keys: &[Key], serializer: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(keys.len()))?;
    for key in keys {
        seq.serialize_element(&key.to_string())?;
    }
    seq.end()
}

pub fn broken_links(graph: &Graph) -> Vec<BrokenLink> {
    let existing_keys: HashSet<Key> = graph.keys().into_iter().collect();
    let mut seen = HashSet::new();
    let mut broken = Vec::new();

    for target_key in graph.inclusion_edge_target_keys() {
        if !existing_keys.contains(&target_key) {
            for node_id in graph.get_inclusion_edges_to(&target_key) {
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

    for target_key in graph.reference_edge_target_keys() {
        if !existing_keys.contains(&target_key) && is_ref_url(&target_key.to_string()) {
            for node_id in graph.get_reference_edges_to(&target_key) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rule {
    DanglingLink,
    Orphan,
    SimilarPage,
}

impl Rule {
    pub fn label(&self) -> &'static str {
        match self {
            Rule::DanglingLink => "dangling-link",
            Rule::Orphan => "orphan",
            Rule::SimilarPage => "similar-page",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub rule: Rule,
    #[serde(serialize_with = "serialize_key")]
    pub key: Key,
    #[serde(serialize_with = "serialize_opt_key")]
    pub other: Option<Key>,
    pub message: String,
}

impl Finding {
    pub fn render(&self) -> String {
        format!("{} › {}: {}", self.key, self.rule.label(), self.message)
    }
}

fn serialize_opt_key<S: serde::Serializer>(
    key: &Option<Key>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match key {
        Some(key) => serializer.serialize_str(&key.to_string()),
        None => serializer.serialize_none(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarPage {
    #[serde(serialize_with = "serialize_key")]
    pub key: Key,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatisticsReport {
    #[serde(flatten)]
    pub stats: KeyStatistics,
    pub similar_pages: Vec<SimilarPage>,
}

/// An `index` page (root `index` or any `<dir>/index`) is an intentional entry point, so it is never
/// reported as an orphan even when nothing links to it.
fn is_index_key(key: &str) -> bool {
    matches!(key.rsplit('/').next(), Some("index"))
}

pub fn orphan_keys(graph: &Graph) -> Vec<Key> {
    let mut keys: Vec<Key> = graph
        .keys()
        .into_iter()
        .filter(|key| {
            !is_index_key(key.as_str())
                && graph.get_inclusion_edges_to(key).is_empty()
                && graph.get_reference_edges_to(key).is_empty()
        })
        .collect();
    keys.sort();
    keys
}

/// A search index plus per-key token counts, built once and reused for every similarity query in a
/// run. Building walks each page's corpus text (title + plain body) a single time to feed both the
/// BM25 index and the token map that drives the size gate.
pub struct SimilarityIndex {
    index: Bm25Index,
    tokens: HashMap<Key, usize>,
    threshold: f32,
}

impl SimilarityIndex {
    pub fn build(graph: &Graph, language: Language) -> Self {
        let entries: Vec<(Key, String, usize)> = graph
            .keys()
            .into_par_iter()
            .map(|key| {
                let text = corpus_text(graph, &key);
                let tokens = count_tokens(&text);
                (key, text, tokens)
            })
            .collect();
        let tokens: HashMap<Key, usize> = entries
            .iter()
            .map(|(key, _, count)| (key.clone(), *count))
            .collect();
        let corpus: Vec<(Key, String)> = entries
            .into_iter()
            .map(|(key, text, _)| (key, text))
            .collect();
        SimilarityIndex {
            index: Bm25Index::build(corpus, language),
            tokens,
            threshold: DEFAULT_SIMILARITY_THRESHOLD,
        }
    }

    /// Replaces the match threshold used by [`similar`](Self::similar) and [`pairs`](Self::pairs).
    /// Lower values report looser matches, higher values only closer ones; the index itself is
    /// unaffected, so one build can answer queries at several thresholds.
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Similar pages for one `key`: its forward matches that also match back (mutually similar — the
    /// `min` of the two directional ratios ≥ the index threshold). Mutuality excludes containment (a
    /// short page inside a long one); the token-size and comparable-length gates exclude too-small
    /// pages and mismatched lengths.
    pub fn similar(&self, key: &Key) -> Vec<SimilarPage> {
        mutual_similar(&self.index, &self.tokens, key, self.threshold)
    }

    /// Every mutually-similar pair across the store, each pair once in alphabetical order. Forward
    /// matches are computed once per page (concurrently); a pair is mutual when each page appears in
    /// the other's forward matches.
    pub fn pairs(&self) -> Vec<(Key, Key)> {
        let forward: HashMap<Key, HashMap<Key, f32>> = self
            .tokens
            .par_iter()
            .filter_map(|(key, _)| {
                let matches = forward_matches(&self.index, &self.tokens, key, self.threshold);
                (!matches.is_empty()).then(|| (key.clone(), matches))
            })
            .collect();

        let mut pairs: Vec<(Key, Key)> = Vec::new();
        for (a, a_matches) in &forward {
            for b in a_matches.keys() {
                if a < b
                    && forward
                        .get(b)
                        .is_some_and(|b_matches| b_matches.contains_key(a))
                {
                    pairs.push((a.clone(), b.clone()));
                }
            }
        }
        pairs.sort();
        pairs
    }
}

fn comparable_length(a: usize, b: usize) -> bool {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    lo > 0 && hi as f32 <= lo as f32 * SIMILARITY_MAX_LENGTH_RATIO
}

/// The pages `key` matches in one direction: each `other` whose BM25 score against `key` clears
/// `threshold` (self-normalized) and that passes the token-size and comparable-length gates.
/// The map value is the forward ratio `score(key → other) / score(key → key)`.
fn forward_matches(
    index: &Bm25Index,
    tokens: &HashMap<Key, usize>,
    key: &Key,
    threshold: f32,
) -> HashMap<Key, f32> {
    let a_tokens = tokens.get(key).copied().unwrap_or(0);
    if a_tokens < SIMILARITY_MIN_TOKENS {
        return HashMap::new();
    }
    index
        .similar_to(key, threshold)
        .into_iter()
        .filter(|(other, _)| {
            let b_tokens = tokens.get(other).copied().unwrap_or(0);
            b_tokens >= SIMILARITY_MIN_TOKENS && comparable_length(a_tokens, b_tokens)
        })
        .collect()
}

fn mutual_similar(
    index: &Bm25Index,
    tokens: &HashMap<Key, usize>,
    key: &Key,
    threshold: f32,
) -> Vec<SimilarPage> {
    let mut ranked: Vec<SimilarPage> = forward_matches(index, tokens, key, threshold)
        .into_iter()
        .filter_map(|(other, forward_ratio)| {
            let self_other = index.self_score(&other).filter(|s| *s > 0.0)?;
            let reverse_ratio = index.score_between(&other, key)? / self_other;
            (reverse_ratio >= threshold).then(|| SimilarPage {
                key: other,
                score: forward_ratio.min(reverse_ratio),
            })
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.key.cmp(&b.key))
    });
    ranked.truncate(SIMILAR_PAGES_CAP);
    ranked
}

/// Whole-store graph-state findings that need no search index: orphan pages and dangling links.
pub fn graph_findings(graph: &Graph) -> Vec<Finding> {
    let mut findings = Vec::new();

    for key in orphan_keys(graph) {
        findings.push(Finding {
            rule: Rule::Orphan,
            key,
            other: None,
            message: "no page links here".to_string(),
        });
    }

    for link in broken_links(graph) {
        findings.push(Finding {
            rule: Rule::DanglingLink,
            message: format!("links to missing '{}'", link.target_key),
            key: link.source_key,
            other: Some(link.target_key),
        });
    }

    findings
}

/// Findings after a create/update: the whole-store [`graph_findings`] plus a similar-page check run
/// only for the authored `targets` (the index must already reflect the post-change store). Token
/// counts are gathered per target over just the target and its floor survivors, so the whole store
/// is never re-tokenized.
pub fn mutation_findings(graph: &Graph, index: &Bm25Index, targets: &[Key]) -> Vec<Finding> {
    let mut findings = graph_findings(graph);

    for target in targets {
        let survivors = index.similar_to(target, DEFAULT_SIMILARITY_THRESHOLD);
        let mut tokens: HashMap<Key, usize> = HashMap::new();
        for key in std::iter::once(target).chain(survivors.iter().map(|(other, _)| other)) {
            tokens
                .entry(key.clone())
                .or_insert_with(|| count_tokens(&corpus_text(graph, key)));
        }
        for page in mutual_similar(index, &tokens, target, DEFAULT_SIMILARITY_THRESHOLD) {
            findings.push(Finding {
                rule: Rule::SimilarPage,
                message: format!("closely matches '{}' ({:.2})", page.key, page.score),
                key: target.clone(),
                other: Some(page.key),
            });
        }
    }

    findings
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatistics {
    pub key: String,
    pub title: String,
    pub sections: usize,
    pub paragraphs: usize,
    pub lines: usize,
    pub words: usize,
    pub included_by_count: usize,
    pub referenced_by_count: usize,
    pub incoming_edges_count: usize,
    pub includes_count: usize,
    pub references_count: usize,
    pub total_edges_count: usize,
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
        self.included_by_count += other.included_by_count;
        self.referenced_by_count += other.referenced_by_count;
        self.incoming_edges_count += other.incoming_edges_count;
        self.includes_count += other.includes_count;
        self.references_count += other.references_count;
        self.total_edges_count += other.total_edges_count;
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
                    stats.references_count += graph.get_line(line_id).ref_keys().len();
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

                stats.included_by_count = graph.get_inclusion_edges_to(key).len();
                stats.referenced_by_count = graph.get_reference_edges_to(key).len();
                stats.incoming_edges_count = stats.included_by_count + stats.referenced_by_count;
                stats.includes_count = graph.get_inclusion_edges_in(key).len();
                stats.total_edges_count = stats.included_by_count
                    + stats.referenced_by_count
                    + stats.includes_count
                    + stats.references_count;

                stats
            })
            .collect();

        key_stats.par_sort_by(|a, b| a.key.cmp(&b.key));
        key_stats
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphStatistics {
    pub total_documents: usize,
    pub total_nodes: usize,
    pub total_paths: usize,

    pub total_sections: usize,
    pub avg_sections_per_doc: f64,
    pub top_docs_by_sections: Vec<KeyStatistics>,
    pub total_paragraphs: usize,
    pub avg_paragraphs_per_doc: f64,

    pub inclusion_edges: usize,
    pub reference_edges: usize,
    pub total_references: usize,
    pub orphaned_documents: usize,
    pub orphaned_percentage: f64,
    #[serde(serialize_with = "serialize_keys")]
    pub orphans: Vec<Key>,
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

        let paths = graph.paths();
        let total_nodes = graph.node_count();
        let total_paths = paths.len();
        let root_sections = paths.iter().filter(|p| p.ids().len() == 1).count();
        let max_path_depth = paths.iter().map(|p| p.ids().len()).max().unwrap_or(0);
        let avg_path_depth = if total_paths > 0 {
            paths.iter().map(|p| p.ids().len()).sum::<usize>() as f64 / total_paths as f64
        } else {
            0.0
        };

        let mut stats = Self::aggregate_statistics(
            key_stats,
            total_nodes,
            total_paths,
            root_sections,
            max_path_depth,
            avg_path_depth,
            broken_links,
        );
        stats.orphans = orphan_keys(graph);
        stats
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

        let total_incoming_block: usize = key_stats.iter().map(|ks| ks.included_by_count).sum();
        let total_incoming_inline: usize = key_stats.iter().map(|ks| ks.referenced_by_count).sum();
        let inclusion_edges = total_incoming_block;
        let reference_edges = total_incoming_inline;

        let orphaned_documents = key_stats
            .iter()
            .filter(|ks| ks.incoming_edges_count == 0 && !is_index_key(&ks.key))
            .count();
        let orphaned_percentage = if total_documents > 0 {
            (orphaned_documents as f64 / total_documents as f64) * 100.0
        } else {
            0.0
        };

        let leaf_documents = key_stats.iter().filter(|ks| ks.includes_count == 0).count();
        let leaf_percentage = if total_documents > 0 {
            (leaf_documents as f64 / total_documents as f64) * 100.0
        } else {
            0.0
        };

        let top_referenced: Vec<KeyStatistics> = key_stats
            .iter()
            .filter(|ks| ks.incoming_edges_count > 0)
            .sorted_by(|a, b| b.incoming_edges_count.cmp(&a.incoming_edges_count))
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
            .filter(|ks| ks.total_edges_count > 0)
            .sorted_by(|a, b| b.total_edges_count.cmp(&a.total_edges_count))
            .take(10)
            .cloned()
            .collect();

        let avg_refs_per_doc = if total_documents > 0 {
            (inclusion_edges + reference_edges) as f64 / total_documents as f64
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
            inclusion_edges,
            reference_edges,
            total_references: inclusion_edges + reference_edges,
            orphaned_documents,
            orphaned_percentage,
            orphans: Vec::new(),
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
