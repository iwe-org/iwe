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
use iwe::projection_args::{parse_projection_extend, parse_projection_replace};
use iwe::render::{FindBlockRenderer, RetrieveRenderer};
use iwe::retrieve::{DocumentReader, RetrieveOptions};
use iwe::filter_args::FilterArgs;
use liwe::query::{
    FieldPath, Filter, Projection as QueryProjection, ProjectionSource, PseudoField,
    Sort as QuerySort, SortDir,
};
use iwe::stats::{render_stats, GraphStatistics};
use liwe::fs::new_for_path;
use liwe::graph::{Graph, GraphContext};
use liwe::locale::get_locale;
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
    Count(Count),
    Normalize(Normalize),
    Tree(TreeArgs),
    Squash(Squash),
    Export(Export),
    Stats(Stats),
    Rename(Rename),
    Delete(Delete),
    Extract(Extract),
    Inline(Inline),
    Update(Update),
    Attach(Attach),
}

#[derive(Debug, Args)]
#[clap(
    about = help::retrieve::ABOUT,
    long_about = help::retrieve::LONG_ABOUT,
    after_help = help::retrieve::AFTER_HELP
)]
struct Retrieve {
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

    #[clap(long, help = "Populate the `includes` array with child document edges")]
    children: bool,

    #[clap(flatten)]
    selector: FilterArgs,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum RetrieveFormat {
    Markdown,
    Keys,
    Json,
    Yaml,
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

    #[clap(long, short = 'l', help = "Maximum results (0 = unlimited)")]
    limit: Option<usize>,

    #[clap(
        long,
        value_parser = parse_projection_replace,
        help = "Projection: comma-list (name, name=path, name=$selector, $selector) or inline YAML mapping. Replaces the default."
    )]
    project: Option<QueryProjection>,

    #[clap(
        long = "add-fields",
        value_parser = parse_projection_extend,
        conflicts_with = "project",
        help = "Additive projection: same grammar as --project, extends defaults rather than replacing."
    )]
    add_fields: Option<QueryProjection>,

    #[clap(
        long,
        help = "Sort by frontmatter field. Format: field:1 (asc) or field:-1 (desc)."
    )]
    sort: Option<String>,

    #[clap(long, short = 'f', value_enum, default_value = "markdown")]
    format: FindFormat,

    #[clap(flatten)]
    selector: FilterArgs,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum FindFormat {
    Markdown,
    Keys,
    Json,
    Yaml,
}

#[derive(Debug, Args)]
#[clap(
    about = help::count::ABOUT,
    long_about = help::count::LONG_ABOUT,
    after_help = help::count::AFTER_HELP
)]
struct Count {
    #[clap(long, short = 'l', help = "Cap the number of matches counted (0 = unlimited)")]
    limit: Option<usize>,

    #[clap(flatten)]
    selector: FilterArgs,
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
        help = "Output format: markdown (nested list with links), keys, json, yaml"
    )]
    format: TreeFormat,

    #[clap(
        long,
        short = 'd',
        default_value = "4",
        help = "Maximum depth to traverse"
    )]
    depth: u8,

    #[clap(
        long,
        value_parser = parse_projection_replace,
        help = "Projection: comma-list (name, name=path, name=$selector, $selector) or inline YAML mapping. Replaces user-frontmatter additions."
    )]
    project: Option<QueryProjection>,

    #[clap(
        long = "add-fields",
        value_parser = parse_projection_extend,
        conflicts_with = "project",
        help = "Additive projection: extends each tree node's default fields. Same grammar as --project."
    )]
    add_fields: Option<QueryProjection>,

    #[clap(flatten)]
    selector: FilterArgs,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum TreeFormat {
    Markdown,
    Keys,
    Json,
    Yaml,
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

    #[clap(
        long,
        short = 'k',
        help = "Document key for per-document stats. Omit for aggregate graph statistics."
    )]
    key: Option<String>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum StatsFormat {
    Markdown,
    Csv,
    Json,
    Yaml,
}

#[derive(Debug, Args)]
#[clap(
    about = help::export::ABOUT,
    long_about = help::export::LONG_ABOUT,
    after_help = help::export::AFTER_HELP
)]
struct Export {
    #[clap(
        long,
        short = 'f',
        value_enum,
        default_value = "dot",
        help = "Output format"
    )]
    format: Format,
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

    #[clap(flatten)]
    selector: FilterArgs,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum Format {
    Dot,
}

#[derive(Debug, Clone, clap::ValueEnum, PartialEq, Eq)]
enum MutationFormat {
    Markdown,
    Keys,
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

    #[clap(
        long,
        short = 'f',
        value_enum,
        default_value = "markdown",
        help = "Output format. `keys` prints affected document keys (one per line) and suppresses progress."
    )]
    format: MutationFormat,

    #[clap(long = "keys", hide = true)]
    keys_legacy: bool,
}

