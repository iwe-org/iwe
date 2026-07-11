use std::env;
use std::fs::create_dir;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::generate;
use clap_complete_nushell::Nushell;

mod help;
use itertools::Itertools;

use diwe::config::{load_config, ActionDefinition, Configuration, InlineType, LinkType};
use diwe::graph_from_path;
use diwe::schema::{
    explain_documents, explain_documents_against_file, pending_from_changes, render_reports_text,
    validate_pending_documents,
};
use diwe::search_query::build_index;
use diwe::stats::{graph_findings, mutation_findings, KeyStatisticsReport, SimilarityIndex};
use diwe::tokens::Truncation;
use iwe::export::{dot_details_exporter, dot_exporter, graph_data};
use iwe::filter_args::FilterArgs;
use iwe::find::{DocumentFinder, FindOptions};
use iwe::new::{read_stdin_if_available, CreateOptions, DocumentCreator, IfExists};
use iwe::projection_args::{parse_projection_extend, parse_projection_replace};
use iwe::render::{FindBlockRenderer, RetrieveRenderer};
use iwe::retrieve::{DocumentReader, RetrieveOptions};
use iwe::stats::{render_stats, GraphStatistics};
use liwe::graph::{Graph, GraphContext};
use liwe::locale::get_locale;
use liwe::model::node::NodePointer;
use liwe::model::tree::TreeIter;
use liwe::model::Key;
use liwe::operations::{
    attach_reference, delete as op_delete, extract as op_extract, inline as op_inline, references,
    rename as op_rename, sections, select_reference, select_section, AttachTarget, Changes,
    ExtractConfig, InlineConfig, SelectError,
};
use liwe::query::block::{
    parse_block_predicate, BlockOp, BlockPredicate, BlockRegex, MatchesSource,
};
use liwe::query::{
    FieldPath, Filter, Projection as QueryProjection, ProjectionField, ProjectionSource,
    Sort as QuerySort, SortDir,
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
    Schema(Schema),
    Stats(Stats),
    Rename(Rename),
    Delete(Delete),
    Extract(Extract),
    Inline(Inline),
    Update(Update),
    Attach(Attach),
    Completions(Completions),
}

#[derive(Debug, Args)]
#[clap(
    about = help::completions::ABOUT,
    long_about = help::completions::LONG_ABOUT,
    after_help = help::completions::AFTER_HELP
)]
struct Completions {
    #[clap(value_enum, help = "Target shell")]
    shell: CompletionShell,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum CompletionShell {
    Bash,
    Elvish,
    Fish,
    Nushell,
    Powershell,
    Zsh,
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
        value_name = "N",
        num_args = 0..=1,
        default_missing_value = "1",
        conflicts_with = "depth",
        help = "Expand into child documents to depth N (bare = 1, 0 = unbounded, omitted = not followed)."
    )]
    expand_includes: Option<u64>,

    #[clap(
        long,
        value_name = "N",
        num_args = 0..=1,
        default_missing_value = "1",
        conflicts_with = "context",
        help = "Expand into parent documents to depth N (bare = 1, 0 = unbounded, omitted = not followed)."
    )]
    expand_included_by: Option<u64>,

    #[clap(
        long,
        value_name = "N",
        num_args = 0..=1,
        default_missing_value = "1",
        conflicts_with = "links",
        help = "Expand along outbound reference links to depth N (bare = 1, 0 = unbounded, omitted = not followed)."
    )]
    expand_references: Option<u64>,

    #[clap(
        long,
        value_name = "N",
        num_args = 0..=1,
        default_missing_value = "1",
        help = "Expand along inbound reference links to depth N (bare = 1, 0 = unbounded, omitted = not followed)."
    )]
    expand_referenced_by: Option<u64>,

    #[clap(long, help = "Seed search: BM25 full-text query on title and body.")]
    lexical: Option<String>,

    #[clap(long, help = "Seed search: fuzzy query on title and key.")]
    fuzzy: Option<String>,

    #[clap(long, short = 'd', hide = true)]
    depth: Option<u8>,

    #[clap(long, short = 'c', hide = true)]
    context: Option<u8>,

    #[clap(long, short = 'l', hide = true)]
    links: bool,

    #[clap(
        long,
        short = 'e',
        help = "Exclude document key(s) from results (can be specified multiple times)"
    )]
    exclude: Vec<String>,

    #[clap(
        long,
        short = 'b',
        num_args = 0..=1,
        default_value_t = true,
        default_missing_value = "true",
        help = "Include incoming references (--backlinks false to disable)"
    )]
    backlinks: bool,

    #[clap(long, short = 'f', value_enum, default_value = "markdown")]
    format: RetrieveFormat,

    #[clap(long, help = "Populate the `includes` array with child document edges")]
    children: bool,

    #[clap(
        long,
        help = "Cap the number of seed documents kept before expansion — top-N by relevance when searching, the first N of the selection otherwise (0 = unlimited)"
    )]
    limit: Option<usize>,

    #[clap(
        long,
        help = "Cap the number of documents returned after expansion, trimming periphery first (0 = unlimited)"
    )]
    max_documents: Option<usize>,

    #[clap(
        long,
        help = "Cap total content tokens across all documents (0 = unlimited)"
    )]
    max_tokens: Option<usize>,

    #[clap(long, help = "Cap content tokens per document (0 = unlimited)")]
    max_document_tokens: Option<usize>,

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
    #[clap(
        help = "DEPRECATED: bare query defaults to fuzzy; use --fuzzy or --lexical",
        conflicts_with = "fuzzy"
    )]
    pattern: Option<String>,

    #[clap(long, help = "Fuzzy match on document title and key")]
    fuzzy: Option<String>,

    #[clap(long, help = "Lexical (BM25) full-text match on title and body")]
    lexical: Option<String>,

    #[clap(long, short = 'l', help = "Maximum results (0 = unlimited)")]
    limit: Option<usize>,

    #[clap(
        long,
        help = "Cap total content tokens across all results (0 = unlimited)"
    )]
    max_tokens: Option<usize>,

    #[clap(
        long,
        help = "Cap projected `$content` tokens per result (0 = unlimited)"
    )]
    max_document_tokens: Option<usize>,

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
        value_name = "PRED",
        help = "Locate blocks: adds a `blocks` field listing each block matching the predicate. PRED is an inline block predicate, e.g. '{ $within: Goals, $text: Q3 }'."
    )]
    blocks: Option<String>,

    #[clap(
        long,
        value_name = "PATTERN",
        help = "Grep over blocks: restricts results to documents whose content matches PATTERN and adds a `matches` field with the matching lines. PATTERN is a Rust regex."
    )]
    matches: Option<String>,

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
    #[clap(
        long,
        short = 'l',
        help = "Cap the number of matches counted (0 = unlimited)"
    )]
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
        short = 'k',
        help = "Explicit document key, bypassing the template's key derivation. Subdirectory keys allowed (e.g. people/ada); omit the file extension. Defaults --if-exists to fail."
    )]
    key: Option<String>,

    #[clap(
        long,
        short = 'i',
        value_enum,
        help = "Behavior when file already exists: suffix (append -1, -2, etc.), override (overwrite), skip (do nothing), fail (error). Default: suffix, or fail when --key is given."
    )]
    if_exists: Option<IfExists>,

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
    about = help::schema::ABOUT,
    long_about = help::schema::LONG_ABOUT,
    after_help = help::schema::AFTER_HELP
)]
struct Schema {
    #[command(subcommand)]
    command: Option<SchemaCommand>,

