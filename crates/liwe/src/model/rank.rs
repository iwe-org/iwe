use super::NodeId;
use crate::graph::Graph;

pub fn node_rank(graph: &Graph, id: NodeId) -> usize {
    if !graph.visit_node(id).is_primary_section() {
        return 0;
    }

    let inline_refs_count = graph
        .visit_node(id)
        .to_document()
        .and_then(|doc| doc.document_key())
        .map(|key| graph.get_inline_references_to(&key).len())
        .unwrap_or(0);

    let block_refs_count = graph
        .visit_node(id)
        .to_document()
        .and_then(|doc| doc.document_key())
        .map(|key| graph.get_block_references_to(&key).len())
        .unwrap_or(0);

    return inline_refs_count + block_refs_count;
}
