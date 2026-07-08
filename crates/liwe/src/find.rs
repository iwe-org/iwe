use crate::graph::Graph;
use crate::model::Key;
use crate::query::project::{apply_projection, ProjectionContext};
use crate::query::sort::sort_in_place;
use crate::query::{self, Filter, InclusionAnchor, Projection, ReferenceAnchor, Sort};
use crate::search::rrf_weight;
use crate::tokens::{
    apply_budget, count_tokens, truncate_to_tokens, truncation_marker, Budget, Truncation,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::Serialize;
use serde_yaml::{Mapping, Value};
use std::collections::{HashMap, HashSet};

pub type FindResult = Mapping;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindOutput {
    pub query: Option<String>,
    pub limit: Option<usize>,
    pub total: usize,
    pub results: Vec<FindResult>,
    #[serde(skip)]
    pub keys: Vec<Key>,
    #[serde(skip)]
    pub titles: Vec<String>,
    #[serde(skip)]
    pub truncation: Truncation,
}

#[derive(Debug, Clone, Default)]
pub struct FindOptions {
    pub fuzzy: Option<String>,
    pub lexical: Option<String>,
    pub refs_to: Option<Key>,
    pub refs_from: Option<Key>,
    pub filter: Option<Filter>,
    pub limit: Option<usize>,
    pub sort: Option<Sort>,
    pub project: Option<Projection>,
    pub max_tokens: Option<usize>,
    pub max_document_tokens: Option<usize>,
}

struct FindRow {
    key: Key,
    title: String,
    result: FindResult,
}

pub struct DocumentFinder<'a> {
    graph: &'a Graph,
}

enum Order<'a> {
    Query {
        fuzzy: Option<&'a str>,
        lexical: Option<&'a str>,
    },
    Rank,
}

impl<'a> Order<'a> {
    fn from_options(options: &'a FindOptions) -> Order<'a> {
        match (&options.fuzzy, &options.lexical) {
            (None, None) => Order::Rank,
            (fuzzy, lexical) => Order::Query {
                fuzzy: fuzzy.as_deref(),
                lexical: lexical.as_deref(),
            },
        }
    }
}

impl<'a> DocumentFinder<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    pub fn find(&self, options: &FindOptions) -> FindOutput {
        let candidates = self.candidates(options);

        let candidates = match &options.sort {
            Some(_) => self.matched_keys(
                candidates,
                options.fuzzy.as_deref(),
                options.lexical.as_deref(),
            ),
            None => candidates,
        };

        let ordered = if let Some(s) = &options.sort {
            self.sort_by_frontmatter(candidates, s)
        } else {
            self.order(candidates, Order::from_options(options))
        };

        let total = ordered.len();
        let take = options.limit.filter(|&l| l > 0).unwrap_or(total);
        let kept: Vec<Key> = ordered.into_iter().take(take).collect();

        let projection = options.project.clone().unwrap_or_else(Projection::document);
        let content_names = content_field_names(&projection);
        let mut rows: Vec<FindRow> = kept
            .into_iter()
            .map(|key| {
                let title = self
                    .graph
                    .get_key_title(&key)
                    .unwrap_or_else(|| key.to_string());
                let result = self.build_result(&key, &projection);
                FindRow { key, title, result }
            })
            .collect();

        let budget = Budget {
            limit: None,
            max_tokens: options.max_tokens,
            max_document_tokens: options.max_document_tokens,
        };
        let truncation = apply_budget(
            &mut rows,
            &budget,
            total,
            |row| row.key.to_string(),
            |row| content_tokens_of(&row.result, &content_names),
            |row, max| cap_content_fields(&mut row.result, &content_names, max),
        );

        let limit = options.limit.filter(|&l| l > 0 && l < total);
        let titles: Vec<String> = rows.iter().map(|r| r.title.clone()).collect();
        let keys: Vec<Key> = rows.iter().map(|r| r.key.clone()).collect();
        let results: Vec<FindResult> = rows.into_iter().map(|r| r.result).collect();

