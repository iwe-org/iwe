use chrono::Local;
use indoc::indoc;
use liwe::model::config::{BlockAction, Configuration, Extract, LinkType};

mod fixture;
use crate::fixture::*;

#[test]
fn to_level_extract_not_allowed() {
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
}

#[test]
fn no_action_on_list() {
    assert_no_action(
        indoc! {"
            - test
            "},
        0,
    );
}

#[test]
fn extract_section() {
    assert_extracted(
        indoc! {"
            # test

            ## test2
            "},
        2,
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
fn extract_section_wiki_link() {
    assert_extracted_wiki(
        indoc! {"
            # test

            ## test2
            "},
        2,
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
fn extract_helix_section() {
    assert_extracted_helix(
        indoc! {"
            # test

            ## test2
            "},
        2,
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
fn extract_middle_section_test() {
    assert_extracted(
        indoc! {"
            # test

            ## test1

            ## test2

            ## test3
        "},
        4,
        indoc! {"
            # test

            [test2](2)

            ## test1

            ## test3
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_middle_section_wiki_link() {
    assert_extracted_wiki(
        indoc! {"
            # test

            ## test1

            ## test2

            ## test3
        "},
        4,
        indoc! {"
            # test

            [[2]]

            ## test1

            ## test3
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_after_list() {
    assert_extracted(
        indoc! {"
            # test

            - item1

            ## test2

            - item2
            "},
        4,
        indoc! {"
            # test

            - item1

            [test2](2)
            "},
        indoc! {"
            # test2

            - item2
        "},
    );
}

#[test]
fn extract_after_para() {
    assert_extracted(
        indoc! {"
            # test

            para1

            ## test2
            "},
        4,
        indoc! {"
            # test

            para1

            [test2](2)
            "},
        indoc! {"
            # test2
        "},
    );
}

#[test]
fn extract_third_level_section_test() {
    assert_extracted(
        indoc! {"
            # test

            ## test2

            ### test3
            "},
        4,
        indoc! {"
            # test

            ## test2

            [test3](2)
            "},
        indoc! {"
            # test3
        "},
    );
}

#[test]
fn extract_one_of_sub_level_section() {
    assert_extracted(
        indoc! {"
            # test

            para

            ## test2

            - item

            ## test3

            - item
            "},
        4,
        indoc! {"
            # test

            para

            [test2](2)

            ## test3

            - item
            "},
        indoc! {"
            # test2

            - item
        "},
    );
}

#[test]
fn test_extracted_relative() {
    let config = extract_config();
    Fixture::with_options_and_client(
        vec![("d/1".to_string(), "# test\n\n## target".to_string())]
            .into_iter()
            .collect(),
        config,
        "",
    )
    .code_action(
        uri_from("d/1").to_code_action_params(2, "custom.extract"),
        vec![
            uri_from("d/2").to_create_file(),
            uri_from("d/2").to_edit("# target\n"),
            uri_from("d/1").to_edit("# test\n\n[target](2)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "custom.extract"),
    );
}

#[test]
fn extract_section_with_date_template() {
    assert_extracted_with_date_template(
        indoc! {"
            # test

            ## target_section
            "},
        2,
        indoc! {"
            # test

            [target_section]({{today}})
            "},
        indoc! {"
            # target_section
        "},
    );
}

fn assert_extracted(source: &str, line: u32, target: &str, extracted: &str) {
    Fixture::with_config(source, extract_config()).code_action(
        uri(1).to_code_action_params(line, "custom.extract"),
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit(extracted),
            uri(1).to_edit(target),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "custom.extract"),
    );
}

fn assert_extracted_wiki(source: &str, line: u32, target: &str, extracted: &str) {
    Fixture::with_config(
        source,
        create_extract_config("{{id}}", Some(LinkType::WikiLink)),
    )
    .code_action(
        uri(1).to_code_action_params(line, "custom.extract"),
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit(extracted),
            uri(1).to_edit(target),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "custom.extract"),
    );
}

fn assert_extracted_helix(source: &str, line: u32, target: &str, extracted: &str) {
    use liwe::state::from_indoc;
    Fixture::with_options_and_client(from_indoc(source), extract_config(), "helix").code_action(
        uri(1).to_code_action_params(line, "custom.extract"),
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit(extracted),
            uri(1).to_edit(target),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "custom.extract"),
    );
}

#[test]
fn extract_section_with_simple_key_collision() {
    let mut files = std::collections::HashMap::new();
    files.insert(
        "1".to_string(),
        indoc! {"
        # test

        ## target_section
    "}
        .to_string(),
    );
    files.insert("extracted".to_string(), "# existing content\n".to_string());

    Fixture::with_options_and_client(files, create_extract_config("extracted", None), "")
        .code_action(
            uri(1).to_code_action_params(2, "custom.extract"),
            vec![
                uri_from("extracted-1").to_create_file(),
                uri_from("extracted-1").to_edit("# target_section\n"),
                uri(1).to_edit("# test\n\n[target_section](extracted-1)\n"),
            ]
            .to_workspace_edit()
            .to_code_action("Extract section", "custom.extract"),
        );
}

#[test]
fn extract_section_with_multiple_simple_collisions() {
    let mut files = std::collections::HashMap::new();
    files.insert(
        "1".to_string(),
        indoc! {"
        # test

        ## target_section
    "}
        .to_string(),
    );
    files.insert("extracted".to_string(), "# existing content\n".to_string());
    files.insert(
        "extracted-1".to_string(),
        "# existing content 1\n".to_string(),
    );
    files.insert(
        "extracted-2".to_string(),
        "# existing content 2\n".to_string(),
    );

    Fixture::with_options_and_client(files, create_extract_config("extracted", None), "")
        .code_action(
            uri(1).to_code_action_params(2, "custom.extract"),
            vec![
                uri_from("extracted-3").to_create_file(),
                uri_from("extracted-3").to_edit("# target_section\n"),
                uri(1).to_edit("# test\n\n[target_section](extracted-3)\n"),
            ]
            .to_workspace_edit()
            .to_code_action("Extract section", "custom.extract"),
        );
}

fn assert_extracted_with_date_template(source: &str, line: u32, target: &str, extracted: &str) {
    let date = Local::now().date_naive();
    let formatted_date = date.format("%Y-%m-%d").to_string();

    let target_with_date = target.replace("{{today}}", &formatted_date);

    Fixture::with_config(source, create_extract_config("{{today}}", None)).code_action(
        uri(1).to_code_action_params(line, "custom.extract"),
        vec![
            uri_from(&formatted_date).to_create_file(),
            uri_from(&formatted_date).to_edit(extracted),
            uri(1).to_edit(&target_with_date),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "custom.extract"),
    );
}

fn assert_no_action(source: &str, line: u32) {
    Fixture::with_config(source, extract_config())
        .no_code_action(uri(1).to_code_action_params(line, "custom.extract"));
}

#[test]
fn extract_section_with_title_template() {
    Fixture::with_config(
        indoc! {"
            # ParentSection

            ## Target Section
            "},
        create_extract_config("{{title}}", None),
    )
    .code_action(
        uri(1).to_code_action_params(2, "custom.extract"),
        vec![
            uri_from("Target%20Section").to_create_file(),
            uri_from("Target%20Section").to_edit("# Target Section\n"),
            uri(1).to_edit("# ParentSection\n\n[Target Section](Target Section)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "custom.extract"),
    );
}

#[test]
fn extract_section_with_parent_title_template() {
    Fixture::with_config(
        indoc! {"
            # Parent

            ## Child
            "},
        create_extract_config("{{parent.title}}-{{title}}", None),
    )
    .code_action(
        uri(1).to_code_action_params(2, "custom.extract"),
        vec![
            uri_from("Parent-Child").to_create_file(),
            uri_from("Parent-Child").to_edit("# Child\n"),
            uri(1).to_edit("# Parent\n\n[Child](Parent-Child)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "custom.extract"),
    );
}

#[test]
fn extract_section_with_special_characters_in_title() {
    Fixture::with_config(
        indoc! {"
            # Document

            ## Target/With*Special:Chars
            "},
        create_extract_config("{{title}}", None),
    )
    .code_action(
        uri(1).to_code_action_params(2, "custom.extract"),
        vec![
            uri_from("TargetWithSpecialChars").to_create_file(),
            uri_from("TargetWithSpecialChars").to_edit("# Target/With*Special:Chars\n"),
            uri(1).to_edit("# Document\n\n[Target/With*Special:Chars](TargetWithSpecialChars)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "custom.extract"),
    );
}

#[test]
fn extract_section_with_source_title_template() {
    let mut files = std::collections::HashMap::new();
    files.insert(
        "source-document".to_string(),
        indoc! {"
            # Source Document Title

            ## Target Section
            "}
        .to_string(),
    );

    Fixture::with_options_and_client(files, create_extract_config("{{source.title}}-{{title}}", None), "")
        .code_action(
            uri_from("source-document").to_code_action_params(2, "custom.extract"),
            vec![
                uri_from("Source%20Document%20Title-Target%20Section").to_create_file(),
                uri_from("Source%20Document%20Title-Target%20Section").to_edit("# Target Section\n"),
                uri_from("source-document").to_edit(
                    "# Source Document Title\n\n[Target Section](Source Document Title-Target Section)\n",
                ),
            ]
            .to_workspace_edit()
            .to_code_action("Extract section", "custom.extract"),
        );
}

#[test]
fn extract_section_with_source_template() {
    let mut files = std::collections::HashMap::new();
    files.insert(
        "docs/guide".to_string(),
        indoc! {"
            # User Guide

            ## Installation
            "}
        .to_string(),
    );

    Fixture::with_options_and_client(
        files,
        create_extract_config("{{source.file}}-{{title}}", None),
        "",
    )
    .code_action(
        uri_from("docs/guide").to_code_action_params(2, "custom.extract"),
        vec![
            uri_from("docs/guide-Installation").to_create_file(),
            uri_from("docs/guide-Installation").to_edit("# Installation\n"),
            uri_from("docs/guide").to_edit("# User Guide\n\n[Installation](guide-Installation)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "custom.extract"),
    );
}

#[test]
fn extract_section_with_path_template() {
    let mut files = std::collections::HashMap::new();
    files.insert(
        "docs/tutorial/basics".to_string(),
        indoc! {"
            # Basic Tutorial

            ## Getting Started
            "}
        .to_string(),
    );

    Fixture::with_options_and_client(
        files,
        create_extract_config("extracted-{{title}}", None),
        "",
    )
    .code_action(
        uri_from("docs/tutorial/basics").to_code_action_params(2, "custom.extract"),
        vec![
            uri_from("docs/tutorial/extracted-Getting%20Started").to_create_file(),
            uri_from("docs/tutorial/extracted-Getting%20Started").to_edit("# Getting Started\n"),
            uri_from("docs/tutorial/basics")
                .to_edit("# Basic Tutorial\n\n[Getting Started](extracted-Getting Started)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Extract section", "custom.extract"),
    );
}

fn create_extract_config(key_template: &str, link_type: Option<LinkType>) -> Configuration {
    let mut config = Configuration::default();
    config.actions.insert(
        "extract".to_string(),
        BlockAction::Extract(Extract {
            title: "Extract section".to_string(),
            link_type,
            key_template: key_template.to_string(),
        }),
    );
    config
}

fn extract_config() -> Configuration {
    create_extract_config("{{id}}", None)
}
