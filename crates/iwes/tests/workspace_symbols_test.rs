use indoc::indoc;
use lsp_types::{
    Position, Range, SymbolInformation, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};

use fixture::uri;

use crate::fixture::Fixture;

mod fixture;

#[test]
fn one_file() {
    let fixture = Fixture::with(indoc! {"
            # test
            "});

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: "".to_string(),
        },
        WorkspaceSymbolResponse::Flat(vec![SymbolInformation {
            name: "test".to_string(),
            kind: lsp_types::SymbolKind::OBJECT,
            location: lsp_types::Location {
                uri: uri(1),
                range: Range::new(Position::new(0, 0), Position::new(1, 0)),
            },
            container_name: None,
            tags: None,
            deprecated: None,
        }]),
    );
}

#[test]
fn one_file_two_headers() {
    let fixture = Fixture::with(indoc! {"
            # test

            ## test 2
            "});

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: "".to_string(),
        },
        WorkspaceSymbolResponse::Flat(vec![
            SymbolInformation {
                name: "test".to_string(),
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(1),
                    range: Range::new(Position::new(0, 0), Position::new(1, 0)),
                },
                container_name: None,
                tags: None,
                deprecated: None,
            },
            SymbolInformation {
                name: "test • test 2".to_string(),
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(1),
                    range: Range::new(Position::new(2, 0), Position::new(3, 0)),
                },
                container_name: None,
                tags: None,
                deprecated: None,
            },
        ]),
    );
}

#[test]
fn one_file_two_headers_same_level() {
    let fixture = Fixture::with(indoc! {"
            # test

            # test 2
            "});

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: "".to_string(),
        },
        WorkspaceSymbolResponse::Flat(vec![
            SymbolInformation {
                name: "test".to_string(),
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(1),
                    range: Range::new(Position::new(0, 0), Position::new(1, 0)),
                },
                container_name: None,
                tags: None,
                deprecated: None,
            },
            SymbolInformation {
                name: "test 2".to_string(),
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(1),
                    range: Range::new(Position::new(2, 0), Position::new(3, 0)),
                },
                container_name: None,
                tags: None,
                deprecated: None,
            },
        ]),
    );
}

#[test]
fn two_files() {
    let fixture = Fixture::with(indoc! {"
            # test 1
            _
            # test 2
            "});

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: "".to_string(),
        },
        WorkspaceSymbolResponse::Flat(vec![
            SymbolInformation {
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(1),
                    range: Range::new(Position::new(0, 0), Position::new(1, 0)),
                },
                name: "test 1".to_string(),
                container_name: None,
                tags: None,
                deprecated: None,
            },
            SymbolInformation {
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(2),
                    range: Range::new(Position::new(0, 0), Position::new(1, 0)),
                },
                name: "test 2".to_string(),
                container_name: None,
                tags: None,
                deprecated: None,
            },
        ]),
    )
}

#[test]
fn nested_files() {
    let fixture = Fixture::with(indoc! {"
            # test 1
            _
            # test 2

            [test 1](1)
            "});

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: "".to_string(),
        },
        WorkspaceSymbolResponse::Flat(vec![
            SymbolInformation {
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(1),
                    range: Range::new(Position::new(0, 0), Position::new(1, 0)),
                },
                name: "test 2 • test 1".to_string(),
                container_name: None,
                tags: None,
                deprecated: None,
            },
            SymbolInformation {
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(2),
                    range: Range::new(Position::new(0, 0), Position::new(1, 0)),
                },
                name: "test 2".to_string(),
                container_name: None,
                tags: None,
                deprecated: None,
            },
        ]),
    )
}

#[test]
fn two_nested_nested_files() {
    let fixture = Fixture::with(indoc! {"
            # test 1
            _
            # test 2

            [test 1](1)
            _
            # test 3

            [test 2](2)
            "});

    fixture.workspace_symbols(
        WorkspaceSymbolParams {
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            query: "".to_string(),
        },
        WorkspaceSymbolResponse::Flat(vec![
            SymbolInformation {
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(1),
                    range: Range::new(Position::new(0, 0), Position::new(1, 0)),
                },
                name: "test 3 • test 2 • test 1".to_string(),
                container_name: None,
                tags: None,
                deprecated: None,
            },
            SymbolInformation {
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(2),
                    range: Range::new(Position::new(0, 0), Position::new(1, 0)),
                },
                name: "test 3 • test 2".to_string(),
                container_name: None,
                tags: None,
                deprecated: None,
            },
            SymbolInformation {
                kind: lsp_types::SymbolKind::OBJECT,
                location: lsp_types::Location {
                    uri: uri(3),
                    range: Range::new(Position::new(0, 0), Position::new(1, 0)),
                },
                name: "test 3".to_string(),
                container_name: None,
                tags: None,
                deprecated: None,
            },
        ]),
    )
}
