use std::sync::Once;

use indoc::indoc;
use pretty_assertions::assert_str_eq;

use liwe::model::config::MarkdownOptions;
use liwe::{
    graph::Graph,
    markdown::MarkdownReader,
    model::State,
    state::{from_indoc, to_indoc},
};

#[test]
fn links_text_updated_from_referenced_header() {
    normalize(
        indoc! {"
            [title](2)
            _
            # title
            "},
        indoc! {"
            [another title](2)
            _
            # title
            "},
    );
}

#[test]
fn piped_wiki_links_text_not_updated_from_referenced_header() {
    normalize(
        indoc! {"
            [[2|custom title]]
            _
            # title
            "},
        indoc! {"
            [[2|custom title]]
            _
            # title
            "},
    );
}

#[test]
fn ref_links_updated_two_ways() {
    normalize(
        indoc! {"
            # title 1

            [title 2](2)
            _
            # title 2

            [title 1](1)
            "},
        indoc! {"
            # title 1

            [another title](2)
            _
            # title 2

            [another title](1)
            "},
    );
}

#[test]
fn keep_unknow_refs_as_is() {
    normalize(
        indoc! {"
            [some title](key)
            "},
        indoc! {"
            [some title](key)
            "},
    );
}

#[test]
fn keep_unknow_wiki_refs_as_is() {
    normalize(
        indoc! {"
            [[key]]
            "},
        indoc! {"
            [[key]]
            "},
    );
}

#[test]
fn keep_unknow_piped_wiki_refs_as_is() {
    normalize(
        indoc! {"
            [[key|title]]
            "},
        indoc! {"
            [[key|title]]
            "},
    );
}

#[test]
fn keep_title_there_is_no_title_in_referenced_file() {
    normalize(
        indoc! {"
        [title](2)
        _
        para
        "},
        indoc! {"
        [title](2)
        _
        para
        "},
    );
}

#[test]
fn normalization_drop_extension() {
    normalize(
        indoc! {"
        [title](1)
        "},
        indoc! {"
        [title](1.md)
        "},
    );
}

#[test]
fn normalization_ref_extension() {
    compare_with_extensions(
        indoc! {"
        [text](text.md)
        "},
        indoc! {"
        [text](text)
        "},
    );
}

#[test]
fn normalization_ref_existing_extension() {
    compare_with_extensions(
        indoc! {"
        [text](text.md)
        "},
        indoc! {"
        [text](text.md)
        "},
    );
}

#[test]
fn sub_links_text_updated_from_referenced_header() {
    compare_state(
        vec![("1", "[title](d/2)\n"), ("d/2", "# title\n")],
        vec![("1", "[old title](d/2)"), ("d/2", "# title")],
    );
}

fn normalize(expected: &str, denormalized: &str) {
    setup();

    let graph = Graph::import(
        &from_indoc(denormalized),
        MarkdownOptions {
            refs_extension: String::default(),
        },
    );

    let normalized = to_indoc(&graph.export());

    assert_str_eq!(expected, normalized);
}

pub type Documents = Vec<(&'static str, &'static str)>;

fn compare_state(exp: Documents, den: Documents) {
    setup();

    let expected: State = exp
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let denormalized: State = den
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let graph = Graph::import(
        &denormalized
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        MarkdownOptions {
            refs_extension: String::default(),
        },
    );

    let normalized = &graph.export();

    assert_eq!(&expected, normalized);
}

fn compare_with_extensions(expected: &str, denormalized: &str) {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
    });

    graph.from_markdown("key".into(), denormalized, MarkdownReader::new());

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!(expected, normalized);
}

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::builder().init();
    });
}
