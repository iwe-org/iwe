use indoc::indoc;
use lsp_server::ResponseError;
use lsp_types::{
    Position, PrepareRenameResponse, Range, RenameParams, TextDocumentIdentifier,
    TextDocumentPositionParams,
};

mod fixture;
use crate::fixture::*;

#[test]
fn basic_prepare_rename() {
    assert_prepare_rename(
        indoc! {"
            [text text](key)
            "},
        "key",
    );
}

#[test]
fn basic_rename() {
    assert_rename(
        indoc! {"
            [](1)
            _
            # file 2
            "},
        indoc! {"
            [](new_name)
        "},
    );
}

#[test]
fn rename_to_an_existing_key() {
    assert_rename_error(
        indoc! {"
            [](1)
            _
            # file 2
            "},
        "The file name 2 is already taken",
        Position::new(0, 0),
        "2",
    );
}

#[test]
fn rename_both_references() {
    assert_rename(
        indoc! {"
            [](1)

            [](1)
            _
            # file 2
            "},
        indoc! {"
            [](new_name)

            [](new_name)
        "},
    );
}

#[test]
fn rename_updates_affected_files() {
    assert_rename_updates_second_file(
        indoc! {"
            [](1)
            _
            # file 2

            [](1)
            "},
        indoc! {"
            [](new_name)
        "},
        indoc! {"
            # file 2

            [](new_name)
        "},
    );
}

#[test]
fn rename_inline_references() {
    assert_rename_at(
        indoc! {"
            # title

            [](1) text
            "},
        indoc! {"
            # title

            [](new_name) text
        "},
        Position::new(2, 0),
        "new_name",
    );
}

fn assert_prepare_rename(source: &str, _: &str) {
    let fixture = Fixture::with(source);

    fixture.prepare_rename(
        TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            position: Position::new(0, 0),
        },
        PrepareRenameResponse::RangeWithPlaceholder {
            range: Range::new(Position::new(0, 12), Position::new(0, 15)),
            placeholder: "key".to_string(),
        },
    )
}
fn assert_rename(source: &str, expected: &str) {
    assert_rename_at(source, expected, Position::new(0, 0), "new_name");
}

fn assert_rename_at(source: &str, expected: &str, position: Position, new_name: &str) {
    let fixture = Fixture::with(source);
    let new_uri = uri_from(new_name);

    fixture.rename(
        RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position,
            },
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        },
        vec![
            uri(1).to_delete_file(),
            new_uri.clone().to_create_file(),
            new_uri.to_edit_with_range(
                expected,
                Range::new(Position::new(0, 0), Position::new(0, 0)),
            ),
        ]
        .to_workspace_edit(),
    );
}

fn assert_rename_error(source: &str, expected: &str, position: Position, new_name: &str) {
    let fixture = Fixture::with(source);

    fixture.rename_err(
        RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position,
            },
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        },
        ResponseError {
            code: 1,
            message: expected.to_string(),
            data: None,
        },
    );
}

fn assert_rename_updates_second_file(source: &str, expected1: &str, expected2: &str) {
    let fixture = Fixture::with(source);
    let new_uri = uri_from("new_name");

    fixture.rename(
        RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(0, 0),
            },
            new_name: "new_name".to_string(),
            work_done_progress_params: Default::default(),
        },
        vec![
            uri(2).to_edit(expected2),
            uri(1).to_delete_file(),
            new_uri.clone().to_create_file(),
            new_uri.to_edit_with_range(
                expected1,
                Range::new(Position::new(0, 0), Position::new(0, 0)),
            ),
        ]
        .to_workspace_edit(),
    );
}
