use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::{
    execute, Filter, InclusionAnchor, Operation, Outcome, Update, UpdateOp, UpdateOperator,
};
use liwe::state::{from_indoc, to_indoc};
use pretty_assertions::assert_str_eq;


fn assert_update(docs: &str, op: UpdateOp, expected: &str) {
    let state = from_indoc(docs);
    let graph = Graph::import(&state, MarkdownOptions::default(), None);
    let outcome = execute(&Operation::Update(op), &graph);
    match outcome {
        Outcome::Update { changes, failed } => {
            assert!(failed.is_empty(), "update failures: {:?}", failed);


            let mut new_state = graph.export();
            for (key, markdown) in changes {
                new_state.insert(key.to_string(), markdown);
            }
            assert_str_eq!(expected, to_indoc(&new_state));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn set_writes_new_field() {
    assert_update(
        indoc! {"
            ---
            status: draft
            ---
            # A
        "},
        UpdateOp::new(
            Filter::eq("status", "draft"),
            Update::new(vec![UpdateOperator::set("reviewed", true)]),
        ),
        indoc! {"
            ---
            status: draft
            reviewed: true
            ---

            # A
        "},
    );
}

#[test]
fn unset_removes_existing_field() {
    assert_update(
        indoc! {"
            ---
            status: draft
            reviewed: false
            ---
            # A
        "},
        UpdateOp::new(
            Filter::eq("status", "draft"),
            Update::new(vec![UpdateOperator::unset("reviewed")]),
        ),
        indoc! {"
            ---
            status: draft
            ---

            # A
        "},
    );
}

#[test]
fn set_dotted_path_persists_nested_structure() {
    assert_update(
        indoc! {"
            ---
            status: draft
            ---
            # A
        "},
        UpdateOp::new(
            Filter::eq("status", "draft"),
            Update::new(vec![
                UpdateOperator::set("author.name", "dmytro"),
                UpdateOperator::set("author.email", "d@example.com"),
            ]),
        ),
        indoc! {"
            ---
            status: draft
            author:
              name: dmytro
              email: d@example.com
            ---

            # A
        "},
    );
}

#[test]
fn update_creates_frontmatter_when_absent() {
    assert_update(
        indoc! {"
            # A

            body
        "},
        UpdateOp::new(
            Filter::all(),
            Update::new(vec![UpdateOperator::set("status", "published")]),
        ),
        indoc! {"
            ---
            status: published
            ---

            # A

            body
        "},
    );
}

#[test]
fn update_only_matched_docs_change() {
    assert_update(
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
        "},
        UpdateOp::new(
            Filter::eq("status", "draft"),
            Update::new(vec![UpdateOperator::set("reviewed", true)]),
        ),
        indoc! {"
            ---
            status: draft
            reviewed: true
            ---

            # A
            _
            ---
            status: published
            ---

            # B
        "},
    );
}

#[test]
fn update_with_graph_filter_targets_descendants() {
    assert_update(
        indoc! {"
            ---
            status: draft
            ---
            [b](2)
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
        UpdateOp::new(
            Filter::IncludedBy(vec![InclusionAnchor::with_max("1", 5)]),
            Update::new(vec![UpdateOperator::set("reviewed", true)]),
        ),
        indoc! {"
            ---
            status: draft
            ---

            [B](2)
            _
            ---
            status: draft
            reviewed: true
            ---

            # B
            _
            ---
            status: draft
            ---

            # C
        "},
    );
}