#[derive(Debug, Args)]
#[clap(
    about = help::delete::ABOUT,
    long_about = help::delete::LONG_ABOUT,
    after_help = help::delete::AFTER_HELP
)]
struct Delete {
    #[clap(help = "Document key to delete (sugar for --filter '$key: K')")]
    key: Option<String>,

    #[clap(long, help = "Filter expression (inline YAML). Required if positional KEY omitted.")]
    filter: Option<String>,

    #[clap(long, help = "Preview changes without writing to disk")]
    dry_run: bool,

    #[clap(long, help = "Suppress progress output")]
    quiet: bool,

    #[clap(
        long,
        short = 'f',
        value_enum,
        default_value = "markdown",
        help = "Output format. `keys` prints affected document keys (one per line) and suppresses progress."
    )]
    format: MutationFormat,

    #[clap(long = "keys", hide = true)]
    keys_legacy: bool,
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

    #[clap(
        long,
        short = 'f',
        value_enum,
        default_value = "markdown",
        help = "Output format. `keys` prints affected document keys (one per line) and suppresses progress."
    )]
    format: MutationFormat,

    #[clap(long = "keys", hide = true)]
    keys_legacy: bool,
}

#[derive(Debug, Args)]
#[clap(
    about = help::update::ABOUT,
    long_about = help::update::LONG_ABOUT,
    after_help = help::update::AFTER_HELP
)]
struct Update {
    #[clap(long, short = 'k', help = "Document key. Required for body-overwrite mode; optional in frontmatter mutation mode.")]
    key: Option<String>,

    #[clap(
        long,
        short = 'c',
        help = "New full markdown content (body-overwrite mode). Use '-' to read from stdin."
    )]
    content: Option<String>,

    #[clap(
        long,
        help = "Filter expression for frontmatter mutation mode (inline YAML). Combined with -k via AND."
    )]
    filter: Option<String>,

    #[clap(
        long,
        help = "Frontmatter $set assignment FIELD=VALUE. VALUE is parsed as a YAML scalar."
    )]
    set: Vec<String>,

    #[clap(long, help = "Frontmatter $unset field name.")]
    unset: Vec<String>,

    #[clap(long, help = "Preview without writing")]
    dry_run: bool,

    #[clap(long, help = "Suppress progress output")]
    quiet: bool,
}

#[derive(Debug, Args)]
#[clap(
    about = help::attach::ABOUT,
    long_about = help::attach::LONG_ABOUT,
    after_help = help::attach::AFTER_HELP
)]
struct Attach {
    #[clap(
        long,
        help = "Configured attach action(s) to attach to. Repeatable for multiple targets."
    )]
    to: Vec<String>,

    #[clap(long, short = 'k', help = "Source document key to attach")]
    key: Option<String>,

    #[clap(long, help = "List configured attach actions")]
    list: bool,

    #[clap(long, help = "Preview without writing")]
    dry_run: bool,

    #[clap(long, help = "Suppress progress output")]
    quiet: bool,
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

    #[clap(
        long,
        short = 'f',
        value_enum,
        default_value = "markdown",
        help = "Output format. `keys` prints affected document keys (one per line) and suppresses progress."
    )]
    format: MutationFormat,

    #[clap(long = "keys", hide = true)]
    keys_legacy: bool,
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
        Command::Count(count) => count_command(count),
        Command::Export(export) => export_command(export),
        Command::Stats(stats) => stats_command(stats),
        Command::Rename(rename) => rename_command(rename),
        Command::Delete(delete) => delete_command(delete),
        Command::Extract(extract) => extract_command(extract),
        Command::Inline(inline) => inline_command(inline),
        Command::Update(update) => update_command(update),
        Command::Attach(attach) => attach_command(attach),
    }
}

