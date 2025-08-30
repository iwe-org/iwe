use std::u32;

use indoc::indoc;
use liwe::model::config::{BlockAction, Configuration, Context::Document, Model, Transform};
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionParams, Position, Range, TextDocumentIdentifier,
};

use fixture::{action_kind, action_kinds, uri};

use crate::fixture::Fixture;

mod fixture;

#[test]
fn block_action_target() {
    assert_action(
        action(),
        indoc! {"
            test
            "},
        0,
    );
}

fn assert_action(action: BlockAction, source: &str, line: u32) {
    let config = Configuration {
        actions: vec![("action".to_string(), action)].into_iter().collect(),
        models: vec![("model".to_string(), Model::default())]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    let fixture = Fixture::with_config(source, config);

    fixture.code_action_menu(
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: action_kinds("custom.action"),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CodeAction {
            title: "title".to_string(),
            kind: action_kind("custom.action"),
            ..Default::default()
        },
    )
}

fn action() -> BlockAction {
    BlockAction::Transform(Transform {
        title: "title".to_string(),
        model: "model".to_string(),
        prompt_template: "".to_string(),
        context: Document,
    })
}
