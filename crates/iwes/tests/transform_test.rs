use std::collections::HashMap;

use indoc::indoc;
use liwe::model::config::{ActionDefinition, Command, Configuration, Transform};

mod fixture;
use crate::fixture::*;

#[test]
fn transform_action_appears_in_code_action_menu() {
    let config = Configuration {
        actions: vec![(
            "uppercase".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Uppercase".to_string(),
                command: "uppercase".to_string(),
                input_template: "{{context}}".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: vec![(
            "uppercase".to_string(),
            Command {
                run: "tr '[:lower:]' '[:upper:]'".to_string(),
                timeout_seconds: Some(5),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            hello world
        "},
        config,
    )
    .code_action_menu(
        uri(1).to_code_action_params(0, "custom.uppercase"),
        lsp_types::CodeAction {
            title: "Uppercase".to_string(),
            kind: action_kind("custom.uppercase"),
            ..Default::default()
        },
    );
}

#[test]
fn transform_action_executes_command_with_cat() {
    let config = Configuration {
        actions: vec![(
            "echo".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Echo".to_string(),
                command: "echo".to_string(),
                input_template: "{{context}}".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: vec![(
            "echo".to_string(),
            Command {
                run: "cat".to_string(),
                timeout_seconds: Some(5),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            hello world
        "},
        config,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.echo"),
        vec![uri(1).to_edit("<update_here>\n\nhello world\n\n</update_here>\n")]
            .to_workspace_edit()
            .to_code_action("Echo", "custom.echo"),
    );
}

#[test]
fn transform_action_with_empty_command_shows_in_menu_but_no_resolution() {
    let config = Configuration {
        actions: vec![(
            "empty".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Empty".to_string(),
                command: "empty".to_string(),
                input_template: "{{context}}".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: vec![(
            "empty".to_string(),
            Command {
                run: "".to_string(),
                timeout_seconds: Some(5),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            hello world
        "},
        config,
    )
    .code_action_menu(
        uri(1).to_code_action_params(0, "custom.empty"),
        lsp_types::CodeAction {
            title: "Empty".to_string(),
            kind: action_kind("custom.empty"),
            ..Default::default()
        },
    );
}

#[test]
fn transform_action_with_missing_command_shows_in_menu() {
    let config = Configuration {
        actions: vec![(
            "missing".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Missing".to_string(),
                command: "nonexistent".to_string(),
                input_template: "{{context}}".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: Default::default(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            hello world
        "},
        config,
    )
    .code_action_menu(
        uri(1).to_code_action_params(0, "custom.missing"),
        lsp_types::CodeAction {
            title: "Missing".to_string(),
            kind: action_kind("custom.missing"),
            ..Default::default()
        },
    );
}

#[test]
fn transform_action_with_static_output() {
    let config = Configuration {
        actions: vec![(
            "static".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Static Output".to_string(),
                command: "static".to_string(),
                input_template: "ignored".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: vec![(
            "static".to_string(),
            Command {
                run: "echo 'replaced content'".to_string(),
                timeout_seconds: Some(5),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            original content
        "},
        config,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.static"),
        vec![uri(1).to_edit("replaced content\n")]
            .to_workspace_edit()
            .to_code_action("Static Output", "custom.static"),
    );
}

#[test]
fn transform_action_with_multiline_content() {
    let config = Configuration {
        actions: vec![(
            "reverse".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Reverse Lines".to_string(),
                command: "reverse".to_string(),
                input_template: "{{context}}".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: vec![(
            "reverse".to_string(),
            Command {
                run: "tac".to_string(),
                timeout_seconds: Some(5),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            # Header

            line1

            line2

            line3
        "},
        config,
    )
    .code_action_menu(
        uri(1).to_code_action_params(0, "custom.reverse"),
        lsp_types::CodeAction {
            title: "Reverse Lines".to_string(),
            kind: action_kind("custom.reverse"),
            ..Default::default()
        },
    );
}

#[test]
fn transform_action_processes_input_template() {
    let config = Configuration {
        actions: vec![(
            "prefix".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Add Prefix".to_string(),
                command: "prefix".to_string(),
                input_template: "PREFIX: {{context}}".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: vec![(
            "prefix".to_string(),
            Command {
                run: "cat".to_string(),
                timeout_seconds: Some(5),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            hello
        "},
        config,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.prefix"),
        vec![uri(1).to_edit("PREFIX: <update_here>\n\nhello\n\n</update_here>\n")]
            .to_workspace_edit()
            .to_code_action("Add Prefix", "custom.prefix"),
    );
}

#[test]
fn transform_action_with_timeout() {
    let config = Configuration {
        actions: vec![(
            "slow".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Slow Command".to_string(),
                command: "slow".to_string(),
                input_template: "{{context}}".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: vec![(
            "slow".to_string(),
            Command {
                run: "cat".to_string(),
                timeout_seconds: Some(60),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            test content
        "},
        config,
    )
    .code_action_menu(
        uri(1).to_code_action_params(0, "custom.slow"),
        lsp_types::CodeAction {
            title: "Slow Command".to_string(),
            kind: action_kind("custom.slow"),
            ..Default::default()
        },
    );
}

#[test]
fn transform_action_with_shell_false_and_args() {
    let config = Configuration {
        actions: vec![(
            "direct".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Direct Exec".to_string(),
                command: "direct".to_string(),
                input_template: "{{context}}".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: vec![(
            "direct".to_string(),
            Command {
                run: "cat".to_string(),
                args: Some(vec!["-".to_string()]),
                shell: Some(false),
                timeout_seconds: Some(5),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            direct execution test
        "},
        config,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.direct"),
        vec![uri(1).to_edit("<update_here>\n\ndirect execution test\n\n</update_here>\n")]
            .to_workspace_edit()
            .to_code_action("Direct Exec", "custom.direct"),
    );
}

#[test]
fn transform_action_with_env_vars() {
    std::env::set_var("IWE_TEST_VAR", "expanded_value");

    let mut env = HashMap::new();
    env.insert("OUTPUT_VAR".to_string(), "$IWE_TEST_VAR".to_string());

    let config = Configuration {
        actions: vec![(
            "envtest".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Env Test".to_string(),
                command: "envtest".to_string(),
                input_template: "{{context}}".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: vec![(
            "envtest".to_string(),
            Command {
                run: "echo $OUTPUT_VAR".to_string(),
                env: Some(env),
                timeout_seconds: Some(5),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            env test content
        "},
        config,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.envtest"),
        vec![uri(1).to_edit("expanded_value\n")]
            .to_workspace_edit()
            .to_code_action("Env Test", "custom.envtest"),
    );

    std::env::remove_var("IWE_TEST_VAR");
}

#[test]
fn transform_action_with_cwd() {
    let tmp_path = std::fs::canonicalize("/tmp")
        .expect("canonicalize /tmp")
        .to_string_lossy()
        .to_string();

    let config = Configuration {
        actions: vec![(
            "cwdtest".to_string(),
            ActionDefinition::Transform(Transform {
                title: "Cwd Test".to_string(),
                command: "cwdtest".to_string(),
                input_template: "{{context}}".to_string(),
            }),
        )]
        .into_iter()
        .collect(),
        commands: vec![(
            "cwdtest".to_string(),
            Command {
                run: "pwd".to_string(),
                cwd: Some("/tmp".to_string()),
                timeout_seconds: Some(5),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    Fixture::with_config(
        indoc! {"
            cwd test content
        "},
        config,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.cwdtest"),
        vec![uri(1).to_edit(&format!("{}\n", tmp_path))]
            .to_workspace_edit()
            .to_code_action("Cwd Test", "custom.cwdtest"),
    );
}