        FindOutput {
            query: options.fuzzy.clone().or_else(|| options.lexical.clone()),
            limit,
            total,
            results,
            keys,
            titles,
            truncation,
        }
    }

    fn matched_keys(
        &self,
        candidates: Vec<Key>,
        fuzzy: Option<&str>,
        lexical: Option<&str>,
    ) -> Vec<Key> {
        if fuzzy.is_none() && lexical.is_none() {
            return candidates;
        }
        let mut matched: HashSet<Key> = HashSet::new();
        if let Some(q) = lexical {
            matched.extend(self.graph.search(q).into_iter().map(|sd| sd.id));
        }
        if let Some(q) = fuzzy {
            let matcher = SkimMatcherV2::default();
            for key in &candidates {
                if matcher.fuzzy_match(&self.fuzzy_text(key), q).unwrap_or(0) > 0 {
                    matched.insert(key.clone());
                }
            }
        }
        candidates
            .into_iter()
            .filter(|key| matched.contains(key))
            .collect()
    }

    fn fuzzy_text(&self, key: &Key) -> String {
        let title = self.graph.get_key_title(key).unwrap_or_default();
        format!("{} {}", key, title)
    }

    fn fuzzy_ranked(&self, candidates: &[Key], query: &str) -> Vec<Key> {
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(Key, i64)> = candidates
            .iter()
            .filter_map(|key| {
                let score = matcher
                    .fuzzy_match(&self.fuzzy_text(key), query)
                    .unwrap_or(0);
                (score > 0).then_some((key.clone(), score))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        scored.into_iter().map(|(k, _)| k).collect()
    }

    fn lexical_ranked(&self, candidates: &HashSet<&Key>, query: &str) -> Vec<Key> {
        self.graph
            .search(query)
            .into_iter()
            .filter(|scored| candidates.contains(&scored.id))
            .map(|scored| scored.id)
            .collect()
    }

    fn sort_by_frontmatter(&self, candidates: Vec<Key>, sort: &Sort) -> Vec<Key> {
        let mut rows: Vec<(Key, Mapping)> = candidates
            .into_iter()
            .map(|k| {
                let m = self.graph.frontmatter(&k).cloned().unwrap_or_default();
                (k, m)
            })
            .collect();
        sort_in_place(&mut rows, sort);
        rows.into_iter().map(|(k, _)| k).collect()
    }

    fn candidates(&self, options: &FindOptions) -> Vec<Key> {
        match build_filter(options) {
            None => self.graph.keys(),
            Some(f) => query::evaluate(&f, self.graph),
        }
    }

    fn order(&self, candidates: Vec<Key>, order: Order<'_>) -> Vec<Key> {
        match order {
            Order::Query { fuzzy, lexical } => {
                let candidate_set: HashSet<&Key> = candidates.iter().collect();
                let mut lists: Vec<Vec<Key>> = Vec::new();
                if let Some(q) = fuzzy {
                    lists.push(self.fuzzy_ranked(&candidates, q));
                }
                if let Some(q) = lexical {
                    lists.push(self.lexical_ranked(&candidate_set, q));
                }
                rrf_fuse(lists)
            }
            Order::Rank => {
                let mut scored: Vec<(Key, i64)> = candidates
                    .into_iter()
                    .map(|key| {
                        let rank = self.node_rank(&key) as i64;
                        (key, rank)
                    })
                    .collect();
                scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                scored.into_iter().map(|(k, _)| k).collect()
            }
        }
    }

    fn build_result(&self, key: &Key, project: &Projection) -> FindResult {
        let ctx = ProjectionContext::new(self.graph, key);
        apply_projection(&ctx, project)
    }

    fn node_rank(&self, key: &Key) -> usize {
        self.graph.get_reference_edges_to(key).len() + self.graph.get_inclusion_edges_to(key).len()
    }
}

/// Output names of the projected fields sourced from `$content`.
///
/// The token budget (`max_tokens` / `max_document_tokens`) only counts and caps these fields,
/// so a metadata-only index (no `$content` projected) carries ~0 content tokens and is bounded
/// solely by `limit`. `find` never counts the metadata columns — its index rows are ~1 line.
fn content_field_names(project: &Projection) -> Vec<String> {
    project
        .fields
        .iter()
        .filter(|f| f.source.is_content_shaped())
        .map(|f| f.output.clone())
        .collect()
}

fn content_tokens_of(result: &FindResult, names: &[String]) -> usize {
    names
        .iter()
        .filter_map(|name| result.get(Value::String(name.clone())))
        .filter_map(|v| v.as_str())
        .map(count_tokens)
        .sum()
}

fn cap_content_fields(result: &mut FindResult, names: &[String], max: usize) -> Option<usize> {
    let mut omitted_total = 0usize;
    for name in names {
        let key = Value::String(name.clone());
        let Some(text) = result.get(&key).and_then(|v| v.as_str()) else {
            continue;
        };
        let (head, omitted) = truncate_to_tokens(text, max);
        if omitted > 0 {
            let capped = format!("{}{}", head, truncation_marker(omitted));
            result.insert(key, Value::String(capped));
            omitted_total += omitted;
        }
    }
    (omitted_total > 0).then_some(omitted_total)
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

fn build_filter(options: &FindOptions) -> Option<Filter> {
    let mut conjuncts: Vec<Filter> = options.filter.clone().into_iter().collect();
    if let Some(target) = &options.refs_to {
        conjuncts.push(Filter::Or(vec![
            Filter::Includes(Box::new(InclusionAnchor::with_max(target.to_string(), 1))),
            Filter::References(Box::new(ReferenceAnchor::with_max(target.to_string(), 1))),
        ]));
    }
    if let Some(source) = &options.refs_from {
        conjuncts.push(Filter::Or(vec![
            Filter::IncludedBy(Box::new(InclusionAnchor::with_max(source.to_string(), 1))),
            Filter::ReferencedBy(Box::new(ReferenceAnchor::with_max(source.to_string(), 1))),
        ]));
    }
    if conjuncts.is_empty() {
        None
    } else {
        Some(Filter::And(conjuncts))
    }
}