    #[clap(flatten)]
    fields: SchemaFields,
}

#[derive(Debug, Subcommand)]
enum SchemaCommand {
    Validate(SchemaValidate),
}

#[derive(Debug, Args)]
struct SchemaFields {
    #[clap(
        long,
        short = 'f',
        value_enum,
        default_value = "markdown",
        help = "Output format for schema"
    )]
    format: SchemaFormat,

    #[clap(long, help = "Restrict output to a specific field (and its children)")]
    field: Option<String>,

    #[clap(flatten)]
    selector: FilterArgs,
}

#[derive(Debug, Args)]
#[clap(about = "Validate documents against their configured schemas")]
struct SchemaValidate {
    #[clap(
        long,
        short = 'f',
        value_enum,
        default_value = "text",
        help = "Output format for validation reports"
    )]
    format: ValidateFormat,

    #[clap(
        long = "schema-file",
        help = "Validate the selected documents against this schema file directly, bypassing the [schemas] config bindings"
    )]
    schema_file: Option<PathBuf>,

    #[clap(
        long,
        help = "Print the binding trace (which section/block bound to which schema entry) instead of validating"
    )]
    explain: bool,

    #[clap(flatten)]
    selector: FilterArgs,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum SchemaFormat {
    Markdown,
    Json,
    Yaml,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum ValidateFormat {
    Text,
    Json,
}

#[derive(Debug, Args)]
#[clap(
    about = help::stats::ABOUT,
    long_about = help::stats::LONG_ABOUT,
    after_help = help::stats::AFTER_HELP
)]
struct Stats {
    #[command(subcommand)]
    command: Option<StatsCommand>,

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

#[derive(Debug, Subcommand)]
enum StatsCommand {
    #[clap(
        about = "List pages with near-identical, mutually-similar counterparts across the store"
    )]
    Similarity,
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

    #[clap(
        long,
        help = "Filter expression (inline YAML). Required if positional KEY omitted."
    )]
    filter: Option<String>,

    #[clap(
        long,
        value_name = "ARG",
        help = "Document-level expect guard: assert the number of matched documents. ARG is N or '{ min: M, max: N }'."
    )]
    expect: Option<String>,

    #[clap(
        long,
        help = "Require the document-level --expect guard. Aborts before deleting if it is missing. Exempt under --dry-run."
    )]
    strict: bool,

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

    #[clap(
        long,
        help = "Section title to extract (case-insensitive)",
        conflicts_with = "block"
    )]
    section: Option<String>,

    #[clap(
        long,
        help = "Block number to extract (1-indexed)",
        conflicts_with = "section"
    )]
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
    #[clap(
        long,
        short = 'k',
        help = "Match by document key. Repeatable: 1 key uses $eq, 2+ uses $in. Body-overwrite mode requires exactly one."
    )]
    key: Vec<String>,

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

    #[clap(
        long = "replace",
        value_name = "ARG",
        help = "$replace: replace each selected block. ARG is '{ <selector>, content: <markdown> }'."
    )]
    replace: Option<String>,

    #[clap(
        long = "replace-text",
        value_name = "ARG",
        help = "$replaceText: rewrite own text of each selected block. ARG is '{ <selector>, from: X, to: Y }'; omit 'from' and 'to' replaces the entire own text."
    )]
    replace_text: Option<String>,

    #[clap(
        long = "insert-before",
        value_name = "ARG",
        help = "$insertBefore: insert sibling content before each selected block. ARG is '{ <selector>, content: <markdown> }'."
    )]
    insert_before: Option<String>,

    #[clap(
        long = "insert-after",
        value_name = "ARG",
        help = "$insertAfter: insert sibling content after each selected block. ARG is '{ <selector>, content: <markdown> }'."
    )]
    insert_after: Option<String>,

    #[clap(
        long = "append",
        value_name = "ARG",
        help = "$append: append child content to each selected container. ARG is '{ <selector>, content: <markdown> }'."
    )]
    append: Option<String>,

    #[clap(
        long = "delete",
        value_name = "ARG",
        help = "$delete: remove each selected block. ARG is the '{ <selector> }' mapping ('{}' selects every block)."
    )]
    delete: Option<String>,

    #[clap(
        long,
        value_name = "ARG",
        help = "Document-level expect guard: assert the number of matched documents. ARG is N or '{ min: M, max: N }'."
    )]
    expect: Option<String>,

    #[clap(
        long,
        help = "Require an expect guard on every mutating application (document-level --expect and each block operator's expect). Aborts before writing if any is missing. Exempt under --dry-run."
    )]
    strict: bool,

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
        Command::Schema(schema) => schema_command(schema),
        Command::Stats(stats) => stats_command(stats),
        Command::Rename(rename) => rename_command(rename),
        Command::Delete(delete) => delete_command(delete),
        Command::Extract(extract) => extract_command(extract),
        Command::Inline(inline) => inline_command(inline),
        Command::Update(update) => update_command(update),
        Command::Attach(attach) => attach_command(attach),
        Command::Completions(completions) => completions_command(completions),
    }
}

