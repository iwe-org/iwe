use itertools::Itertools;
use liwe::{
    graph::{Graph, GraphContext},
    model::{node::NodePointer, tree::Tree, Key, NodeId},
};
use std::collections::{HashMap, HashSet};

#[derive(Default, PartialEq, Debug, Clone)]
struct GraphData {
    sections: HashMap<NodeId, Section>,
    subgraphs: HashMap<String, Subgraph>,
    sub_sections: Vec<(NodeId, NodeId)>,
    references: Vec<(NodeId, NodeId)>,
}

impl GraphData {
    pub fn merge(&mut self, other: GraphData) {
        self.sections.extend(other.sections);
        self.subgraphs.extend(other.subgraphs);
        self.sub_sections.extend(other.sub_sections);
        self.references.extend(other.references);
    }
}

#[derive(PartialEq, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Section {
    pub id: NodeId,
    pub title: String,
}

#[derive(PartialEq, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Subgraph {
    pub key: String,
    pub nodes: Vec<NodeId>,
}

pub struct DotExporter {
    key: Option<Key>,
    depth: u8,
}

impl DotExporter {
    pub fn new(key_filter: Option<Key>, depth_limit: u8) -> Self {
        Self {
            key: key_filter,
            depth: depth_limit,
        }
    }

    pub fn export(&self, graph: &Graph) -> String {
        let mut output = String::new();
        let graph_data = self.graph_data(graph);

        output.push_str(&self.generate_graph_opening());
        output.push_str(&self.generate_nodes(&graph_data));
        output.push_str(&self.generate_subgraphs(&graph_data));
        output.push_str(&self.generate_edges(&graph_data));
        output.push_str(&self.generate_graph_closing());

        output
    }

    fn graph_data(&self, graph: &Graph) -> GraphData {
        let keys = self.filter_keys(graph);
        keys.iter()
            .map(|key| {
                let graph_data = DotExporter::build_graph_data(graph, key);
                graph_data
            })
            .fold(
                GraphData {
                    sections: HashMap::new(),
                    subgraphs: HashMap::new(),
                    sub_sections: Vec::new(),
                    references: Vec::new(),
                },
                |mut acc, data| {
                    acc.merge(data);
                    acc
                },
            )
    }

    fn generate_graph_opening(&self) -> String {
        "digraph G {\n".to_string()
    }

    fn generate_nodes(&self, graph_data: &GraphData) -> String {
        let mut nodes_output = String::new();

        for section in graph_data.sections.values() {
            let escaped_title = section
                .title
                .replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", " ")
                .replace("\r", " ")
                .replace("\t", " ");

            nodes_output.push_str(&format!(
                "  {} [label=\"{}\"];\n",
                section.id, escaped_title
            ));
        }

        nodes_output.push_str("\n");
        nodes_output
    }

    fn generate_subgraphs(&self, graph_data: &GraphData) -> String {
        let mut subgraphs_output = String::new();
        let mut i = 0;

        for subgraph in graph_data.subgraphs.values() {
            i = i + 1;
            subgraphs_output.push_str(&format!("  subgraph cluster_{} {{\n", i));
            subgraphs_output.push_str(&format!(
                "    label=\"{}\";\n",
                subgraph.key.replace("\"", "\\\"")
            ));

            for &node_id in &subgraph.nodes {
                subgraphs_output.push_str(&format!("    {};\n", node_id));
            }

            subgraphs_output.push_str("  }\n");
        }

        subgraphs_output.push_str("\n");
        subgraphs_output
    }

    fn generate_edges(&self, graph_data: &GraphData) -> String {
        let mut edges_output = String::new();

        for (from_id, to_id) in &graph_data.sub_sections {
            edges_output.push_str(&format!("  {} -> {};\n", from_id, to_id));
        }

        for (from_id, to_id) in &graph_data.references {
            edges_output.push_str(&format!("  {} -> {};\n", from_id, to_id));
        }
        edges_output
    }

    fn generate_graph_closing(&self) -> String {
        "}\n".to_string()
    }

