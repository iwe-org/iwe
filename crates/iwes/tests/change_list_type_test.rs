#![allow(dead_code, unused_imports, unused_variables, deprecated)]

use std::u32;

use indoc::indoc;
use liwe::model::Title;
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionKind, CodeActionOrCommand, CodeActionParams,
    CompletionItem, CompletionList, CompletionParams, CompletionResponse, CreateFile,
    CreateFileOptions, DocumentChangeOperation, DocumentChanges, Documentation, OneOf,
    OptionalVersionedTextDocumentIdentifier, PartialResultParams, Position, Range, ResourceOp,
    SymbolInformation, SymbolKind, TextDocumentEdit, TextDocumentIdentifier,
    TextDocumentPositionParams, TextEdit, Url, WorkDoneProgressParams, WorkspaceEdit,
    WorkspaceSymbolParams, WorkspaceSymbolResponse,
};

use fixture::{action_kind, action_kinds, uri};

use crate::fixture::Fixture;

mod fixture;

#[test]
fn change_to_ordered() {
    assert_list_change(
        indoc! {"
            - test
            - test2
            "},
        0,
        indoc! {"
            1.  test
            2.  test2
        "},
        "Change to ordered list",
    );
}

#[test]
fn change_to_bullet() {
    assert_list_change(
        indoc! {"
            1.  test
            2.  test2
            "},
        0,
        indoc! {"
            - test
            - test2
        "},
        "Change to bullet list",
    );
}

fn assert_list_change(source: &str, line: u32, expected: &str, title: &str) {
    let fixture = Fixture::with(source);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("refactor.rewrite.list.type"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        vec![CodeActionOrCommand::CodeAction(CodeAction {
            title: title.to_string(),
            kind: action_kind("refactor.rewrite.list.type"),
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
        })],
    )
}