fn completions_command(args: Completions) {
    let mut cmd = App::command();
    let bin_name = cmd.get_name().to_string();
    let mut out = std::io::stdout();
    match args.shell {
        CompletionShell::Bash => generate(clap_complete::Shell::Bash, &mut cmd, bin_name, &mut out),
        CompletionShell::Elvish => {
            generate(clap_complete::Shell::Elvish, &mut cmd, bin_name, &mut out)
        }
        CompletionShell::Fish => generate(clap_complete::Shell::Fish, &mut cmd, bin_name, &mut out),
        CompletionShell::Powershell => generate(
            clap_complete::Shell::PowerShell,
            &mut cmd,
            bin_name,
            &mut out,
        ),
        CompletionShell::Zsh => generate(clap_complete::Shell::Zsh, &mut cmd, bin_name, &mut out),
        CompletionShell::Nushell => generate(Nushell, &mut cmd, bin_name, &mut out),
    }
}

fn print_truncation_warning(noun: &str, count_knob: &str, truncation: &Truncation) {
    if !truncation.is_truncated() {
        return;
    }
    let mut msg = format!(
        "warning: output truncated — returned {}/{} {}",
        truncation.emitted, truncation.matched, noun
    );
    if !truncation.clipped.is_empty() {
        msg.push_str(&format!(
            ", {} clipped to --max-document-tokens",
            truncation.clipped.len()
        ));
    }
    match truncation.budget {
        Some(budget) => msg.push_str(&format!(
            "; ~{} tokens (budget {})",
            truncation.tokens, budget
        )),
        None => msg.push_str(&format!("; ~{} tokens", truncation.tokens)),
    }
    let mut knobs: Vec<&str> = Vec::new();
    if truncation.emitted < truncation.matched {
        knobs.push(count_knob);
    }
    if truncation.budget.is_some() {
        knobs.push("--max-tokens");
    }
    if !truncation.clipped.is_empty() {
        knobs.push("--max-document-tokens");
    }
    msg.push_str(". Narrow with --filter");
    if !knobs.is_empty() {
        msg.push_str(&format!(" or raise {}", knobs.join("/")));
    }
    msg.push('.');
    eprintln!("{}", msg);
}

#[derive(Debug, Clone, Default)]
struct Expansion {
    includes: u32,
    included_by: u32,
    references: u32,
    referenced_by: u32,
}

fn expand_direction(new: Option<u64>, legacy: Option<u32>) -> u32 {
    match new {
        Some(n) => diwe::retrieve::expand_depth(n),
        None => legacy.unwrap_or(0),
    }
}

fn resolve_expansion(args: &Retrieve) -> Expansion {
    Expansion {
        includes: expand_direction(args.expand_includes, args.depth.map(u32::from)),
        included_by: expand_direction(args.expand_included_by, args.context.map(u32::from)),
        references: expand_direction(args.expand_references, args.links.then_some(1)),
        referenced_by: args
            .expand_referenced_by
            .map(diwe::retrieve::expand_depth)
            .unwrap_or(0),
    }
}

#[tracing::instrument(level = "debug")]
fn retrieve_command(args: Retrieve) {
    let config = get_configuration();
    let searching = args.lexical.is_some() || args.fuzzy.is_some();

    let (graph, index) = if searching {
        let (g, i) = load_search_graph(&config);
        (g, Some(i))
    } else {
        (load_graph(&config), None)
    };

    let expansion = resolve_expansion(&args);
    let exclude: std::collections::HashSet<Key> =
        args.exclude.iter().map(|s| Key::name(s)).collect();
    let mut options = RetrieveOptions {
        includes: expansion.includes,
        included_by: expansion.included_by,
        references: expansion.references,
        referenced_by: expansion.referenced_by,
        backlinks: args.backlinks,
        exclude,
        children: args.children,
        filter: None,
        limit: args.limit,
        max_documents: args.max_documents,
        max_tokens: args.max_tokens,
        max_document_tokens: args.max_document_tokens,
    };

    let reader = DocumentReader::new(&graph);

    let output = if searching {
        let candidate_filter = resolve_filter(&args.selector, &graph);
        let candidates: Vec<Key> = match &candidate_filter {
            None => graph.keys(),
            Some(f) => liwe::query::evaluate(f, &graph),
        };
        let index = index.as_ref().expect("search graph carries an index");
        if let Some(q) = args.lexical.as_deref() {
            if !index.has_query_terms(q) {
                eprintln!(
                    "warning: --lexical query '{}' has no searchable terms after stop-word removal and stemming; it matches nothing. Try --fuzzy for common or partial words.",
                    q
                );
            }
        }
        let spec = liwe::query::SearchSpec::new(args.lexical.clone(), args.fuzzy.clone());
        let seeds = diwe::search_query::ranked(&graph, index, &candidates, &spec);
        reader.retrieve_many(&seeds, &options)
    } else {
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
                eprintln!(
                    "Error: No document key provided. Use -k <key>, --filter, --lexical, or pipe keys via stdin."
                );
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

        options.filter = resolve_filter(&args.selector, &graph);
        reader.retrieve_many(&keys, &options)
    };

    match args.format {
        RetrieveFormat::Json => {
            let json = serde_json::to_string_pretty(&output.documents)
                .expect("Failed to serialize to JSON");
            println!("{}", json);
        }
        RetrieveFormat::Yaml => {
            let yaml =
                serde_yaml::to_string(&output.documents).expect("Failed to serialize to YAML");
            print!("{}", yaml);
        }
        RetrieveFormat::Keys => {
            for doc in &output.documents {
                println!("{}", doc.key);
            }
        }
        RetrieveFormat::Markdown => {
            let md_options = graph.format_options().markdown_options();
            let renderer =
                RetrieveRenderer::new(&output, &md_options, &graph, args.max_document_tokens);
            print!("{}", renderer.render());
        }
    }

    print_truncation_warning("documents", "--max-documents", &output.truncation);
}

#[tracing::instrument(level = "debug")]
fn lower_block_flags(args: &Find) -> Result<(Vec<ProjectionField>, Option<Filter>), String> {
    let mut fields: Vec<ProjectionField> = Vec::new();
    let mut filter: Option<Filter> = None;

    if let Some(arg) = &args.blocks {
        let value: serde_yaml::Value =
            serde_yaml::from_str(arg).map_err(|e| format!("invalid --blocks predicate: {}", e))?;
        let pred = parse_block_predicate(&value, "$blocks")
            .map_err(|e| format!("invalid --blocks predicate: {}", e))?;
        fields.push(ProjectionField {
            output: "blocks".to_string(),
            source: ProjectionSource::Blocks(pred),
        });
    }

    if let Some(pattern) = &args.matches {
        let regex = BlockRegex::compile(pattern)
            .map_err(|e| format!("invalid --matches pattern: {}", e))?;
        fields.push(ProjectionField {
            output: "matches".to_string(),
            source: ProjectionSource::Matches(MatchesSource {
                pattern: regex.clone(),
                scope: BlockPredicate::empty(),
            }),
        });
        filter = Some(Filter::Content(BlockPredicate(vec![BlockOp::Matches(
            regex,
        )])));
    }

    Ok((fields, filter))
}

