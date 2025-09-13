use indoc::indoc;
use lsp_types::{
    CompletionItem, CompletionList, CompletionParams, CompletionResponse, Position,
    TextDocumentIdentifier, TextDocumentPositionParams,
};

mod fixture;
use crate::fixture::*;

#[test]
fn completion_test() {
    let fixture = Fixture::with(indoc! {"
            # test
            "});

    fixture.completion(
        CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri(1) },
                position: Position::new(2, 0),
            },
            context: None,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: vec![CompletionItem {
                documentation: None,
                filter_text: Some("test".to_string()),
                sort_text: Some("test".to_string()),
                insert_text: Some("[test](1)".to_string()),
                label: "🔗 test".to_string(),
                preselect: Some(true),
                ..Default::default()
            }],
        }),
    )
}

#[test]
fn completion_relative_test() {
    let fixture = Fixture::with_documents(vec![
        (
            "dir/sub",
            indoc! {"
            # sub-document
            "},
        ),
        (
            "top",
            indoc! {"
                # top-level
                "},
        ),
    ]);

    fixture.completion(
        CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: uri_from("dir/sub"),
                },
                position: Position::new(2, 0),
            },
            context: None,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        },
        CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: vec![
                CompletionItem {
                    documentation: None,
                    filter_text: Some("sub-document".to_string()),
                    sort_text: Some("sub-document".to_string()),
                    insert_text: Some("[sub-document](sub)".to_string()),
                    label: "🔗 sub-document".to_string(),
                    preselect: Some(true),
                    ..Default::default()
                },
                CompletionItem {
                    documentation: None,
                    filter_text: Some("top-level".to_string()),
                    sort_text: Some("top-level".to_string()),
                    insert_text: Some("[top-level](../top)".to_string()),
                    label: "🔗 top-level".to_string(),
                    preselect: Some(true),
                    ..Default::default()
                },
            ],
        }),
    )
}
