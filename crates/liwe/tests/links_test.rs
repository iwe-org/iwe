use std::sync::Once;

use indoc::indoc;
use pretty_assertions::assert_str_eq;

use liwe::{
    graph::Graph,
    markdown::MarkdownReader,
    model::graph::MarkdownOptions,
    state::{new_form_indoc, to_indoc},
};

#[test]
fn links_text_updated_from_referenced_header() {
    compare(
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
fn ref_links_updated_two_ways() {
    compare(
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
    compare(
        indoc! {"
            [some title](key)
            "},
        indoc! {"
            [some title](key)
            "},
    );
}

#[test]
fn keep_title_there_is_no_title_in_referenced_file() {
    compare(
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
    compare(
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

fn compare(expected: &str, denormalized: &str) {
    setup();

    let graph = Graph::import(
        &new_form_indoc(denormalized),
        MarkdownOptions {
            refs_extension: "".to_string(),
        },
    );

    let normalized = to_indoc(&graph.export());

    println!("actual graph \n{:#?}", graph);
    println!("{}", expected);
    println!("normalized:");
    println!("{}", normalized);

    assert_str_eq!(expected, normalized);
}

fn compare_with_extensions(expected: &str, denormalized: &str) {
    setup();

    let mut graph = Graph::new_with_options(MarkdownOptions {
        refs_extension: ".md".to_string(),
    });

    graph.from_markdown("key", denormalized, MarkdownReader::new());

    let normalized = graph.to_markdown("key");

    println!("actual graph \n{:#?}", graph);
    println!("{}", expected);
    println!("normalized:");
    println!("{}", normalized);

    assert_str_eq!(expected, normalized);
}

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::builder().init();
    });
}
