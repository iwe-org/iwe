use indoc::indoc;
use liwe::model::config::MarkdownOptions;
use lsp_types::request::GotoDefinition;
use lsp_types::{
    GotoDefinitionParams, GotoDefinitionResponse, Location, Position, Range,
    TextDocumentIdentifier, TextDocumentPositionParams, Url,
};

use fixture::{uri, uri_from};

use crate::fixture::Fixture;

mod fixture;

#[test]
fn no_definition() {
    let fixture = Fixture::new();

    fixture.assert_response::<GotoDefinition>(
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(0, 0),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        Some(GotoDefinitionResponse::Array(vec![])),
    );
}

#[test]
fn definition() {
    let fixture = Fixture::with(indoc! {"
            # test

            [test](link)

            "});

    fixture.go_to_definition(
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 0),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Scalar(Location::new(
            Url::parse("file:///basepath/link.md").unwrap(),
            Range::default(),
        )),
    )
}

#[test]
fn definition_in_paragraph() {
    let fixture = Fixture::with(indoc! {"
            # test

            text [test](link) text

            "});

    fixture.go_to_definition(
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 5),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Scalar(Location::new(
            Url::parse("file:///basepath/link.md").unwrap(),
            Range::default(),
        )),
    );

    fixture.go_to_definition(
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 17),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Array(vec![]),
    );
}

#[test]
fn definition_in_paragraph_wiki_link() {
    let fixture = Fixture::with(indoc! {"
            # test

            text [[link]] text

            "});

    fixture.go_to_definition(
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 5),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Scalar(Location::new(
            Url::parse("file:///basepath/link.md").unwrap(),
            Range::default(),
        )),
    );

    fixture.go_to_definition(
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 17),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Array(vec![]),
    );
}

#[test]
fn definition_in_paragraph_piped_wiki_link() {
    let fixture = Fixture::with(indoc! {"
            # test

            text [[link|title]] text

            "});

    fixture.go_to_definition(
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 7),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Scalar(Location::new(
            Url::parse("file:///basepath/link.md").unwrap(),
            Range::default(),
        )),
    );

    fixture.go_to_definition(
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 1),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Array(vec![]),
    );
}

#[test]
fn definition_in_list() {
    let fixture = Fixture::with(indoc! {"
            # test

            - [test](link)

            "});

    fixture.go_to_definition(
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 5),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Scalar(Location::new(
            Url::parse("file:///basepath/link.md").unwrap(),
            Range::default(),
        )),
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
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(4, 8),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Scalar(Location::new(
            Url::parse("file:///basepath/link.md").unwrap(),
            Range::default(),
        )),
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
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 0),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Scalar(Location::new(
            Url::parse("file:///basepath/link.md").unwrap(),
            Range::default(),
        )),
    )
}

#[test]
fn definition_with_relative_path() {
    let fixture = Fixture::with_documents(vec![("d/1", "[](2)")]);

    fixture.go_to_definition(
        GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: uri_from("d/1"),
                },
                position: Position::new(0, 0),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        GotoDefinitionResponse::Scalar(Location::new(
            Url::parse("file:///basepath/d/2.md").unwrap(),
            Range::default(),
        )),
    );
}
