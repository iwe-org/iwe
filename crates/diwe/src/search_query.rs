use std::collections::{HashMap, HashSet};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rayon::prelude::*;

use liwe::graph::Graph;
use liwe::model::Key;
use liwe::query::block_update::EvalError;
use liwe::query::{self, Operation, Outcome, QueryScores, SearchSpec};

use crate::search::{rrf_weight, Bm25Index, Language};

const PARALLEL_BUILD_THRESHOLD: usize = 128;

fn corpus_text(graph: &Graph, key: &Key) -> String {
    let title = graph.get_key_title(key).unwrap_or_default();
    format!("{}\n{}", title, graph.to_plain_text(key))
}

/// Build a BM25 index over every document in `graph` (title + plain-text body per key).
pub fn build_index(graph: &Graph, language: Language) -> Bm25Index {
    let keys = graph.keys();
    let docs: Vec<(Key, String)> = if keys.len() < PARALLEL_BUILD_THRESHOLD {
        keys.iter()
            .map(|key| (key.clone(), corpus_text(graph, key)))
            .collect()
    } else {
        keys.par_iter()
            .map(|key| (key.clone(), corpus_text(graph, key)))
            .collect()
    };
    Bm25Index::build(docs, language)
}

fn fuzzy_text(graph: &Graph, key: &Key) -> String {
    let title = graph.get_key_title(key).unwrap_or_default();
    format!("{} {}", key, title)
}

fn fuzzy_ranked(graph: &Graph, candidates: &[Key], query: &str) -> Vec<Key> {
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(Key, i64)> = candidates
        .iter()
        .filter_map(|key| {
            let score = matcher
                .fuzzy_match(&fuzzy_text(graph, key), query)
                .unwrap_or(0);
            (score > 0).then_some((key.clone(), score))
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    scored.into_iter().map(|(k, _)| k).collect()
}

fn lexical_ranked(index: &Bm25Index, candidates: &HashSet<&Key>, query: &str) -> Vec<Key> {
    index
        .search(query)
        .into_iter()
        .filter(|scored| candidates.contains(&scored.id))
        .map(|scored| scored.id)
        .collect()
}

fn search_lists(
    graph: &Graph,
    index: &Bm25Index,
    candidates: &[Key],
    spec: &SearchSpec,
) -> Vec<Vec<Key>> {
    let candidate_set: HashSet<&Key> = candidates.iter().collect();
    let mut lists: Vec<Vec<Key>> = Vec::new();
    if let Some(q) = spec.fuzzy.as_deref() {
        lists.push(fuzzy_ranked(graph, candidates, q));
    }
    if let Some(q) = spec.lexical.as_deref() {
        lists.push(lexical_ranked(index, &candidate_set, q));
    }
    lists
}

fn rrf_scores(lists: &[Vec<Key>]) -> HashMap<Key, f64> {
    let mut scores: HashMap<Key, f64> = HashMap::new();
    for list in lists {
        for (rank, key) in list.iter().enumerate() {
            *scores.entry(key.clone()).or_insert(0.0) += rrf_weight(rank);
        }
    }
    scores
}

fn order_by_scores(scores: HashMap<Key, f64>) -> Vec<Key> {
    let mut fused: Vec<(Key, f64)> = scores.into_iter().collect();
    fused.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    fused.into_iter().map(|(key, _)| key).collect()
}

/// Restrict `candidates` to the documents matching `spec` and order them by relevance.
///
/// A single ranker (`lexical` only or `fuzzy` only) orders by that ranker; both rankers fuse with
/// RRF. Candidates with no BM25 hit / no skim score are dropped, so the result is the joint set of
/// search matches within the candidate set, relevance-ordered, ties broken by key ascending.
pub fn ranked(graph: &Graph, index: &Bm25Index, candidates: &[Key], spec: &SearchSpec) -> Vec<Key> {
    order_by_scores(rrf_scores(&search_lists(graph, index, candidates, spec)))
}

/// Restrict `candidates` to the documents matching `spec`, preserving the incoming candidate order.
///
/// Used when an explicit `sort` supplies the ordering: search contributes membership only.
pub fn matched(
    graph: &Graph,
    index: &Bm25Index,
    candidates: Vec<Key>,
    spec: &SearchSpec,
) -> Vec<Key> {
    let matches: HashSet<Key> = rrf_scores(&search_lists(graph, index, &candidates, spec))
        .into_keys()
        .collect();
    candidates
        .into_iter()
        .filter(|k| matches.contains(k))
        .collect()
}

/// Resolve `spec` over `candidates` into a [`QueryScores`] for the query engine.
pub fn resolve_scores(
    graph: &Graph,
    index: &Bm25Index,
    candidates: &[Key],
    spec: &SearchSpec,
) -> QueryScores {
    QueryScores::from_fused(rrf_scores(&search_lists(graph, index, candidates, spec)))
}

/// True when a `lexical` query is present but reduces to no searchable terms after stop-word
/// removal and stemming — it matches nothing, and callers surface a warning.
pub fn lexical_has_no_terms(index: &Bm25Index, spec: &SearchSpec) -> bool {
    spec.lexical
        .as_deref()
        .map(|q| !index.has_query_terms(q))
        .unwrap_or(false)
}

/// The warning message for a `lexical` query with no searchable terms.
pub fn no_terms_warning(spec: &SearchSpec) -> String {
    let query = spec.lexical.as_deref().unwrap_or_default();
    format!(
        "lexical query '{}' has no searchable terms after stop-word removal and stemming; it matches nothing",
        query
    )
}

/// Execute `op` against `graph`, resolving any `search` clause through `index`.
///
/// A `find` query without a `search` clause runs on the pure engine. A `search` clause requires
/// `index`; without one it fails with [`EvalError::SearchIndexMissing`], preserving the behavior
/// of the former graph-owned BM25 index.
pub fn execute(
    op: &Operation,
    graph: &Graph,
    index: Option<&Bm25Index>,
) -> Result<Outcome, EvalError> {
    let spec = match op {
        Operation::Find(find) => find.search.as_ref(),
        _ => None,
    };
    match spec {
        None => query::execute(op, graph),
        Some(spec) => {
            let Some(index) = index else {
                return Err(EvalError::SearchIndexMissing);
            };
            let filter = match op {
                Operation::Find(find) => find.filter.as_ref(),
                _ => None,
            };
            let candidates: Vec<Key> = match filter {
                None => {
                    let mut k = graph.keys();
                    k.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
                    k
                }
                Some(f) => query::evaluate(f, graph),
            };
            let scores = resolve_scores(graph, index, &candidates, spec);
            query::execute_with_scores(op, graph, &scores)
        }
    }
}
