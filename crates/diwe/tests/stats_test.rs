use diwe::stats::KeyStatistics;
use indoc::indoc;
use liwe::graph::Graph;
use liwe::markdown::MarkdownReader;

#[test]
fn list_items_are_not_counted_as_sections() {
    let mut graph = Graph::new();
    graph.from_markdown(
        "key".into(),
        indoc! {"
        # Header

        - item one
        - item two
        "},
        MarkdownReader::new(),
    );
    let stats = KeyStatistics::from_graph(&graph);
    assert_eq!(1, stats[0].sections);
    assert_eq!(1, stats[0].bullet_lists);
}
