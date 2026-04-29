use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::{
    execute, CountArg, Filter, FindOp, InclusionAnchor, KeyOp, MaxDepth, NumExpr,
    Operation, Outcome, ReferenceAnchor,
};
use liwe::state::from_indoc;

fn run_find_keys(docs: &str, op: FindOp) -> Vec<String> {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    match execute(&Operation::Find(op), &graph) {
        Outcome::Find { matches } => matches.into_iter().map(|m| m.key.to_string()).collect(),
        other => panic!("expected Find, got {:?}", other),
    }
}

fn assert_keys(docs: &str, op: FindOp, expected: &[&str]) {
    let mut actual = run_find_keys(docs, op);
    actual.sort();
    let mut expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn key_eq_selects_one() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
            _
            # C
        "},
        FindOp::new().filter(Filter::key(KeyOp::eq("2"))),
        &["2"],
    );
}

#[test]
fn key_in_selects_subset() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
            _
            # C
        "},
        FindOp::new().filter(Filter::key(KeyOp::in_(&["1", "3"]))),
        &["1", "3"],
    );
}

#[test]
fn key_ne_excludes() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
            _
            # C
        "},
        FindOp::new().filter(Filter::key(KeyOp::ne("2"))),
        &["1", "3"],
    );
}

#[test]
fn included_by_count_zero_selects_roots() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludedByCount(
            CountArg::direct(NumExpr::eq(0)),
        )),
        &["1"],
    );
}

#[test]
fn includes_count_zero_selects_leaves() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludesCount(CountArg::direct(
            NumExpr::eq(0),
        ))),
        &["3"],
    );
}

#[test]
fn includes_count_transitive_any() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludesCount(CountArg {
            count: NumExpr::gte(2),
            min_depth: 1,
            max_depth: MaxDepth::Any,
        })),
        &["1"],
    );
}

#[test]
fn included_by_direct() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludedBy(vec![
            InclusionAnchor::with_max("1", 1),
        ])),
        &["2"],
    );
}

#[test]
fn included_by_transitive() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludedBy(vec![
            InclusionAnchor::with_max("1", 5),
        ])),
        &["2", "3"],
    );
}

#[test]
fn included_by_range_excludes_direct() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludedBy(vec![
            InclusionAnchor::new("1", 2, 5),
        ])),
        &["3"],
    );
}

#[test]
fn includes_outbound() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::Includes(vec![
            InclusionAnchor::with_max("3", 5),
        ])),
        &["1", "2"],
    );
}

#[test]
fn anchor_excluded_from_walk_results() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludedBy(vec![
            InclusionAnchor::with_max("1", 5),
        ])),
        &["2", "3"],
    );
}

#[test]
fn multi_anchor_intersects() {
    assert_keys(
        indoc! {"
            [c](3)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludedBy(vec![
            InclusionAnchor::with_max("1", 5),
            InclusionAnchor::with_max("2", 5),
        ])),
        &["3"],
    );
}

#[test]
fn references_by_inline_link_in_text() {
    assert_keys(
        indoc! {"
            # A

            See [other](2) for details.
            _
            # B
        "},
        FindOp::new().filter(Filter::References(vec![
            ReferenceAnchor::with_max("2", 1),
        ])),
        &["1"],
    );
}

#[test]
fn referenced_by_inline() {
    assert_keys(
        indoc! {"
            # A

            See [other](2) for details.
            _
            # B
        "},
        FindOp::new().filter(Filter::ReferencedBy(vec![
            ReferenceAnchor::with_max("1", 1),
        ])),
        &["2"],
    );
}

#[test]
fn missing_anchor_returns_empty() {
    assert_keys(
        indoc! {"
            # A
        "},
        FindOp::new().filter(Filter::IncludedBy(vec![
            InclusionAnchor::with_max("does-not-exist", 5),
        ])),
        &[],
    );
}

#[test]
fn empty_corpus_returns_empty() {
    let graph = Graph::import(
        &std::collections::HashMap::new(),
        MarkdownOptions::default(),
        None,
    );
    let op = FindOp::new().filter(Filter::IncludesCount(CountArg::direct(
        NumExpr::eq(0),
    )));
    match execute(&Operation::Find(op), &graph) {
        Outcome::Find { matches } => assert!(matches.is_empty()),
        other => panic!("{:?}", other),
    }
}

#[test]
fn graph_op_combines_with_frontmatter_predicate() {
    assert_keys(
        indoc! {"
            ---
            status: draft
            ---
            [b](2)
            _
            ---
            status: published
            ---
            # B
        "},
        FindOp::new().filter(Filter::and(vec![
            Filter::IncludedByCount(CountArg::direct(NumExpr::eq(0))),
            Filter::eq("status", "draft"),
        ])),
        &["1"],
    );
}

#[test]
fn key_nin_excludes_listed() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
            _
            # C
            _
            # D
        "},
        FindOp::new().filter(Filter::key(KeyOp::nin(&["2", "4"]))),
        &["1", "3"],
    );
}

#[test]
fn key_eq_against_missing_returns_empty() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
        "},
        FindOp::new().filter(Filter::key(KeyOp::eq("missing"))),
        &[],
    );
}

#[test]
fn included_by_count_polyhierarchy() {
    assert_keys(
        indoc! {"
            [c](3)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludedByCount(
            CountArg::direct(NumExpr::gte(2)),
        )),
        &["3"],
    );
}

#[test]
fn includes_count_exact_n() {
    assert_keys(
        indoc! {"
            [b](2)

            [c](3)
            _
            # B
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludesCount(
            CountArg::direct(NumExpr::eq(2)),
        )),
        &["1"],
    );
}

