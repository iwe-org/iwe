use indoc::indoc;
use liwe::model::config::MarkdownOptions;
use std::str::FromStr;

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
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
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
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
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
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 17),
        goto_definition_response_empty(),
    );
}

#[test]
fn definition_in_paragraph_wiki_link_with_space() {
    Fixture::with(indoc! {"
            # test

            text [[link to something]] text

            "})
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 9),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link%20to%20something.md").unwrap(),
        ),
    )
    .go_to_definition(
        uri(1).to_goto_definition_params(2, 2),
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
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
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
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
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
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
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
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/link.md").unwrap(),
        ),
    );
}

#[test]
fn definition_with_relative_path() {
    Fixture::with_documents(vec![("d/1", "[](2)")]).go_to_definition(
        uri_from("d/1").to_goto_definition_params(0, 0),
        goto_definition_response_single(
            lsp_types::Uri::from_str("file:///basepath/d/2.md").unwrap(),
        ),
    );
}

#[test]
fn definition_external_https_url() {
    Fixture::with(indoc! {"
            # test

            [example](https://example.com)

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 5),
        "https://example.com",
    );
}

#[test]
fn definition_external_http_url() {
    Fixture::with(indoc! {"
            # test

            [example](http://example.com)

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 5),
        "http://example.com",
    );
}

#[test]
fn definition_external_mailto_url() {
    Fixture::with(indoc! {"
            # test

            [email](mailto:test@example.com)

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 5),
        "mailto:test@example.com",
    );
}

#[test]
fn definition_bare_https_url() {
    Fixture::with(indoc! {"
            # test

            Check out https://example.com for more

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 15),
        "https://example.com",
    );
}

#[test]
fn definition_bare_http_url() {
    Fixture::with(indoc! {"
            # test

            Visit http://example.org today

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 10),
        "http://example.org",
    );
}

#[test]
fn definition_bare_mailto_url() {
    Fixture::with(indoc! {"
            # test

            Contact mailto:test@example.com

            "})
    .go_to_definition_external(
        uri(1).to_goto_definition_params(2, 15),
        "mailto:test@example.com",
    );
}
