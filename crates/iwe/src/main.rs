use std::env;
use std::fs::create_dir;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use itertools::Itertools;

use iwe::export::dot_exporter::DotExporter;
use iwe::export::exporter::GraphExporter;
use iwe::export::graph_data;
use iwe::export::json_exporter::JsonExporter;
use iwe::stats::GraphStatistics;
use liwe::fs::new_for_path;
use liwe::graph::path::NodePath;
use liwe::graph::{Graph, GraphContext};
use liwe::model::config::{load_config, Configuration};

use liwe::model::node::NodePointer;
use liwe::model::tree::TreeIter;
use liwe::model::Key;
use log::{debug, error, info};

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
    Export(Export),
    Stats(Stats),
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
#[clap(about = "Display graph statistics")]
struct Stats {
    #[clap(
        long,
        short = 'f',
        value_enum,
        default_value = "markdown",
        help = "Output format for statistics"
    )]
    format: StatsFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum StatsFormat {
    Markdown,
    Csv,
}

#[derive(Debug, Args)]
#[clap(about = "Export the graph structure in various formats")]
struct Export {
    #[clap(subcommand)]
    format: Format,
    #[clap(
        long,
        short = 'k',
        help = "Filter nodes by specific key. If not provided, exports all root notes by default"
    )]
    key: Option<String>,
    #[clap(
        long,
        short = 'd',
        global = true,
        required = false,
        default_value = "0"
    )]
    depth: u8,
}

#[derive(Debug, Clone, Subcommand)]
enum Format {
    Dot(DotFormat),
    Json,
}

#[derive(Debug, Clone, Args)]
struct DotFormat {
    #[clap(
        long,
        global = true,
        required = false,
        default_value = "false",
        help = "Include section headers and create subgraphs for detailed visualization. When enabled, shows document structure with sections grouped in colored subgraphs"
    )]
    include_headers: bool,
}

impl From<DotFormat> for DotExporter{
    fn from(val: DotFormat) -> Self {
        Self {
            include_headers: val.include_headers,
        }
    }
}

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
    verbose: u8,
}

fn main() {
    debug!("parsing arguments");
    let app = App::parse();

    if app.global_opts.verbose > 1 {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(std::io::stderr)
            .init();
    } else if app.global_opts.verbose > 0 {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(std::io::stderr)
            .init();
    }

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
        Command::Export(export) => export_command(export),
        Command::Stats(stats) => stats_command(stats),
    }
}

#[tracing::instrument(level = "debug")]
fn init_command(init: Init) {
    info!("initializing IWE");
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
    info!("IWE initialized in the current location. Default config added to .iwe/config.json");
}

#[tracing::instrument(level = "debug")]
fn paths_command(args: Paths) {
    let config = get_configuration();
    let graph = load_graph(&config);

    graph
        .paths()
        .iter()
        .filter(|n| n.ids().len() <= args.depth as usize)
        .map(|n| render(n, &graph))
        .sorted()
        .unique()
        .for_each(|string| println!("{}", string));
}

#[tracing::instrument(level = "debug")]
fn contents_command(args: Contents) {
    let config = get_configuration();
    let graph = load_graph(&config);

    println!("# Contents\n");

    graph
        .paths()
        .iter()
        .filter(|n| n.ids().len() <= 1_usize)
        .map(|n| (&graph).node(n.first_id()).node_key())
        .map(|key| render_block_reference(&key, &graph))
        .sorted()
        .unique()
        .for_each(|string| println!("{}\n", string));
}

#[tracing::instrument(level = "debug")]
fn normalize_command(args: Normalize) {
    let configuration = get_configuration();
    let graph = load_graph(&configuration);
    write_graph(graph, &configuration);
}

#[tracing::instrument(level = "debug")]
fn squash_command(args: Squash) {
    let config = get_configuration();
    let graph = &load_graph(&config);
    let mut patch = Graph::new();
    let squashed = graph.squash(&Key::name(&args.key), args.depth);

    patch.build_key_from_iter(&args.key.clone().into(), TreeIter::new(&squashed));

    print!("{}", patch.export_key(&args.key.into()).unwrap())
}

fn write_graph(graph: Graph, configuration: &Configuration) {
    liwe::fs::write_store_at_path(&graph.export(), &get_library_path(configuration))
        .expect("Failed to write graph")
}

fn load_graph(configuration: &Configuration) -> Graph {
    Graph::import(
        &new_for_path(&get_library_path(configuration)),
        configuration.markdown.clone(),
    )
}

fn get_library_path(configuration: &Configuration) -> PathBuf {
    let current_dir = env::current_dir().expect("to get current dir");

    let mut library_path = current_dir;

    if !configuration.library.path.is_empty() {
        library_path.push(configuration.library.path.clone());
    }

    library_path
}

fn get_configuration() -> Configuration {
    let config = load_config();
    if log::log_enabled!(log::Level::Debug) {
        let formatted_config =
            toml::to_string_pretty(&config).unwrap_or_else(|_| format!("{:#?}", config));
        debug!("using config:\n{}", formatted_config);
    }
    config
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
        .map(|id| context.get_text(*id).trim().to_string())
        .collect_vec()
        .join(" • ")
}

#[tracing::instrument(level = "debug")]
fn stats_command(args: Stats) {
    let config = get_configuration();
    let graph = load_graph(&config);

    match args.format {
        StatsFormat::Markdown => {
            let stats = GraphStatistics::from_graph(&graph);
            let output = stats.render();
            print!("{}", output);
        }
        StatsFormat::Csv => {
            let stdout = std::io::stdout();
            if let Err(e) = GraphStatistics::export_csv(&graph, stdout.lock()) {
                error!("Failed to export CSV: {}", e);
                std::process::exit(1);
            }
        }
    }
}

#[tracing::instrument]
fn export_command(args: Export) {
    let config = get_configuration();
    let graph = load_graph(&config);
    let data = graph_data::graph_data(
        args.key.clone().map(|s| Key::name(&s)).clone(),
        args.depth,
        &graph,
    );

    let output = match args.format {
        Format::Dot(format_args) => {
            DotExporter::from(format_args).export(&data).expect("Can't export to dot format")
        }
        Format::Json => {
            JsonExporter::default().export(&data).expect("Can't export to json format")
        }
    };

    print!("{}", output);
}
