use chrono::Local;
use indoc::{formatdoc, indoc};
use liwe::model::config::{ActionDefinition, Attach, Configuration};

mod fixture;
use crate::fixture::*;

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
    let now = Local::now();
    let markdown_format = now.format("%b %d, %Y").to_string();
    let key_format = now.format("%Y-%m-%d").to_string();

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
        ActionDefinition::Attach(Attach {
            title: "Attach".into(),
            key_template: "3".into(),
            document_template: "# template\n\n{{content}}".into(),
        }),
    );

    Fixture::with_config(source, configuration).code_action(
        uri(1).to_code_action_params(line, "custom.attach"),
        vec![uri(3).to_edit(expected)]
            .to_workspace_edit()
            .to_code_action("Attach", "custom.attach"),
    );
}

fn assert_attached_new_key(source: &str, line: u32, expected: &str) {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "attach".into(),
        ActionDefinition::Attach(Attach {
            title: "Attach".into(),
            key_template: "3".into(),
            document_template: "# template\n\n{{content}}".into(),
        }),
    );

    Fixture::with_config(source, configuration).code_action(
        uri(1).to_code_action_params(line, "custom.attach"),
        vec![uri(3).to_create_file(), uri(3).to_edit(expected)]
            .to_workspace_edit()
            .to_code_action("Attach", "custom.attach"),
    );
}

fn assert_no_action(source: &str, line: u32) {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "attach".into(),
        ActionDefinition::Attach(Attach {
            title: "Attach".into(),
            key_template: "3".into(),
            document_template: "# template\n\n{{content}}".into(),
        }),
    );

    Fixture::with_config(source, configuration)
        .no_code_action(uri(1).to_code_action_params(line, "custom.attach"));
}

fn assert_attached_template(source: &str, line: u32, expected: &str, expected_key: &str) {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "attach".into(),
        ActionDefinition::Attach(Attach {
            title: "Attach".into(),
            key_template: "{{today}}".into(),
            document_template: "# {{today}}\n\n{{content}}".into(),
        }),
    );

    let expected_uri = uri_from(expected_key);

    Fixture::with_config(source, configuration).code_action(
        uri(1).to_code_action_params(line, "custom.attach"),
        vec![
            expected_uri.clone().to_create_file(),
            expected_uri.to_edit(expected),
        ]
        .to_workspace_edit()
        .to_code_action("Attach", "custom.attach"),
    );
}

#[test]
fn attach_with_time_format() {
    let now = Local::now();
    let markdown_format = now.format("%b %d, %Y %H:%M").to_string();
    let key_format = now.format("%Y%m%d%H%M").to_string();

    let mut configuration = Configuration::default();
    configuration.markdown.date_format = Some("%b %d, %Y %H:%M".into());
    configuration.library.date_format = Some("%Y%m%d%H%M".into());

    configuration.actions.insert(
        "attach".into(),
        ActionDefinition::Attach(Attach {
            title: "Attach".into(),
            key_template: "{{today}}".into(),
            document_template: "# {{today}}\n\n{{content}}".into(),
        }),
    );

    let expected_uri = uri_from(&key_format);
    let expected_doc = format!("# {}\n\n[title b](2)\n", markdown_format);

    Fixture::with_config(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b
            "},
        configuration,
    )
    .code_action(
        uri(1).to_code_action_params(2, "custom.attach"),
        vec![
            expected_uri.clone().to_create_file(),
            expected_uri.to_edit(&expected_doc),
        ]
        .to_workspace_edit()
        .to_code_action("Attach", "custom.attach"),
    );
}

#[test]
fn attach_with_separate_locales() {
    use chrono::Locale;

    let now = Local::now();
    let key_format = now
        .format_localized("%A-%B-%d", Locale::en_US)
        .to_string();
    let markdown_format = now
        .format_localized("%A, %d. %B %Y", Locale::de_DE)
        .to_string();

    let mut configuration = Configuration::default();
    configuration.library.locale = Some("en_US".into());
    configuration.library.date_format = Some("%A-%B-%d".into());
    configuration.markdown.locale = Some("de_DE".into());
    configuration.markdown.date_format = Some("%A, %d. %B %Y".into());

    configuration.actions.insert(
        "attach".into(),
        ActionDefinition::Attach(Attach {
            title: "Attach".into(),
            key_template: "{{today}}".into(),
            document_template: "# {{today}}\n\n{{content}}".into(),
        }),
    );

    let expected_uri = uri_from(&key_format);
    let expected_doc = format!("# {}\n\n[title b](2)\n", markdown_format);

    Fixture::with_config(
        indoc! {"
            # title a

            [title b](2)
            _
            # title b
            "},
        configuration,
    )
    .code_action(
        uri(1).to_code_action_params(2, "custom.attach"),
        vec![
            expected_uri.clone().to_create_file(),
            expected_uri.to_edit(&expected_doc),
        ]
        .to_workspace_edit()
        .to_code_action("Attach", "custom.attach"),
    );
}

#[test]
fn attach_inline_link() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "attach".into(),
        ActionDefinition::Attach(Attach {
            title: "Attach".into(),
            key_template: "3".into(),
            document_template: "# template\n\n{{content}}".into(),
        }),
    );

    Fixture::with_config(
        indoc! {"
            # title a

            Some text with [title b](2) link.
            _
            # title b
            _
            # target
        "},
        configuration,
    )
    .code_action(
        uri(1).to_code_action_params_at_position(2, 17, "custom.attach"),
        vec![uri(3).to_edit(indoc! {"
            # target

            [title b](2)
        "})]
        .to_workspace_edit()
        .to_code_action("Attach", "custom.attach"),
    );
}

#[test]
fn attach_inline_link_no_action_outside_link() {
    let mut configuration = Configuration::default();

    configuration.actions.insert(
        "attach".into(),
        ActionDefinition::Attach(Attach {
            title: "Attach".into(),
            key_template: "3".into(),
            document_template: "# template\n\n{{content}}".into(),
        }),
    );

    Fixture::with_config(
        indoc! {"
            # title a

            Some text with [title b](2) link.
            _
            # title b
            _
            # target
        "},
        configuration,
    )
    .no_code_action(uri(1).to_code_action_params_at_position(2, 5, "custom.attach"));
}
