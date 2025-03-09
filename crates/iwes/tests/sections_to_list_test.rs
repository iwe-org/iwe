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
fn wrap_single_section() {
    assert_list(
        indoc! {"
            # test
            "},
        0,
        "- test\n",
    );
}

#[test]
fn wrap_parent_section() {
    assert_list(
        indoc! {"
            # test

            ## test2
            "},
        0,
        indoc! {"
            - test
              # test2
        "},
    );
}

#[test]
fn wrap_section_with_para() {
    assert_list(
        indoc! {"
            # test

            test2
            "},
        0,
        indoc! {"
            - test

              test2
        "},
    );
}

#[test]
fn wrap_nested_section() {
    assert_list(
        indoc! {"
            # test

            ## test2

            "},
        2,
        indoc! {"
            # test

            - test2
        "},
    );
}

#[test]
fn wrap_list_after_para_test() {
    assert_list(
        indoc! {"
            para

            # test
            "},
        2,
        indoc! {"
            para

            - test
        "},
    );
}

#[test]
fn wrap_list_after_para_with_para_test() {
    assert_list(
        indoc! {"
            para

            # test

            para2
            "},
        2,
        indoc! {"
            para

            - test

              para2
        "},
    );
}

#[test]
fn wrap_list_something() {
    assert_list(
        indoc! {"
            # test1

            para

            ## test2

            para2
            "},
        4,
        indoc! {"
            # test1

            para

            - test2

              para2
        "},
    );
}

fn assert_list(source: &str, line: u32, expected: &str) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.rewrite.section.list"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "Section to list".to_string(),
            kind: action_kind("refactor.rewrite.section.list"),
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
