use std::collections::{HashMap, HashSet};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use crate::graph::Graph;
use crate::model::Key;
use crate::search::rrf_weight;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSpec {
    pub lexical: Option<String>,
    pub fuzzy: Option<String>,
}

impl SearchSpec {
    pub fn new(lexical: Option<String>, fuzzy: Option<String>) -> Self {
        SearchSpec { lexical, fuzzy }
    }

    pub fn is_empty(&self) -> bool {
        self.lexical.is_none() && self.fuzzy.is_none()
    }
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

fn lexical_ranked(graph: &Graph, candidates: &HashSet<&Key>, query: &str) -> Vec<Key> {
    graph
        .search(query)
        .into_iter()
        .filter(|scored| candidates.contains(&scored.id))
        .map(|scored| scored.id)
        .collect()
}

fn rrf_fuse(lists: Vec<Vec<Key>>) -> Vec<Key> {
    let mut scores: HashMap<Key, f64> = HashMap::new();
    for list in &lists {
        for (rank, key) in list.iter().enumerate() {
            *scores.entry(key.clone()).or_insert(0.0) += rrf_weight(rank);
        }
    }
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
pub fn ranked(graph: &Graph, candidates: &[Key], spec: &SearchSpec) -> Vec<Key> {
    let candidate_set: HashSet<&Key> = candidates.iter().collect();
    let mut lists: Vec<Vec<Key>> = Vec::new();
    if let Some(q) = spec.fuzzy.as_deref() {
        lists.push(fuzzy_ranked(graph, candidates, q));
    }
    if let Some(q) = spec.lexical.as_deref() {
        lists.push(lexical_ranked(graph, &candidate_set, q));
    }
    rrf_fuse(lists)
}

/// Restrict `candidates` to the documents matching `spec`, preserving the incoming candidate order.
///
/// Used when an explicit `sort` supplies the ordering: search contributes membership only.
pub fn matched(graph: &Graph, candidates: Vec<Key>, spec: &SearchSpec) -> Vec<Key> {
    let mut set: HashSet<Key> = HashSet::new();
    if let Some(q) = spec.lexical.as_deref() {
        set.extend(graph.search(q).into_iter().map(|scored| scored.id));
    }
    if let Some(q) = spec.fuzzy.as_deref() {
        let matcher = SkimMatcherV2::default();
        for key in &candidates {
            if matcher.fuzzy_match(&fuzzy_text(graph, key), q).unwrap_or(0) > 0 {
                set.insert(key.clone());
            }
        }
    }
    candidates.into_iter().filter(|k| set.contains(k)).collect()
}

/// True when a `lexical` query is present but reduces to no searchable terms after stop-word
/// removal and stemming — it matches nothing, and callers surface a warning.
pub fn lexical_has_no_terms(graph: &Graph, spec: &SearchSpec) -> bool {
    spec.lexical
        .as_deref()
        .map(|q| !graph.lexical_query_has_terms(q))
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
