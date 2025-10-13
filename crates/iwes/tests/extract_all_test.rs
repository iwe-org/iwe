use chrono::Local;
use indoc::indoc;
use liwe::model::config::{ActionDefinition, Configuration, ExtractAll, LinkType};

mod fixture;
use crate::fixture::*;

#[test]
fn no_sub_sections_to_extract() {
    assert_no_action(
        indoc! {"
            # test
            "},
        0,
    );

    assert_no_action(
        indoc! {"
            # test

            # test
            "},
        2,
    );

    assert_no_action(
        indoc! {"
            # test

            ## test
            "},
        2,
    );
}

#[test]
fn extract_sub_section_after_para_test() {
    assert_extracted(
        indoc! {"
            # test

            para

            ## test2
            "},
        0,
        indoc! {"
            # test

            para

            [test2](2)
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_sub_sections_test() {
    assert_extracted(
        indoc! {"
            # test

            ## test2
            "},
        0,
        indoc! {"
            # test

            [test2](2)
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_sub_sections_wiki_link() {
    assert_extracted_wiki(
        indoc! {"
            # test

            ## test2
            "},
        0,
        indoc! {"
            # test

            [[2]]
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_multiple_sub_sections() {
    assert_extracted_multiple(
        indoc! {"
            # test

            ## section1

            content 1

            ## section2

            content 2

            ## section3

            content 3
            "},
        0,
        indoc! {"
            # test

            [section1](2)

            [section2](3)

            [section3](4)
            "},
        vec![
            (2, "# section1\n\ncontent 1\n"),
            (3, "# section2\n\ncontent 2\n"),
            (4, "# section3\n\ncontent 3\n"),
        ],
    );
}

#[test]
fn extract_sub_sections_with_date_template() {
    assert_extracted_with_date_template(
        indoc! {"
            # test

            ## target_section
            "},
        0,
        indoc! {"
            # test

            [target_section]({{today}})
            "},
        indoc! {"
            # target_section
        "},
    );
}

#[test]
fn extract_sub_sections_with_title_template() {
    Fixture::with_config(
        indoc! {"
            # ParentSection

            ## Target Section
            "},
        create_extract_all_config("{{title}}", LinkType::Markdown),
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.extract_all"),
        vec![
            uri_from("Target%20Section").to_create_file(),
            uri_from("Target%20Section").to_edit("# Target Section\n"),
            uri(1).to_edit("# ParentSection\n\n[Target Section](Target Section)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Extract all subsections", "custom.extract_all"),
    );
}

#[test]
fn extract_multiple_sub_sections_with_title_template() {
    Fixture::with_config(
        indoc! {"
            # ParentSection

            ## Target Section

            ## Target Section

            ## Target Section
            "},
        create_extract_all_config("{{title}}", LinkType::Markdown),
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.extract_all"),
        vec![
            uri_from("Target%20Section").to_create_file(),
            uri_from("Target%20Section").to_edit(indoc! {"
                # Target Section
                "}),
            uri_from("Target%20Section-1").to_create_file(),
            uri_from("Target%20Section-1").to_edit(indoc! {"
                # Target Section
                "}),
            uri_from("Target%20Section-2").to_create_file(),
            uri_from("Target%20Section-2").to_edit(indoc! {"
                # Target Section
                "}),
            uri(1).to_edit(indoc! {"
                # ParentSection

                [Target Section](Target Section)

                [Target Section](Target Section-1)

                [Target Section](Target Section-2)
                "}),
        ]
        .to_workspace_edit()
        .to_code_action("Extract all subsections", "custom.extract_all"),
    );
}

fn assert_extracted(source: &str, line: u32, target: &str, extracted: &str) {
    Fixture::with_config(source, extract_all_config()).code_action(
        uri(1).to_code_action_params(line, "custom.extract_all"),
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit(extracted),
            uri(1).to_edit(target),
        ]
        .to_workspace_edit()
        .to_code_action("Extract all subsections", "custom.extract_all"),
    );
}

fn assert_extracted_multiple(
    source: &str,
    line: u32,
    target: &str,
    extracted_sections: Vec<(u32, &str)>,
) {
    let mut changes = vec![];

    for (file_id, content) in &extracted_sections {
        changes.push(uri(*file_id).to_create_file());
        changes.push(uri(*file_id).to_edit(content));
    }

    changes.push(uri(1).to_edit(target));

    Fixture::with_config(source, extract_all_config()).code_action(
        uri(1).to_code_action_params(line, "custom.extract_all"),
        changes
            .to_workspace_edit()
            .to_code_action("Extract all subsections", "custom.extract_all"),
    );
}

fn assert_extracted_wiki(source: &str, line: u32, target: &str, extracted: &str) {
    Fixture::with_config(
        source,
        create_extract_all_config("{{id}}", LinkType::WikiLink),
    )
    .code_action(
        uri(1).to_code_action_params(line, "custom.extract_all"),
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit(extracted),
            uri(1).to_edit(target),
        ]
        .to_workspace_edit()
        .to_code_action("Extract all subsections", "custom.extract_all"),
    );
}

fn assert_no_action(source: &str, line: u32) {
    Fixture::with_config(source, extract_all_config())
        .no_code_action(uri(1).to_code_action_params(line, "custom.extract_all"));
}

#[test]
fn extract_sub_sections_with_parent_slug_template() {
    Fixture::with_config(
        indoc! {"
            # Parent/With*Special:Chars

            ## Child Section
            "},
        create_extract_all_config("{{parent.slug}}-{{slug}}", LinkType::Markdown),
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.extract_all"),
        vec![
            uri_from("parent-with-special-chars-child-section").to_create_file(),
            uri_from("parent-with-special-chars-child-section").to_edit("# Child Section\n"),
            uri(1).to_edit("# Parent/With*Special:Chars\n\n[Child Section](parent-with-special-chars-child-section)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Extract all subsections", "custom.extract_all"),
    );
}

#[test]
fn extract_sub_sections_with_source_slug_template() {
    let mut files = std::collections::HashMap::new();
    files.insert(
        "user-guide-manual".to_string(),
        indoc! {"
            # User Guide & Manual

            ## Installation Section
            "}
        .to_string(),
    );

    Fixture::with_options_and_client(files, create_extract_all_config("{{source.slug}}-{{slug}}", LinkType::Markdown), "")
        .code_action(
            uri_from("user-guide-manual").to_code_action_params(0, "custom.extract_all"),
            vec![
                uri_from("user-guide-manual-installation-section").to_create_file(),
                uri_from("user-guide-manual-installation-section").to_edit("# Installation Section\n"),
                uri_from("user-guide-manual").to_edit(
                    "# User Guide & Manual\n\n[Installation Section](user-guide-manual-installation-section)\n",
                ),
            ]
            .to_workspace_edit()
            .to_code_action("Extract all subsections", "custom.extract_all"),
        );
}

fn create_extract_all_config(key_template: &str, link_type: LinkType) -> Configuration {
    let mut config = Configuration::default();
    config.actions.insert(
        "extract_all".to_string(),
        ActionDefinition::ExtractAll(ExtractAll {
            title: "Extract all subsections".to_string(),
            link_type: Some(link_type),
            key_template: key_template.to_string(),
        }),
    );
    config
}

fn extract_all_config() -> Configuration {
    create_extract_all_config("{{id}}", LinkType::Markdown)
}

fn assert_extracted_with_date_template(source: &str, line: u32, target: &str, extracted: &str) {
    let date = Local::now().date_naive();
    let formatted_date = date.format("%Y-%m-%d").to_string();

    let target_with_date = target.replace("{{today}}", &formatted_date);

    Fixture::with_config(
        source,
        create_extract_all_config("{{today}}", LinkType::Markdown),
    )
    .code_action(
        uri(1).to_code_action_params(line, "custom.extract_all"),
        vec![
            uri_from(&formatted_date).to_create_file(),
            uri_from(&formatted_date).to_edit(extracted),
            uri(1).to_edit(&target_with_date),
        ]
        .to_workspace_edit()
        .to_code_action("Extract all subsections", "custom.extract_all"),
    );
}

#[test]
fn extract_sub_sections_with_slug_template() {
    Fixture::with_config(
        indoc! {"
            # Document

            ## Target/With*Special:Chars
            "},
        create_extract_all_config("{{slug}}", LinkType::Markdown),
    )
    .code_action(
        uri(1).to_code_action_params(0, "custom.extract_all"),
        vec![
            uri_from("target-with-special-chars").to_create_file(),
            uri_from("target-with-special-chars").to_edit("# Target/With*Special:Chars\n"),
            uri(1)
                .to_edit("# Document\n\n[Target/With*Special:Chars](target-with-special-chars)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Extract all subsections", "custom.extract_all"),
    );
}
