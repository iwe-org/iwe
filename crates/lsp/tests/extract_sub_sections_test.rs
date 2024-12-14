#![allow(dead_code, unused_imports, unused_variables, deprecated)]

use std::u32;

use indoc::indoc;
use lsp_types::*;

use fixture::uri;

use crate::fixture::Fixture;

mod fixture;

#[test]
fn extract_section_test() {
    assert_extracted(
        indoc! {"
            # test
            "},
        0,
        "[test](2)\n",
        "# test\n",
    );
}

#[test]
fn extract_sub_section_test() {
    assert_extracted(
        indoc! {"
            # test

            # test2
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
fn extract_middle_section_test() {
    assert_extracted(
        indoc! {"
            # test

            # test2

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

#[test]
fn extract_sub_level_section_test() {
    assert_extracted(
        indoc! {"
            # test

            ## test2
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
fn extract_one_of_sub_level_section() {
    assert_extracted(
        indoc! {"
            # test

            para

            ## test2

            - item

            ## test3

            - item
            "},
        4,
        indoc! {"
            # test

            para

            [test2](2)

            ## test3

            - item
            "},
        indoc! {"
            # test2

            - item
        "},
    );
}

#[test]
fn extract_after_list() {
    assert_extracted(
        indoc! {"
            # test

            - item1

            # test2

            - item2
            "},
        4,
        indoc! {"
            # test

            - item1

            [test2](2)
            "},
        indoc! {"
            # test2

            - item2
        "},
    );
}

fn assert_extracted(source: &str, line: u32, target: &str, extracted: &str) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: Some(vec![lsp_types::CodeActionKind::REFACTOR_EXTRACT]),
                trigger_kind: None,
            },
        },
        vec![CodeActionOrCommand::CodeAction(CodeAction {
            title: "Extract section".to_string(),
            kind: Some(lsp_types::CodeActionKind::REFACTOR_EXTRACT),
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
        })],
    )
}
