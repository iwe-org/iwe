use std::u32;

use chrono::Local;
use indoc::{formatdoc, indoc};
use liwe::model::config::{Attach, BlockAction, Configuration};
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionParams, CreateFile, CreateFileOptions,
    DocumentChangeOperation, DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier,
    Position, Range, ResourceOp, TextDocumentEdit, TextDocumentIdentifier, TextEdit, WorkspaceEdit,
};

use fixture::{action_kind, action_kinds, uri};

use crate::fixture::{uri_from, Fixture};

mod fixture;

#[test]
fn basic_attach() {
    assert_attached(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b
            _
            # target
            "},
        2,
        indoc! {"
            # target

            [title b](2)
        "},
    );
}

#[test]
fn alreary_attached() {
    assert_no_action(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b
            _
            # target

            [title b](2)
            "},
        2,
    );
}

#[test]
fn attach_to_date_template() {
    let date = Local::now().date_naive();
    let markdown_format = date.format("%b %d, %Y").to_string();
    let key_format = date.format("%Y-%m-%d").to_string();

    assert_attached_template(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b
            "},
        2,
        &formatdoc! {"
            # {date}

            [title b](2)
        ",
        date = markdown_format },
        &key_format,
    );
}

#[test]
fn attach_no_key() {
    assert_attached_new_key(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b
            "},
        2,
        indoc! {"
            # template

            [title b](2)
        "},
    );
}

#[test]
fn basic_attach_non_empty() {
    assert_attached(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b
            _
            # target

            [title a](1)
            "},
        2,
        indoc! {"
            # target

            [title a](1)

            [title b](2)
        "},
    );
}

#[test]
fn basic_attach_pre_header() {
    assert_attached(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b
            _
            # target

            ## header
            "},
        2,
        indoc! {"
            # target

            [title b](2)

            ## header
        "},
    );
}

#[test]
fn basic_attach_pre_header_multiple() {
    assert_attached(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b
            _
            # target

            [title a](1)

            ## header
            "},
        2,
        indoc! {"
            # target

            [title a](1)

            [title b](2)

            ## header
        "},
    );
}

#[test]
fn basic_attach_no_header() {
    assert_attached(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b
            _
            "},
        2,
        indoc! {"
            [title b](2)
        "},
    );
}

fn assert_attached(source: &str, line: u32, expected: &str) {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "attach".into(),
        BlockAction::Attach(Attach {
            title: "Attach".into(),
            key_template: "3".into(),
            document_template: "# template\n\n{{content}}".into(),
        }),
    );

    let fixture = Fixture::with_config(source, configuration);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.attach"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "Attach".to_string(),
            kind: action_kind("custom.attach"),
            edit: Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(vec![
                    DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: uri(3),
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
        },
    )
}

fn assert_attached_new_key(source: &str, line: u32, expected: &str) {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "attach".into(),
        BlockAction::Attach(Attach {
            title: "Attach".into(),
            key_template: "3".into(),
            document_template: "# template\n\n{{content}}".into(),
        }),
    );

    let fixture = Fixture::with_config(source, configuration);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.attach"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "Attach".to_string(),
            kind: action_kind("custom.attach"),
            edit: Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(vec![
                    DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                        uri: uri(3),
                        options: Some(CreateFileOptions {
                            overwrite: Some(false),
                            ignore_if_exists: Some(false),
                        }),
                        annotation_id: None,
                    })),
                    DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: uri(3),
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
        },
    )
}

fn assert_no_action(source: &str, line: u32) {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "attach".into(),
        BlockAction::Attach(Attach {
            title: "Attach".into(),
            key_template: "3".into(),
            document_template: "# template\n\n{{content}}".into(),
        }),
    );

    let fixture = Fixture::with_config(source, configuration);

    fixture.no_code_action(CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        range: Range::new(Position::new(line, 0), Position::new(line, 0)),
        context: CodeActionContext {
            diagnostics: Default::default(),
            only: action_kinds("custom.attach"),
            trigger_kind: None,
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    })
}

fn assert_attached_template(source: &str, line: u32, expected: &str, expected_key: &str) {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "attach".into(),
        BlockAction::Attach(Attach {
            title: "Attach".into(),
            key_template: "{{today}}".into(),
            document_template: "# {{today}}\n\n{{content}}".into(),
        }),
    );

    let fixture = Fixture::with_config(source, configuration);

    fixture.code_action(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.attach"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "Attach".to_string(),
            kind: action_kind("custom.attach"),
            edit: Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(vec![
                    DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                        uri: uri_from(expected_key),
                        options: Some(CreateFileOptions {
                            overwrite: Some(false),
                            ignore_if_exists: Some(false),
                        }),
                        annotation_id: None,
                    })),
                    DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: uri_from(expected_key),
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
        },
    )
}
