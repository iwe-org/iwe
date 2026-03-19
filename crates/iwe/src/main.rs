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
use liwe::fs::new_for_path;
use liwe::graph::{Graph, GraphContext};
use liwe::model::config::{
    load_config, ActionDefinition, Configuration, InlineType, LinkType,
};
use liwe::model::node::{Node, NodePointer};
use liwe::model::tree::{Tree as ModelTree, TreeIter};
use liwe::model::Key;
use liwe::operations::{
    delete as op_delete, extract as op_extract, inline as op_inline, rename as op_rename,
    Changes, ExtractConfig, InlineConfig,
};

use minijinja::{context, Environment};
use std::io::{self, Write as IoWrite};
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
    Tree(TreeArgs),
    Squash(Squash),
    Export(Export),
    Stats(Stats),
    Rename(Rename),
    Delete(Delete),
    Extract(Extract),
    Inline(Inline),
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
    long_about = help::normalize::LONG_ABOUT,
    after_help = help::normalize::AFTER_HELP
)]
struct Normalize {}

#[derive(Debug, Args)]
#[clap(
    about = help::init::ABOUT,
    long_about = help::init::LONG_ABOUT,
    after_help = help::init::AFTER_HELP
)]
struct Init {}

#[derive(Debug, Args)]
#[clap(
    about = help::new::ABOUT,
    long_about = help::new::LONG_ABOUT,
    after_help = help::new::AFTER_HELP
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
    about = help::tree::ABOUT,
    long_about = help::tree::LONG_ABOUT,
    after_help = help::tree::AFTER_HELP
)]
struct TreeArgs {
    #[clap(
        long,
        short = 'f',
        value_enum,
        default_value = "markdown",
        help = "Output format: markdown (nested list with links), keys, json"
    )]
    format: TreeFormat,

    #[clap(
        long,
        short = 'k',
        help = "Filter to paths starting from specific document(s)"
    )]
    key: Vec<String>,

    #[clap(
        long,
        short = 'd',
        default_value = "4",
        help = "Maximum depth to traverse"
    )]
    depth: u8,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum TreeFormat {
    Markdown,
    Keys,
    Json,
}

#[derive(Debug, serde::Serialize)]
struct TreeNode {
    key: String,
    title: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<TreeNode>,
}

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
    long_about = help::squash::LONG_ABOUT,
    after_help = help::squash::AFTER_HELP
)]
struct Squash {
    #[clap(help = "Document key to squash")]
    key: String,
    #[clap(long, short, global = true, required = false, default_value = "2")]
    depth: u8,
}


#[derive(Debug, Args)]
struct GlobalOpts {
    #[clap(long, short, global = true, required = false, default_value = "0")]
    verbose: u8,
}

#[derive(Debug, Args)]
#[clap(
    about = help::rename::ABOUT,
    long_about = help::rename::LONG_ABOUT,
    after_help = help::rename::AFTER_HELP
)]
struct Rename {
    #[clap(help = "Current document key")]
    old_key: String,

    #[clap(help = "New document key")]
    new_key: String,

    #[clap(long, help = "Preview changes without writing to disk")]
    dry_run: bool,

    #[clap(long, help = "Suppress progress output")]
    quiet: bool,

    #[clap(long, help = "Print affected document keys (one per line)")]
    keys: bool,
}

#[derive(Debug, Args)]
#[clap(
    about = help::delete::ABOUT,
    long_about = help::delete::LONG_ABOUT,
    after_help = help::delete::AFTER_HELP
)]
struct Delete {
    #[clap(help = "Document key to delete")]
    key: String,

    #[clap(long, help = "Preview changes without writing to disk")]
    dry_run: bool,

    #[clap(long, help = "Suppress progress output")]
    quiet: bool,

    #[clap(long, help = "Print affected document keys (one per line)")]
    keys: bool,

    #[clap(long, help = "Skip confirmation prompt")]
    force: bool,
}

#[derive(Debug, Args)]
#[clap(
    about = help::extract::ABOUT,
    long_about = help::extract::LONG_ABOUT,
    after_help = help::extract::AFTER_HELP
)]
struct Extract {
    #[clap(help = "Document key containing the section to extract")]
    key: String,

    #[clap(long, help = "Section title to extract (case-insensitive)")]
    section: Option<String>,

    #[clap(long, help = "Block number to extract (1-indexed)")]
    block: Option<usize>,

    #[clap(long, help = "List all sections with block numbers")]
    list: bool,

