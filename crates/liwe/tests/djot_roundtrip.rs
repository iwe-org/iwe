use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::{DjotOptions, FormatOptions, FormattingOptions};

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

fn format_with(input: &str, formatting: FormattingOptions) -> String {
    let mut graph = Graph::new_with_options(FormatOptions::Djot(DjotOptions {
        formatting,
        ..Default::default()
    }));
    graph.insert_document("key".into(), input.to_string());
    graph.to_markdown(&"key".into())
}

#[test]
fn preserve_newlines_keeps_soft_breaks() {
    let input = indoc! {"
        first line
        second line
        "};
    assert_eq!(
        input,
        format_with(
            input,
            FormattingOptions {
                preserve_newlines: Some(true),
                ..Default::default()
            }
        )
    );
}

#[test]
fn soft_breaks_join_with_space_by_default() {
    assert_eq!(
        "first line second line\n",
        format_with(
            indoc! {"
                first line
                second line
                "},
            FormattingOptions::default()
        )
    );
}

#[test]
fn wrap_column_wraps_long_paragraph() {
    assert_eq!(
        indoc! {"
            alpha beta gamma delta epsilon
            zeta eta theta iota kappa
            "},
        format_with(
            "alpha beta gamma delta epsilon zeta eta theta iota kappa\n",
            FormattingOptions {
                wrap_column: Some(30),
                ..Default::default()
            }
        )
    );
}

#[test]
fn wrap_column_keeps_inline_code_and_link_url_atomic() {
    assert_eq!(
        indoc! {"
            alpha `code with spaces` [link
            text](https://example.com/a/b)
            omega
            "},
        format_with(
            "alpha `code with spaces` [link text](https://example.com/a/b) omega\n",
            FormattingOptions {
                wrap_column: Some(30),
                ..Default::default()
            }
        )
    );
}

#[test]
fn wrap_column_wraps_each_preserved_newline_separately() {
    assert_eq!(
        indoc! {"
            alpha beta gamma delta
            epsilon zeta
            eta theta iota
            "},
        format_with(
            "alpha beta gamma delta epsilon zeta\neta theta iota\n",
            FormattingOptions {
                wrap_column: Some(25),
                preserve_newlines: Some(true),
                ..Default::default()
            }
        )
    );
}

#[test]
fn wrap_column_subtracts_list_indent() {
    assert_eq!(
        indoc! {"
            - alpha beta gamma
              delta epsilon zeta
            "},
        format_with(
            "- alpha beta gamma delta epsilon zeta\n",
            FormattingOptions {
                wrap_column: Some(20),
                ..Default::default()
            }
        )
    );
}

#[test]
fn escaped_bullet_marker_survives_normalization() {
    let input = "\\- not a list\n";
    assert_eq!(input, roundtrip(input));
}

#[test]
fn escaped_heading_marker_survives_normalization() {
    let input = "\\# not a heading\n";
    assert_eq!(input, roundtrip(input));
}

#[test]
fn escaped_quote_marker_survives_normalization() {
    let input = "\\> not a quote\n";
    assert_eq!(input, roundtrip(input));
}

#[test]
fn escaped_ordered_marker_survives_normalization() {
    let input = "1\\. not ordered\n";
    assert_eq!(input, roundtrip(input));
}

#[test]
fn escaped_table_marker_survives_normalization() {
    let input = "\\| not a table\n";
    assert_eq!(input, roundtrip(input));
}

#[test]
fn escaped_marker_inside_list_item_survives_normalization() {
    let input = "- \\- not nested\n";
    assert_eq!(input, roundtrip(input));
}

#[test]
fn block_markers_are_not_escaped_mid_paragraph() {
    let input = "normal - dash - text\n";
    assert_eq!(input, roundtrip(input));
}

#[test]
fn marker_without_trailing_space_is_not_escaped() {
    let input = "-dash no space\n";
    assert_eq!(input, roundtrip(input));
}

#[test]
fn wrapped_continuation_line_does_not_escape_markers() {
    assert_eq!(
        indoc! {"
            alphabeta gammadelta
            - epsilon
            "},
        format_with(
            "alphabeta gammadelta - epsilon\n",
            FormattingOptions {
                wrap_column: Some(20),
                ..Default::default()
            }
        )
    );
}
