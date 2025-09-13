use indoc::indoc;
use liwe::model::config::MarkdownOptions;

mod fixture;
use crate::fixture::*;

#[test]
fn no_definition() {
    Fixture::new().go_to_definition(
        uri(1).to_goto_definition_params(0, 0),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition() {
    Fixture::with(indoc! {"
            # test

            [test](link)

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 0),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    );
}

#[test]
fn definition_in_paragraph() {
    Fixture::with(indoc! {"
            # test

            text [test](link) text

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 5),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 17),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_paragraph_wiki_link() {
    Fixture::with(indoc! {"
            # test

            text [[link]] text

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 5),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 17),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_paragraph_piped_wiki_link() {
    Fixture::with(indoc! {"
            # test

            text [[link|title]] text

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 7),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 1),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_list() {
    Fixture::with(indoc! {"
            # test

            - [test](link)

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 5),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    );
}

#[test]
fn definition_in_nested_list() {
    Fixture::with(indoc! {"
            # test

            - list
              - item
              - [test](link)

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(4, 8),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    );
}

#[test]
fn definition_with_md_extension() {
    Fixture::with_options(
        indoc! {"
            # test

            [test](link.md)

            "},
        MarkdownOptions {
            refs_extension: ".md".to_string(),
            ..Default::default()
        },
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 0),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    );
}

#[test]
fn definition_with_relative_path() {
    Fixture::with_documents(vec![("d/1", "[](2)")]).go_to_definition(
        uri_from("d/1").to_goto_definition_params(0, 0),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/d/2.md").unwrap()),
    );
}