    #[clap(long, help = "Action name from config to use for extraction")]
    action: Option<String>,

    #[clap(long, help = "Preview changes without writing to disk")]
    dry_run: bool,

    #[clap(long, help = "Suppress progress output")]
    quiet: bool,

    #[clap(long, help = "Print affected document keys (one per line)")]
    keys: bool,
}

#[derive(Debug, Args)]
#[clap(
    about = help::inline::ABOUT,
    long_about = help::inline::LONG_ABOUT,
    after_help = help::inline::AFTER_HELP
)]
struct Inline {
    #[clap(help = "Document key containing the reference to inline")]
    key: String,

    #[clap(long, help = "Reference key or title to inline")]
    reference: Option<String>,

    #[clap(long, help = "Block number to inline (1-indexed)")]
    block: Option<usize>,

    #[clap(long, help = "List all block references with numbers")]
    list: bool,

    #[clap(long, help = "Action name from config to use for inlining")]
    action: Option<String>,

    #[clap(long, help = "Inline as blockquote instead of section")]
    as_quote: bool,

    #[clap(long, help = "Keep the target document after inlining")]
    keep_target: bool,

    #[clap(long, help = "Preview changes without writing to disk")]
    dry_run: bool,

    #[clap(long, help = "Suppress progress output")]
    quiet: bool,

    #[clap(long, help = "Print affected document keys (one per line)")]
    keys: bool,
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
        Command::Tree(tree) => {
            tree_command(tree);
        }
        Command::Squash(squash) => {
            squash_command(squash);
        }
        Command::Init(init) => init_command(init),
        Command::New(new) => new_command(new),
        Command::Retrieve(retrieve) => retrieve_command(retrieve),
        Command::Find(find) => find_command(find),
        Command::Export(export) => export_command(export),
        Command::Stats(stats) => stats_command(stats),
        Command::Rename(rename) => rename_command(rename),
        Command::Delete(delete) => delete_command(delete),
        Command::Extract(extract) => extract_command(extract),
        Command::Inline(inline) => inline_command(inline),
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
fn tree_command(args: TreeArgs) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let root_keys: Vec<Key> = if args.key.is_empty() {
        let paths = graph.paths();
        paths
            .iter()
            .filter(|n| n.ids().len() == 1)
            .map(|n| (&graph).node(n.first_id()).node_key())
            .sorted()
            .unique()
            .collect()
    } else {
        args.key.iter().map(|k| Key::name(k)).collect()
    };

    for root_key in &root_keys {
        if (&graph).get_node_id(root_key).is_none() {
            eprintln!("Error: Document '{}' not found", root_key);
            std::process::exit(1);
        }
    }

    match args.format {
        TreeFormat::Json => {
            let mut trees: Vec<TreeNode> = Vec::new();
            for root_key in &root_keys {
                let mut visited: std::collections::HashSet<Key> = std::collections::HashSet::new();
                if let Some(node) = build_tree_node(&graph, root_key, args.depth, &mut visited) {
                    trees.push(node);
                }
            }
            let json = serde_json::to_string_pretty(&trees).expect("Failed to serialize to JSON");
            println!("{}", json);
        }
        _ => {
            let mut tree_lines: std::collections::BTreeMap<String, Vec<(usize, String)>> =
                std::collections::BTreeMap::new();

            for root_key in &root_keys {
                let root_key_str = root_key.to_string();
                let mut visited: std::collections::HashSet<Key> = std::collections::HashSet::new();
                build_tree_lines(
                    &graph,
                    root_key,
                    1,
                    args.depth,
                    &args.format,
                    &mut visited,
                    &mut tree_lines,
                    &root_key_str,
                );
            }

            for (_root, lines) in tree_lines {
                for (depth, line) in lines {
                    let indent = match args.format {
                        TreeFormat::Markdown => "  ".repeat(depth.saturating_sub(1)),
                        _ => "\t".repeat(depth.saturating_sub(1)),
                    };
                    let prefix = match args.format {
                        TreeFormat::Markdown => format!("{}- ", indent),
                        _ => indent,
                    };
                    println!("{}{}", prefix, line);
                }
            }
        }
    }
}

