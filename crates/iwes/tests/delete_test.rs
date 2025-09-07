use std::u32;

use indoc::indoc;
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionParams, DeleteFile, DocumentChangeOperation,
    DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Position, Range, ResourceOp,
    TextDocumentEdit, TextDocumentIdentifier, TextEdit, WorkspaceEdit,
};

use fixture::{action_kind, action_kinds, uri};

use crate::fixture::Fixture;

mod fixture;

#[test]
fn delete_block_reference_no_other_references() {
    assert_deleted(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b

            some content
        "},
        2,
        vec![(
            1,
            indoc! {"
                # title a
            "},
        )],
    );
}

#[test]
fn delete_multiple_block_references() {
    assert_deleted(
        indoc! {"
            # title a

            [title b](2)

            [title b](2)
            _
            # title b

            some content
        "},
        2,
        vec![(
            1,
            indoc! {"
                # title a
            "},
        )],
    );
}

#[test]
fn delete_updates_other_files() {
    assert_deleted(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b

            some content
            _
            # title c

            [title b](2)
        "},
        2,
        vec![
            (
                1,
                indoc! {"
                # title a
            "},
            ),
            (
                3,
                indoc! {"
                # title c
            "},
            ),
        ],
    );
}

#[test]
fn delete_updates_inline_references() {
    assert_deleted(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b

            some content
            _
            # title c

            inline link to [title b](2)
        "},
        2,
        vec![
            (
                1,
                indoc! {"
                # title a
            "},
            ),
            (
                3,
                indoc! {"
                # title c

                inline link to title b
            "},
            ),
        ],
    );
}

#[test]
fn delete_updates_all_references() {
    assert_deleted(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b

            some content
            _
            # title c

            [title b](2)

            ## subtitle

            [title b](2)

            inline link to [title b](2)

            inline link to [title b](2) 2
        "},
        2,
        vec![
            (
                1,
                indoc! {"
                # title a
            "},
            ),
            (
                3,
                indoc! {"
                # title c

                ## subtitle

                inline link to title b

                inline link to title b 2
            "},
            ),
        ],
    );
}

#[test]
fn delete_non_block_reference_no_action() {
    assert_no_delete_action(
        indoc! {"
            # title a

            Some regular content here.
        "},
        0,
    );
}

fn assert_deleted(source: &str, line: u32, expected_edits: Vec<(u32, &str)>) {
    let fixture = Fixture::with(source);

    let mut operations = vec![DocumentChangeOperation::Op(ResourceOp::Delete(
        DeleteFile {
            uri: uri(2),
            options: None,
        },
    ))];

    for (uri_num, expected_text) in expected_edits {
        operations.push(DocumentChangeOperation::Edit(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri(uri_num),
                version: None,
            },
            edits: vec![OneOf::Left(TextEdit {
                range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
                new_text: expected_text.to_string(),
            })],
        }));
    }

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.delete"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "Delete".to_string(),
            kind: action_kind("refactor.delete"),
            edit: Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(operations)),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
}

fn assert_no_delete_action(source: &str, line: u32) {
    let fixture = Fixture::with(source);

    fixture.no_code_action(CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        range: Range::new(Position::new(line, 0), Position::new(line, 0)),
        context: CodeActionContext {
            diagnostics: Default::default(),
            only: action_kinds("refactor.delete"),
            trigger_kind: None,
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    })
}
