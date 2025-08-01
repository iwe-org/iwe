use std::env;
use std::fs::{create_dir, OpenOptions};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use itertools::Itertools;
use serde_json;

use liwe::fs::new_for_path;
use liwe::graph::path::NodePath;
use liwe::graph::{Graph, GraphContext};
use liwe::model::config::Configuration;

use liwe::model::node::NodePointer;
use liwe::model::tree::TreeIter;
use liwe::model::Key;
use log::{debug, error};

const CONFIG_FILE_NAME: &str = "config.toml";
const IWE_MARKER: &str = ".iwe";

#[derive(Debug, Parser)]
#[clap(name = "iwe", version)]
pub struct App {
    #[clap(flatten)]
    global_opts: GlobalOpts,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init(Init),
    Normalize(Normalize),
    Paths(Paths),
    Squash(Squash),
    Contents(Contents),
    /// Export the graph structure as JSON
    ExportJson(ExportJson),
    /// Export the graph structure as Graphviz DOT format
    ExportGraphviz(ExportGraphviz),
}

#[derive(Debug, Args)]
struct Search {
    #[clap(long, short = 'p')]
    prompt: String,
}

#[derive(Debug, Args)]
struct Normalize {}

#[derive(Debug, Args)]
struct Init {}

#[derive(Debug, Args)]
struct Contents {}

#[derive(Debug, Args)]
#[clap(about = "Export the graph structure as JSON")]
struct ExportJson {}

#[derive(Debug, Args)]
#[clap(about = "Export the graph structure as Graphviz DOT format")]
struct ExportGraphviz {}

#[derive(Debug, Args)]
struct Squash {
    #[clap(long, short = 'k')]
    key: String,
    #[clap(long, short, global = true, required = false, default_value = "2")]
    depth: u8,
}

#[derive(Debug, Args)]
struct Paths {
    #[clap(long, short, global = true, required = false, default_value = "4")]
    depth: u8,
}

#[derive(Debug, Args)]
struct GlobalOpts {
    #[clap(long, short, global = true, required = false, default_value = "0")]
    verbose: usize,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GraphNode {
    id: i64,
    title: String,
    subnodes: Vec<i64>,
}

impl GraphNode {
    fn new(id: i64, title: &str) -> Self {
        GraphNode {
            id,
            title: title.to_string(),
            subnodes: Vec::new(),
        }
    }

    fn add_subnode(&mut self, subnode_id: i64) {
        self.subnodes.push(subnode_id);
    }
}

fn main() {
    if env::var("IWE_DEBUG").is_ok() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("iwe.log")
                    .expect("to open log file"),
            )
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(std::io::stderr)
            .init();
    }

    debug!("parsing arguments");
    let app = App::parse();

    debug!("starting command processing");
    match app.command {
        Command::Normalize(normalize) => {
            normalize_command(normalize);
        }
        Command::Paths(paths) => {
            paths_command(paths);
        }
        Command::Squash(squash) => {
            squash_command(squash);
        }
        Command::Init(init) => init_command(init),
        Command::Contents(contents) => contents_command(contents),
        Command::ExportJson(export_json) => export_json_command(export_json),
        Command::ExportGraphviz(export_graphviz) => export_graphviz_command(export_graphviz),
    }
}

#[tracing::instrument]
fn init_command(init: Init) {
    debug!("Initializing IWE");

    let mut path = env::current_dir().expect("to get current dir");
    path.push(IWE_MARKER);
    if path.is_dir() {
        error!("IWE is already initialized in the current location.");
        return;
    }
    if path.exists() {
        error!("Initialization failed: '.iwe' path already exists in the current location.");
        return;
    }
    create_dir(&path).expect("to create .iwe directory");

    let toml = toml::to_string(&Configuration::template()).unwrap();

    std::fs::write(path.join(CONFIG_FILE_NAME), toml).expect("Failed to write to config.json");
    debug!("IWE initialized in the current location. Default config added to .iwe/config.json");
}

#[tracing::instrument]
fn paths_command(args: Paths) {
    let graph = load_graph();

    graph
        .paths()
        .iter()
        .filter(|n| n.ids().len() <= args.depth as usize)
        .map(|n| render(&n, &graph))
        .sorted()
        .unique()
        .for_each(|string| println!("{}", string));
}

#[tracing::instrument]
fn contents_command(args: Contents) {
    let graph = load_graph();

    println!("# Contents\n");

    graph
        .paths()
        .iter()
        .filter(|n| n.ids().len() <= 1 as usize)
        .map(|n| (&graph).node(n.first_id()).node_key())
        .map(|key| render_block_reference(&key, &graph))
        .sorted()
        .unique()
        .for_each(|string| println!("{}\n", string));
}

