use std::sync::Once;

use indoc::indoc;
use liwe::markdown::MarkdownReader;
use pretty_assertions::assert_str_eq;

use liwe::graph::Graph;

#[test]
fn normalization_list_item() {
    compare(
        indoc! {"
        - item 1
        "},
        indoc! {"
        - item 1
        "},
    );
}

#[test]
fn normalization_dual_dash_list_item() {
    compare(
        indoc! {"
        - item 1
        "},
        indoc! {"
        - - item 1
        "},
    );
}

#[test]
fn normalization_dual_dash_two_items_list_item() {
    compare(
        indoc! {"
        - item 1
        - item 2
        "},
        indoc! {"
        - - item 1
          - item 2
        "},
    );
}

#[test]
fn normalization_dual_dash_three_items_list_item() {
    compare(
        indoc! {"
        - item 1
        - item 2
        "},
        indoc! {"
        - - - item 1
          - item 2
        "},
    );
}

#[test]
fn normalization_dual_dash_two_items_ordered_list_item() {
    compare(
        indoc! {"
        1.  item 1
        2.  item 2
        "},
        indoc! {"
        1.  1.  item 1
            2.  item 2
        "},
    );
}

#[test]
fn normalization_list_items_with_text() {
    compare(
        indoc! {"
        - item 1

          text

          text
        "},
        indoc! {"
        - item 1

          text

          text
        "},
    );
}

#[test]
fn normalization_numbered_list_item() {
    compare(
        indoc! {"
        1.  item 1
        "},
        indoc! {"
        1.  item 1
        "},
    );
}

#[test]
fn normalization_numbered_list_2_items() {
    compare(
        indoc! {"
        1.  item 1
        2.  item 1
        "},
        indoc! {"
        1.  item 1
        1.  item 1
        "},
    );
}

#[test]
fn normalization_numbered_list_in_list() {
    compare(
        indoc! {"
        1.  item 1
            1.  item 1
        "},
        indoc! {"
        1.  item 1
            1.  item 1
        "},
    );
}

#[test]
fn normalization_numbered_list_with_para() {
    compare(
        indoc! {"
        1.  item 1

            para
        "},
        indoc! {"
        1.  item 1

            para
        "},
    );
}

#[test]
fn normalization_list_after_para() {
    compare(
        indoc! {"
        - item

        text

        - item-2
        "},
        indoc! {"
        - item

        text

        - item-2
        "},
    );
}

#[test]
fn normalization_two_list_items() {
    compare(
        indoc! {"
        - item 1
        - item 2
        "},
        indoc! {"
        - item 1
        - item 2
        "},
    );
}

#[test]
fn normalization_sub_list_item() {
    compare(
        indoc! {"
        - item 1
          - sub item 1
        "},
        indoc! {"
        - item 1
          - sub item 1
        "},
    );
}

#[test]
fn normalization_two_sub_list_item() {
    compare(
        indoc! {"
        - item 1
          - sub item 1
          - sub item 2
        "},
        indoc! {"
        - item 1
          - sub item 1
          - sub item 2
        "},
    );
}

#[test]
fn normalization_three_sub_lists_item() {
    compare(
        indoc! {"
        - item 1
          - sub item 1
            - sub item 2
        "},
        indoc! {"
        - item 1
          - sub item 1
            - sub item 2
        "},
    );
}

#[test]
fn normalization_two_list_with_sub_items() {
    compare(
        indoc! {"
        - item 1
          - sub item 1
        - item 2
          - sub item 2
        "},
        indoc! {"
        - item 1
          - sub item 1
        - item 2
          - sub item 2
        "},
    );
}

#[test]
fn normalization_ordered_list() {
    compare(
        indoc! {"
        1.  list item
        "},
        indoc! {"
        1. list item
        "},
    );
}

#[test]
fn normalization_ordered_list_with_sub_list() {
    compare(
        indoc! {"
        1.  list item
            - sub list item
        "},
        indoc! {"
        1. list item
           - sub list item
        "},
    );
}

#[test]
fn normalization_ordered_list_and_bullet_list() {
    compare(
        indoc! {"
        - sub list item

        1.  list item
        "},
        indoc! {"
        - sub list item

        1. list item
        "},
    );
}

#[test]
fn checkmark_list() {
    compare(
        indoc! {"
        - [x] list item
        - [ ] list item
        "},
        indoc! {"
        - [x] list item
        - [ ] list item
        "},
    );
}

fn compare(expected: &str, denormalized: &str) {
    setup();

    let mut graph = Graph::new();

    graph.from_markdown("key".into(), denormalized, MarkdownReader::new());

    let normalized = graph.to_markdown(&"key".into());

    println!("{:#?}", graph);
    println!("{}", expected);
    println!("normalized:");
    println!("{}", normalized);

    assert_str_eq!(expected, normalized);
}

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::builder().init();
    });
}
