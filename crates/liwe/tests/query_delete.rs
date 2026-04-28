use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::{execute, DeleteOp, Filter, Operation, Outcome};
use liwe::state::from_indoc;

fn assert_delete(docs: &str, op: DeleteOp, expected: &[&str]) {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    match execute(&Operation::Delete(op), &graph) {
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
        DeleteOp::new(Filter::eq("status", "archived")),
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
        DeleteOp::new(Filter::all()),
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
        DeleteOp::new(Filter::eq("status", "archived")).limit(2),
        &["1", "2"],
    );
}
