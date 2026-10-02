use itertools::Itertools;
use liwe::graph::{Graph, GraphContext};
use liwe::model::Key;

pub fn require_documents(graph: &Graph, keys: &[Key]) -> Result<(), String> {
    let missing: Vec<&Key> = keys
        .iter()
        .filter(|key| graph.get_node_id(key).is_none())
        .unique()
        .collect();
    match missing.as_slice() {
        [] => Ok(()),
        [key] => Err(format!("Document '{}' not found", key)),
        _ => Err(format!(
            "Documents not found: {}",
            missing.iter().map(|key| format!("'{}'", key)).join(", ")
        )),
    }
}
