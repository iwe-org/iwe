use indoc::indoc;
use lsp_types::{
    InlayHint, InlayHintLabel, InlayHintParams, Position, Range, TextDocumentIdentifier,
};

use fixture::uri;

use crate::fixture::Fixture;

mod fixture;

#[test]
fn single_ref() {
    assert_extracted(
        indoc! {"
            # test
            _
            # header hint

            [test](1)
            "},
        "header hint",
    );
}

#[test]
fn multiple_refs() {
    assert_extracted_multile(
        indoc! {"
            # test
            _
            # header hint

            [test](1)
            _
            # header hint 2

            [test](1)
            "},
        "header hint",
        "header hint 2",
    );
}

fn assert_extracted(source: &str, hint_text: &str) {
    let fixture = Fixture::with(source);
    let text = &format!("↖{}", hint_text);

    fixture.inlay_hint(
        InlayHintParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            work_done_progress_params: Default::default(),
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        },
        vec![InlayHint {
            label: InlayHintLabel::String(text.to_string()),
            position: Position::new(0, 120),
            kind: None,
            text_edits: None,
            tooltip: None,
            padding_left: Some(true),
            padding_right: None,
            data: None,
        }],
    )
}

fn assert_extracted_multile(source: &str, hint_text: &str, hint_text2: &str) {
    let fixture = Fixture::with(source);
    let text = &format!("↖{}", hint_text);
    let text2 = &format!("↖{}", hint_text2);

    fixture.inlay_hint(
        InlayHintParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            work_done_progress_params: Default::default(),
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        },
        vec![
            InlayHint {
                label: InlayHintLabel::String(text.to_string()),
                position: Position::new(0, 120),
                kind: None,
                text_edits: None,
                tooltip: None,
                padding_left: Some(true),
                padding_right: None,
                data: None,
            },
            InlayHint {
                label: InlayHintLabel::String(text2.to_string()),
                position: Position::new(0, 120),
                kind: None,
                text_edits: None,
                tooltip: None,
                padding_left: Some(true),
                padding_right: None,
                data: None,
            },
        ],
    )
}