#[tracing::instrument(level = "debug")]
fn retrieve_command(args: Retrieve) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let explicit_keys = args.selector.key.clone();
    let other_selectors_present = args.selector.has_non_key_clauses();

    let key_strings: Vec<String> = if explicit_keys.is_empty() {
        let stdin_content = read_stdin_if_available();
        let keys: Vec<String> = stdin_content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if keys.is_empty() && !other_selectors_present {
            eprintln!("Error: No document key provided. Use -k <key>, --filter, or pipe keys via stdin.");
            std::process::exit(1);
        }
        keys
    } else {
        explicit_keys
    };

    let mut keys = Vec::new();
    for key_str in &key_strings {
        let key = Key::name(key_str);
        if (&graph).get_node_id(&key).is_none() {
            eprintln!("Error: Document '{}' not found", key_str);
            std::process::exit(1);
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
        children: args.children,
        filter: resolve_filter(&args.selector, &graph),
    };

    let output = reader.retrieve_many(&keys, &options);

    if args.dry_run {
        let doc_count = output.documents.len();
        let total_lines: usize = output
            .documents
            .iter()
            .map(|doc| doc.content.lines().count())
            .sum();
        match args.format {
            RetrieveFormat::Json => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "documents": doc_count,
                    "lines": total_lines,
                }))
                .expect("Failed to serialize to JSON");
                println!("{}", json);
            }
            RetrieveFormat::Yaml => {
                let mut map = serde_yaml::Mapping::new();
                map.insert("documents".into(), (doc_count as u64).into());
                map.insert("lines".into(), (total_lines as u64).into());
                let yaml = serde_yaml::to_string(&map).expect("Failed to serialize to YAML");
                print!("{}", yaml);
            }
            _ => {
                println!("documents: {}", doc_count);
                println!("lines: {}", total_lines);
            }
        }
        return;
    }

    match args.format {
        RetrieveFormat::Json => {
            let json = serde_json::to_string_pretty(&output.documents)
                .expect("Failed to serialize to JSON");
            println!("{}", json);
        }
        RetrieveFormat::Yaml => {
            let yaml = serde_yaml::to_string(&output.documents)
                .expect("Failed to serialize to YAML");
            print!("{}", yaml);
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

#[tracing::instrument(level = "debug")]
fn find_command(args: Find) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let sort = args.sort.as_deref().map(parse_sort_arg).transpose().unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        std::process::exit(2);
    });
    let project = args.project.clone().or_else(|| args.add_fields.clone());

    let finder = DocumentFinder::new(&graph);
    let options = FindOptions {
        query: args.query,
        refs_to: None,
        refs_from: None,
        filter: resolve_filter(&args.selector, &graph),
        limit: args.limit,
        sort,
        project: project.clone(),
    };

    let output = finder.find(&options);

    match args.format {
        FindFormat::Json => {
            let json = serde_json::to_string_pretty(&output.results)
                .expect("Failed to serialize to JSON");
            println!("{}", json);
        }
        FindFormat::Yaml => {
            let yaml = serde_yaml::to_string(&output.results)
                .expect("Failed to serialize to YAML");
            print!("{}", yaml);
        }
        FindFormat::Keys => {
            for result in &output.results {
                if let Some(key) = result.get("key").and_then(|v| v.as_str()) {
                    println!("{}", key);
                }
            }
        }
        FindFormat::Markdown => {
            let content_output_names: Vec<String> = match &project {
                Some(p) => p
                    .fields
                    .iter()
                    .filter(|f| matches!(&f.source, ProjectionSource::Pseudo(PseudoField::Content)))
                    .map(|f| f.output.clone())
                    .collect(),
                None => Vec::new(),
            };
            let md_options = graph.markdown_options();
            let renderer = FindBlockRenderer::new(&md_options, &graph);
            print!(
                "{}",
                renderer.render(&output.keys, &output.results, &content_output_names)
            );
        }
    }
}