fn build_tree_node(
    graph: &Graph,
    key: &Key,
    max_depth: u8,
    visited: &mut std::collections::HashSet<Key>,
) -> Option<TreeNode> {
    if graph.get_node_id(key).is_none() {
        return None;
    }

    let title = graph.get_ref_text(key).unwrap_or_default();
    let key_str = key.to_string();

    if visited.contains(key) {
        return Some(TreeNode {
            key: key_str,
            title,
            children: vec![],
        });
    }
    visited.insert(key.clone());

    let children = if max_depth > 1 {
        let ref_node_ids = graph.get_block_references_in(key);
        ref_node_ids
            .iter()
            .filter_map(|id| graph.graph_node(*id).ref_key())
            .sorted()
            .filter_map(|ref_key| build_tree_node(graph, &ref_key, max_depth - 1, visited))
            .collect()
    } else {
        vec![]
    };

    Some(TreeNode {
        key: key_str,
        title,
        children,
    })
}

fn build_tree_lines(
    graph: &Graph,
    key: &Key,
    depth: u8,
    max_depth: u8,
    format: &TreeFormat,
    visited: &mut std::collections::HashSet<Key>,
    tree_lines: &mut std::collections::BTreeMap<String, Vec<(usize, String)>>,
    root_key_str: &str,
) {
    if depth > max_depth {
        return;
    }

    if graph.get_node_id(key).is_none() {
        return;
    }

    let line = match format {
        TreeFormat::Keys => key.to_string(),
        TreeFormat::Markdown => {
            let text = graph.get_ref_text(key).unwrap_or_default();
            format!("[{}]({})", text, key)
        }
        TreeFormat::Json => unreachable!(),
    };

    tree_lines
        .entry(root_key_str.to_string())
        .or_default()
        .push((depth as usize, line));

    if visited.contains(key) {
        return;
    }
    visited.insert(key.clone());

    let ref_node_ids = graph.get_block_references_in(key);
    let ref_keys: Vec<Key> = ref_node_ids
        .iter()
        .filter_map(|id| graph.graph_node(*id).ref_key())
        .sorted()
        .collect();
    for ref_key in &ref_keys {
        build_tree_lines(
            graph,
            ref_key,
            depth + 1,
            max_depth,
            format,
            visited,
            tree_lines,
            root_key_str,
        );
    }
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

fn apply_changes(changes: &Changes, configuration: &Configuration) {
    let library_path = get_library_path(configuration);

    for key in &changes.removes {
        let file_path = library_path.join(format!("{}.md", key));
        if file_path.exists() {
            std::fs::remove_file(&file_path).expect("Failed to delete document file");
        }
    }

    for (key, markdown) in &changes.creates {
        let file_path = library_path.join(format!("{}.md", key));
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&file_path, markdown).expect("Failed to write document file");
    }

    for (key, markdown) in &changes.updates {
        let file_path = library_path.join(format!("{}.md", key));
        std::fs::write(&file_path, markdown).expect("Failed to write document file");
    }
}

