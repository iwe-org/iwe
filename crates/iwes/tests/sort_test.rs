use diwe::config::{ActionDefinition, Configuration, Sort};
use indoc::indoc;

use crate::fixture::*;

#[test]
fn sort_simple_list() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        ActionDefinition::Sort(Sort {
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
    );
}

#[test]
fn sort_not_offered_when_already_sorted_ascending() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        ActionDefinition::Sort(Sort {
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
    .no_code_action(uri(1).to_code_action_params(0, "custom.sort"));
}

#[test]
fn sort_not_offered_when_already_sorted_descending() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        ActionDefinition::Sort(Sort {
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
    .no_code_action(uri(1).to_code_action_params(0, "custom.sort"));
}

#[test]
fn sort_offered_when_partially_sorted() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        ActionDefinition::Sort(Sort {
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
    );
}

#[test]
fn sort_list_descending() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        ActionDefinition::Sort(Sort {
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
    );
}

#[test]
fn sort_ordered_list() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        ActionDefinition::Sort(Sort {
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
                1. apple
                2. banana
                3. zebra
                "})]
        .to_workspace_edit()
        .to_code_action("Sort", "custom.sort"),
    );
}

#[test]
fn sort_ignores_case_accents_and_compares_numbers_by_value() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        ActionDefinition::Sort(Sort {
            title: "Sort".into(),
            reverse: Some(false),
        }),
    );

    Fixture::with_config(
        indoc! {"
            - Zebra
            - Note 10
            - apple
            - Émile
            - Note 2
            - 🚀 banana
            - Apple
            "},
        configuration,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.sort"),
        vec![uri(1).to_edit(indoc! {"
                - apple
                - Apple
                - 🚀 banana
                - Émile
                - Note 2
                - Note 10
                - Zebra
                "})]
        .to_workspace_edit()
        .to_code_action("Sort", "custom.sort"),
    );
}

#[test]
fn sort_not_offered_when_sorted_by_natural_order() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "sort".into(),
        ActionDefinition::Sort(Sort {
            title: "Sort".into(),
            reverse: Some(false),
        }),
    );

    Fixture::with_config(
        indoc! {"
            - apple
            - Banana
            - Note 2
            - Note 10
            - Zebra
            "},
        configuration,
    )
    .no_code_action(uri(1).to_code_action_params(0, "custom.sort"));
}

#[test]
fn sort_follows_workspace_locale() {
    let mut configuration = Configuration::default();
    configuration.workspace.locale = Some("sv-SE".into());

    configuration.actions.insert(
        "sort".into(),
        ActionDefinition::Sort(Sort {
            title: "Sort".into(),
            reverse: Some(false),
        }),
    );

    Fixture::with_config(
        indoc! {"
            - ähnlich
            - zebra
            - apple
            "},
        configuration,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.sort"),
        vec![uri(1).to_edit(indoc! {"
                - apple
                - zebra
                - ähnlich
                "})]
        .to_workspace_edit()
        .to_code_action("Sort", "custom.sort"),
    );
}

#[test]
fn sort_danish_locale_places_double_a_after_z() {
    let mut configuration = Configuration::default();
    configuration.workspace.locale = Some("da-DK".into());

    configuration.actions.insert(
        "sort".into(),
        ActionDefinition::Sort(Sort {
            title: "Sort".into(),
            reverse: Some(false),
        }),
    );

    Fixture::with_config(
        indoc! {"
            - Aalborg
            - Zebra
            - Arhus
            "},
        configuration,
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.sort"),
        vec![uri(1).to_edit(indoc! {"
                - Arhus
                - Zebra
                - Aalborg
                "})]
        .to_workspace_edit()
        .to_code_action("Sort", "custom.sort"),
    );
}
