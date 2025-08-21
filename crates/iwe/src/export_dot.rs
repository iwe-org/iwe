use itertools::Itertools;
use liwe::{
    graph::{Graph, GraphContext},
    model::{node::NodePointer, tree::Tree, Key, NodeId},
};
use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    hash::Hash,
};

use std::hash::{DefaultHasher, Hasher};

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
    pub key: String,
    pub key_depth: u8,
    pub depth: u8,
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
            .map(|pair| {
                let graph_data = DotExporter::build_graph_data(graph, pair.0, *pair.1);
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
        r###"digraph G {
            graph [
              rankdir=LR
              fontname="Verdana"
              fontsize=13
              nodesep=0.7
              splines=polyline
              pad="0.5,0.2"
              ranksep=1.2
              overlap=false

            ];
            node [
              style="filled,rounded"
              fillcolor="#ffffff"
              fontname="Verdana"
              fontsize=11
              shape=box
              color="#b3b3b3"
              penwidth=1.5
            ];
            edge [
              color="#38546c66"
              arrowhead=normal
              penwidth=1.2
            ];

            "###
        .to_string()
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

            let font_size = max(12, 16 + section.key_depth * 8 - max(0, 8 * section.depth));
            let colors = key_colors(&section.key);
            nodes_output.push_str(&format!(
                "  {} [label=\"{}\", fillcolor=\"{}\", fontsize=\"{}\"];\n",
                section.id, escaped_title, colors.node_background, font_size,
            ));
        }

        nodes_output.push_str("\n");
        nodes_output
    }

    fn generate_subgraphs(&self, graph_data: &GraphData) -> String {
        let mut subgraphs_output = String::new();
        for (i, subgraph) in graph_data.subgraphs.values().enumerate() {
            let colors = key_colors(&subgraph.key);
            subgraphs_output.push_str(&format!("  subgraph cluster_{} {{\n", i));
            subgraphs_output.push_str(&format!(
                r###"
                labeljust="l"
                style="filled,rounded"
                color="{}"
                fillcolor="{}"
                fontcolor="{}"
                penwidth=40
                "###,
                colors.subgraph_fill, colors.subgraph_fill, colors.subgraph_text
            ));

            subgraphs_output.push_str("");
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
        key: &str,
        key_depth: u8,
        depth: u8,
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
                        key_depth: key_depth,
                        depth: depth,
                        id: child.id.unwrap(),
                        title: child.node.plain_text(),
                        key: key.to_string(),
                    },
                );
                if tree.is_section() {
                    sub_sections.insert((tree.id.unwrap(), child.id.unwrap()));
                }
                DotExporter::build_sections(
                    key,
                    key_depth,
                    depth + 1,
                    child,
                    sections,
                    sub_sections,
                    references,
                );
            }

            if child.is_reference() {
                if let Some(key) = child.node.reference_key() {
                    references.insert((tree.id.unwrap(), key));
                }
            }
        })
    }

    fn filter_keys(&self, graph: &Graph) -> HashMap<Key, u8> {
        self.key
            .clone()
            .map(|key| {
                if graph.maybe_key(&key).is_none() {
                    return HashMap::new();
                }
                let mut keys = HashMap::new();
                DotExporter::get_keys_for_depth(graph, &key, self.depth, &mut keys);
                keys
            })
            .unwrap_or_else(|| {
                graph
                    .keys()
                    .into_iter()
                    .filter_map(|key| {
                        if graph.maybe_key(&key).is_some() {
                            Some((key, 0))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
    }

    fn get_keys_for_depth(
        graph: &Graph,
        key: &Key,
        depth: u8,
        collected_keys: &mut HashMap<Key, u8>,
    ) {
        collected_keys.insert(key.clone(), depth);

        if depth == 0 {
            return;
        }
        let keys = graph.collect(key).get_all_block_reference_keys();
        for k in keys {
            if !collected_keys.contains_key(&k) && graph.maybe_key(&k).is_some() {
                DotExporter::get_keys_for_depth(graph, &k, depth - 1, collected_keys);
            }
        }
    }

    fn build_graph_data(graph: &Graph, key: &Key, key_depth: u8) -> GraphData {
        let mut sections = HashMap::<NodeId, Section>::new();
        let mut sub_sections = HashSet::<(NodeId, NodeId)>::new();
        let mut references = HashSet::<(NodeId, Key)>::new();
        let mut subgraphs = HashMap::new();

        let tree = graph.collect(key);

        DotExporter::build_sections(
            &key.to_string(),
            key_depth,
            0,
            &tree,
            &mut sections,
            &mut sub_sections,
            &mut references,
        );

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
                        key_depth: 0,
                        depth: 0,
                        id: 1,
                        title: "test 1".to_string(),
                        key: "1".to_string(),
                    },
                ),
                (
                    4,
                    Section {
                        key_depth: 0,
                        depth: 0,
                        id: 4,
                        title: "test 2".to_string(),
                        key: "2".to_string(),
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

fn key_colors(key: &str) -> SubgraphColor {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    SUBGRAPH_COLORS[(hasher.finish() as usize) % 14]
}
#[derive(Debug, Clone, Copy)]
pub struct SubgraphColor {
    pub subgraph_fill: &'static str,   // fillcolor
    pub subgraph_text: &'static str,   // fontcolor
    pub node_background: &'static str, // fillcolor for node
}

pub const SUBGRAPH_COLORS: [SubgraphColor; 14] = [
    SubgraphColor {
        // Pastel Blue
        subgraph_fill: "#eff8fd",
        subgraph_text: "#283747",
        node_background: "#e1f5fe",
    },
    SubgraphColor {
        // Pastel Green
        subgraph_fill: "#f6fcf5",
        subgraph_text: "#185c37",
        node_background: "#e9f9ef",
    },
    SubgraphColor {
        // Pastel Pink
        subgraph_fill: "#fff4fa",
        subgraph_text: "#a7475a",
        node_background: "#fae1ee",
    },
    SubgraphColor {
        // Pastel Yellow
        subgraph_fill: "#fffbea",
        subgraph_text: "#a67c00",
        node_background: "#fff9de",
    },
    SubgraphColor {
        // Pastel Lavender
        subgraph_fill: "#f8f8ff",
        subgraph_text: "#442b7e",
        node_background: "#eeebfa",
    },
    SubgraphColor {
        // Pastel Mint
        subgraph_fill: "#f3fcf7",
        subgraph_text: "#257257",
        node_background: "#d9fae7",
    },
    SubgraphColor {
        // Pastel Peach
        subgraph_fill: "#fff6eb",
        subgraph_text: "#b36a36",
        node_background: "#ffeee0",
    },
    SubgraphColor {
        // Pastel Aqua
        subgraph_fill: "#f0fbfc",
        subgraph_text: "#1d4c56",
        node_background: "#dcf7f7",
    },
    SubgraphColor {
        // Pastel Lilac
        subgraph_fill: "#fdf7fa",
        subgraph_text: "#764470",
        node_background: "#f6e5ee",
    },
    SubgraphColor {
        // Pastel Lemon
        subgraph_fill: "#fefeec",
        subgraph_text: "#96902d",
        node_background: "#fbfbda",
    },
    SubgraphColor {
        // Pastel Coral
        subgraph_fill: "#fef7f7",
        subgraph_text: "#c35151",
        node_background: "#ffeaea",
    },
    SubgraphColor {
        // Pastel Teal
        subgraph_fill: "#f0fafb",
        subgraph_text: "#225c5a",
        node_background: "#e2f6f6",
    },
    SubgraphColor {
        // Pastel Grey
        subgraph_fill: "#f7f9fa",
        subgraph_text: "#4a525a",
        node_background: "#eef2f5",
    },
    SubgraphColor {
        // Pastel Olive
        subgraph_fill: "#faffef",
        subgraph_text: "#847c36",
        node_background: "#f2fadf",
    },
];