    fn build_sections(
        tree: &Tree,
        sections: &mut HashMap<NodeId, Section>,
        sub_sections: &mut HashSet<(NodeId, NodeId)>,
        references: &mut HashSet<(NodeId, Key)>,
    ) {
        tree.children.iter().for_each(|child| {
            if child.is_list() {
                return;
            }

            if child.is_section() {
                sections.insert(
                    child.id.unwrap(),
                    Section {
                        id: child.id.unwrap(),
                        title: child.node.plain_text(),
                    },
                );
                if tree.is_section() {
                    sub_sections.insert((tree.id.unwrap(), child.id.unwrap()));
                }
                DotExporter::build_sections(child, sections, sub_sections, references);
            }

            if child.is_reference() {
                if let Some(key) = child.node.reference_key() {
                    references.insert((tree.id.unwrap(), key));
                }
            }
        })
    }

    fn filter_keys(&self, graph: &Graph) -> Vec<Key> {
        self.key
            .clone()
            .map(|key| {
                if graph.maybe_key(&key).is_none() {
                    return Vec::new();
                }
                let mut keys = HashSet::<Key>::new();
                DotExporter::get_keys_for_depth(graph, &key, self.depth, &mut keys);
                keys.into_iter().collect_vec()
            })
            .unwrap_or_else(|| graph.keys().clone())
    }

    fn get_keys_for_depth(graph: &Graph, key: &Key, depth: u8, collected_keys: &mut HashSet<Key>) {
        collected_keys.insert(key.clone());

        if depth == 0 {
            return;
        }
        let keys = graph.collect(key).get_all_block_reference_keys();
        for k in keys {
            if collected_keys.insert(k.clone()) {
                DotExporter::get_keys_for_depth(graph, &k, depth - 1, collected_keys);
            }
        }
    }

    fn build_graph_data(graph: &Graph, key: &Key) -> GraphData {
        let mut sections = HashMap::<NodeId, Section>::new();
        let mut sub_sections = HashSet::<(NodeId, NodeId)>::new();
        let mut references = HashSet::<(NodeId, Key)>::new();
        let mut subgraphs = HashMap::new();

        let tree = graph.collect(key);

        DotExporter::build_sections(&tree, &mut sections, &mut sub_sections, &mut references);

        subgraphs.insert(
            key.to_string(),
            Subgraph {
                key: key.to_string(),
                nodes: sections.clone().into_keys().collect_vec(),
            },
        );

        GraphData {
            sections,
            subgraphs,
            sub_sections: sub_sections.into_iter().collect_vec(),
            references: references
                .into_iter()
                .map(|r| {
                    (
                        r.0,
                        graph
                            .get_node_id(&r.1.clone())
                            .map(|doc_id| graph.node(doc_id).child_id().unwrap_or_default())
                            .unwrap_or_default(),
                    )
                })
                .collect_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use liwe::{model::config::MarkdownOptions, state::from_indoc};

    use super::*;

    #[test]
    fn test_graphviz_exporter_basic_export() {
        let graph = create_simple_graph();
        let exporter = DotExporter::new(None, 0);
        let output = exporter.export(&graph);

        assert!(output.starts_with("digraph G {"));
        assert!(output.ends_with("}\n"));
    }

    fn create_simple_graph() -> Graph {
        let mut state = HashMap::new();
        state.insert(
            "1".to_string(),
            "# Test Document\n\nSome content here.".to_string(),
        );
        state.insert(
            "2".to_string(),
            "# Another Document\n\n[Link to test](1)".to_string(),
        );

        Graph::import(&state, MarkdownOptions::default())
    }

    #[test]
    fn test_graphviz_exporter_references() {
        let state = from_indoc(indoc! {"
                        # test 1

                        [test 2](2)
                        _
                        # test 2
                        "});

        let graph = Graph::import(
            &state,
            MarkdownOptions {
                refs_extension: String::default(),
            },
        );

        let exporter = DotExporter::new(None, 0);

        let actual = exporter.graph_data(&graph);
        let expected = GraphData {
            sections: vec![
                (
                    1,
                    Section {
                        id: 1,
                        title: "test 1".to_string(),
                    },
                ),
                (
                    4,
                    Section {
                        id: 4,
                        title: "test 2".to_string(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            subgraphs: vec![
                (
                    "2".to_string(),
                    Subgraph {
                        key: "2".to_string(),
                        nodes: vec![4],
                    },
                ),
                (
                    "1".to_string(),
                    Subgraph {
                        key: "1".to_string(),
                        nodes: vec![1],
                    },
                ),
            ]
            .into_iter()
            .collect(),
            sub_sections: vec![],
            references: vec![(1, 4)],
        };

        assert_eq!(expected, actual);
    }
}
