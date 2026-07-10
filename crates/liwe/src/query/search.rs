/// The `search` clause on a `find` query: an optional lexical (BM25) query and/or fuzzy query.
///
/// The query engine treats this as a membership + relevance annotation only — it computes no
/// scores itself. The caller resolves the spec into a [`crate::query::QueryScores`] (BM25 / fuzzy
/// live in the `diwe` engine crate) and injects it via `execute_with_scores`.
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
