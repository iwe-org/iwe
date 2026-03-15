use std::env;
use std::fs::create_dir;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use clap::{Args, Parser, Subcommand};

mod help;
use itertools::Itertools;

use iwe::export::{dot_details_exporter, dot_exporter, graph_data};
use iwe::find::{DocumentFinder, FindOptions};
use iwe::new::{read_stdin_if_available, CreateOptions, DocumentCreator, IfExists};
use iwe::retrieve::render::RetrieveRenderer;
use iwe::retrieve::{DocumentReader, RetrieveOptions};
use iwe::stats::GraphStatistics;
use minijinja::{context, Environment};
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
    New(New),
    Retrieve(Retrieve),
    Find(Find),
    Normalize(Normalize),
    Paths(Paths),
    Squash(Squash),
    Contents(Contents),
    Export(Export),
    Stats(Stats),
}

#[derive(Debug, Args)]
#[clap(
    about = help::retrieve::ABOUT,
    long_about = help::retrieve::LONG_ABOUT,
    after_help = help::retrieve::AFTER_HELP
)]
struct Retrieve {
    #[clap(long, short = 'k', help = "Document key(s) to retrieve (can be specified multiple times)")]
    key: Vec<String>,

    #[clap(
        long,
        short = 'd',
        default_value = "1",
        help = "Follow block refs down N levels"
    )]
    depth: u8,

    #[clap(
        long,
        short = 'c',
        default_value = "1",
        help = "Include N levels of parent context"
    )]
    context: u8,

    #[clap(long, short = 'l', help = "Include inline references")]
    links: bool,

    #[clap(long, short = 'e', help = "Exclude document key(s) from results (can be specified multiple times)")]
    exclude: Vec<String>,

    #[clap(long, short = 'b', default_value_t = true, help = "Include incoming references")]
    backlinks: bool,

    #[clap(long, short = 'f', value_enum, default_value = "markdown")]
    format: RetrieveFormat,

    #[clap(long, help = "Show document count and total lines without content")]
    dry_run: bool,

    #[clap(long, help = "Exclude document content from results (metadata only)")]
    no_content: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum RetrieveFormat {
    Markdown,
    Keys,
    Json,
}

#[derive(Debug, Args)]
struct Search {
    #[clap(long, short = 'p')]
    prompt: String,
}

#[derive(Debug, Args)]
#[clap(
    about = help::find::ABOUT,
    long_about = help::find::LONG_ABOUT,
    after_help = help::find::AFTER_HELP
)]
struct Find {
    #[clap(help = "Search query (fuzzy match on title and key)")]
    query: Option<String>,

    #[clap(long, help = "Only root documents (no incoming block refs)")]
    roots: bool,

    #[clap(long, help = "Documents that reference this key")]
    refs_to: Option<String>,

    #[clap(long, help = "Documents referenced by this key")]
    refs_from: Option<String>,

    #[clap(long, short = 'l', default_value = "50", help = "Maximum results")]
    limit: usize,

    #[clap(long, short = 'f', value_enum, default_value = "markdown")]
    format: FindFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum FindFormat {
    Markdown,
    Keys,
    Json,
}

#[derive(Debug, Args)]
#[clap(
    about = help::normalize::ABOUT,
    long_about = help::normalize::LONG_ABOUT
)]
struct Normalize {}

#[derive(Debug, Args)]
#[clap(about = help::init::ABOUT)]
struct Init {}

#[derive(Debug, Args)]
#[clap(
    about = help::new::ABOUT,
    long_about = help::new::LONG_ABOUT
)]
struct New {
    #[clap(help = "Title for the new document")]
    title: String,

    #[clap(long, short = 't', help = "Template name from config")]
    template: Option<String>,

    #[clap(long, short = 'c', help = "Content for the new document")]
    content: Option<String>,

    #[clap(
        long,
        short = 'i',
        value_enum,
        default_value = "suffix",
        help = "Behavior when file already exists: suffix (append -1, -2, etc.), override (overwrite), skip (do nothing)"
    )]
    if_exists: IfExists,

    #[clap(long, short = 'e', help = "Open created file in $EDITOR")]
    edit: bool,
}

#[derive(Debug, Args)]
#[clap(
    about = help::contents::ABOUT,
    long_about = help::contents::LONG_ABOUT
)]
struct Contents {}

#[derive(Debug, Args)]
#[clap(
    about = help::stats::ABOUT,
    long_about = help::stats::LONG_ABOUT,
    after_help = help::stats::AFTER_HELP
)]
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
#[clap(
    about = help::export::ABOUT,
    long_about = help::export::LONG_ABOUT,
    after_help = help::export::AFTER_HELP
)]
struct Export {
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
    #[clap(
        long,
        global = true,
        required = false,
        default_value = "false",
        help = "Include section headers and create subgraphs for detailed visualization. When enabled, shows document structure with sections grouped in colored subgraphs"
    )]
    include_headers: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum Format {
    Dot,
}

