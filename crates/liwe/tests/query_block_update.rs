use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::{execute, parse_operation, OperationKind, Outcome};
use liwe::state::{from_indoc, to_indoc};
use pretty_assertions::assert_str_eq;

fn assert_update(docs: &str, yaml: &str, expected: &str) {
    let state = from_indoc(docs);
    let graph = Graph::import(&state, MarkdownOptions::default(), None);
    let op = parse_operation(yaml, OperationKind::Update).expect("operation parses");
    match execute(&op, &graph).expect("update succeeds") {
        Outcome::Update { changes } => {
            let mut new_state = graph.export();
            for (key, markdown) in changes {
                new_state.insert(key.to_string(), markdown);
            }
            assert_str_eq!(expected, to_indoc(&new_state));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

fn update_err(docs: &str, yaml: &str) -> String {
    let state = from_indoc(docs);
    let graph = Graph::import(&state, MarkdownOptions::default(), None);
    let op = parse_operation(yaml, OperationKind::Update).expect("operation parses");
    match execute(&op, &graph) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("expected evaluation error"),
    }
}

fn parse_err(yaml: &str) -> String {
    parse_operation(yaml, OperationKind::Update)
        .expect_err("expected parse error")
        .to_string()
}

#[test]
fn delete_paragraph() {
    assert_update(
        indoc! {"
            # Doc

            keep

            drop
        "},
        indoc! {"
            filter: {}
            update:
              $delete:
                $paragraph: { $text: drop }
        "},
        indoc! {"
            # Doc

            keep
        "},
    );
}

#[test]
fn delete_empty_predicate_clears_body() {
    assert_update(
        indoc! {"
            # Doc

            para
        "},
        indoc! {"
            filter: {}
            update:
              $delete: {}
        "},
        "",
    );
}

#[test]
fn replace_text_edits_header() {
    assert_update(
        indoc! {"
            # Doc

            ## Q3 Milestones

            Ship it
        "},
        indoc! {"
            filter: {}
            update:
              $replaceText:
                $header: Q3 Milestones
                from: Q3 Milestones
                to: Q3 2026 Milestones
                expect: 1
        "},
        indoc! {"
            # Doc

            ## Q3 2026 Milestones

            Ship it
        "},
    );
}

#[test]
fn replace_text_without_from_rewrites_whole_header() {
    assert_update(
        indoc! {"
            # Doc

            ## Goals

            Ship it
        "},
        indoc! {"
            filter: {}
            update:
              $replaceText:
                $header: Goals
                to: Aims
                expect: 1
        "},
        indoc! {"
            # Doc

            ## Aims

            Ship it
        "},
    );
}

#[test]
fn replace_text_without_from_rewrites_paragraph() {
    assert_update(
        indoc! {"
            # Doc

            The Q3 target is 100 units.
        "},
        indoc! {"
            filter: {}
            update:
              $replaceText:
                $paragraph: {}
                to: The Q4 target is 250 units.
        "},
        indoc! {"
            # Doc

            The Q4 target is 250 units.
        "},
    );
}

#[test]
fn replace_text_without_from_still_rejects_no_own_text() {
    let err = update_err(
        indoc! {"
            # Doc

            - a
            - b
        "},
        indoc! {"
            filter: {}
            update:
              $replaceText:
                $list: {}
                to: x
        "},
    );
    assert_str_eq!(
        err,
        indoc! {"
            $replaceText target has no editable own text
              1 › \"- a\""}
    );
}

#[test]
fn replace_block() {
    assert_update(
        indoc! {"
            # Doc

            old
        "},
        indoc! {"
            filter: {}
            update:
              $replace:
                $paragraph: old
                content: new
        "},
        indoc! {"
            # Doc

            new
        "},
    );
}

#[test]
fn insert_before_and_after() {
    assert_update(
        indoc! {"
            # Doc

            anchor
        "},
        indoc! {"
            filter: {}
            update:
              $insertBefore:
                $paragraph: anchor
                content: before
        "},
        indoc! {"
            # Doc

            before

            anchor
        "},
    );
}

#[test]
fn insert_after_sibling() {
    assert_update(
        indoc! {"
            # Doc

            anchor
        "},
        indoc! {"
            filter: {}
            update:
              $insertAfter:
                $paragraph: anchor
                content: after
        "},
        indoc! {"
            # Doc

            anchor

            after
        "},
    );
}

#[test]
fn delete_section_removes_subtree() {
    assert_update(
        indoc! {"
            # Doc

            ## Keep

            k

            ## Drop

            d
        "},
        indoc! {"
            filter: {}
            update:
              $delete:
                $section: Drop
        "},
        indoc! {"
            # Doc

            ## Keep

            k
        "},
    );
}

#[test]
fn append_items_to_list() {
    assert_update(
        indoc! {"
            # List

            - a
            - b
        "},
        indoc! {"
            filter: {}
            update:
              $append:
                $list: {}
                content: \"- c\"
        "},
        indoc! {"
            # List

            - a
            - b
            - c
        "},
    );
}

#[test]
fn expect_range_allows_bulk_delete() {
    assert_update(
        indoc! {"
            # Doc

            drop

            drop
        "},
        indoc! {"
            filter: {}
            update:
              $delete:
                $paragraph: { $text: drop }
                expect: { max: 20 }
        "},
        indoc! {"
            # Doc
        "},
    );
}

#[test]
fn append_under_header() {
    assert_update(
        indoc! {"
            # Status

            existing
        "},
        indoc! {"
            filter: {}
            update:
              $append:
                $header: Status
                content: Reviewed.
        "},
        indoc! {"
            # Status

            existing

            Reviewed.
        "},
    );
}

#[test]
fn insert_after_list_item() {
    assert_update(
        indoc! {"
            # List

            - a
            - b
        "},
        indoc! {"
            filter: {}
            update:
              $insertAfter:
                $item: a
                content: \"- x\"
        "},
        indoc! {"
            # List

            - a
            - x
            - b
        "},
    );
}

#[test]
fn set_and_block_op_combine() {
    assert_update(
        indoc! {"
            ---
            status: active
            ---
            # Status

            old
        "},
        indoc! {"
            filter: {}
            update:
              $set: { reviewed: true }
              $replaceText: { $paragraph: old, from: old, to: new }
        "},
        indoc! {"
            ---
            status: active
            reviewed: true
            ---

            # Status

            new
        "},
    );
}

#[test]
fn expect_violation_reports_selection() {
    let err = update_err(
        indoc! {"
            # Doc

            drop

            drop
        "},
        indoc! {"
            filter: {}
            update:
              $delete:
                $paragraph: { $text: drop }
                expect: 1
        "},
    );
    assert_str_eq!(
        err,
        indoc! {"
            $delete expects 1 block, selected 2
              1 › \"drop\"
              1 › \"drop\"
            hint: narrow with $within or $matches, or raise expect"}
    );
}

#[test]
fn overlap_is_rejected() {
    let err = update_err(
        indoc! {"
            # Section

            para
        "},
        indoc! {"
            filter: {}
            update:
              $delete:
                $section: Section
              $replaceText:
                $paragraph: para
                from: para
                to: gone
        "},
    );
    assert_str_eq!(
        err,
        indoc! {"
            overlapping selections: $delete and $replaceText both touch
              1 › \"Section\"
            hint: block operator extents must be disjoint"}
    );
}

#[test]
fn append_on_paragraph_is_rejected() {
    let err = update_err(
        indoc! {"
            # Doc

            para
        "},
        indoc! {"
            filter: {}
            update:
              $append:
                $paragraph: para
                content: child
        "},
    );
    assert_str_eq!(
        err,
        indoc! {"
            $append target is not a container (header, item, list, or quote)
              1 › \"para\""}
    );
}

#[test]
fn replace_text_missing_anchor_is_rejected() {
    let err = update_err(
        indoc! {"
            # Doc

            para
        "},
        indoc! {"
            filter: {}
            update:
              $replaceText:
                $paragraph: para
                from: absent
                to: x
        "},
    );
    assert_str_eq!(
        err,
        indoc! {"
            $replaceText 'from' must occur exactly once in the selected block
              1 › \"para\""}
    );
}

#[test]
fn unknown_payload_key_is_parse_error() {
    assert_str_eq!(
        parse_err(indoc! {"
            filter: {}
            update:
              $delete: { content: x }
        "}),
        "unknown key 'content' in '$delete'"
    );
}

#[test]
fn missing_payload_is_parse_error() {
    assert_str_eq!(
        parse_err(indoc! {"
            filter: {}
            update:
              $replace:
                $paragraph: x
        "}),
        "'$replace' requires the 'content' key"
    );
}

#[test]
fn block_op_in_find_is_rejected() {
    assert_str_eq!(
        parse_operation(
            indoc! {"
                filter: {}
                update:
                  $delete: {}
            "},
            OperationKind::Find,
        )
        .expect_err("find rejects update")
        .to_string(),
        "'find' does not support the 'update' field"
    );
}

#[test]
fn delete_header_node_dissolves_into_enclosing_section() {
    assert_update(
        indoc! {"
            # Roadmap

            ## Goals

            Ship the editor integration

            ### Q3 Milestones

            Deliver block operations spec
        "},
        indoc! {"
            filter: {}
            update:
              $delete:
                $header: Goals
                expect: 1
        "},
        indoc! {"
            # Roadmap

            Ship the editor integration

            ## Q3 Milestones

            Deliver block operations spec
        "},
    );
}

#[test]
fn replace_header_node_with_heading_retitles() {
    assert_update(
        indoc! {"
            # Roadmap

            ## Goals

            Ship the editor integration

            ### Q3 Milestones

            Deliver block operations spec
        "},
        indoc! {"
            filter: {}
            update:
              $replace:
                $header: Goals
                content: \"## Aims\"
                expect: 1
        "},
        indoc! {"
            # Roadmap

            ## Aims

            Ship the editor integration

            ### Q3 Milestones

            Deliver block operations spec
        "},
    );
}

#[test]
fn replace_header_node_with_non_heading_dissolves_around() {
    assert_update(
        indoc! {"
            # Roadmap

            ## Goals

            Ship the editor integration

            ### Q3 Milestones

            Deliver block operations spec
        "},
        indoc! {"
            filter: {}
            update:
              $replace:
                $header: Goals
                content: A short summary line.
                expect: 1
        "},
        indoc! {"
            # Roadmap

            A short summary line.

            Ship the editor integration

            ## Q3 Milestones

            Deliver block operations spec
        "},
    );
}

#[test]
fn delete_section_still_removes_the_whole_tree() {
    assert_update(
        indoc! {"
            # Roadmap

            ## Goals

            Ship the editor integration

            ### Q3 Milestones

            Deliver block operations spec
        "},
        indoc! {"
            filter: {}
            update:
              $delete:
                $section: Goals
                expect: 1
        "},
        indoc! {"
            # Roadmap
        "},
    );
}

#[test]
fn insert_after_header_node_lands_at_top_of_section() {
    assert_update(
        indoc! {"
            # Roadmap

            ## Goals

            Ship the editor integration
        "},
        indoc! {"
            filter: {}
            update:
              $insertAfter:
                $header: Goals
                content: A framing sentence.
        "},
        indoc! {"
            # Roadmap

            ## Goals

            A framing sentence.

            Ship the editor integration
        "},
    );
}

#[test]
fn insert_after_section_lands_below_the_whole_tree() {
    assert_update(
        indoc! {"
            # Roadmap

            ## Goals

            Ship the editor integration

            ## Later
        "},
        indoc! {"
            filter: {}
            update:
              $insertAfter:
                $section: Goals
                content: \"## Inserted\"
        "},
        indoc! {"
            # Roadmap

            ## Goals

            Ship the editor integration

            ## Inserted

            ## Later
        "},
    );
}

#[test]
fn delete_header_node_beside_replace_text_inside_is_legal() {
    assert_update(
        indoc! {"
            # Roadmap

            ## Goals

            Ship the editor integration
        "},
        indoc! {"
            filter: {}
            update:
              $delete:
                $header: Goals
              $replaceText:
                $within: Goals
                $paragraph: {}
                from: Ship
                to: Deliver
        "},
        indoc! {"
            # Roadmap

            Deliver the editor integration
        "},
    );
}

#[test]
fn delete_all_headers_dissolves_each() {
    assert_update(
        indoc! {"
            # Doc

            ## Chapter

            body one

            ### Part

            body two
        "},
        indoc! {"
            filter: {}
            update:
              $delete:
                $header: {}
                expect: 3
        "},
        indoc! {"
            body one

            body two
        "},
    );
}

#[test]
fn document_expect_passes_when_matched_count_matches() {
    assert_update(
        indoc! {"
            # Doc

            drop
        "},
        indoc! {"
            filter: {}
            expect: 1
            update:
              $delete:
                $paragraph: drop
        "},
        indoc! {"
            # Doc
        "},
    );
}

#[test]
fn document_expect_violation_aborts_and_lists_documents() {
    let err = update_err(
        indoc! {"
            # Alpha

            keep
            _
            # Beta

            keep
        "},
        indoc! {"
            filter: {}
            expect: 1
            update:
              $set: { reviewed: true }
        "},
    );
    assert_str_eq!(
        err,
        indoc! {"
            update expects 1 document, matched 2
              1 › Alpha
              2 › Beta
            hint: adjust the filter or raise expect"}
    );
}

#[test]
fn document_expect_counts_are_independent_of_block_expect() {
    let err = update_err(
        indoc! {"
            # Doc

            drop

            drop
        "},
        indoc! {"
            filter: {}
            expect: 2
            update:
              $delete:
                $paragraph: { $text: drop }
                expect: 2
        "},
    );
    assert_str_eq!(
        err,
        indoc! {"
            update expects 2 documents, matched 1
              1 › Doc
            hint: adjust the filter or raise expect"}
    );
}

#[test]
fn document_expect_in_find_is_parse_error() {
    assert_str_eq!(
        parse_operation(
            indoc! {"
                filter: {}
                expect: 1
            "},
            OperationKind::Find,
        )
        .expect_err("find rejects expect")
        .to_string(),
        "'find' does not support the 'expect' field"
    );
}

#[test]
fn document_expect_in_count_is_parse_error() {
    assert_str_eq!(
        parse_operation(
            indoc! {"
                filter: {}
                expect: 1
            "},
            OperationKind::Count,
        )
        .expect_err("count rejects expect")
        .to_string(),
        "'count' does not support the 'expect' field"
    );
}
