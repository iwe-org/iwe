use std::u32;

use indoc::indoc;
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionParams, CreateFile, CreateFileOptions,
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    Position, Range, ResourceOp, TextDocumentEdit, TextDocumentIdentifier, TextEdit,
};

use fixture::{action_kind, action_kinds, uri};

use crate::fixture::Fixture;

mod fixture;

#[test]
#[ignore]
fn extract_list_test() {
    assert_extracted(
        indoc! {"
            - test
            "},
        0,
        "[test](2)\n",
        "# test\n",
    );
}

#[test]
#[ignore]
fn extract_sub_list_test() {
    assert_extracted(
        indoc! {"
            # test

            - test2
            "},
        2,
        indoc! {"
            # test

            [test2](2)
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
#[ignore]
fn extract_middle_list_test() {
    assert_extracted(
        indoc! {"
            # test

            - test2

            # test3
            "},
        2,
        indoc! {"
            # test

            [test2](2)

            # test3
            "},
        indoc! {"
            # test2
        "},
    );
}

fn assert_extracted(source: &str, line: u32, target: &str, extracted: &str) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.extract.list"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "Extract list".to_string(),
            kind: action_kind("refactor.extract.list"),
            edit: Some(lsp_types::WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(vec![
                    DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                        uri: uri(2),
                        options: Some(CreateFileOptions {
                            overwrite: Some(false),
                            ignore_if_exists: Some(false),
                        }),
                        annotation_id: None,
                    })),
                    DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: uri(2),
                            version: None,
                        },
                        edits: vec![OneOf::Left(TextEdit {
                            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
                            new_text: extracted.to_string(),
                        })],
                    }),
                    DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: uri(1),
                            version: None,
                        },
                        edits: vec![OneOf::Left(TextEdit {
                            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
                            new_text: target.to_string(),
                        })],
                    }),
                ])),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
}