fn find_command(args: Find) {
    let config = get_configuration();
    let (graph, index) = load_search_graph(&config);

    let sort = args
        .sort
        .as_deref()
        .map(parse_sort_arg)
        .transpose()
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            std::process::exit(2);
        });
    let (extra_fields, matches_filter) = lower_block_flags(&args).unwrap_or_else(|e| {
        eprintln!("error: {}", e);
        std::process::exit(2);
    });
    let base_project = args.project.clone().or_else(|| args.add_fields.clone());
    let project = match base_project {
        Some(mut p) => {
            p.fields.extend(extra_fields);
            Some(p)
        }
        None if !extra_fields.is_empty() => Some(QueryProjection::extend(extra_fields)),
        None => None,
    };

    let fuzzy = match args.pattern {
        Some(p) => {
            eprintln!(
                "warning: the bare `find <query>` form is deprecated and defaults to fuzzy \
                 matching; it will be removed. Use `find --fuzzy <query>` or `find --lexical <query>`."
            );
            Some(p)
        }
        None => args.fuzzy,
    };

    let filter = match (resolve_filter(&args.selector, &graph), matches_filter) {
        (Some(f), Some(mf)) => Some(Filter::And(vec![f, mf])),
        (Some(f), None) => Some(f),
        (None, Some(mf)) => Some(mf),
        (None, None) => None,
    };

    let finder = DocumentFinder::with_index(&graph, &index);
    let options = FindOptions {
        fuzzy,
        lexical: args.lexical,
        refs_to: None,
        refs_from: None,
        filter,
        limit: args.limit,
        sort,
        project: project.clone(),
        max_tokens: args.max_tokens,
        max_document_tokens: args.max_document_tokens,
    };

    let output = finder.find(&options);

    if let Some(q) = options.lexical.as_deref() {
        if !index.has_query_terms(q) {
            eprintln!(
                "warning: --lexical query '{}' has no searchable terms after stop-word removal and stemming; it matches nothing. Try --fuzzy for common or partial words.",
                q
            );
        }
    }

    match args.format {
        FindFormat::Json => {
            let json =
                serde_json::to_string_pretty(&output.results).expect("Failed to serialize to JSON");
            println!("{}", json);
        }
        FindFormat::Yaml => {
            let yaml = serde_yaml::to_string(&output.results).expect("Failed to serialize to YAML");
            print!("{}", yaml);
        }
        FindFormat::Keys => {
            for key in &output.keys {
                println!("{}", key);
            }
        }
        FindFormat::Markdown => {
            let content_output_names: Vec<String> = match &project {
                Some(p) => p
                    .fields
                    .iter()
                    .filter(|f| f.source.is_content_shaped())
                    .map(|f| f.output.clone())
                    .collect(),
                None => Vec::new(),
            };
            let narrowed_content = project
                .as_ref()
                .map(|p| {
                    p.fields
                        .iter()
                        .any(|f| matches!(&f.source, ProjectionSource::ContentBlocks(_)))
                })
                .unwrap_or(false);
            let grep_output_names: Vec<String> = match &project {
                Some(p) => p
                    .fields
                    .iter()
                    .filter(|f| f.source.is_block_lines())
                    .map(|f| f.output.clone())
                    .collect(),
                None => Vec::new(),
            };
            let md_options = graph.format_options().markdown_options();
            let renderer = FindBlockRenderer::new(
                &md_options,
                &graph,
                args.max_document_tokens,
                &output.truncation.clipped,
            );
            print!(
                "{}",
                renderer.render(
                    &output.keys,
                    &output.results,
                    &content_output_names,
                    narrowed_content,
                    &grep_output_names
                )
            );
        }
    }

    print_truncation_warning("documents", "--limit", &output.truncation);
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

    match execute(&Operation::Count(op), &graph).expect("count query does not fail") {
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

    let if_exists = args.if_exists.unwrap_or(if args.key.is_some() {
        IfExists::Fail
    } else {
        IfExists::Suffix
    });

    let creator = DocumentCreator::new(&config, library_path);
    let options = CreateOptions {
        title: args.title,
        template_name: args.template,
        content,
        key: args.key,
        if_exists,
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
            eprintln!("Error: {}", e);
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
                if let Some(node) =
                    build_tree_node(&graph, root_key, args.depth, project.as_ref(), &mut visited)
                {
                    trees.push(node);
                }
            }
            match args.format {
                TreeFormat::Yaml => {
                    let yaml = serde_yaml::to_string(&trees).expect("Failed to serialize to YAML");
                    print!("{}", yaml);
                }
                _ => {
                    let json =
                        serde_json::to_string_pretty(&trees).expect("Failed to serialize to JSON");
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
            .filter_map(|ref_key| build_tree_node(graph, &ref_key, max_depth - 1, project, visited))
            .collect()
    } else {
        vec![]
    };

    let mut node = serde_yaml::Mapping::new();
    node.insert(
        serde_yaml::Value::from("key"),
        serde_yaml::Value::from(key_str),
    );
    node.insert(
        serde_yaml::Value::from("title"),
        serde_yaml::Value::from(title),
    );

    if let Some(p) = project {
        let ctx = ProjectionContext::new(graph, key);
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

    let children_value =
        serde_yaml::to_value(&children).unwrap_or_else(|_| serde_yaml::Value::Sequence(Vec::new()));
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
    let key = Key::name(&args.key);
    if graph.get_node_id(&key).is_none() {
        eprintln!("Error: Document '{}' not found", args.key);
        std::process::exit(1);
    }
    let mut patch = Graph::new();
    let squashed = graph.squash(&key, args.depth);

    patch.build_key_from_iter(&args.key.clone().into(), TreeIter::new(&squashed));

    print!("{}", patch.export_key(&args.key.into()).unwrap_or_default())
}

fn write_graph(graph: Graph, configuration: &Configuration) {
    diwe::fs::write_store_at_path(
        &graph.export(),
        &get_library_path(configuration),
        configuration.format,
    )
    .expect("Failed to write graph")
}

fn apply_changes(changes: &Changes, configuration: &Configuration) {
    diwe::fs::apply_changes(
        changes,
        &get_library_path(configuration),
        configuration.format,
    )
    .expect("Failed to write document file");
}

fn load_graph(configuration: &Configuration) -> Graph {
    graph_from_path(
        &get_library_path(configuration),
        false,
        configuration.format_options(),
        configuration.library.frontmatter_document_title.clone(),
    )
}

fn load_search_graph(configuration: &Configuration) -> (Graph, diwe::search::Bm25Index) {
    let graph = load_graph(configuration);
    let index = build_index(&graph, configuration.search_language());
    (graph, index)
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
        _ => {
            return Err(format!(
                "invalid sort direction '{}': expected 1 or -1",
                dir
            ))
        }
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
    let config = load_config().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });
    if log::log_enabled!(log::Level::Debug) {
        let formatted_config =
            toml::to_string_pretty(&config).unwrap_or_else(|_| format!("{:#?}", config));
        debug!("using config:\n{}", formatted_config);
    }
    config
}

