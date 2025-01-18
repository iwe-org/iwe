#![allow(dead_code, unused_imports, unused_variables, deprecated)]

use indoc::indoc;
use lsp_types::{
    DidChangeTextDocumentParams, Position, Range, SymbolInformation,
    TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier, WorkspaceSymbolParams,
    WorkspaceSymbolResponse,
};

use fixture::uri;

use crate::fixture::Fixture;

mod fixture;

#[test]
fn did_change_test_once() {
    let fixture = Fixture::with(indoc! {"
            # test
            "});

    fixture.did_change_text_document(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri(1),
            version: 1,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "# updated".to_string(),
        }],
    });

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: "".to_string(),
        },
        WorkspaceSymbolResponse::Flat(vec![SymbolInformation {
            kind: lsp_types::SymbolKind::OBJECT,
            location: lsp_types::Location {
                uri: uri(1),
                range: Range::new(Position::new(0, 0), Position::new(1, 0)),
            },
            name: "updated".to_string(),
            container_name: None,
            tags: None,
            deprecated: None,
        }]),
    );
}

#[test]
fn new_file() {
    let fixture = Fixture::new();

    fixture.did_change_text_document(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri(2),
            version: 1,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "# test".to_string(),
        }],
    });

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: "".to_string(),
        },
        WorkspaceSymbolResponse::Flat(vec![SymbolInformation {
            kind: lsp_types::SymbolKind::OBJECT,
            location: lsp_types::Location {
                uri: uri(2),
                range: Range::new(Position::new(0, 0), Position::new(1, 0)),
            },
            name: "test".to_string(),
            container_name: None,
            tags: None,
            deprecated: None,
        }]),
    );
}

#[test]
fn did_change_test_two_times() {
    let fixture = Fixture::with(indoc! {"
            # test
            "});

    fixture.did_change_text_document(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri(1),
            version: 1,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "# updated".to_string(),
        }],
    });

    fixture.did_change_text_document(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri(1),
            version: 1,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "# updated again".to_string(),
        }],
    });

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: "".to_string(),
        },
        WorkspaceSymbolResponse::Flat(vec![SymbolInformation {
            kind: lsp_types::SymbolKind::OBJECT,
            location: lsp_types::Location {
                uri: uri(1),
                range: Range::new(Position::new(0, 0), Position::new(1, 0)),
            },
            name: "updated again".to_string(),
            container_name: None,
            tags: None,
            deprecated: None,
        }]),
    );
}