#[derive(Debug, Args)]
#[clap(
    about = help::squash::ABOUT,
    long_about = help::squash::LONG_ABOUT
)]
struct Squash {
    #[clap(long, short = 'k')]
    key: String,
    #[clap(long, short, global = true, required = false, default_value = "2")]
    depth: u8,
}

#[derive(Debug, Args)]
#[clap(
    about = help::paths::ABOUT,
    long_about = help::paths::LONG_ABOUT
)]
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
        Command::New(new) => new_command(new),
        Command::Retrieve(retrieve) => retrieve_command(retrieve),
        Command::Find(find) => find_command(find),
        Command::Contents(contents) => contents_command(contents),
        Command::Export(export) => export_command(export),
        Command::Stats(stats) => stats_command(stats),
    }
}

#[tracing::instrument(level = "debug")]
fn retrieve_command(args: Retrieve) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let key_strings: Vec<String> = if args.key.is_empty() {
        let stdin_content = read_stdin_if_available();
        let keys: Vec<String> = stdin_content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if keys.is_empty() {
            eprintln!("Error: No document key provided. Use -k <key> or pipe keys via stdin.");
            std::process::exit(1);
        }
        keys
    } else {
        args.key
    };

    let mut keys = Vec::new();
    for key_str in &key_strings {
        let key = Key::name(key_str);
        if (&graph).get_node_id(&key).is_none() {
            eprintln!("Error: Document '{}' not found", key_str);
            std::process::exit(2);
        }
        keys.push(key);
    }

    let reader = DocumentReader::new(&graph);
    let exclude: std::collections::HashSet<Key> = args
        .exclude
        .iter()
        .map(|s| Key::name(s))
        .collect();
    let options = RetrieveOptions {
        depth: args.depth,
        context: args.context,
        links: args.links,
        backlinks: args.backlinks,
        exclude,
        no_content: args.no_content,
    };

    let output = reader.retrieve_many(&keys, &options);

    if args.dry_run {
        let doc_count = output.documents.len();
        let total_lines: usize = output
            .documents
            .iter()
            .map(|doc| doc.content.lines().count())
            .sum();
        println!("documents: {}", doc_count);
        println!("lines: {}", total_lines);
        return;
    }

    match args.format {
        RetrieveFormat::Json => {
            let json = serde_json::to_string_pretty(&output).expect("Failed to serialize to JSON");
            println!("{}", json);
        }
        RetrieveFormat::Keys => {
            for doc in &output.documents {
                println!("{}", doc.key);
            }
        }
        RetrieveFormat::Markdown => {
            let options = graph.markdown_options();
            let renderer = RetrieveRenderer::new(&output, &options, &graph);
            print!("{}", renderer.render());
        }
    }
}

const FIND_TEMPLATE: &str = include_str!("../templates/find.md.jinja");

#[tracing::instrument(level = "debug")]
fn find_command(args: Find) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let finder = DocumentFinder::new(&graph);
    let options = FindOptions {
        query: args.query,
        roots: args.roots,
        refs_to: args.refs_to.map(|s| Key::name(&s)),
        refs_from: args.refs_from.map(|s| Key::name(&s)),
        limit: args.limit,
    };

    let output = finder.find(&options);

    match args.format {
        FindFormat::Json => {
            let json = serde_json::to_string_pretty(&output).expect("Failed to serialize to JSON");
            println!("{}", json);
        }
        FindFormat::Keys => {
            for result in &output.results {
                println!("{}", result.key);
            }
        }
        FindFormat::Markdown => {
            let rendered = render_find_template(&output);
            print!("{}", rendered);
        }
    }
}

fn render_find_template(output: &iwe::find::output::FindOutput) -> String {
    let env = Environment::new();
    let template = env
        .template_from_str(FIND_TEMPLATE)
        .expect("Failed to parse template");

    template
        .render(context! {
            query => output.query,
            limit => output.limit,
            total => output.total,
            results => output.results,
        })
        .expect("Failed to render template")
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
fn new_command(args: New) {
    let config = get_configuration();
    let library_path = get_library_path(&config);

    let content = args.content.or_else(|| {
        let stdin_content = read_stdin_if_available();
        if stdin_content.is_empty() {
            None
        } else {
            Some(stdin_content)
        }
    });

    let creator = DocumentCreator::new(&config, library_path);
    let options = CreateOptions {
        title: args.title,
        template_name: args.template,
        content,
        if_exists: args.if_exists,
    };

    match creator.create(options) {
        Ok(Some(doc)) => {
            println!("{}", doc.path.display());

            if args.edit {
                open_in_editor(&doc.path);
            }
        }
        Ok(None) => {}
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    }
}

fn open_in_editor(path: &std::path::Path) {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    let status = ProcessCommand::new(&editor).arg(path).status();

    match status {
        Ok(exit_status) => {
            if !exit_status.success() {
                error!("Editor exited with non-zero status");
            }
        }
        Err(e) => {
            error!("Failed to open editor '{}': {}", editor, e);
        }
    }
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
        Format::Dot => {
            if args.include_headers {
                dot_details_exporter::export_dot_with_headers(&data)
            } else {
                dot_exporter::export_dot(&data)
            }
        }
    };

    print!("{}", output);
}