fn schema_command(args: Schema) {
    match args.command {
        Some(SchemaCommand::Validate(validate)) => schema_validate_command(validate),
        None => schema_infer_command(args.fields),
    }
}

fn schema_infer_command(args: SchemaFields) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let keys: Vec<Key> = match resolve_filter(&args.selector, &graph) {
        Some(filter) => liwe::query::evaluate(&filter, &graph),
        None => {
            let mut k = graph.keys();
            k.sort();
            k
        }
    };

    let mut fields = liwe::schema::infer_schema(&graph, &keys);

    if let Some(ref field_name) = args.field {
        fields.retain(|f| f.name == *field_name || f.name.starts_with(&format!("{}.", field_name)));
    }

    match args.format {
        SchemaFormat::Json => {
            let json = serde_json::to_string_pretty(&fields).expect("Failed to serialize schema");
            println!("{}", json);
        }
        SchemaFormat::Yaml => {
            let yaml = serde_yaml::to_string(&fields).expect("Failed to serialize schema");
            print!("{}", yaml);
        }
        SchemaFormat::Markdown => {
            let output = iwe::schema::render_schema(&fields);
            print!("{}", output);
        }
    }
}

fn schema_validate_command(args: SchemaValidate) {
    let config = get_configuration();
    let graph = load_graph(&config);

    let keys: Vec<Key> = match resolve_filter(&args.selector, &graph) {
        Some(filter) => liwe::query::evaluate(&filter, &graph),
        None => {
            let mut k = graph.keys();
            k.sort();
            k
        }
    };

    if args.explain {
        let result = match &args.schema_file {
            Some(path) => explain_documents_against_file(&graph, &keys, path),
            None => explain_documents(&config, &graph, &keys),
        };
        match result {
            Ok(trace) => print!("{}", trace),
            Err(errors) => {
                for error in errors {
                    eprintln!("error: {}", error);
                }
                std::process::exit(2);
            }
        }
        return;
    }

    let result = match &args.schema_file {
        Some(path) => diwe::schema::validate_documents_against_file(&graph, &keys, path),
        None => diwe::schema::validate_documents(&config, &graph, &keys),
    };

    let reports = match result {
        Ok(reports) => reports,
        Err(errors) => {
            for error in errors {
                eprintln!("error: {}", error);
            }
            std::process::exit(2);
        }
    };

    if reports.is_empty() {
        return;
    }

    match args.format {
        ValidateFormat::Text => print!("{}", render_reports_text(&reports)),
        ValidateFormat::Json => {
            let json = serde_json::to_string_pretty(&reports).expect("Failed to serialize reports");
            println!("{}", json);
        }
    }

    std::process::exit(1);
}

fn gate_pending(config: &Configuration, docs: &[(Key, String)]) {
    match validate_pending_documents(config, docs) {
        Ok(reports) if reports.is_empty() => {}
        Ok(reports) => {
            eprintln!("error: --strict blocked the write: schema validation failed");
            eprint!("{}", render_reports_text(&reports));
            std::process::exit(2);
        }
        Err(errors) => {
            for error in errors {
                eprintln!("error: {}", error);
            }
            std::process::exit(2);
        }
    }
}

fn apply_changes_to_graph(graph: &mut Graph, changes: &Changes) {
    for key in &changes.removes {
        graph.remove_document(key.clone());
    }
    for (key, markdown) in &changes.creates {
        graph.insert_document(key.clone(), markdown.clone());
    }
    for (key, markdown) in &changes.updates {
        graph.update_document(key.clone(), markdown.clone());
    }
}

fn warn_stats(config: &Configuration, graph: &Graph, targets: &[Key]) {
    let findings = if targets.is_empty() {
        graph_findings(graph)
    } else {
        let index = build_index(graph, config.search_language());
        mutation_findings(graph, &index, targets)
    };
    for finding in findings {
        eprintln!("stats: {}", finding.render());
    }
}

