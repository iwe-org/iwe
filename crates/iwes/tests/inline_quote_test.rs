use indoc::indoc;
use liwe::model::config::{BlockAction, Configuration, Inline, InlineType};
use lsp_types::{Position, Range};

mod fixture;
use crate::fixture::*;

#[test]
fn inline_quote_test() {
    assert_inlined(
        indoc! {"
            # test

            [test2](2)
            _
            # test2

            para
            "},
        2,
        indoc! {"
            # test

            > # test2
            >
            > para
        "},
    );
}

#[test]
fn inline_with_content_after_ref() {
    assert_inlined(
        indoc! {"
            # test

            [test2](2)

            ## test3
            _
            # test2

            para
            "},
        2,
        indoc! {"
            # test

            > # test2
            >
            > para

            ## test3
        "},
    );
}

#[test]
fn inline_quote_default_removes_all_references() {
    assert_inlined_remove_target(
        indoc! {"
            # test

            [test2](2)
            _
            # test2

            para content
            _
            # test3

            [test2](2)

            inline link to [test2](2) text
            "},
        2,
        indoc! {"
            # test

            > # test2
            >
            > para content
            "},
        indoc! {"
            # test3

            inline link to test2 text
            "},
    );
}

#[test]
fn inline_quote_with_keep_target_true_basic() {
    assert_inlined_with_keep_target(
        indoc! {"
            # test

            [test2](2)
            _
            # test2

            para
            "},
        2,
        indoc! {"
            # test

            > # test2
            >
            > para
        "},
    );
}

#[test]
fn inline_quote_with_keep_target_true_keeps_other_references() {
    assert_inlined_with_keep_target(
        indoc! {"
            # test

            [test2](2)
            _
            # test2

            para content
            _
            # test3

            [test2](2)

            inline link to [test2](2) text
            "},
        2,
        indoc! {"
            # test

            > # test2
            >
            > para content
            "},
    );
}

fn assert_inlined_with_keep_target(source: &str, line: u32, inlined: &str) {
    let mut config = Configuration::template();
    config.actions.insert(
        "inline_quote_keep".into(),
        BlockAction::Inline(Inline {
            title: "Inline quote (keep target)".into(),
            inline_type: InlineType::Quote,
            keep_target: Some(true),
        }),
    );

    let fixture = Fixture::with_config(source, config);

    fixture.code_action(
        uri(1).to_code_action_params(
            Range::new(Position::new(line, 0), Position::new(line, 0)),
            "custom.inline_quote_keep",
        ),
        vec![uri(1).to_edit(inlined)]
            .to_workspace_edit()
            .to_code_action("Inline quote (keep target)", "custom.inline_quote_keep"),
    )
}

fn assert_inlined(source: &str, line: u32, inlined: &str) {
    let fixture = Fixture::with_config(source, Configuration::template());

    fixture.code_action(
        uri(1).to_code_action_params(
            Range::new(Position::new(line, 0), Position::new(line, 0)),
            "custom.inline_quote",
        ),
        vec![uri(2).to_delete_file(), uri(1).to_edit(inlined)]
            .to_workspace_edit()
            .to_code_action("Inline quote", "custom.inline_quote"),
    )
}

fn assert_inlined_remove_target(source: &str, line: u32, inlined: &str, additional_updates: &str) {
    let fixture = Fixture::with_config(source, Configuration::template());

    fixture.code_action(
        uri(1).to_code_action_params(
            Range::new(Position::new(line, 0), Position::new(line, 0)),
            "custom.inline_quote",
        ),
        vec![
            uri(2).to_delete_file(),
            uri(1).to_edit(inlined),
            uri(3).to_edit(additional_updates),
        ]
        .to_workspace_edit()
        .to_code_action("Inline quote", "custom.inline_quote"),
    )
}
