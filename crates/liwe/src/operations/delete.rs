use std::collections::HashSet;
use std::slice::from_ref;

use itertools::Itertools;

use crate::graph::{Graph, GraphContext};
use crate::model::node::{NodeIter, Target};
use crate::model::Key;

use super::changes::{Changes, OperationError};

pub fn delete(graph: &Graph, target_key: &Key) -> Result<Changes, OperationError> {
    delete_all(graph, from_ref(target_key))
}

pub fn delete_all(graph: &Graph, target_keys: &[Key]) -> Result<Changes, OperationError> {
    if let Some(missing) = target_keys
        .iter()
        .find(|key| graph.get_node_id(key).is_none())
    {
        return Err(OperationError::NotFound(missing.clone()));
    }

    let mut result = Changes::default();
    let format = graph.format_options().clone();
    let targets: HashSet<&Key> = target_keys.iter().collect();

    let affected: HashSet<Key> = target_keys
        .iter()
        .flat_map(|target_key| {
            graph
                .get_inclusion_edges_to(target_key)
                .into_iter()
                .chain(graph.get_reference_edges_to(target_key))
        })
        .map(|node_id| graph.key_of(node_id))
        .filter(|key| !targets.contains(key))
        .collect();

    let links_to = |target: &Target| targets.contains(&graph.target_key(target));

    for affected_key in affected.iter().sorted() {
        let tree = graph.collect(affected_key);
        let updated = tree
            .remove_inclusion_edges_to(&links_to)
            .remove_inline_links_to(&links_to);
        let markdown = updated.iter().to_text(&affected_key.parent(), &format);
        result.add_update(affected_key.clone(), markdown);
    }

    for target_key in target_keys.iter().unique() {
        result.add_remove(target_key.clone());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;

    fn graph_of(documents: &[(&str, &str)]) -> Graph {
        let mut graph = Graph::new();
        for (key, content) in documents {
            graph.insert_document((*key).into(), content.to_string());
        }
        graph
    }

    #[test]
    fn delete_all_cleans_every_target_out_of_a_shared_parent() {
        let graph = graph_of(&[
            (
                "parent",
                indoc! {"
                    # Parent

                    [First](first)

                    [Second](second)

                    [Third](third)

                    Text with [First](first) and [Second](second).
                "},
            ),
            ("first", "# First\n"),
            ("second", "# Second\n"),
            ("third", "# Third\n"),
        ]);

        let changes = delete_all(&graph, &["first".into(), "second".into()]).unwrap();

        assert_eq!(
            changes.updates,
            vec![(
                Key::name("parent"),
                indoc! {"
                    # Parent

                    [Third](third)

                    Text with First and Second.
                "}
                .to_string()
            )]
        );
        assert_eq!(
            changes.removes,
            vec![Key::name("first"), Key::name("second")]
        );
    }

    #[test]
    fn delete_all_does_not_rewrite_documents_that_are_deleted_too() {
        let graph = graph_of(&[
            ("first", "# First\n\n[Second](second)\n"),
            ("second", "# Second\n"),
        ]);

        let changes = delete_all(&graph, &["first".into(), "second".into()]).unwrap();

        assert_eq!(changes.updates, vec![]);
        assert_eq!(
            changes.removes,
            vec![Key::name("first"), Key::name("second")]
        );
    }

    #[test]
    fn delete_all_reports_a_missing_target() {
        let graph = graph_of(&[("first", "# First\n")]);

        let error = delete_all(&graph, &["first".into(), "missing".into()]).unwrap_err();

        assert_eq!(
            error.to_string(),
            OperationError::NotFound("missing".into()).to_string()
        );
    }
}
