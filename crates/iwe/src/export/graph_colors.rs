use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Debug, Clone, Copy)]
pub struct SubgraphColor {
    pub subgraph_fill: &'static str,   // fillcolor
    pub subgraph_text: &'static str,   // fontcolor
    pub node_background: &'static str, // fillcolor for node
}

pub fn key_colors(key: &str) -> SubgraphColor {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    SUBGRAPH_COLORS[(hasher.finish() as usize) % 14]
}

const SUBGRAPH_COLORS: [SubgraphColor; 14] = [
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
