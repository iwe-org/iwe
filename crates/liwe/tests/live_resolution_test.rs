use std::collections::BTreeSet;

use itertools::Itertools;
use pretty_assertions::assert_eq;

use liwe::graph::{Graph, GraphContext};
use liwe::markdown::MarkdownReader;
use liwe::model::config::{MarkdownOptions, WikiLinkPath};
use liwe::model::inline::inlines_to_markdown;
use liwe::model::node::Node;
use liwe::model::tree::Tree;
use liwe::model::{Key, State};
use liwe::operations::{delete, rename};
use liwe::query::edges::references;

#[test]
fn wiki_link_resolves_document_inserted_later() {
    let mut graph = Graph::new();
    graph.insert_document("note".into(), "[[target]]\n".to_string());
    assert_eq!(Vec::<Key>::new(), included_by(&graph, "sub/target"));

    graph.insert_document("sub/target".into(), "# Target\n".to_string());

    assert_eq!(vec![Key::name("note")], included_by(&graph, "sub/target"));
    assert_eq!(Vec::<Key>::new(), included_by(&graph, "target"));
    assert_matches_fresh_load(&graph);
}

#[test]
fn wiki_link_resolves_document_parsed_later() {
    let mut graph = Graph::new();
    graph.from_markdown("note".into(), "[[target]]\n", MarkdownReader::new());
    graph.from_markdown("sub/target".into(), "# Target\n", MarkdownReader::new());

    assert_eq!(vec![Key::name("note")], included_by(&graph, "sub/target"));
    assert_matches_fresh_load(&graph);
}

#[test]
fn wiki_link_resolves_document_updated_later() {
    let mut graph = Graph::new();
    graph.update_document("note".into(), "text [[target]]\n".to_string());
    graph.update_document("sub/target".into(), "# Target\n".to_string());

    assert_eq!(vec![Key::name("note")], referenced_by(&graph, "sub/target"));
    assert_matches_fresh_load(&graph);
}

#[test]
fn wiki_link_resolves_prebuilt_document_after_reindex() {
    let mut graph = Graph::new();
    graph.insert_document("note".into(), "[[target]]\n".to_string());
    graph.build_key_from_iter(&"sub/target".into(), prebuilt("sub/target").iter());
    graph.reindex_keys(&["sub/target".into()]);

    assert_eq!(vec![Key::name("note")], included_by(&graph, "sub/target"));
    assert_matches_fresh_load(&graph);
}

#[test]
fn wiki_link_resolves_prebuilt_document_without_reindex() {
    let mut graph = Graph::new();
    graph.insert_document("note".into(), "[[target]]\n".to_string());
    graph.build_key_from_iter(
        &"sub/target".into(),
        prebuilt_with("sub/target", "# Target\n\n[[note]]\n").iter(),
    );

    assert_eq!(vec![Key::name("note")], included_by(&graph, "sub/target"));
    assert_eq!(Vec::<Key>::new(), included_by(&graph, "note"));

    graph.reindex_keys(&["sub/target".into()]);

    assert_eq!(vec![Key::name("sub/target")], included_by(&graph, "note"));
    assert_matches_fresh_load(&graph);
}

#[test]
fn shallower_document_takes_over_and_releases_wiki_link() {
    let mut graph = Graph::new();
    graph.insert_document("note".into(), "[[target]]\n".to_string());
    graph.insert_document("deep/sub/target".into(), "# Deep\n".to_string());
    assert_eq!(
        vec![Key::name("note")],
        included_by(&graph, "deep/sub/target")
    );

    graph.insert_document("target".into(), "# Target\n".to_string());
    assert_eq!(vec![Key::name("note")], included_by(&graph, "target"));
    assert_eq!(Vec::<Key>::new(), included_by(&graph, "deep/sub/target"));
    assert_matches_fresh_load(&graph);

    graph.remove_document("target".into());
    assert_eq!(
        vec![Key::name("note")],
        included_by(&graph, "deep/sub/target")
    );
    assert_matches_fresh_load(&graph);
}

