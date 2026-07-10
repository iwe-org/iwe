use crate::blocks::{
    any, header, headers, items, lists, matches, nor, or, paragraph, quotes, references, section,
    text, within, within_section,
};
use crate::queries::{blocks, content, field, filter, find, grep, key_eq};
use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::{
    execute, parse_operation, FindOp, OperationKind, Outcome, Projection, ProjectionSource,
};
use liwe::state::from_indoc;
use pretty_assertions::assert_eq;
use serde_yaml::Mapping;

fn run(docs: &str, op: FindOp) -> Vec<Mapping> {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    match execute(&find(op), &graph).expect("query succeeds") {
        Outcome::Find { matches, .. } => matches.into_iter().map(|m| m.document).collect(),
        other => panic!("expected Find, got {:?}", other),
    }
}

fn assert_results(docs: &str, op: FindOp, expected: &str) {
    let actual = run(docs, op);
    let expected: Vec<Mapping> = serde_yaml::from_str(expected).expect("expected parses");
    assert_eq!(actual, expected);
}

fn assert_fields(docs: &str, fields: Vec<(&str, ProjectionSource)>, expected: &str) {
    let fields = fields.into_iter().map(|(n, s)| field(n, s)).collect();
    assert_results(
        docs,
        filter(key_eq("1")).project(Projection::replace(fields)),
        expected,
    );
}

fn assert_projection(docs: &str, name: &str, source: ProjectionSource, expected: &str) {
    assert_fields(docs, vec![(name, source)], expected);
}

fn assert_yaml_results(docs: &str, yaml: &str, expected: &str) {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    let op = parse_operation(yaml, OperationKind::Find).expect("operation parses");
    let actual: Vec<Mapping> = match execute(&op, &graph).expect("query succeeds") {
        Outcome::Find { matches, .. } => matches.into_iter().map(|m| m.document).collect(),
        other => panic!("expected Find, got {:?}", other),
    };
    let expected: Vec<Mapping> = serde_yaml::from_str(expected).expect("expected parses");
    assert_eq!(actual, expected);
}

fn parse_err(yaml: &str) -> String {
    parse_operation(yaml, OperationKind::Find)
        .expect_err("expected parse error")
        .to_string()
}

