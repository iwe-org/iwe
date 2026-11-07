use crate::blocks::{and, comment, comments, matches, or, text, text_eq, within_section};
use indoc::{formatdoc, indoc};
use liwe::graph::{Graph, GraphContext};
use liwe::markdown::MarkdownReader;
use liwe::model::node::Node;
use liwe::model::tree::Tree;
use liwe::model::Key;
use liwe::query::block::{parse_block_predicate, BlockPredicate};
use liwe::query::block_eval::BlockIndex;
use liwe::query::{evaluate, Filter};
use pretty_assertions::assert_str_eq;

fn round_trip(markdown: &str) -> String {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), markdown, MarkdownReader::new());
    graph.to_markdown(&"key".into())
}

fn assert_round_trip(markdown: &str) {
    assert_str_eq!(markdown, round_trip(markdown));
}

fn tree(markdown: &str) -> Tree {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), markdown, MarkdownReader::new());
    (&graph).collect(&Key::name("key"))
}

fn nodes(tree: &Tree) -> Vec<Node> {
    let mut out = vec![tree.node.clone()];
    for child in &tree.children {
        out.extend(nodes(child));
    }
    out
}

#[test]
fn comment_lines_after_a_paragraph() {
    assert_round_trip(indoc! {"
        Paragraph text.
        <!-- first note -->
        <!-- second note -->

        Next paragraph.
        "});
}

#[test]
fn comment_line_after_a_heading() {
    assert_round_trip(indoc! {"
        # Title
        <!-- rename this -->

        Body.
        "});
}

#[test]
fn comment_line_after_a_list_item_stays_in_the_list() {
    let markdown = indoc! {"
        - first
          <!-- about first -->
        - second
        "};
    assert_round_trip(markdown);
    let lists = nodes(&tree(markdown))
        .into_iter()
        .filter(|node| matches!(node, Node::BulletList()))
        .count();
    assert_eq!(lists, 1);
}

#[test]
fn comment_line_after_an_ordered_item() {
    assert_round_trip(indoc! {"
        1. first
           <!-- about first -->
        2. second
        "});
}

#[test]
fn comment_line_after_a_quoted_line() {
    assert_round_trip(indoc! {"
        > quoted
        > <!-- about the quote -->
        "});
}

#[test]
fn comment_line_after_a_code_block() {
    assert_round_trip(indoc! {"
        ``` rust
        let value = 1;
        ```
        <!-- note -->
        "});
}

#[test]
fn comment_line_after_a_table() {
    assert_round_trip(indoc! {"
        | a   | b   |
        | --- | --- |
        | 1   | 2   |
        <!-- note -->
        "});
}

#[test]
fn comment_line_after_a_rule() {
    let rule = round_trip(indoc! {"
        ---
        "});
    assert_round_trip(&formatdoc! {"
        Before.

        {rule}<!-- note -->
        "});
}

#[test]
fn comments_before_the_first_block() {
    assert_round_trip(indoc! {"
        <!-- document comment -->

        # Title
        "});
}

#[test]
fn comment_nodes_are_siblings_after_their_block() {
    let tree = tree(indoc! {"
        Para.
        <!-- one -->
        <!-- two -->
        "});
    let kinds: Vec<Node> = tree.children.iter().map(|c| c.node.clone()).collect();
    assert_eq!(kinds.len(), 3);
    assert_eq!(kinds[1], Node::Comment(" one ".to_string()));
    assert_eq!(kinds[2], Node::Comment(" two ".to_string()));
}

#[test]
fn inline_comment_in_a_paragraph() {
    assert_round_trip(indoc! {"
        Some text <!-- note -->, more text.
        "});
}

#[test]
fn inline_comment_in_a_heading() {
    assert_round_trip(indoc! {"
        # Title <!-- shorter -->

        Body.
        "});
}

#[test]
fn inline_comment_in_a_list_item() {
    assert_round_trip(indoc! {"
        - first <!-- why first -->
        - second
        "});
}

#[test]
fn inline_comment_in_a_task() {
    assert_round_trip(indoc! {"
        - [ ] task <!-- when -->
        "});
}

#[test]
fn inline_comment_in_a_quote() {
    assert_round_trip(indoc! {"
        > quoted <!-- source? -->
        "});
}

#[test]
fn inline_comment_in_a_table_cell() {
    assert_round_trip(indoc! {"
        | a              | b   |
        | -------------- | --- |
        | 1 <!-- one --> | 2   |
        "});
}

#[test]
fn inline_comment_in_an_inclusion_link_paragraph() {
    assert_round_trip(indoc! {"
        [Child](child) <!-- split this -->
        "});
}

#[test]
fn inline_comment_in_code_span_is_literal() {
    let markdown = indoc! {"
        Use `<!-- x -->` here.
        "};
    assert_round_trip(markdown);
    let has_comment = nodes(&tree(markdown)).iter().any(|node| match node {
        Node::Leaf(inlines) => inlines
            .iter()
            .any(|i| matches!(i, liwe::model::Inline::Comment(_))),
        _ => false,
    });
    assert!(!has_comment);
}

#[test]
fn heading_title_excludes_an_inline_comment() {
    let markdown = indoc! {"
        # Title <!-- shorter -->
        "};
    let tree = tree(markdown);
    assert_eq!(tree.children[0].node.plain_text().trim(), "Title");
}

#[test]
fn inclusion_link_with_an_inline_comment_is_a_reference() {
    let tree = tree(indoc! {"
        [Child](child) <!-- split this -->
        "});
    match &tree.children[0].node {
        Node::Reference(reference) => {
            assert_eq!(reference.text(), "Child");
            assert_eq!(reference.comments, vec![" split this ".to_string()]);
        }
        other => panic!("expected a reference, got {other:?}"),
    }
}

#[test]
fn other_html_blocks_are_kept() {
    assert_round_trip(indoc! {"
        text

        <div>block</div>

        text 2
        "});
}

#[test]
fn a_comment_followed_by_text_on_its_line_is_kept_as_html() {
    assert_round_trip(indoc! {"
        text

        <!-- c --> trailing
        "});
}

#[test]
fn other_inline_html_is_unchanged() {
    assert_round_trip(indoc! {"
        <em>text</em> and <!-- note --> here
        "});
}

fn documents(markdown: &str, predicate: BlockPredicate) -> usize {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), markdown, MarkdownReader::new());
    evaluate(&Filter::content(predicate), &graph).len()
}

fn selected(markdown: &str, predicate: BlockPredicate) -> Vec<String> {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), markdown, MarkdownReader::new());
    let index = BlockIndex::build(&graph, &Key::name("key"));
    index
        .select(&predicate)
        .into_iter()
        .map(|i| {
            format!(
                "{}: {}",
                index.kind_name(i),
                index.own_text(i).unwrap_or("")
            )
        })
        .collect()
}

fn parse_error(predicate: &str) -> Option<String> {
    let value: serde_yaml::Value = serde_yaml::from_str(predicate).expect("valid yaml");
    parse_block_predicate(&value, "$content")
        .err()
        .map(|error| error.to_string())
}

#[test]
fn comment_predicate_matches_documents_with_block_and_inline_comments() {
    let block = indoc! {"
        Para.
        <!-- some note -->
        "};
    let inline = indoc! {"
        Para <!-- some note -->.
        "};
    let none = indoc! {"
        Para about a note.
        "};
    assert_eq!(documents(block, comment(text("note"))), 1);
    assert_eq!(documents(inline, comment(text("note"))), 1);
    assert_eq!(documents(none, comment(text("note"))), 0);
    assert_eq!(documents(block, comments()), 1);
    assert_eq!(documents(none, comments()), 0);
}

#[test]
fn comment_line_belongs_to_the_paragraph_before_it() {
    let markdown = indoc! {"
        # Title

        Para.
        <!-- note -->
        "};
    assert_eq!(selected(markdown, comments()), vec!["paragraph: Para."]);
}

#[test]
fn comment_line_belongs_to_the_heading_it_follows() {
    let markdown = indoc! {"
        # Title
        <!-- note -->

        Body.
        "};
    assert_eq!(selected(markdown, comments()), vec!["header: Title"]);
}

#[test]
fn comment_line_after_a_list_belongs_to_the_list() {
    let markdown = indoc! {"
        - first
        - second

        <!-- note -->
        "};
    assert_eq!(selected(markdown, comments()), vec!["list: "]);
}

#[test]
fn comment_line_inside_an_item_belongs_to_the_item() {
    let markdown = indoc! {"
        - first
          <!-- note -->
        - second
        "};
    assert_eq!(selected(markdown, comments()), vec!["item: first"]);
}

#[test]
fn comment_line_after_a_quote_belongs_to_the_quote() {
    let markdown = indoc! {"
        > quoted

        <!-- note -->
        "};
    assert_eq!(selected(markdown, comments()), vec!["quote: "]);
}

#[test]
fn comment_lines_before_the_first_block_belong_to_it() {
    let markdown = indoc! {"
        <!-- first -->
        <!-- second -->

        # Title
        "};
    assert_eq!(selected(markdown, comment("second")), vec!["header: Title"]);
}

#[test]
fn comment_lines_without_any_block_have_no_owner() {
    let markdown = indoc! {"
        <!-- first -->
        <!-- second -->
        "};
    assert_eq!(selected(markdown, comments()), Vec::<String>::new());
    assert_eq!(documents(markdown, comments()), 0);
}

#[test]
fn text_predicates_skip_comments() {
    let block = indoc! {"
        Para.
        <!-- hidden -->
        "};
    let inline = indoc! {"
        Para <!-- hidden -->.
        "};
    assert_eq!(selected(block, text("hidden")), Vec::<String>::new());
    assert_eq!(selected(inline, text("hidden")), Vec::<String>::new());
    assert_eq!(selected(inline, matches("hidden")), Vec::<String>::new());
}

#[test]
fn text_predicates_skip_nested_inline_comments() {
    let markdown = indoc! {"
        Para *with <!-- hidden --> emphasis*.
        "};
    assert_eq!(selected(markdown, text("hidden")), Vec::<String>::new());
    assert_eq!(
        selected(markdown, comment("hidden")),
        vec!["paragraph: Para *with emphasis*."]
    );
}

#[test]
fn own_text_collapses_the_space_left_by_a_comment() {
    let between = indoc! {"
        First <!-- hidden --> second
        "};
    let trailing = indoc! {"
        # Title <!-- hidden -->
        "};
    let leading = indoc! {"
        Para *<!-- hidden --> emphasis*
        "};
    assert_eq!(
        selected(between, text_eq("First second")),
        vec!["paragraph: First second"]
    );
    assert_eq!(selected(trailing, text_eq("Title")), vec!["header: Title"]);
    assert_eq!(
        selected(leading, text_eq("Para *emphasis*")),
        vec!["paragraph: Para *emphasis*"]
    );
}

#[test]
fn comment_scalar_matches_the_whole_comment_text() {
    let markdown = indoc! {"
        Para.
        <!-- some note -->
        "};
    assert_eq!(selected(markdown, comment("note")), Vec::<String>::new());
    assert_eq!(
        selected(markdown, comment("Some Note")),
        vec!["paragraph: Para."]
    );
    assert_eq!(
        selected(markdown, comment(text("note"))),
        vec!["paragraph: Para."]
    );
}

#[test]
fn comment_argument_rejects_structural_predicates() {
    let message =
        "'$comment' argument accepts only text predicates ($text, $matches, $and, $or, $nor)";
    assert_eq!(
        parse_error("{ $comment: { $header: {} } }"),
        Some(message.to_string())
    );
    assert_eq!(
        parse_error("{ $comment: { $or: [ { $text: a }, { $within: Title } ] } }"),
        Some(message.to_string())
    );
}

#[test]
#[should_panic(expected = "$comment argument accepts only text predicates")]
fn comment_builder_rejects_structural_predicates() {
    comment(or(vec![text("a"), within_section("Title")]));
}

#[test]
fn comment_argument_composes_text_predicates() {
    let markdown = indoc! {"
        First.
        <!-- alpha note -->

        Second.
        <!-- beta note -->
        "};
    assert_eq!(
        selected(markdown, comment(and(vec![text("note"), matches("^beta")]))),
        vec!["paragraph: Second."]
    );
}

#[test]
fn tool_markers_are_comments_to_a_query() {
    let markdown = indoc! {"
        Para.
        <!-- TOC -->
        "};
    assert_eq!(selected(markdown, comments()), vec!["paragraph: Para."]);
}

#[test]
fn blank_line_before_a_comment_line_is_removed() {
    let markdown = indoc! {"
        Para.

        <!-- note -->

        Next.
        "};
    let normalized = round_trip(markdown);
    assert_str_eq!(
        indoc! {"
            Para.
            <!-- note -->

            Next.
            "},
        normalized
    );
    assert_eq!(
        selected(&normalized, comments()),
        selected(markdown, comments())
    );
}

#[test]
fn comment_line_after_a_list_stays_with_the_list_when_normalized() {
    let markdown = indoc! {"
        - first
        - second

        <!-- note -->
        "};
    let normalized = round_trip(markdown);
    assert_str_eq!(
        indoc! {"
            - first
            - second
            <!-- note -->
            "},
        normalized
    );
    assert_eq!(selected(&normalized, comments()), vec!["list: "]);
}

#[test]
fn comment_line_after_a_quote_stays_with_the_quote_when_normalized() {
    let markdown = indoc! {"
        > quoted

        <!-- note -->
        "};
    let normalized = round_trip(markdown);
    assert_str_eq!(
        indoc! {"
            > quoted
            <!-- note -->
            "},
        normalized
    );
    assert_eq!(selected(&normalized, comments()), vec!["quote: "]);
}
