use crate::graph::Graph;
use crate::model::Key;
use crate::query::document::{ProjectionSource, PseudoField};
use crate::query::project::{apply_projection_or_default, ProjectionContext};
use crate::query::sort::sort_in_place;
use crate::query::{self, Filter, InclusionAnchor, Projection, ReferenceAnchor, Sort};
use crate::tokens::{
    apply_budget, count_tokens, truncate_to_tokens, truncation_marker, Budget, Truncation,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::Serialize;
use serde_yaml::{Mapping, Value};

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
    pub query: Option<String>,
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
    Fuzzy(&'a str),
    Rank,
}

impl<'a> Order<'a> {
    fn from_options(options: &'a FindOptions) -> Order<'a> {
        match &options.query {
            Some(q) => Order::Fuzzy(q),
            None => Order::Rank,
        }
    }
}

impl<'a> DocumentFinder<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    pub fn find(&self, options: &FindOptions) -> FindOutput {
        let candidates = self.candidates(options);

        let candidates = match (&options.sort, &options.query) {
            (Some(_), _) => self.fuzzy_filter_only(candidates, options.query.as_deref()),
            (None, _) => candidates,
        };

        let ordered = if let Some(s) = &options.sort {
            self.sort_by_frontmatter(candidates, s)
        } else {
            self.order(candidates, Order::from_options(options))
        };

        let total = ordered.len();
        let take = options.limit.filter(|&l| l > 0).unwrap_or(total);
        let kept: Vec<Key> = ordered.into_iter().take(take).collect();

        let content_names = content_field_names(options.project.as_ref());
        let mut rows: Vec<FindRow> = kept
            .into_iter()
            .map(|key| {
                let title = self
                    .graph
                    .get_key_title(&key)
                    .unwrap_or_else(|| key.to_string());
                let result = self.build_result(&key, options.project.as_ref());
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
            query: options.query.clone(),
            limit,
            total,
            results,
            keys,
            titles,
            truncation,
        }
    }

    fn fuzzy_filter_only(&self, candidates: Vec<Key>, query: Option<&str>) -> Vec<Key> {
        let Some(q) = query else {
            return candidates;
        };
        let matcher = SkimMatcherV2::default();
        candidates
            .into_iter()
            .filter(|key| {
                let title = self.graph.get_key_title(key).unwrap_or_default();
                let text = format!("{} {}", key, title);
                matcher.fuzzy_match(&text, q).unwrap_or(0) > 0
            })
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
        let mut scored: Vec<(Key, i64)> = match order {
            Order::Fuzzy(q) => {
                let matcher = SkimMatcherV2::default();
                candidates
                    .into_iter()
                    .filter_map(|key| {
                        let title = self.graph.get_key_title(&key).unwrap_or_default();
                        let text = format!("{} {}", key, title);
                        let score = matcher.fuzzy_match(&text, q).unwrap_or(0);
                        (score > 0).then_some((key, score))
                    })
                    .collect()
            }
            Order::Rank => candidates
                .into_iter()
                .map(|key| {
                    let rank = self.node_rank(&key) as i64;
                    (key, rank)
                })
                .collect(),
        };
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        scored.into_iter().map(|(k, _)| k).collect()
    }

    fn build_result(&self, key: &Key, project: Option<&Projection>) -> FindResult {
        let ctx = ProjectionContext {
            graph: self.graph,
            key,
        };
        apply_projection_or_default(&ctx, project)
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
fn content_field_names(project: Option<&Projection>) -> Vec<String> {
    match project {
        Some(p) => p
            .fields
            .iter()
            .filter(|f| matches!(&f.source, ProjectionSource::Pseudo(PseudoField::Content)))
            .map(|f| f.output.clone())
            .collect(),
        None => Vec::new(),
    }
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
