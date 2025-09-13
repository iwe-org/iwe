use indoc::indoc;
use liwe::model::config::MarkdownOptions;

mod fixture;
use crate::fixture::*;

#[test]
fn no_definition() {
    let fixture = Fixture::new();

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(0, 0),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition() {
    let fixture = Fixture::with(indoc! {"
            # test

            [test](link)

            "});

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(2, 0),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    )
}

#[test]
fn definition_in_paragraph() {
    let fixture = Fixture::with(indoc! {"
            # test

            text [test](link) text

            "});

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(2, 5),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    );

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(2, 17),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_paragraph_wiki_link() {
    let fixture = Fixture::with(indoc! {"
            # test

            text [[link]] text

            "});

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(2, 5),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    );

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(2, 17),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_paragraph_piped_wiki_link() {
    let fixture = Fixture::with(indoc! {"
            # test

            text [[link|title]] text

            "});

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(2, 7),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    );

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(2, 1),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_list() {
    let fixture = Fixture::with(indoc! {"
            # test

            - [test](link)

            "});

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(2, 5),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    );
}

#[test]
fn definition_in_nested_list() {
    let fixture = Fixture::with(indoc! {"
            # test

            - list
              - item
              - [test](link)

            "});

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(4, 8),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    );
}

#[test]
fn definition_with_md_extension() {
    let fixture = Fixture::with_options(
        indoc! {"
            # test

            [test](link.md)

            "},
        MarkdownOptions {
            refs_extension: ".md".to_string(),
            ..Default::default()
        },
    );

    fixture.go_to_definition(
        uri(1).to_goto_definition_params(2, 0),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/link.md").unwrap()),
    )
}

#[test]
fn definition_with_relative_path() {
    let fixture = Fixture::with_documents(vec![("d/1", "[](2)")]);

    fixture.go_to_definition(
        uri_from("d/1").to_goto_definition_params(0, 0),
        goto_definition_response_single(lsp_types::Url::parse("file:///basepath/d/2.md").unwrap()),
    );
}
