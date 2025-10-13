use indoc::indoc;
use liwe::model::config::{ActionDefinition, Configuration, Link, LinkType};

mod fixture;
use crate::fixture::*;

#[test]
fn link_word_at_start_of_line() {
    assert_linked(
        indoc! {"
            # test

            word in paragraph
            "},
        2,
        0,
        indoc! {"
            # test

            [word](2) in paragraph
            "},
        indoc! {"
            # word
        "},
    );
}

#[test]
fn link_word_in_middle_of_line() {
    assert_linked(
        indoc! {"
            # test

            this is a word here
            "},
        2,
        10, // cursor on 'w' in "word"
        indoc! {"
            # test

            this is a [word](2) here
            "},
        indoc! {"
            # word
        "},
    );
}

#[test]
fn link_word_at_end_of_line() {
    assert_linked(
        indoc! {"
            # test

            this is a word
            "},
        2,
        10, // cursor on 'w' in "word"
        indoc! {"
            # test

            this is a [word](2)
            "},
        indoc! {"
            # word
        "},
    );
}

#[test]
fn link_word_with_cursor_in_middle() {
    assert_linked(
        indoc! {"
            # test

            important
            "},
        2,
        5, // cursor in middle of "important" (between 'r' and 't')
        indoc! {"
            # test

            [important](2)
            "},
        indoc! {"
            # important
        "},
    );
}

#[test]
fn link_word_with_hyphen() {
    assert_linked(
        indoc! {"
            # test

            multi-word
            "},
        2,
        3,
        indoc! {"
            # test

            [multi-word](2)
            "},
        indoc! {"
            # multi-word
        "},
    );
}

#[test]
fn link_word_with_underscore() {
    assert_linked(
        indoc! {"
            # test

            some_function
            "},
        2,
        5,
        indoc! {"
            # test

            [some_function](2)
            "},
        indoc! {"
            # some_function
        "},
    );
}

#[test]
fn link_word_wiki_link() {
    assert_linked_wiki(
        indoc! {"
            # test

            word here
            "},
        2,
        0,
        indoc! {"
            # test

            [[2]] here
            "},
        indoc! {"
            # word
        "},
    );
}

#[test]
fn no_action_on_multiline_selection() {
    use lsp_types::{
        CodeActionContext, CodeActionKind, CodeActionParams, PartialResultParams, Position, Range,
        TextDocumentIdentifier, WorkDoneProgressParams,
    };

    let params = CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        range: Range::new(Position::new(2, 0), Position::new(3, 5)),
        context: CodeActionContext {
            only: Some(vec![CodeActionKind::from("custom.link".to_string())]),
            ..Default::default()
        },
        work_done_progress_params: WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: PartialResultParams {
            partial_result_token: None,
        },
    };

    Fixture::with_config(
        indoc! {"
            # test

            first line
            second line
            "},
        link_config(),
    )
    .no_code_action(params);
}

#[test]
fn no_action_on_empty_space() {
    Fixture::with_config(
        indoc! {"
            # test

            word   word
            "},
        link_config(),
    )
    .no_code_action(uri(1).to_code_action_params_at_position(2, 6, "custom.link"));
}

#[test]
fn link_word_in_list_item() {
    assert_linked(
        indoc! {"
            # test

            - item with word
            "},
        2,
        12, // cursor on 'w' in "word"
        indoc! {"
            # test

            - item with [word](2)
            "},
        indoc! {"
            # word
        "},
    );
}

#[test]
fn link_word_in_header() {
    assert_linked(
        indoc! {"
            # test

            ## section word here
            "},
        2,
        11, // cursor on 'w' in "word"
        indoc! {"
            # test

            ## section [word](2) here
            "},
        indoc! {"
            # word
        "},
    );
}

#[test]
fn link_word_with_collision() {
    let mut files = std::collections::HashMap::new();
    files.insert(
        "1".to_string(),
        indoc! {"
        # test

        word here
    "}
        .to_string(),
    );
    files.insert("existing".to_string(), "# existing\n".to_string());

    Fixture::with_options_and_client(files, create_link_config("existing", None), "").code_action(
        uri(1).to_code_action_params(2, "custom.link"),
        vec![
            uri_from("existing-1").to_create_file(),
            uri_from("existing-1").to_edit("# word\n"),
            uri(1).to_edit("# test\n\n[word](existing-1) here\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Link word", "custom.link"),
    );
}

#[test]
fn link_word_with_slug_template() {
    Fixture::with_config(
        indoc! {"
            # test

            Important Word
            "},
        create_link_config("{{slug}}", None),
    )
    .code_action(
        uri(1).to_code_action_params(2, "custom.link"),
        vec![
            uri_from("important").to_create_file(),
            uri_from("important").to_edit("# Important\n"),
            uri(1).to_edit("# test\n\n[Important](important) Word\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Link word", "custom.link"),
    );
}

#[test]
fn link_word_with_title_template() {
    Fixture::with_config(
        indoc! {"
            # test

            MyWord
            "},
        create_link_config("{{title}}", None),
    )
    .code_action(
        uri(1).to_code_action_params(2, "custom.link"),
        vec![
            uri_from("MyWord").to_create_file(),
            uri_from("MyWord").to_edit("# MyWord\n"),
            uri(1).to_edit("# test\n\n[MyWord](MyWord)\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Link word", "custom.link"),
    );
}

#[test]
fn link_unicode_word() {
    assert_linked(
        indoc! {"
            # test

            test here
            "},
        2,
        0,
        indoc! {"
            # test

            [test](2) here
            "},
        indoc! {"
            # test
        "},
    );
}

#[test]
fn link_selected_text_simple() {
    assert_linked_with_range(
        indoc! {"
            # test

            this is some text here
            "},
        2,
        8,  // start of "some"
        12, // end of "some"
        indoc! {"
            # test

            this is [some](2) text here
            "},
        indoc! {"
            # some
        "},
    );
}

#[test]
fn link_selected_text_with_spaces() {
    assert_linked_with_range(
        indoc! {"
            # test

            this is some text here
            "},
        2,
        8,  // start of "some text"
        17, // end of "some text"
        indoc! {"
            # test

            this is [some text](2) here
            "},
        indoc! {"
            # some text
        "},
    );
}

#[test]
fn link_selected_text_at_start() {
    assert_linked_with_range(
        indoc! {"
            # test

            word in paragraph
            "},
        2,
        0, // start of "word"
        4, // end of "word"
        indoc! {"
            # test

            [word](2) in paragraph
            "},
        indoc! {"
            # word
        "},
    );
}

#[test]
fn link_selected_text_at_end() {
    assert_linked_with_range(
        indoc! {"
            # test

            this is a word
            "},
        2,
        10, // start of "word"
        14, // end of "word"
        indoc! {"
            # test

            this is a [word](2)
            "},
        indoc! {"
            # word
        "},
    );
}

#[test]
fn link_selected_text_multi_word_phrase() {
    assert_linked_with_range(
        indoc! {"
            # test

            this is a very important concept
            "},
        2,
        10, // start of "very important"
        24, // end of "very important"
        indoc! {"
            # test

            this is a [very important](2) concept
            "},
        indoc! {"
            # very important
        "},
    );
}

#[test]
fn link_selected_text_with_punctuation() {
    assert_linked_with_range(
        indoc! {"
            # test

            this is important!
            "},
        2,
        8,  // start of "important"
        17, // end of "important"
        indoc! {"
            # test

            this is [important](2)!
            "},
        indoc! {"
            # important
        "},
    );
}

#[test]
fn link_selected_text_wiki_link() {
    use lsp_types::{
        CodeActionContext, CodeActionKind, CodeActionParams, PartialResultParams, Position, Range,
        TextDocumentIdentifier, WorkDoneProgressParams,
    };

    let params = CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        range: Range::new(Position::new(2, 8), Position::new(2, 12)), // Select "some"
        context: CodeActionContext {
            only: Some(vec![CodeActionKind::from("custom.link".to_string())]),
            ..Default::default()
        },
        work_done_progress_params: WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: PartialResultParams {
            partial_result_token: None,
        },
    };

    Fixture::with_config(
        indoc! {"
            # test

            this is some text
            "},
        create_link_config("{{id}}", Some(LinkType::WikiLink)),
    )
    .code_action(
        params,
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit("# some\n"),
            uri(1).to_edit("# test\n\nthis is [[2]] text\n"),
        ]
        .to_workspace_edit()
        .to_code_action("Link word", "custom.link"),
    );
}

fn assert_linked_with_range(
    source: &str,
    line: u32,
    start_char: u32,
    end_char: u32,
    target: &str,
    extracted: &str,
) {
    use lsp_types::{
        CodeActionContext, CodeActionKind, CodeActionParams, PartialResultParams, Position, Range,
        TextDocumentIdentifier, WorkDoneProgressParams,
    };

    let params = CodeActionParams {
        text_document: TextDocumentIdentifier { uri: uri(1) },
        range: Range::new(
            Position::new(line, start_char),
            Position::new(line, end_char),
        ),
        context: CodeActionContext {
            only: Some(vec![CodeActionKind::from("custom.link".to_string())]),
            ..Default::default()
        },
        work_done_progress_params: WorkDoneProgressParams {
            work_done_token: None,
        },
        partial_result_params: PartialResultParams {
            partial_result_token: None,
        },
    };

    Fixture::with_config(source, link_config()).code_action(
        params,
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit(extracted),
            uri(1).to_edit(target),
        ]
        .to_workspace_edit()
        .to_code_action("Link word", "custom.link"),
    );
}

fn assert_linked(source: &str, line: u32, character: u32, target: &str, extracted: &str) {
    Fixture::with_config(source, link_config()).code_action(
        uri(1).to_code_action_params_at_position(line, character, "custom.link"),
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit(extracted),
            uri(1).to_edit(target),
        ]
        .to_workspace_edit()
        .to_code_action("Link word", "custom.link"),
    );
}

fn assert_linked_wiki(source: &str, line: u32, character: u32, target: &str, extracted: &str) {
    Fixture::with_config(
        source,
        create_link_config("{{id}}", Some(LinkType::WikiLink)),
    )
    .code_action(
        uri(1).to_code_action_params_at_position(line, character, "custom.link"),
        vec![
            uri(2).to_create_file(),
            uri(2).to_edit(extracted),
            uri(1).to_edit(target),
        ]
        .to_workspace_edit()
        .to_code_action("Link word", "custom.link"),
    );
}

fn create_link_config(key_template: &str, link_type: Option<LinkType>) -> Configuration {
    let mut config = Configuration::default();
    config.actions.insert(
        "link".to_string(),
        ActionDefinition::Link(Link {
            title: "Link word".to_string(),
            link_type,
            key_template: key_template.to_string(),
        }),
    );
    config
}

fn link_config() -> Configuration {
    create_link_config("{{id}}", None)
}