#[test]
fn removed_target_falls_back_then_dangles_then_resolves_again() {
    let mut graph = Graph::new();
    graph.insert_document("note".into(), "[[target]]\n".to_string());
    graph.insert_document("a/target".into(), "# First\n".to_string());
    graph.insert_document("b/target".into(), "# Second\n".to_string());
    assert_eq!(vec![Key::name("note")], included_by(&graph, "a/target"));

    graph.remove_document("a/target".into());
    assert_eq!(vec![Key::name("note")], included_by(&graph, "b/target"));
    assert_matches_fresh_load(&graph);

    graph.remove_document("b/target".into());
    assert_eq!(vec![Key::name("note")], included_by(&graph, "target"));
    assert_eq!(
        vec![Key::name("target")],
        graph.inclusion_edge_target_keys()
    );
    assert!(!graph.has_key(&"target".into()));
    assert_matches_fresh_load(&graph);

    graph.insert_document("target".into(), "# Target\n".to_string());
    assert_eq!(vec![Key::name("note")], included_by(&graph, "target"));
    assert_matches_fresh_load(&graph);
}

#[test]
fn exact_case_candidate_takes_over_only_matching_case() {
    let mut graph = Graph::new();
    graph.insert_document("first".into(), "[[Target]]\n".to_string());
    graph.insert_document("second".into(), "[[target]]\n".to_string());
    graph.insert_document("a/target".into(), "# Lower\n".to_string());
    assert_eq!(
        vec![Key::name("first"), Key::name("second")],
        included_by(&graph, "a/target")
    );

    graph.insert_document("b/Target".into(), "# Upper\n".to_string());

    assert_eq!(vec![Key::name("first")], included_by(&graph, "b/Target"));
    assert_eq!(vec![Key::name("second")], included_by(&graph, "a/target"));
    assert_matches_fresh_load(&graph);
}

#[test]
fn block_with_two_case_variants_links_to_both_targets() {
    let graph = import(vec![
        ("note", "text [[A]] and [[a]]\n"),
        ("x/A", "# Upper\n"),
        ("y/a", "# Lower\n"),
    ]);

    assert_eq!(vec![Key::name("note")], referenced_by(&graph, "x/A"));
    assert_eq!(vec![Key::name("note")], referenced_by(&graph, "y/a"));
}

#[test]
fn nested_wiki_links_resolve_live() {
    let mut graph = Graph::new();
    graph.insert_document("emphasis".into(), "text *[[target]]*\n".to_string());
    graph.insert_document(
        "table".into(),
        "| a | b |\n| - | - |\n| [[target]] | c |\n".to_string(),
    );
    graph.insert_document("sub/target".into(), "# Target\n".to_string());

    assert_eq!(
        vec![Key::name("emphasis"), Key::name("table")],
        referenced_by(&graph, "sub/target")
    );
    assert_matches_fresh_load(&graph);
}

#[test]
fn wiki_link_inside_link_label_resolves_live() {
    let mut graph = Graph::new();
    graph.insert_document(
        "note".into(),
        "text [see [[target]]](https://example.com)\n".to_string(),
    );
    assert_eq!(
        vec![Key::name("target")],
        graph.get_reference_edges_in(&"note".into())
    );

    graph.insert_document("sub/target".into(), "# Target\n".to_string());

    assert_eq!(
        vec![Key::name("sub/target")],
        graph.get_reference_edges_in(&"note".into())
    );
    assert_eq!(vec![Key::name("note")], referenced_by(&graph, "sub/target"));
    assert_matches_fresh_load(&graph);
}

#[test]
fn ingestion_order_does_not_change_the_graph() {
    let documents = vec![
        ("note", "[[target]]\n\ntext [[other]]\n"),
        ("sub/target", "# Target\n\ntext [[note]]\n"),
        ("other", "# Other\n\n[[target]]\n"),
    ];
    let fresh = import(documents.clone());

    for order in documents.iter().permutations(documents.len()) {
        let mut graph = Graph::new();
        for (key, content) in order {
            graph.insert_document(Key::name(key), content.to_string());
        }
        assert_eq!(fresh.export(), graph.export());
        assert_eq!(edges(&fresh), edges(&graph));
    }
}

#[test]
fn dangling_links_are_reported_as_outbound_references() {
    let graph = import(vec![("note", "text [[missing]] and [other](gone)\n")]);

    assert_eq!(
        vec![Key::name("missing"), Key::name("gone")],
        graph.get_reference_edges_in(&"note".into())
    );
    assert_eq!(
        vec!["gone".to_string(), "missing".to_string()],
        references(&graph, &"note".into())
            .into_iter()
            .map(|edge| edge.key)
            .collect_vec()
    );
}