#[tracing::instrument(level = "debug")]
fn stats_command(args: Stats) {
    let config = get_configuration();
    let graph = load_graph(&config);

    if let Some(StatsCommand::Similarity) = args.command {
        let similarity = SimilarityIndex::build(&graph, config.search_language());
        for (a, b) in similarity.pairs() {
            println!("{}\t{}", a, b);
        }
        return;
    }

    if let Some(key_str) = args.key {
        let normalized_key = Key::name(&key_str).to_string();
        let key_stats = diwe::stats::KeyStatistics::from_graph(&graph);
        let entry = key_stats.into_iter().find(|s| s.key == normalized_key);
        match entry {
            Some(s) => {
                let similar = if matches!(args.format, StatsFormat::Csv) {
                    Vec::new()
                } else {
                    SimilarityIndex::build(&graph, config.search_language())
                        .similar(&Key::name(&s.key))
                };
                match args.format {
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
                        for page in &similar {
                            println!("- **Similar page:** {} ({:.2})", page.key, page.score);
                        }
                    }
                    StatsFormat::Csv => {
                        let stdout = std::io::stdout();
                        let mut csv_writer = csv::Writer::from_writer(stdout.lock());
                        csv_writer.serialize(&s).expect("Failed to serialize stats");
                        csv_writer.flush().expect("Failed to flush CSV");
                    }
                    StatsFormat::Json => {
                        let report = KeyStatisticsReport {
                            stats: s,
                            similar_pages: similar,
                        };
                        let json = serde_json::to_string_pretty(&report)
                            .expect("Failed to serialize stats");
                        println!("{}", json);
                    }
                    StatsFormat::Yaml => {
                        let report = KeyStatisticsReport {
                            stats: s,
                            similar_pages: similar,
                        };
                        let yaml =
                            serde_yaml::to_string(&report).expect("Failed to serialize stats");
                        print!("{}", yaml);
                    }
                }
            }
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
            let json = serde_json::to_string_pretty(&stats).expect("Failed to serialize stats");
            println!("{}", json);
        }
        StatsFormat::Yaml => {
            let stats = GraphStatistics::from_graph(&graph);
            let yaml = serde_yaml::to_string(&stats).expect("Failed to serialize stats");
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
    use liwe::query::block_update::check_document_expect;

    let config = get_configuration();
    let mut graph = load_graph(&config);

    let doc_expect = args.expect.as_deref().map(parse_cli_expect);
    if args.strict && !args.dry_run && doc_expect.is_none() {
        eprintln!(
            "error: --strict requires the document-level --expect guard; missing: document-level --expect"
        );
        eprintln!(
            "hint: state the expected count — 1 for a precision edit, '{{ min: 1 }}' for a bulk delete that must match, '{{ min: 0 }}' when zero is acceptable"
        );
        std::process::exit(2);
    }

    let targets = resolve_delete_targets(&args, &graph);

    if !args.dry_run {
        let doc_refs = build_doc_refs(&graph, &targets);
        check_document_expect("delete", doc_expect, &doc_refs).unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            std::process::exit(2);
        });
    }

    if targets.is_empty() {
        if !args.quiet {
            eprintln!("No documents matched");
        }
        return;
    }

    let mut combined = Changes::default();
    for target in &targets {
        match op_delete(&graph, target) {
            Ok(changes) => combined.merge(changes),
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
        if args.strict {
            gate_pending(&config, &pending_from_changes(&combined));
        }
        apply_changes(&combined, &config);
        if args.strict {
            apply_changes_to_graph(&mut graph, &combined);
            warn_stats(&config, &graph, &[]);
        }
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

fn get_extract_config(
    config: &Configuration,
    action_name: Option<&str>,
) -> (String, Option<LinkType>) {
    if let Some(name) = action_name {
        if let Some(ActionDefinition::Extract(extract)) = config.actions.get(name) {
            return (extract.key_template.clone(), extract.link_type.clone());
        }
        eprintln!(
            "Error: Action '{}' not found or not an extract action",
            name
        );
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
            eprintln!("Error: Action '{}' not found or not an inline action", name);
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

    if args.list {
        for section in sections(&tree) {
            println!("{}: {}", section.number, section.title);
        }
        return;
    }

    let selected = match select_section(&tree, args.section.as_deref(), args.block) {
        Ok(section) => section,
        Err(SelectError::NotFound(query)) => {
            eprintln!("Error: No section matches '{}'", query);
            std::process::exit(1);
        }
        Err(SelectError::Ambiguous(query, matches)) => {
            eprintln!("Error: Multiple sections match '{}':", query);
            for section in &matches {
                eprintln!("  {}: {}", section.number, section.title);
            }
            eprintln!("Use --block <n> to select a specific section.");
            std::process::exit(1);
        }
        Err(SelectError::OutOfRange(block, len)) => {
            eprintln!("Error: Block number {} out of range (1-{})", block, len);
            std::process::exit(1);
        }
        Err(SelectError::NoSelector) => {
            eprintln!("Error: Must specify --section, --block, or --list");
            std::process::exit(1);
        }
    };

    let section_title = selected.title;
    let section_id = selected.id;

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

    let result = match op_extract(
        &graph,
        &source_key,
        section_id,
        &extract_config,
        std::time::SystemTime::now(),
    ) {
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

    if args.list {
        for reference in references(&tree) {
            println!(
                "{}: [{}]({})",
                reference.number, reference.title, reference.key
            );
        }
        return;
    }

    let selected = match select_reference(&tree, args.reference.as_deref(), args.block) {
        Ok(reference) => reference,
        Err(SelectError::NotFound(query)) => {
            eprintln!("Error: No reference matches '{}'", query);
            std::process::exit(1);
        }
        Err(SelectError::Ambiguous(query, matches)) => {
            eprintln!("Error: Multiple references match '{}':", query);
            for reference in &matches {
                eprintln!(
                    "  {}: [{}]({})",
                    reference.number, reference.title, reference.key
                );
            }
            eprintln!("Use --block <n> to select a specific reference.");
            std::process::exit(1);
        }
        Err(SelectError::OutOfRange(block, len)) => {
            eprintln!("Error: Block number {} out of range (1-{})", block, len);
            std::process::exit(1);
        }
        Err(SelectError::NoSelector) => {
            eprintln!("Error: Must specify --reference, --block, or --list");
            std::process::exit(1);
        }
    };

    let ref_text = selected.title;
    let inline_key = selected.key;
    let ref_id = selected.id;

    let (inline_type, should_keep_target) = get_inline_config(
        &config,
        args.action.as_deref(),
        args.as_quote,
        args.keep_target,
    );

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

impl Update {
    fn block_edits(&self) -> Vec<(&'static str, &str)> {
        [
            ("$replace", &self.replace),
            ("$replaceText", &self.replace_text),
            ("$insertBefore", &self.insert_before),
            ("$insertAfter", &self.insert_after),
            ("$append", &self.append),
            ("$delete", &self.delete),
        ]
        .into_iter()
        .filter_map(|(op, value)| value.as_deref().map(|arg| (op, arg)))
        .collect()
    }
}

#[tracing::instrument(level = "debug")]
fn update_command(args: Update) {
    let body_mode = args.content.is_some();
    let mutation_mode =
        !args.set.is_empty() || !args.unset.is_empty() || !args.block_edits().is_empty();

    if body_mode && mutation_mode {
        eprintln!("error: --content cannot be combined with mutation flags");
        std::process::exit(1);
    }
    if !body_mode && !mutation_mode {
        eprintln!(
            "error: provide either --content (body overwrite) or a mutation flag \
             (--set/--unset/--replace/--replace-text/--insert-before/--insert-after/--append/--delete)"
        );
        std::process::exit(1);
    }

    if body_mode {
        update_body(args);
    } else {
        update_mutation(args);
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
    let mut graph = load_graph(&config);

    let key_str = match args.key.as_slice() {
        [single] => single.clone(),
        [] => {
            eprintln!("error: -k/--key is required for body-overwrite mode");
            std::process::exit(1);
        }
        _ => {
            eprintln!("error: body-overwrite mode takes exactly one -k/--key");
            std::process::exit(1);
        }
    };
    let key = Key::name(&key_str);
    if (&graph).get_node_id(&key).is_none() {
        eprintln!("error: document '{}' not found", key_str);
        std::process::exit(1);
    }

    let raw = args.content.expect("body mode implies content present");
    let content = if raw == "-" {
        let stdin_content = read_stdin_if_available();
        if stdin_content.is_empty() {
            eprintln!("error: '--content -' requires content piped via stdin");
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
    let file_path = library_path.join(format!("{}.{}", key, config.format.extension()));
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let existing = std::fs::read_to_string(&file_path).unwrap_or_default();
    let (frontmatter, _) = split_raw_frontmatter(&existing);
    let output = match frontmatter {
        Some(fm) => format!("{}{}", fm, content),
        None => content,
    };
    if output == existing {
        if !args.quiet {
            println!("'{}' unchanged", key_str);
        }
        return;
    }

    if args.strict {
        gate_pending(&config, &[(key.clone(), output.clone())]);
    }

    std::fs::write(&file_path, &output).expect("Failed to write document file");

    if args.strict {
        graph.update_document(key.clone(), output.clone());
        warn_stats(&config, &graph, std::slice::from_ref(&key));
    }

    if !args.quiet {
        println!("Updated '{}'", key_str);
    }
}

fn update_mutation(args: Update) {
    use liwe::query::block_update::check_document_expect;
    use liwe::query::wire::RawUpdate;
    use liwe::query::{build_update_doc, execute as run_op, FindOp, Operation, Outcome, UpdateOp};
    use serde_yaml::{Mapping, Value};

    let config = get_configuration();
    let mut graph = load_graph(&config);

    let mut conjuncts: Vec<Filter> = Vec::new();
    let parsed_filter = args.filter.as_ref().map(|expr| {
        liwe::query::parse_filter_expression(expr).unwrap_or_else(|e| {
            eprintln!("error: invalid --filter expression: {}", e);
            std::process::exit(2);
        })
    });
    if !args.key.is_empty() {
        if let Some(parsed) = parsed_filter.as_ref() {
            if filter_has_top_level_key_predicate(parsed) {
                eprintln!(
                    "error: -k / --key conflicts with a $key predicate at the top level of --filter; \
                     use --filter '$or: [{{$key: a}}, {{$key: b}}]' for OR-of-keys, or pick one source"
                );
                std::process::exit(2);
            }
        }
    }
    if let Some(parsed) = parsed_filter {
        conjuncts.push(parsed);
    }
    match args.key.len() {
        0 => {}
        1 => conjuncts.push(Filter::Key(liwe::query::KeyOp::Eq(Key::name(&args.key[0])))),
        _ => conjuncts.push(Filter::Key(liwe::query::KeyOp::In(
            args.key.iter().map(|k| Key::name(k)).collect(),
        ))),
    }
    if conjuncts.is_empty() {
        eprintln!("error: --filter or -k/--key required for mutation mode");
        std::process::exit(1);
    }
    let filter = if conjuncts.len() == 1 {
        conjuncts.into_iter().next().unwrap()
    } else {
        Filter::And(conjuncts)
    };

    let mut update_map = Mapping::new();
    for (op, arg) in args.block_edits() {
        let value: Value = serde_yaml::from_str(arg).unwrap_or_else(|e| {
            eprintln!("error: invalid {} argument: {}", op, e);
            std::process::exit(2);
        });
        update_map.insert(Value::String(op.to_string()), value);
    }

    let mut set_map = Mapping::new();
    for assign in &args.set {
        let (field, value) = parse_set_assignment(assign).unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            std::process::exit(2);
        });
        set_map.insert(Value::String(field), value);
    }
    merge_update_operator(&mut update_map, "$set", set_map);

    let mut unset_map = Mapping::new();
    for field in &args.unset {
        unset_map.insert(Value::String(field.clone()), Value::String(String::new()));
    }
    merge_update_operator(&mut update_map, "$unset", unset_map);

    let update_doc = build_update_doc(RawUpdate(update_map)).unwrap_or_else(|e| {
        eprintln!("error: invalid update: {}", e);
        std::process::exit(2);
    });

    let doc_expect = args.expect.as_deref().map(parse_cli_expect);

    if args.strict && !args.dry_run {
        enforce_strict_update(doc_expect.is_some(), &update_doc);
    }

    let library_path = get_library_path(&config);
    let ext = config.format.extension();

    let docs: Vec<(Key, String)> = if update_doc.block_ops.is_empty() {
        let find_op = FindOp::new().filter(filter);
        let outcome = run_op(&Operation::Find(find_op), &graph).expect("find query does not fail");
        let keys: Vec<Key> = match outcome {
            Outcome::Find { matches, .. } => matches.into_iter().map(|m| m.key).collect(),
            _ => unreachable!(),
        };
        if !args.dry_run {
            let doc_refs = build_doc_refs(&graph, &keys);
            check_document_expect("update", doc_expect, &doc_refs).unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                std::process::exit(2);
            });
        }
        keys.into_iter()
            .filter_map(|key| {
                let file_path = library_path.join(format!("{}.{}", key, ext));
                let raw_content = std::fs::read_to_string(&file_path).ok()?;
                let (_, body) = split_raw_frontmatter(&raw_content);
                let mut mapping = graph.frontmatter(&key).cloned().unwrap_or_default();
                liwe::query::update::apply(&update_doc, &mut mapping);
                liwe::query::frontmatter::strip_reserved(&mut mapping);
                let yaml = if mapping.is_empty() {
                    String::new()
                } else {
                    let serialized = serde_yaml::to_string(&mapping).unwrap_or_default();
                    format!("---\n{}---\n", serialized)
                };
                Some((key, format!("{}{}", yaml, body)))
            })
            .collect()
    } else {
        let mut op = UpdateOp::new(filter, update_doc);
        if !args.dry_run {
            if let Some(expect) = doc_expect {
                op = op.expect(expect);
            }
        }
        let outcome = run_op(&Operation::Update(op), &graph).unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            std::process::exit(2);
        });
        match outcome {
            Outcome::Update { changes } => changes,
            _ => unreachable!(),
        }
    };

    if args.strict && !args.dry_run {
        gate_pending(&config, &docs);
    }

    let (matched, changed) = write_changed_documents(&library_path, ext, &docs, args.dry_run);

    if args.strict && !args.dry_run {
        let targets: Vec<Key> = docs.iter().map(|(key, _)| key.clone()).collect();
        for (key, content) in &docs {
            graph.update_document(key.clone(), content.clone());
        }
        warn_stats(&config, &graph, &targets);
    }

    report_mutation(args.quiet, args.dry_run, matched, changed);
}

fn parse_cli_expect(arg: &str) -> liwe::query::Expect {
    let value: serde_yaml::Value = serde_yaml::from_str(arg).unwrap_or_else(|e| {
        eprintln!("error: invalid --expect: {}", e);
        std::process::exit(2);
    });
    liwe::query::parse_expect(&value).unwrap_or_else(|e| {
        eprintln!("error: invalid --expect: {}", e);
        std::process::exit(2);
    })
}

fn build_doc_refs(graph: &Graph, keys: &[Key]) -> Vec<liwe::query::block_update::DocRef> {
    keys.iter()
        .map(|key| liwe::query::block_update::DocRef {
            key: key.to_string(),
            title: graph.get_key_title(key).unwrap_or_else(|| key.to_string()),
        })
        .collect()
}

fn enforce_strict_update(has_doc_expect: bool, update_doc: &liwe::query::Update) {
    let mut missing: Vec<String> = Vec::new();
    if !has_doc_expect {
        missing.push("document-level --expect".to_string());
    }
    for block_op in &update_doc.block_ops {
        if block_op.expect.is_none() {
            missing.push(format!("{} expect", block_op.op.name()));
        }
    }
    if !missing.is_empty() {
        eprintln!(
            "error: --strict requires an expect guard on every mutating application; missing: {}",
            missing.join(", ")
        );
        eprintln!(
            "hint: state the expected count — 1 for a precision edit, '{{ min: 1 }}' for a bulk edit that must match, '{{ min: 0 }}' when zero is acceptable"
        );
        std::process::exit(2);
    }
}

fn write_changed_documents(
    library_path: &std::path::Path,
    ext: &str,
    docs: &[(Key, String)],
    dry_run: bool,
) -> (usize, usize) {
    let mut changed = 0;
    for (key, content) in docs {
        let file_path = library_path.join(format!("{}.{}", key, ext));
        let existing = std::fs::read_to_string(&file_path).unwrap_or_default();
        if *content == existing {
            continue;
        }
        if !dry_run {
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&file_path, content).expect("Failed to write document file");
        }
        changed += 1;
    }
    (docs.len(), changed)
}

fn report_mutation(quiet: bool, dry_run: bool, matched: usize, changed: usize) {
    if quiet {
        return;
    }
    if matched == 0 {
        println!("No documents matched");
        return;
    }
    if changed == matched {
        let verb = if dry_run { "Would update" } else { "Updated" };
        println!("{} {} document(s)", verb, changed);
    } else {
        let tail = if dry_run { "would change" } else { "changed" };
        println!("Matched {} document(s), {} {}", matched, changed, tail);
    }
}

fn merge_update_operator(
    update_map: &mut serde_yaml::Mapping,
    key: &str,
    fields: serde_yaml::Mapping,
) {
    use serde_yaml::Value;
    if fields.is_empty() {
        return;
    }
    let entry = update_map
        .entry(Value::String(key.to_string()))
        .or_insert_with(|| Value::Mapping(serde_yaml::Mapping::new()));
    if let Value::Mapping(existing) = entry {
        for (k, v) in fields {
            existing.insert(k, v);
        }
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
                let target = match render_key_template(&a.key_template) {
                    Ok(target) => target,
                    Err(e) => {
                        eprintln!("Error: action '{}': {}", name, e);
                        std::process::exit(1);
                    }
                };
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

        let target_key_str = match render_key_template(&attach.key_template) {
            Ok(target) => target,
            Err(e) => {
                eprintln!("Error: action '{}': {}", action_name, e);
                std::process::exit(1);
            }
        };
        let target_key = Key::name(&target_key_str);

        let new_content = match attach_reference(&graph, &target_key, &source_key, &reference_text)
        {
            AttachTarget::AlreadyAttached => continue,
            AttachTarget::Update(content) => content,
            AttachTarget::Create(body) => {
                match render_document_template(&attach.document_template, &body, &config) {
                    Ok(content) => content,
                    Err(e) => {
                        eprintln!("Error: action '{}': {}", action_name, e);
                        std::process::exit(1);
                    }
                }
            }
        };

        if args.dry_run {
            if !args.quiet {
                println!("Would attach '{}' to '{}'", source_key_str, target_key);
            }
            continue;
        }

        let target_path =
            library_path.join(format!("{}.{}", target_key, config.format.extension()));
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&target_path, new_content).expect("Failed to write target file");

        if !args.quiet {
            println!(
                "Attached '{}' to '{}' as [{}]",
                source_key_str, target_key, reference_text
            );
        }
    }
}

fn render_key_template(template: &str) -> Result<String, String> {
    use chrono::Local;
    use minijinja::{context, Environment};
    let now = Local::now();
    let formatted = now.format("%Y-%m-%d").to_string();
    Environment::new()
        .template_from_str(template)
        .map_err(|e| format!("invalid key template: {}", e))?
        .render(context! {
            today => &formatted,
            now => &formatted,
        })
        .map_err(|e| format!("key template rendering failed: {}", e))
}

fn render_document_template(
    template: &str,
    content: &str,
    config: &Configuration,
) -> Result<String, String> {
    use chrono::Local;
    use minijinja::{context, Environment};
    let now = Local::now();
    let date_format = config
        .markdown
        .date_format
        .as_deref()
        .unwrap_or("%b %d, %Y");
    let formatted = now.format(date_format).to_string();
    Environment::new()
        .template_from_str(template)
        .map_err(|e| format!("invalid document template: {}", e))?
        .render(context! {
            today => &formatted,
            now => &formatted,
            content => content,
        })
        .map_err(|e| format!("document template rendering failed: {}", e))
}
