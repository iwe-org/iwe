use indoc::indoc;
use lsp_types::{
    InlayHint, InlayHintLabel, InlayHintParams, Position, Range, TextDocumentIdentifier,
};

mod fixture;
use crate::fixture::*;

#[test]
fn single_ref() {
    assert_inlay_hints(
        indoc! {"
            # test
            _
            # header hint

            [test](1)
            "},
        "↖header hint",
    );
}

#[test]
fn non_existent_ref() {
    assert_no_hints(
        indoc! {"
            # test

            [test](test)
            "},
        "1",
    );
}

#[test]
fn non_existent_key() {
    assert_no_hints(
        indoc! {"
        "},
        "not-a-key",
    );
}

#[test]
fn single_multiple_refs_from_same_key() {
    assert_inlay_hints(
        indoc! {"
            # test
            _
            # header hint

            [test](1)

            [test](1)

            "},
        "↖header hint",
    );
}

#[test]
fn no_refs() {
    assert_no_hints(
        indoc! {"
            # test
            _
            # header hint
            "},
        "1",
    );
}

#[test]
fn multiple_refs() {
    assert_multiple_hints(
        indoc! {"
            # test
            _
            # header hint

            [test](1)
            _
            # header hint 2

            [test](1)
            "},
        "↖header hint",
        "↖header hint 2",
    );
}

#[test]
fn block_reference() {
    assert_no_hints(
        indoc! {"
            para

            [test](2)
            _
            # test
            "},
        "1",
    );
}

#[test]
fn block_reference_2() {
    assert_inlay_hint_at(
        indoc! {"
            para

            [test](2)
            _
            # test
            _
            # from

            [test](2)
            "},
        "↖from",
        2,
    );
}

#[test]
fn block_reference_multiple_from_the_same_key() {
    assert_inlay_hint_at(
        indoc! {"
            para

            [test](2)
            _
            # test
            _
            # from

            [test](2)

            [test](2)
            "},
        "↖from",
        2,
    );
}

fn assert_inlay_hint_at(source: &str, hint_text: &str, line: u32) {
    let fixture = Fixture::with(source);

    fixture.inlay_hint(
        InlayHintParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            work_done_progress_params: Default::default(),
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        },
        vec![InlayHint {
            label: InlayHintLabel::String(hint_text.to_string()),
            position: Position::new(line, 120),
            kind: None,
            text_edits: None,
            tooltip: None,
            padding_left: Some(true),
            padding_right: None,
            data: None,
        }],
    )
}

fn assert_inlay_hints(source: &str, hint_text: &str) {
    let fixture = Fixture::with(source);

    fixture.inlay_hint(
        InlayHintParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            work_done_progress_params: Default::default(),
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        },
        vec![InlayHint {
            label: InlayHintLabel::String(hint_text.to_string()),
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

fn assert_no_hints(source: &str, key: &str) {
    let fixture = Fixture::with(source);

    fixture.inlay_hint(
        InlayHintParams {
            text_document: TextDocumentIdentifier { uri: uri_from(key) },
            work_done_progress_params: Default::default(),
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        },
        vec![],
    )
}

fn assert_multiple_hints(source: &str, hint_text: &str, hint_text2: &str) {
    let fixture = Fixture::with(source);

    fixture.inlay_hint(
        InlayHintParams {
            text_document: TextDocumentIdentifier { uri: uri(1) },
            work_done_progress_params: Default::default(),
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        },
        vec![
            InlayHint {
                label: InlayHintLabel::String(hint_text.to_string()),
                position: Position::new(0, 120),
                kind: None,
                text_edits: None,
                tooltip: None,
                padding_left: Some(true),
                padding_right: None,
                data: None,
            },
            InlayHint {
                label: InlayHintLabel::String(hint_text2.to_string()),
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
