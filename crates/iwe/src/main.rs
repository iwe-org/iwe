use std::env;
use std::fs::{create_dir, OpenOptions};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use itertools::Itertools;

use liwe::fs::new_for_path;
use liwe::graph::path::NodePath;
use liwe::graph::{Graph, GraphContext};
use liwe::model::graph::Settings;

use log::{debug, error};

const CONFIG_FILE_NAME: &str = "config.json";
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
struct Squash {
    #[clap(long, short = 'k')]
    key: String,
    #[clap(long, short, global = true, required = false, default_value = "2")]
    depth: u8,
}

#[derive(Debug, Args)]
struct Paths {}

#[derive(Debug, Args)]
struct GlobalOpts {
    #[clap(long, short, global = true, required = false, default_value = "0")]
    verbose: usize,
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

    debug!("starting command procesing");
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

    let json = serde_json::to_string(&default_settings()).unwrap();

    std::fs::write(path.join(CONFIG_FILE_NAME), json).expect("Failed to write to config.json");
    debug!("IWE initialized in the current location. Default config added to .iwe/config.json");
}

#[tracing::instrument]
fn paths_command(args: Paths) {
    let graph = load_graph();

    graph
        .paths()
        .iter()
        .map(|n| render(&n, &graph))
        .sorted()
        .for_each(|string| println!("{}", string));
}

#[tracing::instrument]
fn normalize_command(args: Normalize) {
    write_graph(load_graph());
}

#[tracing::instrument]
fn squash_command(args: Squash) {
    let graph = &load_graph();
    let mut patch = Graph::new();

    patch.build_key_from_iter(&args.key, graph.squash_vistior(&args.key, args.depth));

    print!("{}", patch.export_key(&args.key).unwrap())
}

#[tracing::instrument]
fn write_graph(graph: Graph) {
    liwe::fs::write_store_at_path(&graph.export(), &get_library_path())
        .expect("Failed to write graph")
}

#[tracing::instrument]
fn load_graph() -> Graph {
    Graph::import(&new_for_path(&get_library_path()), get_settings().markdown)
}

fn get_library_path() -> PathBuf {
    let current_dir = env::current_dir().expect("to get current dir");

    let settings = get_settings();
    let mut library_path = current_dir;

    if !settings.library.path.is_empty() {
        library_path.push(settings.library.path);
    }

    library_path
}

#[tracing::instrument]
fn get_settings() -> Settings {
    let current_dir = env::current_dir().expect("to get current dir");

    let mut path = current_dir.clone();
    path.push(IWE_MARKER);
    path.push(CONFIG_FILE_NAME);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Settings>(&content).ok())
        .unwrap_or(Settings::default())
}

fn render(path: &NodePath, context: impl GraphContext) -> String {
    // for each fragment in the path, get the text and join them with a space
    path.ids()
        .iter()
        .map(|id| context.get_text(id.clone()).trim().to_string())
        .collect_vec()
        .join(" • ")
}

fn default_settings() -> Settings {
    let mut settings = Settings::default();
    settings.markdown.refs_extension = ".md".to_string();
    settings
}
