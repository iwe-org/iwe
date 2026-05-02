use std::sync::Once;

use indoc::indoc;
use pretty_assertions::assert_str_eq;

use liwe::model::config::MarkdownOptions;
use liwe::{
    graph::Graph,
    state::{from_indoc, to_indoc},
};

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        let _ = env_logger::builder().try_init();
    });
}

#[test]
fn links_text_updated_from_frontmatter_title() {
    setup();

    let graph = Graph::import(
        &from_indoc(indoc! {"
            [old](2)
            _
            ---
            title: Frontmatter Title
            ---

            # Header
            "}),
        MarkdownOptions::default(),
        Some("title".to_string()),
    );

    let normalized = to_indoc(&graph.export());

    assert_str_eq!(
        indoc! {"
            [Frontmatter Title](2)
            _
            ---
            title: Frontmatter Title
            ---

            # Header
            "},
        normalized
    );
}

#[test]
fn fallback_to_header_when_frontmatter_key_missing() {
    setup();

    let graph = Graph::import(
        &from_indoc(indoc! {"
            [old](2)
            _
            ---
            other: value
            ---

            # Header Title
            "}),
        MarkdownOptions::default(),
        Some("title".to_string()),
    );

    let normalized = to_indoc(&graph.export());

    assert_str_eq!(
        indoc! {"
            [Header Title](2)
            _
            ---
            other: value
            ---

            # Header Title
            "},
        normalized
    );
}

#[test]
fn fallback_to_header_when_no_frontmatter() {
    setup();

    let graph = Graph::import(
        &from_indoc(indoc! {"
            [old](2)
            _
            # Header Title
            "}),
        MarkdownOptions::default(),
        Some("title".to_string()),
    );

    let normalized = to_indoc(&graph.export());

    assert_str_eq!(
        indoc! {"
            [Header Title](2)
            _
            # Header Title
            "},
        normalized
    );
}

#[test]
fn use_header_when_frontmatter_title_not_configured() {
    setup();

    let graph = Graph::import(
        &from_indoc(indoc! {"
            [old](2)
            _
            ---
            title: Frontmatter
            ---

            # Header
            "}),
        MarkdownOptions::default(),
        None,
    );

    let normalized = to_indoc(&graph.export());

    assert_str_eq!(
        indoc! {"
            [Header](2)
            _
            ---
            title: Frontmatter
            ---

            # Header
            "},
        normalized
    );
}
