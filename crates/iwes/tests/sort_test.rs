use indoc::indoc;
use liwe::model::config::{BlockAction, Configuration, Sort};

mod fixture;
use crate::fixture::*;

#[test]
fn sort_simple_list() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort".into(),
            reverse: Some(false),
        }),
    );

    Fixture::with_config(
        indoc! {"
            - zebra
            - apple
            - banana
            "},
        configuration,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.sort"),
        vec![uri(1).to_edit(indoc! {"
                - apple
                - banana
                - zebra
                "})]
        .to_workspace_edit()
        .to_code_action("Sort", "custom.sort"),
    )
}

#[test]
fn sort_not_offered_when_already_sorted_ascending() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort A-Z".into(),
            reverse: Some(false),
        }),
    );

    Fixture::with_config(
        indoc! {"
            - apple
            - banana
            - zebra
            "},
        configuration,
    )
    .no_code_action(uri(1).to_code_action_params(0, "custom.sort"))
}

#[test]
fn sort_not_offered_when_already_sorted_descending() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort Z-A".into(),
            reverse: Some(true),
        }),
    );

    Fixture::with_config(
        indoc! {"
            - zebra
            - banana
            - apple
            "},
        configuration,
    )
    .no_code_action(uri(1).to_code_action_params(0, "custom.sort"))
}

#[test]
fn sort_offered_when_partially_sorted() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort A-Z".into(),
            reverse: Some(false),
        }),
    );

    Fixture::with_config(
        indoc! {"
            - apple
            - zebra
            - banana
            "},
        configuration,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.sort"),
        vec![uri(1).to_edit(indoc! {"
                - apple
                - banana
                - zebra
                "})]
        .to_workspace_edit()
        .to_code_action("Sort A-Z", "custom.sort"),
    )
}

#[test]
fn sort_list_descending() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort Descending".into(),
            reverse: Some(true),
        }),
    );

    Fixture::with_config(
        indoc! {"
            - zebra
            - apple
            - banana
            "},
        configuration,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.sort"),
        vec![uri(1).to_edit(indoc! {"
                - zebra
                - banana
                - apple
                "})]
        .to_workspace_edit()
        .to_code_action("Sort Descending", "custom.sort"),
    )
}

#[test]
fn sort_ordered_list() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        BlockAction::Sort(Sort {
            title: "Sort".into(),
            reverse: Some(false),
        }),
    );

    Fixture::with_config(
        indoc! {"
            1. zebra
            2. apple
            3. banana
            "},
        configuration,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.sort"),
        vec![uri(1).to_edit(indoc! {"
                1.  apple
                2.  banana
                3.  zebra
                "})]
        .to_workspace_edit()
        .to_code_action("Sort", "custom.sort"),
    )
}
