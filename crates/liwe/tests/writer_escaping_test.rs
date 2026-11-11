use liwe::graph::Graph;
use liwe::markdown::MarkdownReader;
use liwe::model::config::{FormattingOptions, LineBreakStyle, MarkdownOptions};
use pretty_assertions::assert_str_eq;

fn normalize(input: &str, formatting: &FormattingOptions) -> String {
    let mut graph = Graph::new_with_options(MarkdownOptions {
        formatting: formatting.clone(),
        ..Default::default()
    });
    graph.from_markdown("key".into(), input, MarkdownReader::new());
    graph.to_markdown(&"key".into())
}

fn round_trips(input: &str) {
    round_trips_with(input, &FormattingOptions::default());
}

fn round_trips_preserving_breaks(input: &str) {
    round_trips_with(
        input,
        &FormattingOptions {
            preserve_newlines: Some(true),
            preserve_line_breaks: Some(true),
            ..Default::default()
        },
    );
}

fn round_trips_with(input: &str, formatting: &FormattingOptions) {
    let first = normalize(input, formatting);
    assert_str_eq!(input, first);
    assert_str_eq!(input, normalize(&first, formatting));
}

#[test]
fn item_hash_stays_literal() {
    round_trips("- \\# not heading\n");
}

#[test]
fn item_ordered_marker_stays_literal() {
    round_trips("- 1\\. not list\n");
}

#[test]
fn item_dash_marker_stays_literal() {
    round_trips("- \\- not nested\n");
}

#[test]
fn item_plus_marker_stays_literal() {
    round_trips("- \\+ not nested\n");
}

#[test]
fn item_asterisk_marker_stays_literal() {
    round_trips("- \\* not nested\n");
}

#[test]
fn item_paren_marker_stays_literal() {
    round_trips("- 1\\) not list\n");
}

#[test]
fn ordered_item_hash_stays_literal() {
    round_trips("1. \\# not heading\n");
}

#[test]
fn item_backticks_stay_literal() {
    round_trips("- \\`code\\` not code\n");
}

#[test]
fn quote_dash_marker_stays_literal() {
    round_trips("> \\- not bullet\n");
}

#[test]
fn quote_ordered_marker_stays_literal() {
    round_trips("> 1\\. not list\n");
}

#[test]
fn quote_hash_stays_literal() {
    round_trips("> \\# not heading\n");
}

#[test]
fn nested_quote_marker_stays_literal() {
    round_trips("> > \\> not quote\n");
}

#[test]
fn asterisk_inside_emphasis_stays_literal() {
    round_trips("*a\\*b*\n");
}

#[test]
fn asterisk_next_to_emphasis_stays_literal() {
    round_trips("\\*a *b*\n");
}

#[test]
fn split_wiki_brackets_stay_literal() {
    round_trips("\\[[b]]\n");
}

#[test]
fn split_wiki_brackets_after_text_stay_literal() {
    round_trips("a\\[[b]]\n");
}

#[test]
fn split_html_tag_stays_literal() {
    round_trips("\\<a href=x>\n");
}

#[test]
fn tilde_fence_stays_literal() {
    round_trips("\\~~~\n");
}

#[test]
fn tilde_fence_with_text_stays_literal() {
    round_trips("\\~~~ text\n");
}

#[test]
fn brackets_in_link_text_stay_literal() {
    round_trips("[a \\[b\\](c) d](x)\n");
}

#[test]
fn closing_bracket_in_link_text_stays_literal() {
    round_trips("[\\]](x)\n");
}

#[test]
fn closing_bracket_in_image_alt_stays_literal() {
    round_trips("![\\]](x.png)\n");
}

#[test]
fn trailing_hash_in_heading_stays_literal() {
    round_trips("# Heading \\#\n");
}

#[test]
fn trailing_hashes_in_heading_stay_literal() {
    round_trips("# Heading \\##\n");
}

#[test]
fn dash_underline_after_soft_break_stays_literal() {
    round_trips_preserving_breaks("text\n\\---\n");
}

#[test]
fn equals_underline_after_soft_break_stays_literal() {
    round_trips_preserving_breaks("text\n\\===\n");
}

#[test]
fn dash_underline_after_hard_break_stays_literal() {
    round_trips_preserving_breaks("text\\\n\\---\n");
}

#[test]
fn table_delimiter_after_soft_break_stays_literal() {
    round_trips_preserving_breaks("a | b\n\\|---|\n");
}

#[test]
fn bare_table_delimiter_after_soft_break_stays_literal() {
    round_trips_preserving_breaks("a | b\n\\---|---\n");
}

#[test]
fn item_continuation_marker_stays_literal() {
    round_trips_preserving_breaks("- item\n  \\- not nested\n");
}

#[test]
fn tilde_fence_after_soft_break_stays_literal() {
    round_trips_preserving_breaks("text\n\\~~~\n");
}

#[test]
fn quote_hard_break_with_spaces_survives() {
    round_trips_with(
        "> first  \n> second\n",
        &FormattingOptions {
            preserve_line_breaks: Some(true),
            line_break_style: Some(LineBreakStyle::Spaces),
            ..Default::default()
        },
    );
}

#[test]
fn item_quote_marker_stays_literal() {
    round_trips("- \\> not quote\n");
}

#[test]
fn item_backtick_fence_stays_literal() {
    round_trips("- \\`\\`\\`\n");
}

#[test]
fn ordered_item_quote_marker_stays_literal() {
    round_trips("1. \\> not quote\n");
}

#[test]
fn item_quote_stays_quote() {
    round_trips("- > quote\n");
}

#[test]
fn item_code_block_keeps_indentation() {
    round_trips("- ```\n  first\n      indented\n  ```\n");
}

#[test]
fn item_code_block_is_not_wrapped() {
    round_trips_with(
        "- ```\n  first line of code that is longer than the wrap column\n      indented line of code that is also longer than it\n  ```\n",
        &FormattingOptions {
            wrap_column: Some(30),
            ..Default::default()
        },
    );
}

fn normalizes_to(input: &str, expected: &str) {
    let first = normalize(input, &FormattingOptions::default());
    assert_str_eq!(expected, first);
    assert_str_eq!(expected, normalize(&first, &FormattingOptions::default()));
}

#[test]
fn escaped_entity_stays_literal() {
    normalizes_to("&amp;copy; and &copy\n", "\\&copy; and &copy\n");
}

#[test]
fn escaped_autolink_stays_literal() {
    round_trips("\\<https://example.com>\n");
}

#[test]
fn escaped_image_marker_stays_literal() {
    round_trips("\\![alt](image.png)\n");
}

#[test]
fn escaped_double_bracket_before_link_stays_literal() {
    round_trips("\\[\\[x](y)\n");
}

#[test]
fn escaped_bracket_before_link_close_stays_literal() {
    round_trips("\\[x](y)\n");
}

#[test]
fn non_breaking_space_is_kept() {
    round_trips("one\u{a0}two\n");
}