#[test]
fn wiki_display_follows_current_resolution() {
    let documents = vec![
        ("note", "text [[target]] and [[missing]]\n"),
        ("sub/target", "# Target\n"),
    ];

    let mut full = import_with(documents.clone(), WikiLinkPath::Full);
    assert_eq!(
        "text [[sub/target]] and [[missing]]\n",
        full.to_markdown(&"note".into())
    );
    full.insert_document("target".into(), "# Root\n".to_string());
    assert_eq!(
        "text [[target]] and [[missing]]\n",
        full.to_markdown(&"note".into())
    );

    let mut short = import_with(documents.clone(), WikiLinkPath::Short);
    assert_eq!(
        "text [[target]] and [[missing]]\n",
        short.to_markdown(&"note".into())
    );
    short.insert_document("other/target".into(), "# Other\n".to_string());
    assert_eq!(
        "text [[other/target]] and [[missing]]\n",
        short.to_markdown(&"note".into())
    );

    let mut preserve = import_with(documents, WikiLinkPath::Preserve);
    preserve.insert_document("target".into(), "# Root\n".to_string());
    assert_eq!(
        "text [[target]] and [[missing]]\n",
        preserve.to_markdown(&"note".into())
    );
}

#[test]
fn rename_to_new_folder_keeps_bare_wiki_link_untouched() {
    let mut graph = import(vec![("note", "[[target]]\n"), ("a/target", "# Target\n")]);

    let changes = rename(&graph, &"a/target".into(), &"b/target".into()).unwrap();

    assert_eq!(Vec::<(Key, String)>::new(), changes.updates);
    assert_eq!(vec![Key::name("a/target")], changes.removes);

    graph.remove_document("a/target".into());
    graph.insert_document("b/target".into(), "# Target\n".to_string());
    assert_eq!(vec![Key::name("note")], included_by(&graph, "b/target"));
    assert_matches_fresh_load(&graph);
}

#[test]
fn rename_to_new_folder_rewrites_path_suffixed_wiki_link() {
    let graph = import(vec![("note", "[[a/target]]\n"), ("a/target", "# Target\n")]);

    let changes = rename(&graph, &"a/target".into(), &"b/target".into()).unwrap();

    assert_eq!(
        vec![(Key::name("note"), "[[b/target]]\n".to_string())],
        changes.updates
    );
}

#[test]
fn rename_to_unique_file_name_writes_bare_name() {
    let graph = import(vec![("note", "[[target]]\n"), ("a/target", "# Target\n")]);

    let changes = rename(&graph, &"a/target".into(), &"a/renamed".into()).unwrap();

    assert_eq!(
        vec![(Key::name("note"), "[[renamed]]\n".to_string())],
        changes.updates
    );
}

#[test]
fn rename_to_shared_file_name_writes_shortest_unique_suffix() {
    let graph = import(vec![
        ("note", "[[target]]\n"),
        ("a/x/target", "# Target\n"),
        ("b/x/renamed", "# Other\n"),
    ]);

    let changes = rename(&graph, &"a/x/target".into(), &"a/x/renamed".into()).unwrap();

    assert_eq!(
        vec![(Key::name("note"), "[[a/x/renamed]]\n".to_string())],
        changes.updates
    );
}

#[test]
fn rename_keeps_full_path_wiki_link_full() {
    let graph = import(vec![("note", "[[a/target]]\n"), ("a/target", "# Target\n")]);

    let changes = rename(&graph, &"a/target".into(), &"a/renamed".into()).unwrap();

    assert_eq!(
        vec![(Key::name("note"), "[[a/renamed]]\n".to_string())],
        changes.updates
    );
}

#[test]
fn rename_keeps_link_to_another_note_with_the_new_name() {
    let graph = import(vec![
        ("other", "[[renamed]]\n"),
        ("note", "[[target]]\n"),
        ("a/target", "# Target\n"),
        ("c/renamed", "# Other\n"),
    ]);

    let changes = rename(&graph, &"a/target".into(), &"a/renamed".into()).unwrap();

    assert_eq!(
        vec![
            (Key::name("note"), "[[a/renamed]]\n".to_string()),
            (Key::name("other"), "[[c/renamed]]\n".to_string()),
        ],
        changes.updates
    );
}

#[test]
fn rename_lets_dangling_link_resolve_to_new_name() {
    let mut graph = import(vec![("other", "[[renamed]]\n"), ("a/target", "# Target\n")]);

    let changes = rename(&graph, &"a/target".into(), &"a/renamed".into()).unwrap();

    assert_eq!(Vec::<(Key, String)>::new(), changes.updates);

    graph.remove_document("a/target".into());
    graph.insert_document("a/renamed".into(), "# Target\n".to_string());
    assert_eq!(vec![Key::name("other")], included_by(&graph, "a/renamed"));
}

