use liwe::graph::{Graph, GraphContext};
use liwe::markdown::MarkdownReader;
use liwe::model::config::MarkdownOptions;
use liwe::model::node::NodePointer;
use liwe::model::State;

const SIBLINGS: usize = 4000;
const STACK_SIZE: usize = 256 * 1024;

fn headers_document() -> String {
    let mut blocks = vec!["# title".to_string()];
    blocks.extend((0..SIBLINGS).map(|i| format!("## section {i}")));
    format!("{}\n", blocks.join("\n\n"))
}

fn paragraphs_document() -> String {
    let blocks = (0..SIBLINGS)
        .map(|i| format!("paragraph {i}"))
        .collect::<Vec<_>>();
    format!("{}\n", blocks.join("\n\n"))
}

fn list_document() -> String {
    let mut out = String::new();
    for i in 0..SIBLINGS {
        out.push_str(&format!("- item {i}\n"));
    }
    out
}

fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(f)
        .expect("to spawn")
        .join()
        .expect("to join");
}

fn read_graph(content: &str) -> Graph {
    let mut graph = Graph::new();
    graph.from_markdown("key".into(), content, MarkdownReader::new());
    graph
}

fn assert_round_trips(content: String) {
    on_small_stack(move || {
        assert_eq!(content, read_graph(&content).to_markdown(&"key".into()));
    });
}

#[test]
fn from_markdown_reads_wide_headers() {
    assert_round_trips(headers_document());
}

#[test]
fn from_markdown_reads_wide_paragraphs() {
    assert_round_trips(paragraphs_document());
}

#[test]
fn from_markdown_reads_wide_list() {
    assert_round_trips(list_document());
}

#[test]
fn build_key_from_iter_rebuilds_wide_document() {
    on_small_stack(|| {
        let content = headers_document();
        let source = read_graph(&content);
        let tree = (&source).collect(&"key".into());

        let mut target = Graph::new();
        target.build_key_from_iter(&"key".into(), tree.iter());

        assert_eq!(content, target.to_markdown(&"key".into()));
    });
}

#[test]
fn insert_from_iter_rebuilds_wide_document() {
    on_small_stack(|| {
        let content = list_document();
        let source = read_graph(&content);
        let tree = (&source).collect(&"key".into());

        let mut target = Graph::new();
        target
            .build_key(&"key".into())
            .insert_from_iter(tree.iter());

        assert_eq!(content, target.to_markdown(&"key".into()));
    });
}

#[test]
fn import_reads_wide_document() {
    on_small_stack(|| {
        let content = paragraphs_document();
        let state: State = vec![("key".to_string(), content.clone())]
            .into_iter()
            .collect();

        let graph = Graph::import(&state, MarkdownOptions::default(), None);

        assert_eq!(content, graph.to_markdown(&"key".into()));
    });
}

#[test]
fn pointer_walks_wide_document() {
    on_small_stack(|| {
        let graph = read_graph(&headers_document());
        let context = &graph;
        let document_id = graph.get_document_id(&"key".into());
        let ids = context.node(document_id).get_all_sub_nodes();

        assert_eq!(SIBLINGS + 2, ids.len());

        let last = context.node(*ids.last().expect("to have nodes"));

        assert!(!last.is_in_list());
        assert!(last.to_parent().is_some());
        assert!(last.to_document().is_some());
    });
}