#[test]
fn content_section_includes_header() {
    assert_projection(
        indoc! {"
            # header1

            paragraph1

            ## header2

            paragraph2

            ### header3

            paragraph3

            ## header4

            paragraph4
        "},
        "notes",
        content(section("header2")),
        indoc! {"
            - notes: |
                ## header2

                paragraph2

                ### header3

                paragraph3
        "},
    );
}

#[test]
fn content_within_excludes_header_and_keeps_depth() {
    assert_projection(
        indoc! {"
            # top1

            ## group1

            body1

            ### sub1

            body2
        "},
        "inner",
        content(within_section("group1")),
        indoc! {"
            - inner: |
                body1

                ### sub1

                body2
        "},
    );
}

#[test]
fn content_headers_render_outline() {
    assert_projection(
        indoc! {"
            # doc1

            text1

            ## chapter1

            text2

            ### part1

            text3

            ## chapter2

            text4
        "},
        "toc",
        content(headers()),
        indoc! {"
            - toc: |
                # doc1

                ## chapter1

                ### part1

                ## chapter2
        "},
    );
}

#[test]
fn content_keeps_original_depth_when_levels_skipped() {
    assert_projection(
        indoc! {"
            # alpha1

            ## beta1

            ### gamma1

            paragraph1
        "},
        "picked",
        content(or(vec![header("alpha1"), header("gamma1")])),
        indoc! {"
            - picked: |
                # alpha1

                ### gamma1
        "},
    );
}

#[test]
fn content_items_rewrap_into_lists() {
    assert_projection(
        indoc! {"
            # list1

            - item1
            - item2
              - item3
        "},
        "items",
        content(items()),
        indoc! {"
            - items: |
                - item1
                - item2
                  - item3
        "},
    );
}

#[test]
fn content_empty_selection_renders_empty_string() {
    assert_fields(
        indoc! {"
            # title1

            body1
        "},
        vec![
            ("none", content(section("missing1"))),
            ("hits", blocks(text("absent1"))),
        ],
        indoc! {"
            - none: ''
              hits: []
        "},
    );
}

#[test]
fn blocks_locate_paragraph_with_path() {
    assert_projection(
        indoc! {"
            # root1

            ## scope1

            para1

            ### scope2

            para2 marker1
        "},
        "hits",
        blocks(within_section("scope1").text("marker1")),
        indoc! {"
            - hits:
                - type: paragraph
                  path: [scope1, scope2]
                  text: para2 marker1
        "},
    );
}

#[test]
fn blocks_header_composition_selects_header_alone() {
    assert_projection(
        indoc! {"
            # root2

            ## outer1

            text1

            ### inner1

            text2
        "},
        "hits",
        blocks(header("inner1").within_section("outer1")),
        indoc! {"
            - hits:
                - type: header
                  path: [outer1]
                  text: inner1
        "},
    );
}

#[test]
fn blocks_contains_selects_headers_by_descendants() {
    assert_projection(
        indoc! {"
            # trunk1

            ## branch1

            ### branch2

            flag1

            ## branch3

            plain1
        "},
        "hits",
        blocks(headers().contains(matches("flag[0-9]"))),
        indoc! {"
            - hits:
                - type: header
                  path: []
                  text: trunk1
                - type: header
                  path: []
                  text: branch1
                - type: header
                  path: [branch1]
                  text: branch2
        "},
    );
}

#[test]
fn blocks_every_type_reported() {
    assert_projection(
        indoc! {"
            # mixed1

            - entry1
            - entry2
              - entry3

            > cited1

            ``` sh
            code1
            code2
            ```

            | h1 | h2 |
            |----|----|
            | v1 | v2 |

            See [linked1](2) for details.

            [linked1](2)
            _
            # linked1

            body1
        "},
        "hits",
        blocks(any()),
        indoc! {"
            - hits:
                - type: header
                  path: []
                  text: mixed1
                - type: list
                  path: []
                  text: ''
                - type: item
                  path: []
                  text: entry1
                - type: item
                  path: []
                  text: entry2
                - type: list
                  path: []
                  text: ''
                - type: item
                  path: []
                  text: entry3
                - type: quote
                  path: []
                  text: ''
                - type: paragraph
                  path: []
                  text: cited1
                - type: code
                  path: []
                  text: |-
                    code1
                    code2
                - type: table
                  path: []
                  text: |-
                    | h1  | h2  |
                    | v1  | v2  |
                - type: paragraph
                  path: []
                  text: See [linked1](2) for details.
                - type: ref
                  path: []
                  target: '2'
                  text: ''
        "},
    );
}

#[test]
fn blocks_paragraph_by_reference() {
    assert_projection(
        indoc! {"
            # source1

            See [other1](2) here.

            [other1](2)
            _
            # other1
        "},
        "hits",
        blocks(paragraph(references("2"))),
        indoc! {"
            - hits:
                - type: paragraph
                  path: []
                  text: See [other1](2) here.
        "},
    );
}

#[test]
fn grep_finds_lines_across_blocks() {
    assert_projection(
        indoc! {"
            # notes1

            - point1 mark1
            - point2

            ``` sh
            line1
            mark1 line2
            ```
        "},
        "found",
        grep("mark1", any()),
        indoc! {"
            - found:
                - path: []
                  text: point1 mark1
                - path: []
                  text: mark1 line2
        "},
    );
}

#[test]
fn grep_with_scope() {
    assert_projection(
        indoc! {"
            # ledger1

            hit1 outside

            ## bounded1

            hit1 inside
        "},
        "found",
        grep("hit1", within_section("bounded1")),
        indoc! {"
            - found:
                - path: [bounded1]
                  text: hit1 inside
        "},
    );
}

#[test]
fn add_fields_extends_default_projection() {
    assert_results(
        indoc! {"
            # manual1

            ## usage1

            body1
        "},
        FindOp::new().project(Projection::extend(vec![field(
            "hits",
            blocks(header("usage1")),
        )])),
        indoc! {"
            - key: '1'
              title: manual1
              references: []
              includes: []
              referencedBy: []
              includedBy: []
              hits:
                - type: header
                  path: []
                  text: usage1
        "},
    );
}

#[test]
fn content_list_preserves_checkbox_marker() {
    assert_fields(
        indoc! {"
            # board1

            - [x] task1
            - [ ] task2
        "},
        vec![("body", content(lists())), ("hits", blocks(items()))],
        indoc! {"
            - body: |
                - [x] task1
                - [ ] task2
              hits:
                - type: item
                  path: []
                  text: task1
                - type: item
                  path: []
                  text: task2
        "},
    );
}

#[test]
fn content_quote_and_within_peels_wrapper() {
    assert_fields(
        indoc! {"
            # digest1

            > excerpt1
        "},
        vec![
            ("quoted", content(quotes())),
            ("peeled", content(within(quotes()))),
        ],
        indoc! {"
            - quoted: |
                > excerpt1
              peeled: |
                excerpt1
        "},
    );
}

#[test]
fn within_empty_predicate_is_document_interior() {
    assert_projection(
        indoc! {"
            # crown1

            body1

            ## nested1

            body2
        "},
        "top",
        blocks(nor(vec![within(any())])),
        indoc! {"
            - top:
                - type: header
                  path: []
                  text: crown1
        "},
    );
}

#[test]
fn content_empty_predicate_is_full_body() {
    assert_projection(
        indoc! {"
            # whole1

            lead1

            ## part1

            lead2
        "},
        "body",
        content(any()),
        indoc! {"
            - body: |
                # whole1

                lead1

                ## part1

                lead2
        "},
    );
}

#[test]
fn top_level_predicate_projects_headers_form() {
    assert_yaml_results(
        indoc! {"
            # doc1

            text1

            ## chapter1

            text2

            ### part1

            text3
        "},
        "project: { $header: {} }",
        indoc! {"
            - key: '1'
              content: |
                # doc1

                ## chapter1

                ### part1
        "},
    );
}

#[test]
fn top_level_predicate_conjoins_operators() {
    assert_yaml_results(
        indoc! {"
            # doc1

            ## chapter1

            ### part1

            text1

            ## chapter2

            ### part2
        "},
        "project: { $header: {}, $within: chapter1 }",
        indoc! {"
            - key: '1'
              content: |
                ### part1
        "},
    );
}

#[test]
fn parse_error_top_level_mixed_project() {
    assert_eq!(
        parse_err("project: { key: $key, $header: {} }"),
        "bare key 'key' is not allowed in a block predicate"
    );
}

#[test]
fn parse_error_top_level_projection_source() {
    assert_eq!(
        parse_err("project: { $blocks: {} }"),
        "unknown block operator '$blocks'"
    );
}

#[test]
fn parse_error_unknown_block_operator() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $bogus: 1 } } }"),
        "unknown block operator '$bogus'"
    );
}

#[test]
fn parse_error_bare_key_in_block_predicate() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { status: draft } } }"),
        "bare key 'status' is not allowed in a block predicate"
    );
}

#[test]
fn parse_error_ref_scalar() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $ref: Title } } }"),
        "'$ref' does not accept a scalar shorthand"
    );
}

#[test]
fn parse_error_quote_text_predicate() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $quote: { $text: x } } } }"),
        "'$quote' argument does not accept text predicates ($text, $matches)"
    );
}

#[test]
fn parse_error_list_scalar() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $list: items } } }"),
        "'$list' does not accept a scalar shorthand"
    );
}

#[test]
fn parse_error_hr_text_predicate() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $hr: { $matches: x } } } }"),
        "'$hr' argument does not accept text predicates ($text, $matches)"
    );
}

#[test]
fn parse_error_matches_missing_pattern() {
    assert_eq!(
        parse_err("project: { found: { $matches: { $within: Goals } } }"),
        "'$matches' mapping requires a 'pattern' key"
    );
}

#[test]
fn parse_error_within_node_operator_argument() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $within: { $header: {} } } } }"),
        "'$within' expects a content-carrying argument: {}, or a predicate containing $section, $quote, or $list"
    );
}

#[test]
fn parse_error_within_typed_header_argument() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $within: { $header: title1 } } } }"),
        "'$within' expects a content-carrying argument: {}, or a predicate containing $section, $quote, or $list"
    );
}

#[test]
fn parse_error_within_text_argument() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $within: { $text: marker1 } } } }"),
        "'$within' expects a content-carrying argument: {}, or a predicate containing $section, $quote, or $list"
    );
}