#[test]
fn includes_count_range_band() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            [d](4)
            _
            # D
        "},
        FindOp::new().filter(Filter::IncludesCount(CountArg {
            count: NumExpr::gte(1),
            min_depth: 2,
            max_depth: MaxDepth::Bounded(4),
        })),
        &["1", "2"],
    );
}

#[test]
fn includes_count_zero_at_depth_2() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludesCount(CountArg {
            count: NumExpr::eq(0),
            min_depth: 2,
            max_depth: MaxDepth::Any,
        })),
        &["2", "3"],
    );
}

#[test]
fn included_by_chain_at_depth_3() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            [d](4)
            _
            # D
        "},
        FindOp::new().filter(Filter::IncludedBy(vec![
            InclusionAnchor::new("1", 3, 3),
        ])),
        &["4"],
    );
}

#[test]
fn includes_outbound_chain() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            [d](4)
            _
            # D
        "},
        FindOp::new().filter(Filter::Includes(vec![
            InclusionAnchor::with_max("4", 5),
        ])),
        &["1", "2", "3"],
    );
}

#[test]
fn included_by_polyhierarchy_set_semantics() {
    assert_keys(
        indoc! {"
            [c](3)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::IncludedBy(vec![
            InclusionAnchor::with_max("1", 5),
            InclusionAnchor::with_max("2", 5),
        ])),
        &["3"],
    );
}

#[test]
fn references_multi_hop() {
    assert_keys(
        indoc! {"
            See [b](2).
            _
            See [c](3).
            _
            # C
        "},
        FindOp::new().filter(Filter::References(vec![
            ReferenceAnchor::with_max("3", 2),
        ])),
        &["1", "2"],
    );
}

#[test]
fn referenced_by_multi_hop() {
    assert_keys(
        indoc! {"
            See [b](2).
            _
            See [c](3).
            _
            # C
        "},
        FindOp::new().filter(Filter::ReferencedBy(vec![
            ReferenceAnchor::with_max("1", 2),
        ])),
        &["2", "3"],
    );
}

#[test]
fn reference_self_link_excluded_from_walk() {
    assert_keys(
        indoc! {"
            See [self](1).
        "},
        FindOp::new().filter(Filter::References(vec![
            ReferenceAnchor::with_max("1", 1),
        ])),
        &[],
    );
}

#[test]
fn referenced_by_range_excludes_direct() {
    assert_keys(
        indoc! {"
            See [b](2).
            _
            See [c](3).
            _
            # C
        "},
        FindOp::new().filter(Filter::ReferencedBy(vec![
            ReferenceAnchor::new("1", 2, 3),
        ])),
        &["3"],
    );
}

#[test]
fn or_of_two_graph_ops() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::or(vec![
            Filter::IncludedByCount(CountArg::direct(NumExpr::eq(0))),
            Filter::IncludedBy(vec![InclusionAnchor::with_max(
                "2", 1,
            )]),
        ])),
        &["1", "3"],
    );
}

#[test]
fn not_wraps_count() {
    assert_keys(
        indoc! {"
            [b](2)

            [c](3)

            [d](4)
            _
            [b](2)
            _
            # B
            _
            # D
        "},
        FindOp::new().filter(Filter::Not(Box::new(Filter::IncludesCount(
            CountArg::direct(NumExpr::gte(2)),
        )))),
        &["2", "3", "4"],
    );
}

#[test]
fn not_wraps_walk() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            # B
            _
            # C
        "},
        FindOp::new().filter(Filter::Not(Box::new(Filter::IncludedBy(
            vec![InclusionAnchor::with_max("1", 5)],
        )))),
        &["1", "3"],
    );
}

#[test]
fn combined_three_predicates_hub_under_anchor() {
    assert_keys(
        indoc! {"
            ---
            status: draft
            ---
            [hub](2)
            _
            ---
            status: draft
            ---
            [a](3)

            [b](4)

            [c](5)
            _
            # A
            _
            # B
            _
            # C
        "},
        FindOp::new().filter(Filter::and(vec![
            Filter::eq("status", "draft"),
            Filter::IncludedBy(vec![InclusionAnchor::with_max("1", 5)]),
            Filter::IncludesCount(CountArg::direct(NumExpr::gte(3))),
        ])),
        &["2"],
    );
}

#[test]
fn or_anchor_or_descendants() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        FindOp::new().filter(Filter::or(vec![
            Filter::key(KeyOp::eq("1")),
            Filter::IncludedBy(vec![InclusionAnchor::with_max(
                "1", 5,
            )]),
        ])),
        &["1", "2", "3"],
    );
}

#[test]
fn exclude_inside_descendants() {
    assert_keys(
        indoc! {"
            [b](2)

            [c](3)
            _
            # B
            _
            # C
        "},
        FindOp::new().filter(Filter::and(vec![
            Filter::IncludedBy(vec![InclusionAnchor::with_max(
                "1", 5,
            )]),
            Filter::key(KeyOp::ne("3")),
        ])),
        &["2"],
    );
}

#[test]
fn disconnected_components() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            # B
            _
            [d](4)
            _
            # D
        "},
        FindOp::new().filter(Filter::IncludedBy(vec![
            InclusionAnchor::with_max("1", 100),
        ])),
        &["2"],
    );
}

#[test]
fn sort_after_graph_filter() {
    use liwe::query::Sort;
    assert_keys(
        indoc! {"
            ---
            created: 2026-03-01
            ---
            [b](2)
            _
            ---
            created: 2026-01-01
            ---
            [d](4)
            _
            # B
            _
            # D
        "},
        FindOp::new()
            .filter(Filter::IncludedByCount(CountArg::direct(
                NumExpr::eq(0),
            )))
            .sort(Sort::asc("created"))
            .limit(2),
        &["1", "3"],
    );
}
