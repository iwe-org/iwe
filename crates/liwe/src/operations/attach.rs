use crate::graph::{Graph, GraphContext};
use crate::model::ids::alloc_node_id;
use crate::model::node::{Node, NodeIter, Reference, ReferenceType};
use crate::model::tree::Tree;
use crate::model::Key;

pub enum AttachTarget {
    AlreadyAttached,
    Update(String),
    Create(String),
}

pub fn attach_reference(
    graph: &Graph,
    target_key: &Key,
    reference_key: &Key,
    reference_text: &str,
) -> AttachTarget {
    let format_options = graph.format_options();
    let reference = Tree {
        id: alloc_node_id(),
        line_range: None,
        node: Node::Reference(Reference {
            key: reference_key.clone(),
            text: reference_text.to_string(),
            reference_type: ReferenceType::Regular,
            url: String::new(),
            display_url: None,
        }),
        children: vec![],
    };

    if graph.get_node_id(target_key).is_some() {
        let tree = graph.collect(target_key);
        if tree.get_all_inclusion_edge_keys().contains(reference_key) {
            return AttachTarget::AlreadyAttached;
        }
        AttachTarget::Update(
            tree.attach(reference)
                .iter()
                .to_text(&target_key.parent(), format_options),
        )
    } else {
        AttachTarget::Create(
            reference
                .iter()
                .to_text(&target_key.parent(), format_options),
        )
    }
}
