use crate::queries::{delete, eq, filter};
use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::execute;
use liwe::query::{DeleteOp, Expect, Filter, Operation, Outcome};
use liwe::state::from_indoc;
use pretty_assertions::assert_str_eq;

fn assert_delete(docs: &str, op: impl Into<DeleteOp>, expected: &[&str]) {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    match execute(&delete(op), &graph).expect("query succeeds") {
        Outcome::Delete { removed } => {
            let actual: Vec<String> = removed.iter().map(|k| k.to_string()).collect();
            let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            assert_eq!(actual, expected);
        }
        other => panic!("expected Delete, got {:?}", other),
    }
}

#[test]
fn delete_removes_matching() {
    assert_delete(
        indoc! {"
            ---
            status: archived
            ---
            # A
            _
            ---
            status: draft
            ---
            # B
            _
            ---
            status: archived
            ---
            # C
        "},
        filter(eq("status", "archived")),
        &["1", "3"],
    );
}

#[test]
fn delete_with_empty_filter_matches_all() {
    assert_delete(
        indoc! {"
            # A
            _
            # B
        "},
        filter(Filter::all()),
        &["1", "2"],
    );
}

#[test]
fn delete_respects_limit() {
    assert_delete(
        indoc! {"
            ---
            status: archived
            ---
            # A
            _
            ---
            status: archived
            ---
            # B
            _
            ---
            status: archived
            ---
            # C
        "},
        filter(eq("status", "archived")).limit(2),
        &["1", "2"],
    );
}

fn delete_err(docs: &str, op: DeleteOp) -> String {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    match execute(&Operation::Delete(op), &graph) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("expected evaluation error"),
    }
}

#[test]
fn document_expect_passes_when_matched_count_matches() {
    assert_delete(
        indoc! {"
            # A
            _
            # B
        "},
        DeleteOp::new(Filter::all()).expect(Expect::Exactly(2)),
        &["1", "2"],
    );
}

#[test]
fn document_expect_violation_aborts_and_lists_documents() {
    let err = delete_err(
        indoc! {"
            # A
            _
            # B
        "},
        DeleteOp::new(Filter::all()).expect(Expect::Exactly(1)),
    );
    assert_str_eq!(
        err,
        indoc! {"
            delete expects 1 document, matched 2
              1 › A
              2 › B
            hint: adjust the filter or raise expect"}
    );
}