#[test]
fn parse_error_within_contains_argument() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $within: { $contains: { $text: marker1 } } } } }"),
        "'$within' expects a content-carrying argument: {}, or a predicate containing $section, $quote, or $list"
    );
}

#[test]
fn parse_error_within_nor_argument() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $within: { $nor: [{ $section: chapter1 }] } } } }"),
        "'$within' expects a content-carrying argument: {}, or a predicate containing $section, $quote, or $list"
    );
}

#[test]
fn parse_error_within_or_mixed_branches() {
    assert_eq!(
        parse_err(
            "project: { hits: { $blocks: { $within: { $or: [{ $section: chapter1 }, { $header: {} }] } } } }"
        ),
        "'$within' expects a content-carrying argument: {}, or a predicate containing $section, $quote, or $list"
    );
}

#[test]
fn within_or_of_sections_unions_interiors() {
    assert_projection(
        indoc! {"
            # top1

            ## chapter1

            body1

            ## chapter2

            body2

            ## chapter3

            body3
        "},
        "hits",
        blocks(within(or(vec![section("chapter1"), section("chapter2")]))),
        indoc! {"
            - hits:
                - type: paragraph
                  path: [chapter1]
                  text: body1
                - type: paragraph
                  path: [chapter2]
                  text: body2
        "},
    );
}

#[test]
fn within_chained_section_argument_parses() {
    assert_yaml_results(
        indoc! {"
            # root1

            ## outer1

            ### inner1

            deep1

            ## inner1

            shallow1
        "},
        indoc! {"
            project:
              hits: { $blocks: { $within: { $section: { $text: { $eq: inner1 }, $within: outer1 } } } }
        "},
        indoc! {"
            - hits:
                - type: paragraph
                  path: [outer1, inner1]
                  text: deep1
        "},
    );
}

#[test]
fn parse_error_empty_or() {
    assert_eq!(
        parse_err("project: { hits: { $blocks: { $or: [] } } }"),
        "'$or' requires a non-empty list"
    );
}

#[test]
fn parse_error_content_scalar_argument() {
    assert_eq!(
        parse_err("project: { notes: { $content: Goals } }"),
        "'$content' expects a mapping"
    );
}

#[test]
fn parse_error_unknown_parameterized_source() {
    assert_eq!(
        parse_err("project: { x: { $bogus: {} } }"),
        "unknown projection source '$bogus'"
    );
}

#[test]
fn parse_error_invalid_regex() {
    let err = parse_err("project: { found: { $matches: '([' } }");
    assert_eq!(
        err,
        indoc! {"
            invalid regex '([': regex parse error:
                ([
                 ^
            error: unclosed character class"}
    );
}
