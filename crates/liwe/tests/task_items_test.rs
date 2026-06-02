use indoc::indoc;
use liwe::graph::{Graph, GraphContext};
use liwe::markdown::MarkdownReader;
use liwe::model::config::{FormattingOptions, MarkdownOptions};
use liwe::model::inline::Inline;
use liwe::model::node::Node;
use liwe::stats::KeyStatistics;
use pretty_assertions::assert_str_eq;

fn compare(expected: &str, input: &str) {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), input, MarkdownReader::new());
    let normalized = graph.to_markdown(&"key".into());
    assert_str_eq!(expected, normalized);
}

fn compare_with(expected: &str, input: &str, formatting: FormattingOptions) {
    let mut graph = Graph::new_with_options(MarkdownOptions {
        formatting,
        ..Default::default()
    });
    graph.from_markdown("key".into(), input, MarkdownReader::new());
    let normalized = graph.to_markdown(&"key".into());
    assert_str_eq!(expected, normalized);
}

#[test]
fn unchecked_task_item_round_trips() {
    compare(
        indoc! {"
        - [ ] one
        "},
        indoc! {"
        - [ ] one
        "},
    );
}

#[test]
fn checked_task_item_round_trips() {
    compare(
        indoc! {"
        - [x] one
        "},
        indoc! {"
        - [x] one
        "},
    );
}

#[test]
fn uppercase_checkbox_is_normalized_to_lowercase() {
    compare(
        indoc! {"
        - [x] one
        "},
        indoc! {"
        - [X] one
        "},
    );
}

#[test]
fn ordered_task_items_round_trip() {
    compare(
        indoc! {"
        1. [ ] one
        2. [x] two
        "},
        indoc! {"
        1. [ ] one
        2. [x] two
        "},
    );
}

#[test]
fn nested_task_items_round_trip() {
    compare(
        indoc! {"
        - [ ] one
          - [x] two
        "},
        indoc! {"
        - [ ] one
          - [x] two
        "},
    );
}

#[test]
fn plain_list_item_is_not_a_task() {
    compare(
        indoc! {"
        - one
        "},
        indoc! {"
        - one
        "},
    );
}

#[test]
fn bracketed_text_is_not_a_checkbox() {
    compare(
        indoc! {"
        - [a] one
        "},
        indoc! {"
        - [a] one
        "},
    );
}

#[test]
fn list_items_are_not_counted_as_sections() {
    let mut graph = Graph::new();
    graph.from_markdown(
        "key".into(),
        indoc! {"
        # Header

        - item one
        - item two
        "},
        MarkdownReader::new(),
    );
    let stats = KeyStatistics::from_graph(&graph);
    assert_eq!(1, stats[0].sections);
    assert_eq!(1, stats[0].bullet_lists);
}

#[test]
fn task_item_keeps_checkbox_with_first_word_when_wrapping() {
    compare_with(
        indoc! {"
        - [x] supercalifragilisticexpialidocious
          word
        "},
        indoc! {"
        - [x] supercalifragilisticexpialidocious word
        "},
        FormattingOptions {
            wrap_column: Some(20),
            ..Default::default()
        },
    );
}

#[test]
fn task_item_state_survives_tree_rebuild() {
    let mut graph = Graph::new();
    graph.from_markdown("a".into(), "- [x] one\n", MarkdownReader::new());
    let tree = (&graph).collect(&"a".into());
    graph.build_key_from_iter(&"b".into(), tree.iter());
    let rebuilt = (&graph).collect(&"b".into());
    assert_eq!(
        Node::Item(Some(true), vec![Inline::Str("one".to_string())]),
        rebuilt.children[0].children[0].node
    );
}

#[test]
fn task_item_respects_bullet_list_content_indent() {
    compare_with(
        indoc! {"
        -   [ ] one
        "},
        indoc! {"
        - [ ] one
        "},
        FormattingOptions {
            bullet_list_content_indent: Some(4),
            ..Default::default()
        },
    );
}
