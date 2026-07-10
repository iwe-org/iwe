use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::{DjotOptions, FormatOptions};

fn djot_options() -> FormatOptions {
    FormatOptions::Djot(DjotOptions::default())
}

fn roundtrip(input: &str) -> String {
    let mut graph = Graph::new_with_options(djot_options());
    graph.insert_document("key".into(), input.to_string());
    graph.to_markdown(&"key".into())
}

#[test]
fn heading_and_paragraph() {
    let input = indoc! {"
        # Title

        A paragraph of text.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn nested_headers() {
    let input = indoc! {"
        # One

        ## Two

        text
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn emphasis_and_strong() {
    let input = indoc! {"
        A _word_ and a *strong* one.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn bullet_list() {
    let input = indoc! {"
        - one
        - two
        - three
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn ordered_list() {
    let input = indoc! {"
        1. one
        2. two
        3. three
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn task_list() {
    let input = indoc! {"
        - [ ] todo
        - [x] done
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn nested_task_list() {
    let input = indoc! {"
        - [ ] parent

          - [x] child
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn hard_break_reflows_to_space() {
    let input = "one\\\ntwo\n";
    assert_eq!("one two\n", roundtrip(input));
}

#[test]
fn nested_bullet_list() {
    let input = indoc! {"
        - one
        - two

          - nested a
          - nested b
        - three
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn multi_paragraph_list_item() {
    let input = indoc! {"
        - first item

          second paragraph of the first item
        - second item
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn link() {
    let input = indoc! {"
        A [link](https://example.com) here.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn reference_link_definition_does_not_panic() {
    let input = indoc! {"
        See [text][ref] here.

        [ref]: https://example.com
        "};
    assert_eq!("See [text](https://example.com) here.\n", roundtrip(input));
}

#[test]
fn inline_and_display_math() {
    let input = indoc! {"
        Inline $`x^2` and display $$`x^2` here.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn autolink() {
    let input = indoc! {"
        See <https://example.com> here.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn inline_code() {
    let input = indoc! {"
        Use `code` here.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn code_block() {
    let input = indoc! {"
        ``` rust
        let x = 1;
        ```
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn block_quote() {
    let input = indoc! {"
        > quoted text
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn mark_insert_delete() {
    let input = indoc! {"
        Text with {=highlight=}, {+insert+}, and {-delete-} here.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn symbol() {
    let input = indoc! {"
        A :smile: in text.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn superscript_and_subscript() {
    let input = indoc! {"
        H~2~O and e^x^ here.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn span_with_class() {
    let input = indoc! {"
        A [highlighted]{.note} word.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn span_with_id_and_classes() {
    let input = indoc! {"
        A [target]{#anchor .a .b} span.
        "};
    assert_eq!(input, roundtrip(input));
}

#[test]
fn span_with_attribute_pair() {
    let input = indoc! {"
        A [x]{lang=en} span.
        "};
    assert_eq!(input, roundtrip(input));
}
