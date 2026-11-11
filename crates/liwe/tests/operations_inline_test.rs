use indoc::indoc;
use pretty_assertions::assert_eq;

use liwe::graph::{Graph, GraphContext};
use liwe::model::config::InlineType;
use liwe::model::Key;
use liwe::operations::{inline, references, InlineConfig};

#[test]
fn inline_self_reference_as_section_is_rejected() {
    assert_eq!(
        "Invalid target: Document cannot be inlined into itself",
        inline_self_reference(InlineType::Section)
    );
}

#[test]
fn inline_self_reference_as_quote_is_rejected() {
    assert_eq!(
        "Invalid target: Document cannot be inlined into itself",
        inline_self_reference(InlineType::Quote)
    );
}

fn inline_self_reference(inline_type: InlineType) -> String {
    let mut graph = Graph::new();
    graph.insert_document(
        Key::name("index"),
        indoc! {"
            # Index

            ## Section

            [Index](index)
        "}
        .to_string(),
    );

    let key = Key::name("index");
    let tree = (&graph).collect(&key);
    let ref_id = references(&tree, graph.key_index())[0].id;

    let config = InlineConfig {
        inline_type,
        keep_target: false,
    };

    match inline(&graph, &key, ref_id, &config) {
        Ok(_) => "ok".to_string(),
        Err(error) => error.to_string(),
    }
}
