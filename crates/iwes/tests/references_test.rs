#![allow(dead_code, unused_imports, unused_variables, deprecated)]

use std::u32;

use indoc::indoc;
use itertools::Itertools;
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionOrCommand, CodeActionParams, CompletionItem,
    CompletionList, CompletionParams, CompletionResponse, CreateFile, CreateFileOptions,
    DocumentChangeOperation, DocumentChanges, DocumentFormattingParams, Documentation, InlayHint,
    InlayHintLabel, InlayHintParams, Location, OneOf, OptionalVersionedTextDocumentIdentifier,
    PartialResultParams, Position, Range, ReferenceContext, ReferenceParams, ResourceOp,
    SymbolInformation, SymbolKind, TextDocumentEdit, TextDocumentIdentifier,
    TextDocumentPositionParams, TextEdit, Url, WorkDoneProgressParams, WorkspaceSymbolParams,
    WorkspaceSymbolResponse,
};

use fixture::uri;

use crate::fixture::Fixture;

mod fixture;

#[test]
fn single_reference() {
    assert_extracted(
        indoc! {"
            # test
            _
            # header hint

            [test](1)
            "},
        vec![2],
    );
}

#[test]
fn two_references() {
    assert_extracted(
        indoc! {"
            # test
            _
            # header 1

            [test](1)
            _
            # header 2

            [test](1)
            "},
        vec![2, 3],
    );
}

#[test]
fn link() {
    assert_extracted(
        indoc! {"
            # test
            _
            # header 1

            text and link [test](1)
            "},
        vec![2],
    );
}

fn assert_extracted(source: &str, urls: Vec<u32>) {
    let fixture = Fixture::with(source);

    fixture.references(
        ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(0, 0),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
            context: ReferenceContext {
                include_declaration: false,
            },
        },
        urls.iter()
            .sorted()
            .map(|n| Location {
                uri: uri(*n),
                range: Range::new(Position::new(2, 0), Position::new(3, 0)),
            })
            .collect_vec(),
    )
}
