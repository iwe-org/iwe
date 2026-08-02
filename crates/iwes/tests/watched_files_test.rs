use indoc::indoc;
use lsp_types::notification::DidOpenTextDocument;
use lsp_types::{
    DidChangeWatchedFilesParams, DidOpenTextDocumentParams, FileChangeType, FileEvent,
    TextDocumentItem,
};

use crate::fixture::*;

fn deletions(numbers: &[u32]) -> DidChangeWatchedFilesParams {
    DidChangeWatchedFilesParams {
        changes: numbers
            .iter()
            .map(|number| FileEvent {
                uri: uri(*number),
                typ: FileChangeType::DELETED,
            })
            .collect(),
    }
}

#[test]
fn one_batch_of_deletions_is_reflected_in_symbols() {
    let fixture = Fixture::with(indoc! {"
        # kept one
        _
        # gone two
        _
        # gone three
        _
        # gone four
    "});

    assert_eq!(
        fixture.symbol_names(""),
        vec!["gone four", "gone three", "gone two", "kept one"]
    );

    fixture.did_delete_files(deletions(&[2, 3, 4]));

    assert_eq!(fixture.symbol_names(""), vec!["kept one"]);
}

#[test]
fn deletions_split_across_batches_are_reflected_in_symbols() {
    let fixture = Fixture::with(indoc! {"
        # kept one
        _
        # gone two
        _
        # gone three
    "});

    fixture.did_delete_files(deletions(&[2]));
    assert_eq!(fixture.symbol_names(""), vec!["gone three", "kept one"]);

    fixture.did_delete_files(deletions(&[3]));
    assert_eq!(fixture.symbol_names(""), vec!["kept one"]);
}

#[test]
fn deleting_an_open_document_keeps_it_in_symbols() {
    let fixture = Fixture::with(indoc! {"
        # kept one
        _
        # open two
    "});

    fixture.notification::<DidOpenTextDocument>(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri(2),
            language_id: "markdown".to_string(),
            version: 1,
            text: "# open two\n".to_string(),
        },
    });
    fixture.did_delete_files(deletions(&[2]));

    assert_eq!(fixture.symbol_names(""), vec!["kept one", "open two"]);
}