#[tracing::instrument(level = "debug")]
fn count_command(args: Count) {
    use liwe::query::{execute, CountOp, Operation, Outcome};

    let config = get_configuration();
    let graph = load_graph(&config);

    let mut op = CountOp::new();
    if let Some(f) = resolve_filter(&args.selector, &graph) {
        op = op.filter(f);
    }
    if let Some(n) = args.limit {
        if n > 0 {
            op = op.limit(n as u64);
        }
    }

    match execute(&Operation::Count(op), &graph) {
        Outcome::Count(n) => println!("{}", n),
        _ => unreachable!(),
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

    let toml = toml::to_string(&Configuration::template()).expect("valid TOML");

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

    let explicit_keys: Vec<Key> = args.selector.key.iter().map(|k| Key::name(k)).collect();
    let other_selectors = args.selector.has_non_key_clauses();
    let filter_for_narrowing = if other_selectors {
        let mut s = args.selector.clone();
        s.key.clear();
        resolve_filter(&s, &graph)
    } else {
        None
    };
    let filter = filter_for_narrowing;

    let root_keys: Vec<Key> = if let Some(f) = filter {
        let selector_set: std::collections::HashSet<Key> =
            liwe::query::evaluate(&f, &graph).into_iter().collect();
        if explicit_keys.is_empty() {
            let mut v: Vec<Key> = selector_set.into_iter().collect();
            v.sort();
            v
        } else {
            explicit_keys
                .into_iter()
                .filter(|k| selector_set.contains(k))
                .collect()
        }
    } else if explicit_keys.is_empty() {
        let paths = graph.paths();
        paths
            .iter()
            .filter(|n| n.ids().len() == 1)
            .filter_map(|n| n.first_id())
            .map(|id| (&graph).node(id).node_key())
            .sorted()
            .unique()
            .collect()
    } else {
        explicit_keys
    };

    for root_key in &root_keys {
        if (&graph).get_node_id(root_key).is_none() {
            eprintln!("Error: Document '{}' not found", root_key);
            std::process::exit(1);
        }
    }

    match args.format {
        TreeFormat::Json | TreeFormat::Yaml => {
            let project = args.project.clone().or_else(|| args.add_fields.clone());
            let mut trees: Vec<serde_yaml::Mapping> = Vec::new();
            for root_key in &root_keys {
                let mut visited: std::collections::HashSet<Key> = std::collections::HashSet::new();
                if let Some(node) = build_tree_node(
                    &graph,
                    root_key,
                    args.depth,
                    project.as_ref(),
                    &mut visited,
                ) {
                    trees.push(node);
                }
            }
            match args.format {
                TreeFormat::Yaml => {
                    let yaml = serde_yaml::to_string(&trees).expect("Failed to serialize to YAML");
                    print!("{}", yaml);
                }
                _ => {
                    let json = serde_json::to_string_pretty(&trees)
                        .expect("Failed to serialize to JSON");
                    println!("{}", json);
                }
            }
        }
        TreeFormat::Markdown | TreeFormat::Keys => {
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
    project: Option<&QueryProjection>,
    visited: &mut std::collections::HashSet<Key>,
) -> Option<serde_yaml::Mapping> {
    use liwe::query::project::{apply_projection, ProjectionContext};

    graph.get_node_id(key)?;

    let title = graph.get_ref_text(key).unwrap_or_default();
    let key_str = key.to_string();
    let already_visited = visited.contains(key);
    if !already_visited {
        visited.insert(key.clone());
    }

    let children: Vec<serde_yaml::Mapping> = if !already_visited && max_depth > 1 {
        let ref_node_ids = graph.get_inclusion_edges_in(key);
        ref_node_ids
            .iter()
            .filter_map(|id| graph.graph_node(*id).ref_key())
            .sorted()
            .filter_map(|ref_key| {
                build_tree_node(graph, &ref_key, max_depth - 1, project, visited)
            })
            .collect()
    } else {
        vec![]
    };

    let mut node = serde_yaml::Mapping::new();
    node.insert(serde_yaml::Value::from("key"), serde_yaml::Value::from(key_str));
    node.insert(serde_yaml::Value::from("title"), serde_yaml::Value::from(title));

    if let Some(p) = project {
        let ctx = ProjectionContext { graph, key };
        let projected = apply_projection(&ctx, p);
        for (k, v) in projected {
            if let Some(s) = k.as_str() {
                if matches!(s, "key" | "title" | "children") {
                    continue;
                }
            }
            node.insert(k, v);
        }
    }

    let children_value = serde_yaml::to_value(&children)
        .unwrap_or_else(|_| serde_yaml::Value::Sequence(Vec::new()));
    node.insert(serde_yaml::Value::from("children"), children_value);

    Some(node)
}

#[allow(clippy::too_many_arguments)]
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
        TreeFormat::Json | TreeFormat::Yaml => unreachable!(),
    };

    tree_lines
        .entry(root_key_str.to_string())
        .or_default()
        .push((depth as usize, line));

    if visited.contains(key) {
        return;
    }
    visited.insert(key.clone());

    let ref_node_ids = graph.get_inclusion_edges_in(key);
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

    print!("{}", patch.export_key(&args.key.into()).unwrap_or_default())
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

fn parse_sort_arg(s: &str) -> Result<QuerySort, String> {
    let (field, dir) = s
        .rsplit_once(':')
        .ok_or_else(|| format!("invalid --sort value '{}': expected FIELD:1 or FIELD:-1", s))?;
    let dir = match dir {
        "1" => SortDir::Asc,
        "-1" => SortDir::Desc,
        _ => return Err(format!("invalid sort direction '{}': expected 1 or -1", dir)),
    };
    if field.is_empty() {
        return Err(format!("invalid --sort value '{}': empty field", s));
    }
    Ok(QuerySort {
        key: FieldPath::from_dotted(field),
        dir,
    })
}

fn resolve_filter(args: &FilterArgs, graph: &Graph) -> Option<Filter> {
    let base = args.to_filter().unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        std::process::exit(2);
    });
    apply_roots(base, args.roots, graph)
}

