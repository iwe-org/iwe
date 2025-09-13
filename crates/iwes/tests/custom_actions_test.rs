use std::u32;

use indoc::indoc;
use liwe::model::config::{BlockAction, Configuration, Context::Document, Model, Transform};
use lsp_types::CodeAction;

mod fixture;
use crate::fixture::*;

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
        uri(1).to_code_action_params(line, "custom.action"),
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