fn load_graph(configuration: &Configuration) -> Graph {
    Graph::import(
        &new_for_path(&get_library_path(configuration)),
        configuration.markdown.clone(),
        configuration.library.frontmatter_document_title.clone(),
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

#[tracing::instrument(level = "debug")]
fn rename_command(args: Rename) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let old_key = Key::name(&args.old_key);
    let new_key = Key::name(&args.new_key);

    let result = match op_rename(&graph, &old_key, &new_key) {
        Ok(changes) => changes,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if args.keys {
        for key in result.affected_keys() {
            println!("{}", key);
        }
        if args.dry_run {
            return;
        }
    }

    if !args.quiet && !args.keys {
        if args.dry_run {
            println!("Would rename '{}' to '{}'", old_key, new_key);
            println!("Would update {} document(s)", result.updates.len());
            for (key, _) in &result.updates {
                println!("  {}", key);
            }
            return;
        }
        println!("Renaming '{}' to '{}'", old_key, new_key);
    }

    if !args.dry_run {
        apply_changes(&result, &config);
        if !args.quiet && !args.keys {
            println!("Updated {} document(s)", result.updates.len());
        }
    }
}

#[tracing::instrument(level = "debug")]
fn delete_command(args: Delete) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let target_key = Key::name(&args.key);

    let result = match op_delete(&graph, &target_key) {
        Ok(changes) => changes,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if args.keys {
        for key in result.affected_keys() {
            println!("{}", key);
        }
        if args.dry_run {
            return;
        }
    }

    if !args.quiet && !args.keys {
        if args.dry_run {
            println!("Would delete '{}'", target_key);
            println!("Would update {} document(s)", result.updates.len());
            for (key, _) in &result.updates {
                println!("  {}", key);
            }
            return;
        }
    }

    if !args.force && !args.dry_run {
        print!(
            "Delete '{}' and update {} reference(s)? [y/N] ",
            target_key,
            result.updates.len()
        );
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Aborted");
            return;
        }
    }

    if !args.quiet && !args.keys {
        println!("Deleting '{}'", target_key);
    }

    if !args.dry_run {
        apply_changes(&result, &config);
        if !args.quiet && !args.keys {
            println!("Updated {} document(s)", result.updates.len());
        }
    }
}

fn collect_sections(tree: &ModelTree, sections: &mut Vec<(usize, String, Option<liwe::model::NodeId>)>) {
    match &tree.node {
        Node::Section(inlines) => {
            let title = inlines
                .iter()
                .map(|i| i.plain_text())
                .collect::<String>();
            sections.push((sections.len() + 1, title, tree.id));
        }
        _ => {}
    }
    for child in &tree.children {
        collect_sections(child, sections);
    }
}

fn collect_block_references(
    tree: &ModelTree,
    refs: &mut Vec<(usize, String, Key, Option<liwe::model::NodeId>)>,
) {
    if let Node::Reference(reference) = &tree.node {
        refs.push((
            refs.len() + 1,
            reference.text.clone(),
            reference.key.clone(),
            tree.id,
        ));
    }
    for child in &tree.children {
        collect_block_references(child, refs);
    }
}

fn get_extract_config(config: &Configuration, action_name: Option<&str>) -> (String, Option<LinkType>) {
    if let Some(name) = action_name {
        if let Some(ActionDefinition::Extract(extract)) = config.actions.get(name) {
            return (extract.key_template.clone(), extract.link_type.clone());
        }
        eprintln!("Error: Action '{}' not found or not an extract action", name);
        std::process::exit(1);
    }

    for (_, action) in &config.actions {
        if let ActionDefinition::Extract(extract) = action {
            return (extract.key_template.clone(), extract.link_type.clone());
        }
    }

    ("{{slug}}".to_string(), Some(LinkType::Markdown))
}

fn get_inline_config(
    config: &Configuration,
    action_name: Option<&str>,
    as_quote: bool,
    keep_target: bool,
) -> (InlineType, bool) {
    let mut inline_type = InlineType::Section;
    let mut should_keep_target = false;

    if let Some(name) = action_name {
        if let Some(ActionDefinition::Inline(inline)) = config.actions.get(name) {
            inline_type = inline.inline_type.clone();
            should_keep_target = inline.keep_target.unwrap_or(false);
        } else {
            eprintln!(
                "Error: Action '{}' not found or not an inline action",
                name
            );
            std::process::exit(1);
        }
    }

    if as_quote {
        inline_type = InlineType::Quote;
    }
    if keep_target {
        should_keep_target = true;
    }

    (inline_type, should_keep_target)
}

#[tracing::instrument(level = "debug")]
fn extract_command(args: Extract) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let source_key = Key::name(&args.key);

    if (&graph).get_node_id(&source_key).is_none() {
        eprintln!("Error: Document '{}' not found", args.key);
        std::process::exit(1);
    }

    let tree = (&graph).collect(&source_key);
    let mut sections: Vec<(usize, String, Option<liwe::model::NodeId>)> = Vec::new();
    collect_sections(&tree, &mut sections);

    if args.list {
        for (num, title, _) in &sections {
            println!("{}: {}", num, title);
        }
        return;
    }

    let selected_section = if let Some(ref section_title) = args.section {
        let matches: Vec<_> = sections
            .iter()
            .filter(|(_, title, _)| title.to_lowercase().contains(&section_title.to_lowercase()))
            .collect();

        if matches.is_empty() {
            eprintln!("Error: No section matches '{}'", section_title);
            std::process::exit(1);
        } else if matches.len() > 1 {
            eprintln!("Error: Multiple sections match '{}':", section_title);
            for (num, title, _) in &matches {
                eprintln!("  {}: {}", num, title);
            }
            eprintln!("Use --block <n> to select a specific section.");
            std::process::exit(1);
        }

        matches[0].clone()
    } else if let Some(block_num) = args.block {
        if block_num == 0 || block_num > sections.len() {
            eprintln!(
                "Error: Block number {} out of range (1-{})",
                block_num,
                sections.len()
            );
            std::process::exit(1);
        }
        sections[block_num - 1].clone()
    } else {
        eprintln!("Error: Must specify --section, --block, or --list");
        std::process::exit(1);
    };

    let (_, section_title, section_node_id) = selected_section;
    let section_id = section_node_id.expect("Section must have an ID");

    let (key_template, link_type) = get_extract_config(&config, args.action.as_deref());
    let extract_config = ExtractConfig {
        key_template,
        link_type,
        key_date_format: config
            .library
            .date_format
            .clone()
            .unwrap_or_else(|| "%Y-%m-%d".to_string()),
    };

    let result = match op_extract(&graph, &source_key, section_id, &extract_config) {
        Ok(changes) => changes,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let new_key = result
        .creates
        .first()
        .map(|(k, _)| k.clone())
        .expect("Extract should create a new document");

    if args.keys {
        for key in result.affected_keys() {
            println!("{}", key);
        }
        if args.dry_run {
            return;
        }
    }

    if !args.quiet && !args.keys {
        if args.dry_run {
            println!("Would extract section '{}' to '{}'", section_title, new_key);
            println!("Would update '{}'", source_key);
            return;
        }
        println!("Extracting section '{}' to '{}'", section_title, new_key);
    }

    if !args.dry_run {
        apply_changes(&result, &config);
        if !args.quiet && !args.keys {
            println!("Done");
        }
    }
}

#[tracing::instrument(level = "debug")]
fn inline_command(args: Inline) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let source_key = Key::name(&args.key);

    if (&graph).get_node_id(&source_key).is_none() {
        eprintln!("Error: Document '{}' not found", args.key);
        std::process::exit(1);
    }

    let tree = (&graph).collect(&source_key);
    let mut refs: Vec<(usize, String, Key, Option<liwe::model::NodeId>)> = Vec::new();
    collect_block_references(&tree, &mut refs);

    if args.list {
        for (num, text, key, _) in &refs {
            println!("{}: [{}]({})", num, text, key);
        }
        return;
    }

    let selected_ref = if let Some(ref reference) = args.reference {
        let matches: Vec<_> = refs
            .iter()
            .filter(|(_, text, key, _)| {
                text.to_lowercase().contains(&reference.to_lowercase())
                    || key.to_string().to_lowercase().contains(&reference.to_lowercase())
            })
            .collect();

        if matches.is_empty() {
            eprintln!("Error: No reference matches '{}'", reference);
            std::process::exit(1);
        } else if matches.len() > 1 {
            eprintln!("Error: Multiple references match '{}':", reference);
            for (num, text, key, _) in &matches {
                eprintln!("  {}: [{}]({})", num, text, key);
            }
            eprintln!("Use --block <n> to select a specific reference.");
            std::process::exit(1);
        }

        matches[0].clone()
    } else if let Some(block_num) = args.block {
        if block_num == 0 || block_num > refs.len() {
            eprintln!(
                "Error: Block number {} out of range (1-{})",
                block_num,
                refs.len()
            );
            std::process::exit(1);
        }
        refs[block_num - 1].clone()
    } else {
        eprintln!("Error: Must specify --reference, --block, or --list");
        std::process::exit(1);
    };

    let (_, ref_text, inline_key, ref_node_id) = selected_ref;
    let ref_id = ref_node_id.expect("Reference must have an ID");

    let (inline_type, should_keep_target) =
        get_inline_config(&config, args.action.as_deref(), args.as_quote, args.keep_target);

    let inline_config = InlineConfig {
        inline_type,
        keep_target: should_keep_target,
    };

    let result = match op_inline(&graph, &source_key, ref_id, &inline_config) {
        Ok(changes) => changes,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if args.keys {
        for key in result.affected_keys() {
            println!("{}", key);
        }
        if args.dry_run {
            return;
        }
    }

    if !args.quiet && !args.keys {
        if args.dry_run {
            println!(
                "Would inline [{}]({}) into '{}'",
                ref_text, inline_key, source_key
            );
            if !should_keep_target {
                println!("Would delete '{}'", inline_key);
                if !result.updates.is_empty() {
                    println!(
                        "Would update {} additional document(s)",
                        result.updates.len() - 1
                    );
                }
            }
            return;
        }
        println!(
            "Inlining [{}]({}) into '{}'",
            ref_text, inline_key, source_key
        );
    }

    if !args.dry_run {
        apply_changes(&result, &config);
        if !args.quiet && !args.keys {
            println!("Done");
        }
    }
}