fn apply_roots(base: Option<Filter>, roots: bool, graph: &Graph) -> Option<Filter> {
    if !roots {
        return base;
    }
    let rk: Vec<Key> = graph
        .keys()
        .into_iter()
        .filter(|k| graph.get_inclusion_edges_to(k).is_empty())
        .collect();
    let roots_filter = Filter::Key(liwe::query::KeyOp::In(rk));
    Some(match base {
        Some(f) => Filter::And(vec![f, roots_filter]),
        None => roots_filter,
    })
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

    if let Some(key_str) = args.key {
        let key_stats = liwe::stats::KeyStatistics::from_graph(&graph);
        let entry = key_stats
            .into_iter()
            .find(|s| s.key == key_str);
        match entry {
            Some(s) => match args.format {
                StatsFormat::Markdown => {
                    println!("# {}\n", s.title);
                    println!("- **Key:** {}", s.key);
                    println!("- **Sections:** {}", s.sections);
                    println!("- **Paragraphs:** {}", s.paragraphs);
                    println!("- **Lines:** {}", s.lines);
                    println!("- **Words:** {}", s.words);
                    println!("- **Included by:** {}", s.included_by_count);
                    println!("- **Referenced by:** {}", s.referenced_by_count);
                    println!("- **Incoming edges:** {}", s.incoming_edges_count);
                    println!("- **Includes:** {}", s.includes_count);
                    println!("- **References:** {}", s.references_count);
                    println!("- **Total edges:** {}", s.total_edges_count);
                    println!("- **Bullet lists:** {}", s.bullet_lists);
                    println!("- **Ordered lists:** {}", s.ordered_lists);
                    println!("- **Code blocks:** {}", s.code_blocks);
                    println!("- **Tables:** {}", s.tables);
                    println!("- **Quotes:** {}", s.quotes);
                }
                StatsFormat::Csv => {
                    let stdout = std::io::stdout();
                    let mut csv_writer = csv::Writer::from_writer(stdout.lock());
                    csv_writer.serialize(&s).expect("Failed to serialize stats");
                    csv_writer.flush().expect("Failed to flush CSV");
                }
                StatsFormat::Json => {
                    let json = serde_json::to_string_pretty(&s)
                        .expect("Failed to serialize stats");
                    println!("{}", json);
                }
                StatsFormat::Yaml => {
                    let yaml = serde_yaml::to_string(&s)
                        .expect("Failed to serialize stats");
                    print!("{}", yaml);
                }
            },
            None => {
                eprintln!("Error: Document '{}' not found", key_str);
                std::process::exit(1);
            }
        }
        return;
    }

    match args.format {
        StatsFormat::Markdown => {
            let stats = GraphStatistics::from_graph(&graph);
            let output = render_stats(&stats);
            print!("{}", output);
        }
        StatsFormat::Csv => {
            let stdout = std::io::stdout();
            if let Err(e) = GraphStatistics::export_csv(&graph, stdout.lock()) {
                error!("Failed to export CSV: {}", e);
                std::process::exit(1);
            }
        }
        StatsFormat::Json => {
            let stats = GraphStatistics::from_graph(&graph);
            let json = serde_json::to_string_pretty(&stats)
                .expect("Failed to serialize stats");
            println!("{}", json);
        }
        StatsFormat::Yaml => {
            let stats = GraphStatistics::from_graph(&graph);
            let yaml = serde_yaml::to_string(&stats)
                .expect("Failed to serialize stats");
            print!("{}", yaml);
        }
    }
}

#[tracing::instrument]
fn export_command(args: Export) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let explicit_keys: Vec<Key> = args.selector.key.iter().map(|s| Key::name(s)).collect();
    let filter_for_narrowing = if args.selector.has_non_key_clauses() {
        let mut s = args.selector.clone();
        s.key.clear();
        resolve_filter(&s, &graph)
    } else {
        None
    };

    let resolved_keys: Vec<Key> = if let Some(f) = filter_for_narrowing {
        let selector_set: std::collections::HashSet<Key> =
            liwe::query::evaluate(&f, &graph).into_iter().collect();
        let mut v: Vec<Key> = if explicit_keys.is_empty() {
            selector_set.into_iter().collect()
        } else {
            explicit_keys
                .into_iter()
                .filter(|k| selector_set.contains(k))
                .collect()
        };
        v.sort();
        v
    } else {
        explicit_keys
    };

    let data = graph_data::graph_data(resolved_keys, args.depth, &graph);

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

    let keys_mode = args.format == MutationFormat::Keys || args.keys_legacy;

    if keys_mode {
        for key in result.affected_keys() {
            println!("{}", key);
        }
        if args.dry_run {
            return;
        }
    }

    if !args.quiet && !keys_mode {
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
        if !args.quiet && !keys_mode {
            println!("Updated {} document(s)", result.updates.len());
        }
    }
}