#[tracing::instrument]
fn normalize_command(args: Normalize) {
    write_graph(load_graph());
}

#[tracing::instrument]
fn squash_command(args: Squash) {
    let graph = &load_graph();
    let mut patch = Graph::new();
    let squashed = graph.squash(&Key::from_file_name(&args.key), args.depth);

    patch.build_key_from_iter(&args.key.clone().into(), TreeIter::new(&squashed));

    print!("{}", patch.export_key(&args.key.into()).unwrap())
}

#[tracing::instrument]
fn write_graph(graph: Graph) {
    liwe::fs::write_store_at_path(&graph.export(), &get_library_path())
        .expect("Failed to write graph")
}

#[tracing::instrument]
fn load_graph() -> Graph {
    Graph::import(
        &new_for_path(&get_library_path()),
        get_configuration().markdown,
    )
}

fn get_library_path() -> PathBuf {
    let current_dir = env::current_dir().expect("to get current dir");

    let settings = get_configuration();
    let mut library_path = current_dir;

    if !settings.library.path.is_empty() {
        library_path.push(settings.library.path);
    }

    library_path
}

#[tracing::instrument]
fn get_configuration() -> Configuration {
    let current_dir = env::current_dir().expect("to get current dir");

    let mut path = current_dir.clone();
    path.push(IWE_MARKER);
    path.push(CONFIG_FILE_NAME);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| toml::from_str::<Configuration>(&content).ok())
        .unwrap_or(Configuration::default())
}

fn render_block_reference(key: &Key, context: impl GraphContext) -> String {
    format!(
        "[{}]({})",
        context.get_ref_text(key).unwrap_or_default(),
        key
    )
    .to_string()
}

fn render(path: &NodePath, context: impl GraphContext) -> String {
    // For each fragment in the path, get the text and join them with a space
    path.ids()
        .iter()
        .map(|id| context.get_text(id.clone()).trim().to_string())
        .collect_vec()
        .join(" • ")
}

#[tracing::instrument]
fn export_json_command(args: ExportJson) {
    let graph = load_graph();

    // Build a map of all nodes and their children
    let mut nodes: std::collections::HashMap<u64, GraphNode> = std::collections::HashMap::new();
    let paths = graph.paths();

    // First, create all nodes
    for path in &paths {
        for &node_id in path.ids().iter() {
            if !nodes.contains_key(&node_id) {
                let title = (&graph).get_text(node_id).trim().to_string();
                nodes.insert(node_id, GraphNode::new(node_id as i64, &title));
            }
        }
    }

    // Then, establish parent-child relationships
    for path in &paths {
        let ids = path.ids();
        for i in 0..ids.len() - 1 {
            let parent_id = ids[i];
            let child_id = ids[i + 1];

            if let Some(parent_node) = nodes.get_mut(&parent_id) {
                if !parent_node.subnodes.contains(&(child_id as i64)) {
                    parent_node.add_subnode(child_id as i64);
                }
            }
        }
    }

    // Convert to vector and sort by id for consistent output
    let mut node_list: Vec<GraphNode> = nodes.into_values().collect();
    node_list.sort_by_key(|node| node.id);

    // Output as JSON
    let json = serde_json::to_string_pretty(&node_list).expect("Failed to serialize to JSON");
    println!("{}", json);
}

