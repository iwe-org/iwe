use indoc::indoc;
use lsp_server::ResponseError;
use lsp_types::{
    CreateFile, CreateFileOptions, DeleteFile, DocumentChangeOperation, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, PrepareRenameResponse, Range, RenameParams,
    ResourceOp, TextDocumentEdit, TextDocumentIdentifier, TextDocumentPositionParams, TextEdit,
    WorkspaceEdit,
};

use fixture::{uri, uri_from};

use crate::fixture::Fixture;

mod fixture;

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

    fixture.rename(
        RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position,
            },
            new_name: new_name.to_string(),
            work_done_progress_params: Default::default(),
        },
        WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Operations(vec![
                DocumentChangeOperation::Op(ResourceOp::Delete(DeleteFile {
                    uri: uri(1),
                    options: None,
                })),
                DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                    uri: uri_from("new_name"),
                    options: Some(CreateFileOptions {
                        overwrite: Some(false),
                        ignore_if_exists: Some(false),
                    }),
                    annotation_id: None,
                })),
                DocumentChangeOperation::Edit(TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: uri_from("new_name"),
                        version: None,
                    },
                    edits: vec![OneOf::Left(TextEdit {
                        new_text: expected.to_string(),
                        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    })],
                }),
            ])),
            change_annotations: None,
        },
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

    fixture.rename(
        RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(0, 0),
            },
            new_name: "new_name".to_string(),
            work_done_progress_params: Default::default(),
        },
        WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Operations(vec![
                DocumentChangeOperation::Edit(TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: uri(2),
                        version: None,
                    },
                    edits: vec![OneOf::Left(TextEdit {
                        new_text: expected2.to_string(),
                        range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
                    })],
                }),
                DocumentChangeOperation::Op(ResourceOp::Delete(DeleteFile {
                    uri: uri(1),
                    options: None,
                })),
                DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                    uri: uri_from("new_name"),
                    options: Some(CreateFileOptions {
                        overwrite: Some(false),
                        ignore_if_exists: Some(false),
                    }),
                    annotation_id: None,
                })),
                DocumentChangeOperation::Edit(TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: uri_from("new_name"),
                        version: None,
                    },
                    edits: vec![OneOf::Left(TextEdit {
                        new_text: expected1.to_string(),
                        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    })],
                }),
            ])),
            change_annotations: None,
        },
    );
}