#[tracing::instrument(level = "debug")]
fn delete_command(args: Delete) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let targets = resolve_delete_targets(&args, &graph);
    if targets.is_empty() {
        if !args.quiet {
            eprintln!("No documents matched");
        }
        return;
    }

    let mut combined = liwe::operations::Changes::default();
    for target in &targets {
        match op_delete(&graph, target) {
            Ok(changes) => merge_changes(&mut combined, changes),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    let keys_mode = args.format == MutationFormat::Keys || args.keys_legacy;

    if keys_mode {
        for key in combined.affected_keys() {
            println!("{}", key);
        }
        if args.dry_run {
            return;
        }
    }

    if !args.quiet && !keys_mode && args.dry_run {
        for target in &targets {
            println!("Would delete '{}'", target);
        }
        println!("Would update {} document(s)", combined.updates.len());
        for (key, _) in &combined.updates {
            println!("  {}", key);
        }
        return;
    }

    if !args.quiet && !keys_mode {
        for target in &targets {
            println!("Deleting '{}'", target);
        }
    }

    if !args.dry_run {
        apply_changes(&combined, &config);
        if !args.quiet && !keys_mode {
            println!("Updated {} document(s)", combined.updates.len());
        }
    }
}

fn resolve_delete_targets(args: &Delete, graph: &Graph) -> Vec<Key> {
    let mut targets: Vec<Key> = Vec::new();
    if let Some(k) = &args.key {
        targets.push(Key::name(k));
    }
    if let Some(expr) = &args.filter {
        let filter = liwe::query::parse_filter_expression(expr).unwrap_or_else(|e| {
            eprintln!("error: invalid --filter expression: {}", e);
            std::process::exit(2);
        });
        let matched = liwe::query::evaluate(&filter, graph);
        targets.extend(matched);
    }
    if targets.is_empty() {
        eprintln!("Error: provide a positional KEY or --filter");
        std::process::exit(1);
    }
    targets.sort();
    targets.dedup();
    targets
}

fn merge_changes(into: &mut liwe::operations::Changes, other: liwe::operations::Changes) {
    for k in other.removes {
        if !into.removes.contains(&k) {
            into.removes.push(k);
        }
    }
    for (k, v) in other.creates {
        if !into.creates.iter().any(|(kk, _)| kk == &k) {
            into.creates.push((k, v));
        }
    }
    for (k, v) in other.updates {
        if let Some(slot) = into.updates.iter_mut().find(|(kk, _)| kk == &k) {
            slot.1 = v;
        } else {
            into.updates.push((k, v));
        }
    }
}

fn collect_sections(tree: &ModelTree, sections: &mut Vec<(usize, String, Option<liwe::model::NodeId>)>) {
    if let Node::Section(inlines) = &tree.node {
        let title = inlines
            .iter()
            .map(|i| i.plain_text())
            .collect::<String>();
        sections.push((sections.len() + 1, title, tree.id));
    }
    for child in &tree.children {
        collect_sections(child, sections);
    }
}

fn collect_inclusion_edges(
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
        collect_inclusion_edges(child, refs);
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

    for action in config.actions.values() {
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
    let locale = get_locale(config.library.locale.as_deref());
    let extract_config = ExtractConfig {
        key_template,
        link_type,
        key_date_format: config
            .library
            .date_format
            .clone()
            .unwrap_or_else(|| "%Y-%m-%d".to_string()),
        locale,
    };

    let result = match op_extract(&graph, &source_key, section_id, &extract_config, std::time::SystemTime::now()) {
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

    let keys_mode = args.format == MutationFormat::Keys || args.keys_legacy;

    if keys_mode {
        for key in result.affected_keys() {
            println!("{}", key);
        }
        if args.dry_run {
            return;
        }
    }

    if !args.quiet && !keys_mode {
        if args.dry_run {
            println!("Would extract section '{}' to '{}'", section_title, new_key);
            println!("Would update '{}'", source_key);
            return;
        }
        println!("Extracting section '{}' to '{}'", section_title, new_key);
    }

    if !args.dry_run {
        apply_changes(&result, &config);
        if !args.quiet && !keys_mode {
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
    collect_inclusion_edges(&tree, &mut refs);

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

    let keys_mode = args.format == MutationFormat::Keys || args.keys_legacy;

    if keys_mode {
        for key in result.affected_keys() {
            println!("{}", key);
        }
        if args.dry_run {
            return;
        }
    }

    if !args.quiet && !keys_mode {
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
        if !args.quiet && !keys_mode {
            println!("Done");
        }
    }
}

#[tracing::instrument(level = "debug")]
fn update_command(args: Update) {
    let body_mode = args.content.is_some();
    let mutation_mode = !args.set.is_empty() || !args.unset.is_empty();

    if body_mode && mutation_mode {
        eprintln!("Error: --content cannot be combined with --set or --unset");
        std::process::exit(1);
    }
    if !body_mode && !mutation_mode {
        eprintln!("Error: provide either --content (body overwrite) or --set/--unset (frontmatter mutation)");
        std::process::exit(1);
    }

    if body_mode {
        update_body(args);
    } else {
        update_frontmatter(args);
    }
}

fn split_raw_frontmatter(content: &str) -> (Option<&str>, &str) {
    if !content.starts_with("---\n") && !content.starts_with("---\r\n") {
        return (None, content);
    }
    let after_open = if content.starts_with("---\r\n") { 5 } else { 4 };
    let rest = &content[after_open..];
    if let Some(close_pos) = rest.find("\n---\n") {
        let end = after_open + close_pos + "\n---\n".len();
        return (Some(&content[..end]), &content[end..]);
    }
    if let Some(close_pos) = rest.find("\r\n---\r\n") {
        let end = after_open + close_pos + "\r\n---\r\n".len();
        return (Some(&content[..end]), &content[end..]);
    }
    if rest.ends_with("\n---\n") || rest.ends_with("\n---") {
        if let Some(close_pos) = rest.rfind("\n---") {
            let end = after_open + close_pos + "\n---".len();
            let trailing = &content[end..];
            return (Some(&content[..end + trailing.len()]), "");
        }
    }
    (None, content)
}

fn update_body(args: Update) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let key_str = args.key.clone().unwrap_or_else(|| {
        eprintln!("Error: -k/--key is required for body-overwrite mode");
        std::process::exit(1);
    });
    let key = Key::name(&key_str);
    if (&graph).get_node_id(&key).is_none() {
        eprintln!("Error: Document '{}' not found", key_str);
        std::process::exit(1);
    }

    let raw = args.content.expect("body mode implies content present");
    let content = if raw == "-" {
        let stdin_content = read_stdin_if_available();
        if stdin_content.is_empty() {
            eprintln!("Error: '--content -' requires content piped via stdin");
            std::process::exit(1);
        }
        stdin_content
    } else {
        raw
    };

    if args.dry_run {
        if !args.quiet {
            println!("Would update '{}' ({} bytes)", key_str, content.len());
        }
        return;
    }

    let library_path = get_library_path(&config);
    let file_path = library_path.join(format!("{}.md", key));
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let existing = std::fs::read_to_string(&file_path).unwrap_or_default();
    let (frontmatter, _) = split_raw_frontmatter(&existing);
    let output = match frontmatter {
        Some(fm) => format!("{}{}", fm, content),
        None => content,
    };
    std::fs::write(&file_path, &output).expect("Failed to write document file");

    if !args.quiet {
        println!("Updated '{}'", key_str);
    }
}

fn update_frontmatter(args: Update) {
    use liwe::query::prelude::find;
    use liwe::query::wire::RawUpdate;
    use liwe::query::{build_update_doc, execute as run_op, FindOp, Outcome};
    use serde_yaml::{Mapping, Value};

    let config = get_configuration();
    let graph = load_graph(&config);

    let mut conjuncts: Vec<Filter> = Vec::new();
    let parsed_filter = args.filter.as_ref().map(|expr| {
        liwe::query::parse_filter_expression(expr).unwrap_or_else(|e| {
            eprintln!("error: invalid --filter expression: {}", e);
            std::process::exit(2);
        })
    });
    if let (Some(parsed), Some(_)) = (parsed_filter.as_ref(), args.key.as_ref()) {
        if filter_has_top_level_key_predicate(parsed) {
            eprintln!(
                "error: -k / --key conflicts with a $key predicate at the top level of --filter; \
                 use --filter '$or: [{{$key: a}}, {{$key: b}}]' for OR-of-keys, or pick one source"
            );
            std::process::exit(2);
        }
    }
    if let Some(parsed) = parsed_filter {
        conjuncts.push(parsed);
    }
    if let Some(k) = &args.key {
        conjuncts.push(Filter::Key(liwe::query::KeyOp::Eq(Key::name(k))));
    }
    if conjuncts.is_empty() {
        eprintln!("Error: --filter or -k/--key required for frontmatter mutation mode");
        std::process::exit(1);
    }
    let filter = if conjuncts.len() == 1 {
        conjuncts.into_iter().next().unwrap()
    } else {
        Filter::And(conjuncts)
    };

    let mut set_map = Mapping::new();
    for assign in &args.set {
        let (field, value) = parse_set_assignment(assign).unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            std::process::exit(2);
        });
        set_map.insert(Value::String(field), value);
    }
    let mut unset_map = Mapping::new();
    for field in &args.unset {
        unset_map.insert(Value::String(field.clone()), Value::String(String::new()));
    }
    let raw_update = RawUpdate {
        set: if set_map.is_empty() { None } else { Some(set_map) },
        unset: if unset_map.is_empty() { None } else { Some(unset_map) },
    };
    let update_doc = build_update_doc(raw_update).unwrap_or_else(|e| {
        eprintln!("error: invalid update: {}", e);
        std::process::exit(2);
    });

    let find_op = FindOp::new().filter(filter);
    let outcome = run_op(&find(find_op), &graph);
    let keys: Vec<Key> = match outcome {
        Outcome::Find { matches } => matches.into_iter().map(|m| m.key).collect(),
        _ => unreachable!(),
    };

    if args.dry_run {
        if !args.quiet {
            println!("Would update {} document(s)", keys.len());
            for key in &keys {
                println!("  {}", key);
            }
        }
        return;
    }

    let library_path = get_library_path(&config);
    let mut count = 0;
    for key in &keys {
        let file_path = library_path.join(format!("{}.md", key));
        let raw_content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (_, body) = split_raw_frontmatter(&raw_content);

        let mut mapping = graph.frontmatter(key).cloned().unwrap_or_default();
        liwe::query::update::apply(&update_doc, &mut mapping);
        liwe::query::frontmatter::strip_reserved(&mut mapping);

        let yaml = if mapping.is_empty() {
            String::new()
        } else {
            let serialized = serde_yaml::to_string(&mapping).unwrap_or_default();
            format!("---\n{}---\n", serialized)
        };
        let output = format!("{}{}", yaml, body);

        std::fs::write(&file_path, &output).expect("Failed to write document file");
        count += 1;
    }

    if !args.quiet {
        println!("Updated {} document(s)", count);
    }
}

fn filter_has_top_level_key_predicate(filter: &Filter) -> bool {
    match filter {
        Filter::Key(_) => true,
        Filter::And(children) => children.iter().any(filter_has_top_level_key_predicate),
        _ => false,
    }
}

fn parse_set_assignment(s: &str) -> Result<(String, serde_yaml::Value), String> {
    let (field, value) = s
        .split_once('=')
        .ok_or_else(|| format!("invalid --set assignment '{}': expected FIELD=VALUE", s))?;
    if field.is_empty() {
        return Err(format!("invalid --set assignment '{}': empty field", s));
    }
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(value)
        .map_err(|e| format!("invalid --set value for '{}': {}", field, e))?;
    Ok((field.to_string(), yaml_value))
}

#[tracing::instrument(level = "debug")]
fn attach_command(args: Attach) {
    let config = get_configuration();

    if args.list {
        for (name, action) in &config.actions {
            if let ActionDefinition::Attach(a) = action {
                let target = render_key_template(&a.key_template);
                println!("{}\t{}\t{}", name, a.title, target);
            }
        }
        return;
    }

    if args.to.is_empty() {
        eprintln!("Error: --to <ACTION> is required when not in --list mode (repeatable)");
        std::process::exit(1);
    }
    let source_key_str = args.key.clone().unwrap_or_else(|| {
        eprintln!("Error: --key is required when not in --list mode");
        std::process::exit(1)
    });
    let source_key = Key::name(&source_key_str);

    let graph = load_graph(&config);
    if (&graph).get_node_id(&source_key).is_none() {
        eprintln!("Error: Source document '{}' not found", source_key_str);
        std::process::exit(1);
    }

    let reference_text = (&graph)
        .get_key_title(&source_key)
        .unwrap_or_else(|| source_key_str.clone());

    let library_path = get_library_path(&config);

    for action_name in &args.to {
        let attach = match config.actions.get(action_name) {
            Some(ActionDefinition::Attach(a)) => a.clone(),
            Some(_) => {
                eprintln!("Error: Action '{}' is not an attach action", action_name);
                std::process::exit(1);
            }
            None => {
                eprintln!("Error: Action '{}' not found", action_name);
                std::process::exit(1);
            }
        };

        let target_key_str = render_key_template(&attach.key_template);
        let target_key = Key::name(&target_key_str);

        if (&graph).get_node_id(&target_key).is_some() {
            let tree = (&graph).collect(&target_key);
            if tree.get_all_inclusion_edge_keys().contains(&source_key) {
                continue;
            }
        }

        if args.dry_run {
            if !args.quiet {
                println!(
                    "Would attach '{}' to '{}' as [{}]({})",
                    source_key_str, target_key, reference_text, source_key_str
                );
            }
            continue;
        }

        let target_path = library_path.join(format!("{}.md", target_key));
        let line = format!("[{}]({})\n", reference_text, source_key);

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        if target_path.exists() {
            let mut existing = std::fs::read_to_string(&target_path)
                .expect("Failed to read existing target file");
            if !existing.ends_with('\n') {
                existing.push('\n');
            }
            existing.push('\n');
            existing.push_str(&line);
            std::fs::write(&target_path, existing).expect("Failed to write target file");
        } else {
            let title = render_attach_title(&attach.title);
            let initial = format!("# {}\n\n{}", title, line);
            std::fs::write(&target_path, initial).expect("Failed to write target file");
        }

        if !args.quiet {
            println!(
                "Attached '{}' to '{}' as [{}]",
                source_key_str, target_key, reference_text
            );
        }
    }
}

fn render_key_template(template: &str) -> String {
    use chrono::Local;
    use minijinja::{context, Environment};
    let now = Local::now();
    let formatted = now.format("%Y-%m-%d").to_string();
    Environment::new()
        .template_from_str(template)
        .expect("valid key template")
        .render(context! {
            today => &formatted,
            now => &formatted,
        })
        .expect("key template to render")
}

fn render_attach_title(template: &str) -> String {
    use chrono::Local;
    use minijinja::{context, Environment};
    let now = Local::now();
    let formatted = now.format("%Y-%m-%d").to_string();
    Environment::new()
        .template_from_str(template)
        .map(|t| {
            t.render(context! {
                today => &formatted,
                now => &formatted,
            })
            .unwrap_or_else(|_| template.to_string())
        })
        .unwrap_or_else(|_| template.to_string())
}
