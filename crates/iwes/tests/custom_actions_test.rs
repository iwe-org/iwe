use indoc::indoc;
use liwe::model::config::{ActionDefinition, Command, Configuration, Transform};

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
        commands: vec![("command".to_string(), Command::default())]
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
        command: "command".to_string(),
        input_template: "".to_string(),
    })
}
