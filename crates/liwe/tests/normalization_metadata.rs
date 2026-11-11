use std::sync::Once;

use indoc::indoc;
use liwe::markdown::MarkdownReader;
use pretty_assertions::assert_str_eq;

use liwe::graph::Graph;

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

    graph.from_markdown("key".into(), denormalized, MarkdownReader::new());

    let normalized = graph.to_markdown(&"key".into());

    assert_str_eq!(expected, normalized);
}

#[test]
fn normalization_meta_empty_frontmatter() {
    setup();
    compare(
        indoc! {"
        ---
        {}
        ---

        # Heading
        "},
        "---\n---\n\n# Heading\n",
    );
}

#[test]
fn normalization_meta_block_crlf() {
    setup();
    compare(
        indoc! {"
        ---
        title: value
        date: 2024-01-01
        ---

        # header
        "},
        "---\r\ntitle: value\r\ndate: 2024-01-01\r\n---\r\n\r\n# header\r\n",
    );
}

#[test]
fn normalization_meta_unparseable_yaml_is_kept() {
    let input = indoc! {"
        ---
        count: 99999999999999999999999
        tags: [a
        ---

        # Heading
        "};
    compare(input, input);
}

#[test]
fn normalization_meta_duplicate_key_is_kept() {
    let input = indoc! {"
        ---
        field: 1
        field: 2
        ---

        # Heading

        Text
        "};
    compare(input, input);
}

#[test]
fn normalization_meta_sequence_root_is_kept() {
    let input = indoc! {"
        ---
        - first
        - second
        ---

        # Heading
        "};
    compare(input, input);
}

#[test]
fn normalization_meta_unparseable_yaml_keeps_header_title() {
    setup();
    let mut graph = Graph::new();
    graph.from_markdown(
        "key".into(),
        "---\ntags: [a\n---\n\n# Heading\n",
        MarkdownReader::new(),
    );
    assert_eq!(
        graph.get_key_title(&"key".into()),
        Some("Heading".to_string())
    );
}

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        let _ = env_logger::builder().try_init();
    });
}
