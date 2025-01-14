#![allow(dead_code, unused_imports, unused_variables, deprecated)]

use std::u32;

use indoc::indoc;
use lsp_types::*;

use fixture::{action_kind, action_kinds, uri};

use crate::fixture::Fixture;

mod fixture;

#[test]
fn no_sub_sectins_to_extract() {
    assert_no_action(
        indoc! {"
            # test
            "},
        0,
    );

    assert_no_action(
        indoc! {"
            # test

            # test
            "},
        2,
    );

    assert_no_action(
        indoc! {"
            # test

            ## test
            "},
        2,
    );
}

#[test]
fn extract_sub_section_after_para_test() {
    assert_extracted(
        indoc! {"
            # test

            para

            ## test2
            "},
        0,
        indoc! {"
            # test

            para

            [test2](2)
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_sub_sections_test() {
    assert_extracted(
        indoc! {"
            # test

            ## test2
            "},
        0,
        indoc! {"
            # test

            [test2](2)
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
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.extract.subsections"),
                trigger_kind: None,
            },
        },
        vec![CodeActionOrCommand::CodeAction(CodeAction {
            title: "Extract sub-sections".to_string(),
            kind: action_kind("refactor.extract.subsections"),
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

fn assert_no_action(source: &str, line: u32) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.extract.subsections"),
                trigger_kind: None,
            },
        },
        vec![],
    )
}
