use crate::blocks::{comments, html, html_blocks, paragraphs, text};
use indoc::indoc;
use liwe::graph::{Graph, GraphContext};
use liwe::markdown::MarkdownReader;
use liwe::model::config::{DjotOptions, FormatOptions, FormattingOptions, MarkdownOptions};
use liwe::model::node::Node;
use liwe::model::{Inline, Key};
use liwe::query::block::BlockPredicate;
use liwe::query::block_eval::BlockIndex;
use liwe::query::{execute, parse_operation, OperationKind, Outcome};
use liwe::state::{from_indoc, to_indoc};
use pretty_assertions::assert_str_eq;

fn round_trip(markdown: &str) -> String {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), markdown, MarkdownReader::new());
    graph.to_markdown(&"key".into())
}

fn assert_round_trip(markdown: &str) {
    assert_str_eq!(markdown, round_trip(markdown));
}

fn top_level_nodes(markdown: &str) -> Vec<Node> {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), markdown, MarkdownReader::new());
    (&graph)
        .collect(&Key::name("key"))
        .children
        .into_iter()
        .map(|child| child.node)
        .collect()
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

fn assert_update(docs: &str, yaml: &str, expected: &str) {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    let op = parse_operation(yaml, OperationKind::Update).expect("operation parses");
    match execute(&op, &graph).expect("update succeeds") {
        Outcome::Update { changes } => {
            let mut state = graph.export();
            for (key, markdown) in changes {
                state.insert(key.to_string(), markdown);
            }
            assert_str_eq!(expected, to_indoc(&state));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

fn wrapped(markdown: &str) -> String {
    let mut graph = Graph::new_with_options(MarkdownOptions {
        formatting: FormattingOptions {
            wrap_column: Some(20),
            ..Default::default()
        },
        ..Default::default()
    });
    graph.from_markdown("key".into(), markdown, MarkdownReader::new());
    graph.to_markdown(&"key".into())
}

fn assert_wrapping_is_stable(markdown: &str, expected: &str) {
    let once = wrapped(markdown);
    assert_str_eq!(expected, once);
    assert_str_eq!(once, wrapped(&once));
}

fn to_djot(markdown: &str) -> String {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), markdown, MarkdownReader::new());
    let tree = (&graph).collect(&Key::name("key"));
    let mut djot = Graph::new_with_options(FormatOptions::Djot(DjotOptions::default()));
    djot.build_key_from_iter(&Key::name("key"), tree.iter());
    djot.to_markdown(&"key".into())
}

fn djot_round_trip(djot: &str) -> String {
    let mut graph = Graph::new_with_options(FormatOptions::Djot(DjotOptions::default()));
    graph.insert_document("key".into(), djot.to_string());
    graph.to_markdown(&"key".into())
}

#[test]
fn div_block() {
    assert_round_trip(indoc! {r#"
        Before.

        <div align="center">
        *not emphasis*, [not a link](target)
        </div>

        After.
        "#});
}

#[test]
fn details_block() {
    assert_round_trip(indoc! {"
        <details>
        <summary>More</summary>

        Hidden **text**.

        </details>
        "});
}

#[test]
fn script_block_with_blank_lines() {
    assert_round_trip(indoc! {"
        <script>
        let first = 1;

        let second = 2;
        </script>

        After.
        "});
}

#[test]
fn processing_instruction_block() {
    assert_round_trip(indoc! {"
        <?xml version=\"1.0\"?>

        After.
        "});
}

#[test]
fn declaration_block() {
    assert_round_trip(indoc! {"
        <!DOCTYPE html>

        After.
        "});
}

#[test]
fn cdata_block() {
    assert_round_trip(indoc! {"
        <![CDATA[ raw ]]>

        After.
        "});
}

#[test]
fn custom_tag_block() {
    assert_round_trip(indoc! {"
        <my-widget>

        After.
        "});
}

#[test]
fn html_block_after_a_paragraph_gets_a_blank_line() {
    let markdown = indoc! {"
        Para.
        <div>block</div>
        "};
    assert_str_eq!(
        indoc! {"
            Para.

            <div>block</div>
            "},
        round_trip(markdown)
    );
}

#[test]
fn html_block_in_a_list_item() {
    assert_round_trip(indoc! {"
        - first

          <div>block</div>

        - second
        "});
}

#[test]
fn html_block_in_a_quote() {
    assert_round_trip(indoc! {"
        > quoted
        >
        > <div>block</div>
        "});
}

#[test]
fn html_block_in_a_section_does_not_start_a_section() {
    let markdown = indoc! {"
        # Title

        <div>block</div>

        Body.
        "};
    assert_round_trip(markdown);
    assert_eq!(
        top_level_nodes(markdown).len(),
        1,
        "the html block and the paragraph stay inside the section"
    );
}

#[test]
fn wrapper_pattern_is_three_sibling_blocks() {
    let markdown = indoc! {r#"
        <div align="center">

        Centered *text*.

        </div>
        "#};
    assert_round_trip(markdown);
    assert_eq!(
        top_level_nodes(markdown),
        vec![
            Node::Html(r#"<div align="center">"#.to_string()),
            Node::Leaf(vec![
                Inline::Str("Centered".to_string()),
                Inline::Space,
                Inline::Emph(vec![Inline::Str("text".to_string())]),
                Inline::Str(".".to_string()),
            ]),
            Node::Html("</div>".to_string()),
        ]
    );
}

#[test]
fn comment_followed_by_text_is_one_html_block() {
    let markdown = indoc! {"
        <!-- note --> more *text*
        "};
    assert_round_trip(markdown);
    assert_eq!(
        top_level_nodes(markdown),
        vec![Node::Html("<!-- note --> more *text*".to_string())]
    );
}

#[test]
fn comment_line_after_an_html_block_keeps_its_blank_line() {
    assert_round_trip(indoc! {"
        <div>block</div>

        <!-- note -->
        "});
}

#[test]
fn inline_html_is_kept() {
    assert_round_trip(indoc! {"
        Press <kbd>Ctrl</kbd> and x<sup>2</sup>.
        "});
}

#[test]
fn escaped_tag_stays_escaped() {
    assert_round_trip(indoc! {r"
        Write \<em> to start emphasis.
        "});
}

#[test]
fn less_than_sign_is_not_escaped() {
    assert_round_trip(indoc! {"
        When a < b and 1 <2.
        "});
}

#[test]
fn text_that_is_not_a_tag_is_not_escaped() {
    assert_round_trip(indoc! {"
        <some_name>

        Use <a=b> or </ x>.
        "});
}

#[test]
fn wrapping_never_starts_a_line_with_an_inline_comment() {
    let markdown = indoc! {"
        Some words here and <!-- a long comment here --> and more words after.
        "};
    assert_wrapping_is_stable(
        markdown,
        indoc! {"
            Some words here and <!-- a long comment here -->
            and more words
            after.
            "},
    );
    assert_eq!(
        selected(&wrapped(markdown), comments()),
        vec!["paragraph: Some words here and and more words after."]
    );
}

#[test]
fn wrapping_never_starts_a_line_with_inline_html() {
    assert_wrapping_is_stable(
        indoc! {r#"
            Some words here and <span class="a b">styled</span> words after.
            "#},
        indoc! {r#"
            Some words here and <span class="a b">styled</span>
            words after.
            "#},
    );
    assert_wrapping_is_stable(
        indoc! {"
            Some words here and <div>x</div> words after.
            "},
        indoc! {"
            Some words here and <div>x</div>
            words after.
            "},
    );
}

#[test]
fn wrapping_still_escapes_text_that_looks_like_a_tag() {
    assert_wrapping_is_stable(
        indoc! {r"
            Some words here and \<div words after.
            "},
        indoc! {r"
            Some words here and
            \<div words after.
            "},
    );
}

#[test]
fn html_predicate_selects_html_blocks_by_source() {
    let markdown = indoc! {"
        <details>
        <summary>More</summary>
        </details>

        Para.

        <div>block</div>
        "};
    assert_eq!(
        selected(markdown, html_blocks()),
        vec![
            "html: <details>\n<summary>More</summary>\n</details>",
            "html: <div>block</div>"
        ]
    );
    assert_eq!(
        selected(markdown, html(text("summary"))),
        vec!["html: <details>\n<summary>More</summary>\n</details>"]
    );
    assert_eq!(
        selected(markdown, html("<div>block</div>")),
        vec!["html: <div>block</div>"]
    );
}

#[test]
fn text_predicates_match_html_source() {
    let markdown = indoc! {"
        <div>block</div>

        Para.
        "};
    assert_eq!(
        selected(markdown, text("block")),
        vec!["html: <div>block</div>"]
    );
}

#[test]
fn comment_predicate_does_not_look_inside_html_blocks() {
    let markdown = indoc! {"
        <div><!-- note --></div>

        <!-- note --> more text
        "};
    assert_eq!(selected(markdown, comments()), Vec::<String>::new());
}

#[test]
fn comment_line_after_an_html_block_belongs_to_it() {
    let markdown = indoc! {"
        <div>block</div>

        <!-- note -->
        "};
    assert_eq!(
        selected(markdown, comments()),
        vec!["html: <div>block</div>"]
    );
}

#[test]
fn paragraph_predicate_skips_html_blocks() {
    let markdown = indoc! {"
        <div>block</div>

        Para.
        "};
    assert_eq!(selected(markdown, paragraphs()), vec!["paragraph: Para."]);
}

#[test]
fn delete_html_block() {
    assert_update(
        indoc! {"
            # Doc

            <div>block</div>

            keep
        "},
        indoc! {"
            filter: {}
            update:
              $delete:
                $html: {}
        "},
        indoc! {"
            # Doc

            keep
        "},
    );
}

#[test]
fn replace_text_edits_html_source() {
    assert_update(
        indoc! {r#"
            # Doc

            <div align="left">block</div>
        "#},
        indoc! {r#"
            filter: {}
            update:
              $replaceText:
                $html: {}
                from: left
                to: center
                expect: 1
        "#},
        indoc! {r#"
            # Doc

            <div align="center">block</div>
        "#},
    );
}

#[test]
fn replace_html_block_with_markdown() {
    assert_update(
        indoc! {"
            # Doc

            <div>block</div>
        "},
        indoc! {"
            filter: {}
            update:
              $replace:
                $html: {}
                content: Plain text.
                expect: 1
        "},
        indoc! {"
            # Doc

            Plain text.
        "},
    );
}

#[test]
fn djot_writes_html_block_as_raw_block() {
    assert_str_eq!(
        indoc! {"
            ```=html
            <div>block</div>
            ```

            Para.
            "},
        to_djot(indoc! {"
            <div>block</div>

            Para.
            "})
    );
}

#[test]
fn djot_writes_inline_html_as_raw_inline() {
    assert_str_eq!(
        indoc! {"
            Press `<kbd>`{=html}Ctrl`</kbd>`{=html}.
            "},
        to_djot(indoc! {"
            Press <kbd>Ctrl</kbd>.
            "})
    );
}

#[test]
fn djot_raw_html_block_round_trips() {
    let djot = indoc! {"
        ```=html
        <div>block</div>
        ```

        Para.
        "};
    assert_str_eq!(djot, djot_round_trip(djot));
}

#[test]
fn djot_raw_html_inline_round_trips() {
    let djot = indoc! {"
        Press `<kbd>`{=html}Ctrl`</kbd>`{=html}.
        "};
    assert_str_eq!(djot, djot_round_trip(djot));
}
