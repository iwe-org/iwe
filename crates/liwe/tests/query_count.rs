use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::execute;
use liwe::query::prelude::{count, eq, filter};
use liwe::query::{CountOp, Outcome};
use liwe::state::from_indoc;

fn assert_count(docs: &str, op: CountOp, expected: usize) {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    match execute(&count(op), &graph) {
        Outcome::Count(n) => assert_eq!(n, expected),
        other => panic!("expected Count, got {:?}", other),
    }
}

#[test]
fn count_whole_corpus() {
    assert_count(
        indoc! {"
            # A
            _
            # B
            _
            # C
        "},
        CountOp::new(),
        3,
    );
}

#[test]
fn count_with_filter() {
    assert_count(
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
        2,
    );
}

#[test]
fn count_respects_limit() {
    assert_count(
        indoc! {"
            ---
            status: draft
            ---
            # A
            _
            ---
            status: draft
            ---
            # B
            _
            ---
            status: draft
            ---
            # C
        "},
        filter::<CountOp>(eq("status", "draft")).limit(2),
        2,
    );
}

#[test]
fn count_zero_limit_unbounded() {
    assert_count(
        indoc! {"
            ---
            status: draft
            ---
            # A
            _
            ---
            status: draft
            ---
            # B
        "},
        filter::<CountOp>(eq("status", "draft")).limit(0),
        2,
    );
}
