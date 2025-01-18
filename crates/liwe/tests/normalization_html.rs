use std::sync::Once;

use indoc::indoc;
use liwe::markdown::MarkdownReader;
use pretty_assertions::assert_str_eq;

use liwe::graph::Graph;

#[test]
fn normalization_preserve_inline_html() {
    compare(
        indoc! {r##"
            <em>text</em> <bold>text</bold>
            "##},
        indoc! {r##"
            <em>text</em> <bold>text</bold>
            "##},
    );
}

#[test]
fn normalization_drop_html_block() {
    compare(
        indoc! {r##"
            text

            text 2
            "##},
        indoc! {r##"
            text

            <p>para</p>

            text 2
            "##},
    );
}

fn compare(expected: &str, denormalized: &str) {
    setup();

    let mut graph = Graph::new();

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