#[tracing::instrument]
fn export_graphviz_command(args: ExportGraphviz) {
    let graph = load_graph();

    // Build a map of all nodes and their children
    let mut nodes: std::collections::HashMap<u64, GraphNode> = std::collections::HashMap::new();
    let paths = graph.paths();

    // First, create all nodes
    for path in &paths {
        for &node_id in path.ids().iter() {
            if !nodes.contains_key(&node_id) {
                let title = (&graph).get_text(node_id).trim().to_string();
                nodes.insert(node_id, GraphNode::new(node_id as i64, &title));
            }
        }
    }

    // Then, establish parent-child relationships
    for path in &paths {
        let ids = path.ids();
        for i in 0..ids.len() - 1 {
            let parent_id = ids[i];
            let child_id = ids[i + 1];

            if let Some(parent_node) = nodes.get_mut(&parent_id) {
                if !parent_node.subnodes.contains(&(child_id as i64)) {
                    parent_node.add_subnode(child_id as i64);
                }
            }
        }
    }

    // Calculate node ranks (total number of descendants)
    fn count_descendants(node_id: i64, nodes: &std::collections::HashMap<u64, GraphNode>) -> usize {
        let mut count = 0;
        if let Some(node) = nodes.get(&(node_id as u64)) {
            for &child_id in &node.subnodes {
                count += 1; // Count the child itself
                count += count_descendants(child_id, nodes); // Count its descendants
            }
        }
        count
    }

    // Calculate ranks for all nodes before moving
    let node_ranks: std::collections::HashMap<i64, usize> = nodes
        .iter()
        .map(|(_, node)| (node.id, count_descendants(node.id, &nodes)))
        .collect();

    // Convert to vector and sort by id for consistent output
    let mut node_list: Vec<GraphNode> = nodes.into_values().collect();
    node_list.sort_by_key(|node| node.id);

    // Output as beautiful Graphviz DOT format
    println!("digraph G {{");
    println!("  label=\"IWE Knowledge Graph\";");
    println!("  labelloc=t;");
    println!("  fontsize=20;");
    println!("  fontname=\"Helvetica,Arial,sans-serif\";");
    println!("  fontcolor=\"#2c3e50\";");
    println!("  rankdir=TB;");
    println!("  bgcolor=\"#f8f9fa\";");
    println!("  node [fontname=\"Helvetica,Arial,sans-serif\"];");
    println!("  edge [color=\"#6c757d\", penwidth=1.5];");
    println!("  splines=true;");
    println!("  concentrate=true;");
    println!("  nodesep=1.0;");
    println!("  ranksep=1.5;");
    println!("  overlap=false;");
    println!();

    // Add a legend
    println!("  subgraph cluster_legend {{");
    println!("    label=\"Node Size Legend\";");
    println!("    fontsize=12;");
    println!("    fontcolor=\"#6c757d\";");
    println!("    style=dashed;");
    println!("    bgcolor=\"#ffffff\";");
    println!("    color=\"#dee2e6\";");
    println!("    legend_leaf [label=\"Leaf (0 children)\", width=0.4, height=0.4, fillcolor=\"#e3f2fd\", shape=ellipse, style=\"filled,solid\", fontsize=9];");
    println!("    legend_small [label=\"Small (1-2 children)\", width=0.6, height=0.6, fillcolor=\"#bbdefb\", shape=box, style=\"filled,rounded\", fontsize=9];");
    println!("    legend_large [label=\"Large (11+ children)\", width=1.2, height=1.2, fillcolor=\"#42a5f5\", shape=box, style=\"filled,rounded,bold\", fontsize=9, fontcolor=\"#ffffff\"];");
    println!("  }}");
    println!();

    // Output nodes with beautiful styling based on rank
    for node in &node_list {
        let escaped_title = node
            .title
            .replace("\\", "\\\\")
            .replace("\"", "\\\"")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t");

        let rank = node_ranks.get(&node.id).unwrap_or(&0);

        // Determine node style based on rank
        let (size, color, shape, style) = match *rank {
            0 => ("0.6", "#e3f2fd", "ellipse", "filled,solid"), // Leaf nodes - small, light blue
            1..=2 => ("0.8", "#bbdefb", "box", "filled,rounded"), // Small branches - medium, blue
            3..=5 => ("1.0", "#90caf9", "box", "filled,rounded"), // Medium branches - larger, darker blue
            6..=10 => ("1.3", "#64b5f6", "box", "filled,rounded,bold"), // Large branches - bold, darker
            11..=20 => ("1.6", "#42a5f5", "box", "filled,rounded,bold"), // Major sections - larger, bolder
            _ => ("2.0", "#1e88e5", "box", "filled,rounded,bold"), // Root/major nodes - largest, darkest
        };

        let fontsize = match *rank {
            0 => "10",
            1..=2 => "11",
            3..=5 => "12",
            6..=10 => "13",
            11..=20 => "14",
            _ => "16",
        };

        let fontcolor = if *rank > 10 { "#ffffff" } else { "#2c3e50" };

        println!(
            "  {} [label=\"{}\", width={}, height={}, fillcolor=\"{}\", shape={}, style=\"{}\", fontsize={}, fontcolor=\"{}\"];",
            node.id, escaped_title, size, size, color, shape, style, fontsize, fontcolor
        );
    }

    println!();

    // Output edges with varying styles
    for node in &node_list {
        let parent_rank = node_ranks.get(&node.id).unwrap_or(&0);
        for &subnode_id in &node.subnodes {
            // Thicker edges for higher-rank parent nodes
            let penwidth = match *parent_rank {
                0..=2 => "1.0",
                3..=5 => "1.5",
                6..=10 => "2.0",
                _ => "2.5",
            };

            println!("  {} -> {} [penwidth={}];", node.id, subnode_id, penwidth);
        }
    }

    println!("}}");
}
