use indoc::indoc;
use liwe::model::config::{ActionDefinition, Configuration, Context::Document, Model, Transform};

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

fn assert_action(action: ActionDefinition, source: &str, line: u32) {
    let config = Configuration {
        actions: vec![("action".to_string(), action)].into_iter().collect(),
        models: vec![("model".to_string(), Model::default())]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    Fixture::with_config(source, config).code_action_menu(
        uri(1).to_code_action_params(line, "custom.action"),
        lsp_types::CodeAction {
            title: "title".to_string(),
            kind: action_kind("custom.action"),
            ..Default::default()
        },
    );
}

fn action() -> ActionDefinition {
    ActionDefinition::Transform(Transform {
        title: "title".to_string(),
        model: "model".to_string(),
        prompt_template: "".to_string(),
        context: Document,
    })
}