#[test]
fn rename_keeps_fragment_of_markdown_link() {
    let graph = import(vec![
        ("note", "[text](other#section)\n"),
        ("other", "# Other\n\n## Section\n"),
    ]);

    let changes = rename(&graph, &"other".into(), &"moved".into()).unwrap();

    assert_eq!(
        vec![(Key::name("note"), "[text](moved#section)\n".to_string())],
        changes.updates
    );
}

#[test]
fn delete_removes_only_links_resolving_to_deleted_document() {
    let graph = import(vec![
        ("first", "text [[target|title]]\n"),
        ("second", "text [[sub/target|title]]\n"),
        ("target", "# Root\n"),
        ("sub/target", "# Nested\n"),
    ]);

    let changes = delete(&graph, &"target".into()).unwrap();

    assert_eq!(
        vec![(Key::name("first"), "text title\n".to_string())],
        changes.updates
    );
    assert_eq!(vec![Key::name("target")], changes.removes);
}

#[test]
fn rendered_inlines_do_not_depend_on_other_documents() {
    let mut graph = Graph::new();
    graph.insert_document("note".into(), "text [[target]]\n".to_string());
    graph.insert_document("a/target".into(), "# First\n".to_string());
    let rendered = rendered_leaf(&graph, "note");

    graph.insert_document("target".into(), "# Root\n".to_string());
    assert_eq!(rendered, rendered_leaf(&graph, "note"));

    graph.remove_document("target".into());
    graph.remove_document("a/target".into());
    assert_eq!(rendered, rendered_leaf(&graph, "note"));
}

fn rendered_leaf(graph: &Graph, key: &str) -> String {
    let tree = (&graph).collect(&Key::name(key));
    match &tree.children[0].node {
        Node::Leaf(inlines) => inlines_to_markdown(inlines, &MarkdownOptions::default()),
        other => panic!("expected a leaf, got {:?}", other),
    }
}

fn included_by(graph: &Graph, key: &str) -> Vec<Key> {
    graph
        .get_inclusion_edges_to(&Key::name(key))
        .into_iter()
        .map(|id| graph.key_of(id))
        .sorted()
        .dedup()
        .collect()
}

fn referenced_by(graph: &Graph, key: &str) -> Vec<Key> {
    graph
        .get_reference_edges_to(&Key::name(key))
        .into_iter()
        .map(|id| graph.key_of(id))
        .sorted()
        .dedup()
        .collect()
}

fn edges(graph: &Graph) -> BTreeSet<(bool, Key, Key)> {
    let mut targets: BTreeSet<Key> = graph.keys().into_iter().collect();
    targets.extend(graph.inclusion_edge_target_keys());
    targets.extend(graph.reference_edge_target_keys());

    targets
        .iter()
        .flat_map(|target| {
            graph
                .get_inclusion_edges_to(target)
                .into_iter()
                .map(|id| (true, graph.key_of(id), target.clone()))
                .chain(
                    graph
                        .get_reference_edges_to(target)
                        .into_iter()
                        .map(|id| (false, graph.key_of(id), target.clone())),
                )
                .collect_vec()
        })
        .collect()
}

fn assert_matches_fresh_load(graph: &Graph) {
    let fresh = Graph::from_state(&graph.export(), false, graph.format_options().clone(), None);
    assert_eq!(fresh.export(), graph.export());
    assert_eq!(fresh, *graph);
    assert_eq!(edges(&fresh), edges(graph));
}

fn prebuilt(key: &str) -> Tree {
    prebuilt_with(key, "# Target\n")
}

fn prebuilt_with(key: &str, content: &str) -> Tree {
    let mut scratch = Graph::new();
    scratch.insert_document(Key::name(key), content.to_string());
    (&scratch).collect(&Key::name(key))
}

fn import(documents: Vec<(&str, &str)>) -> Graph {
    import_with(documents, WikiLinkPath::Preserve)
}

fn import_with(documents: Vec<(&str, &str)>, wiki_link_path: WikiLinkPath) -> Graph {
    let state: State = documents
        .into_iter()
        .map(|(key, content)| (key.to_string(), content.to_string()))
        .collect();
    Graph::import(
        &state,
        MarkdownOptions {
            wiki_link_path,
            ..Default::default()
        },
        None,
    )
}
