use std::u32;

use indoc::indoc;
use liwe::model::config::{Attach, BlockAction, Configuration};
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionParams, DocumentChangeOperation, DocumentChanges,
    OneOf, OptionalVersionedTextDocumentIdentifier, Position, Range, TextDocumentEdit,
    TextDocumentIdentifier, TextEdit, WorkspaceEdit,
};

use fixture::{action_kind, action_kinds, uri};
use serde::de::IntoDeserializer;

use crate::fixture::Fixture;

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
            target_key_template: "3".into(),
            target_document_template: "# none".into(),
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
