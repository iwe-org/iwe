use std::u32;

use indoc::indoc;
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionParams, DocumentChangeOperation, DocumentChanges,
    OneOf, OptionalVersionedTextDocumentIdentifier, Position, Range, TextDocumentEdit,
    TextDocumentIdentifier, TextEdit, WorkspaceEdit,
};

use fixture::{action_kind, action_kinds, uri};

use crate::fixture::Fixture;

mod fixture;

#[test]
fn unwrap_single_item_list_test() {
    assert_sections(
        indoc! {"
            - test
            "},
        0,
        "# test\n",
    );
}

#[test]
fn unwrap_list_with_items_test() {
    assert_sections(
        indoc! {"
            - test
              - test2
            "},
        0,
        indoc! {"
            # test

            - test2
        "},
    );
}

#[test]
fn unwrap_list_takes_top_level_list() {
    assert_sections(
        indoc! {"
            - test
              - test2
            "},
        1,
        indoc! {"
            # test

            - test2
        "},
    );
}

#[test]
fn unwrap_list_after_para_test() {
    assert_sections(
        indoc! {"
            para

            - test
            "},
        2,
        indoc! {"
            para

            # test
        "},
    );
}

#[test]
fn unwrap_list_between_para_and_para_test() {
    assert_sections(
        indoc! {"
            para

            - test

            para2
            "},
        2,
        indoc! {"
            para

            # test

            para2
        "},
    );
}

#[test]
fn unwrap_list_with_items_after_para_test() {
    assert_sections(
        indoc! {"
            para

            - test
              - test2
            "},
        2,
        indoc! {"
            para

            # test

            - test2
        "},
    );
}

#[test]
fn unwrap_sub_list_test() {
    assert_sections(
        indoc! {"
            # test

            - test2
            "},
        2,
        indoc! {"
            # test

            ## test2
        "},
    );
}

#[test]
fn unwrap_middle_list_test() {
    assert_sections(
        indoc! {"
            # test

            - test2

            # test3
            "},
        2,
        indoc! {"
            # test

            ## test2

            # test3
        "},
    );
}

#[test]
fn unwrap_list_prior_to_level_two_header_test() {
    assert_sections(
        indoc! {"
            # test

            - test2

            ## test3
            "},
        2,
        indoc! {"
            # test

            ## test2

            ## test3
        "},
    );
}

fn assert_sections(source: &str, line: u32, expected: &str) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.rewrite.list.section"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "List to sections".to_string(),
            kind: action_kind("refactor.rewrite.list.section"),
            edit: Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(vec![
                    DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: uri(1),
                            version: None,
                        },
                        edits: vec![OneOf::Left(TextEdit {
                            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
                            new_text: expected.to_string(),
                        })],
                    }),
                ])),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
}
