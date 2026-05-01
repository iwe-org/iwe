use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::execute;
use liwe::query::prelude::{and, eq, exists, filter, find, gte, or};
use liwe::query::{Filter, FindOp, Outcome, Projection, Sort};
use liwe::state::from_indoc;
use serde_yaml::{Mapping, Value};

fn run_find(docs: &str, op: FindOp) -> Vec<(String, Mapping)> {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    match execute(&find(op), &graph) {
        Outcome::Find { matches } => matches
            .into_iter()
            .map(|m| (m.key.to_string(), m.document))
            .collect(),
        other => panic!("expected Find, got {:?}", other),
    }
}

fn assert_keys(docs: &str, op: FindOp, expected: &[&str]) {
    let actual: Vec<String> = run_find(docs, op).into_iter().map(|(k, _)| k).collect();
    let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(actual, expected);
}

#[test]
fn find_no_filter_returns_whole_corpus() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
            _
            # C
        "},
        FindOp::new(),
        &["1", "2", "3"],
    );
}

#[test]
fn find_filter_status_draft() {
    assert_keys(
        indoc! {"
            ---
            status: draft
            ---
            # A
            _
            ---
            status: published
            ---
            # B
            _
            ---
            status: draft
            ---
            # C
        "},
        filter(eq("status", "draft")),
        &["1", "3"],
    );
}

#[test]
fn find_filter_with_or_and_nested() {
    assert_keys(
        indoc! {"
            ---
            modified: 2026-04-15
            priority: 9
            ---
            # A
            _
            ---
            modified: 2026-04-20
            tags:
              - urgent
            ---
            # B
            _
            ---
            modified: 2026-03-15
            priority: 9
            ---
            # C
            _
            ---
            modified: 2026-04-15
            priority: 5
            ---
            # D
        "},
        filter(and(vec![
            gte("modified", "2026-04-01"),
            or(vec![gte("priority", 8i64), eq("tags", "urgent")]),
        ])),
        &["1", "2"],
    );
}

#[test]
fn find_projection_drops_unprojected_fields() {
    let matches = run_find(
        indoc! {"
            ---
            title: Foo
            author: dmytro
            status: draft
            ---
            # A
        "},
        filter::<FindOp>(Filter::all()).project(Projection::fields(&["title", "status"])),
    );
    assert_eq!(matches.len(), 1);
    let doc = &matches[0].1;
    assert!(doc.contains_key(Value::String("title".into())));
    assert!(doc.contains_key(Value::String("status".into())));
    assert!(!doc.contains_key(Value::String("author".into())));
}

#[test]
fn find_sort_and_limit() {
    assert_keys(
        indoc! {"
            ---
            modified: 2026-04-10
            ---
            # A
            _
            ---
            modified: 2026-04-20
            ---
            # B
            _
            ---
            modified: 2026-04-15
            ---
            # C
        "},
        filter::<FindOp>(Filter::all())
            .sort(Sort::desc("modified"))
            .limit(2),
        &["2", "3"],
    );
}

#[test]
fn find_empty_corpus_returns_empty() {
    let graph = Graph::import(
        &std::collections::HashMap::new(),
        MarkdownOptions::default(),
        None,
    );
    match execute(&find(FindOp::new()), &graph) {
        Outcome::Find { matches } => assert!(matches.is_empty()),
        other => panic!("{:?}", other),
    }
}

#[test]
fn find_doc_without_frontmatter_appears_as_empty_mapping() {
    assert_keys(
        indoc! {"
            # A
            _
            ---
            status: draft
            ---
            # B
        "},
        filter(exists("status", false)),
        &["1"],
    );
}
