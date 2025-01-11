use std::sync::Once;

use indoc::indoc;
use lib::markdown::MarkdownReader;
use pretty_assertions::assert_str_eq;

use lib::graph::Graph;

#[test]
fn normalization_meta_block() {
    setup();
    compare(
        indoc! {"
        ---
        key: value
        ---

        # header
        "},
        indoc! {"
        ---
        key: value
        ---

        # header
        "},
    );
}

fn compare(expected: &str, denormalized: &str) {
    setup();

    let mut graph = Graph::new();

    graph.from_markdown("key", denormalized, MarkdownReader::new());

    let normalized = graph.to_markdown("key");

    println!("actual graph \n{:#?}", graph);
    println!("expected:\n{}", expected);
    println!("normalized:\n{}", normalized);

    assert_str_eq!(expected, normalized);
}

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        env_logger::builder().init();
    });
}
