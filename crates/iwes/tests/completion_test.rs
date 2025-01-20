use indoc::indoc;
use lsp_types::{
    CompletionItem, CompletionList, CompletionParams, CompletionResponse, Position,
    TextDocumentIdentifier, TextDocumentPositionParams,
};

use fixture::uri;

use crate::fixture::Fixture;

mod fixture;

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
            is_incomplete: true,
            items: vec![CompletionItem {
                documentation: None,
                filter_text: Some("test".to_string()),
                insert_text: Some("[test](1)".to_string()),
                label: "test".to_string(),
                preselect: Some(true),
                ..Default::default()
            }],
        }),
    )
}
