use std::collections::HashMap;

use crate::model::Key;

/// Per-query relevance scores injected into the query engine by the caller. The engine itself
/// computes no scores and holds no index — the `diwe` engine crate resolves a
/// [`crate::query::SearchSpec`] (BM25 + fuzzy, fused with RRF) into this map and passes it to
/// `execute_with_scores`.
///
/// When a `find` query carries a `search` clause, the keys present in [`QueryScores::fused`]
/// define the search match set (membership); their scores order the matches (higher is more
/// relevant). An empty `QueryScores` means no search was resolved.
#[derive(Debug, Default, Clone)]
pub struct QueryScores {
    /// Fused relevance score per matched key (RRF over every search method).
    pub fused: HashMap<Key, f64>,
    /// Per-method scores addressable via `$score.<method>` in `sort` (e.g. `lexical`, `fuzzy`).
    pub per_method: HashMap<String, HashMap<Key, f64>>,
}

impl QueryScores {
    /// Build a `QueryScores` from a single fused relevance map (no per-method breakdown).
    pub fn from_fused(fused: HashMap<Key, f64>) -> Self {
        Self {
            fused,
            per_method: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.fused.is_empty() && self.per_method.is_empty()
    }

    /// Relevance score for `key` in the fused ranking, or `None` if it is not a search match.
    pub fn fused_score(&self, key: &Key) -> Option<f64> {
        self.fused.get(key).copied()
    }
}
