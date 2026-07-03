use liwe::graph::{Graph, GraphContext};
use liwe::markdown::MarkdownReader;
use liwe::model::node::Node;
use pretty_assertions::assert_str_eq;

fn compare(expected: &str, input: &str) {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), input, MarkdownReader::new());
    let normalized = graph.to_markdown(&"key".into());
    assert_str_eq!(expected, normalized);
}

#[test]
fn escaped_emphasis_stays_literal() {
    compare("\\*not emphasis\\*\n", "\\*not emphasis\\*\n");
}

#[test]
fn single_asterisk_is_not_escaped() {
    compare("rating 5*\n", "rating 5*\n");
}

#[test]
fn escaped_heading_stays_in_paragraph() {
    compare("\\# not a heading\n", "\\# not a heading\n");
}

#[test]
fn hashtag_without_space_is_not_escaped() {
    compare("#tag stays\n", "#tag stays\n");
}

#[test]
fn escaped_link_stays_literal() {
    compare("[not a link\\](target)\n", "\\[not a link\\](target)\n");
}

#[test]
fn escaped_ordered_marker_stays_literal() {
    compare("1\\. not a list\n", "1\\. not a list\n");
}

#[test]
fn escaped_ordered_paren_marker_stays_literal() {
    compare("1\\) not a list\n", "1\\) not a list\n");
}

#[test]
fn escaped_bullet_marker_stays_literal() {
    compare("\\- not a bullet\n", "\\- not a bullet\n");
}

#[test]
fn escaped_asterisk_bullet_marker_stays_literal() {
    compare("\\* not a list\n", "\\* not a list\n");
}

#[test]
fn escaped_blockquote_marker_stays_literal() {
    compare("\\> not a quote\n", "\\> not a quote\n");
}

#[test]
fn underscores_inside_words_are_not_escaped() {
    compare("file_name_here\n", "file_name_here\n");
}

#[test]
fn lone_underscore_is_not_escaped() {
    compare("_private field\n", "_private field\n");
    compare("name_ here\n", "name_ here\n");
}

#[test]
fn escaped_underscore_emphasis_stays_literal() {
    compare("\\_em\\_\n", "\\_em\\_\n");
}

#[test]
fn escaped_thematic_break_stays_literal() {
    compare("\\---\n", "\\---\n");
}

#[test]
fn inline_code_with_backtick_uses_longer_fence() {
    compare("``a ` b``\n", "`` a ` b ``\n");
}

#[test]
fn inline_html_is_preserved() {
    compare("<em>text</em>\n", "<em>text</em>\n");
}

#[test]
fn escaped_checkbox_with_mark_is_not_a_task() {
    compare("- \\[x] literal\n", "- \\[x\\] literal\n");
}

#[test]
fn escaped_empty_checkbox_is_not_a_task() {
    compare("- \\[ ] literal\n", "- \\[ \\] literal\n");
}

#[test]
fn inline_code_resembling_a_checkbox_is_not_escaped() {
    compare(
        "- `[x]` immediately when a check completes\n",
        "- `[x]` immediately when a check completes\n",
    );
    compare("- `[ ]` code span\n", "- `[ ]` code span\n");
}

#[test]
fn escaped_empty_checkbox_node_is_a_plain_item() {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), "- \\[ \\] literal\n", MarkdownReader::new());
    let tree = (&graph).collect(&"key".into());
    let Node::Item(checked, _) = &tree.children[0].children[0].node else {
        panic!("expected an item node");
    };
    assert_eq!(&None, checked);
}

#[test]
fn flattened_block_markers_in_list_items_are_not_escaped() {
    compare("- > flattened quote\n", "- > flattened quote\n");
    compare("- --- text\n", "- --- text\n");
}
