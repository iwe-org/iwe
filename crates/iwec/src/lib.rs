pub mod watcher;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use liwe::find::{DocumentFinder, FindOptions, FindOutput};
use liwe::query::cli::parse_projection;
use liwe::query::{self, Filter, InclusionAnchor, ProjectionMode};
use liwe::retrieve::{DocumentReader, RetrieveOptions, RetrieveOutput};
use liwe::stats::{GraphStatistics, KeyStatistics};
use liwe::fs::{new_for_path, new_from_hashmap};
use liwe::graph::{Graph, GraphContext};
use chrono::Local;
use liwe::model::config::{ActionDefinition, CompletionOptions, Configuration, MarkdownOptions, NoteTemplate, DEFAULT_KEY_DATE_FORMAT};
use liwe::model::node::{Node, NodeIter, NodePointer, Reference, ReferenceType};
use liwe::model::tree::{Tree, TreeIter};
use liwe::model::Key;
use liwe::operations::{
    delete as op_delete, extract as op_extract, inline as op_inline, rename as op_rename, Changes,
    ExtractConfig, InlineConfig, OperationError,
};
use minijinja::{context, Environment};
use rmcp::handler::server::router::prompt::PromptRouter;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::schemars::JsonSchema;
use rmcp::service::RequestContext;
use rmcp::{prompt, prompt_handler, prompt_router, tool, tool_router, RoleServer};
use rmcp::{ErrorData as McpError, ServerHandler, tool_handler};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

fn to_json_result<T: Serialize>(output: &T) -> Result<CallToolResult, McpError> {
    let json =
        serde_json::to_string(output).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

fn to_text_result(text: String) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum KeyDepthParam {
    Bare(String),
    Qualified {
        key: String,
        depth: Option<u8>,
    },
}

impl KeyDepthParam {
    fn anchor(&self, default_depth: Option<u8>) -> InclusionAnchor {
        let (key, depth) = match self {
            KeyDepthParam::Bare(s) => (s.clone(), None),
            KeyDepthParam::Qualified { key, depth } => (key.clone(), *depth),
        };
        let raw = depth.or(default_depth);
        let max = match raw {
            None => u32::MAX,
            Some(0) => u32::MAX,
            Some(n) => u32::from(n),
        };
        InclusionAnchor::with_max(key, max)
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct SelectorParams {
    #[schemars(
        description = "Restrict to candidates that are sub-documents of EVERY listed key (AND). Each entry is either a bare KEY or {key, depth}."
    )]
    #[serde(rename = "in", default)]
    pub in_: Vec<KeyDepthParam>,
    #[schemars(
        description = "Restrict to candidates that are sub-documents of AT LEAST ONE listed key (OR)."
    )]
    #[serde(default)]
    pub in_any: Vec<KeyDepthParam>,
    #[schemars(
        description = "Exclude candidates that are sub-documents of ANY listed key (NOT)."
    )]
    #[serde(default)]
    pub not_in: Vec<KeyDepthParam>,
    #[schemars(
        description = "Default depth for in / in_any / not_in entries that don't specify their own depth. Omit for unbounded."
    )]
    #[serde(default)]
    pub max_depth: Option<u8>,
}

impl SelectorParams {
    pub fn is_empty(&self) -> bool {
        self.in_.is_empty()
            && self.in_any.is_empty()
            && self.not_in.is_empty()
            && self.max_depth.is_none()
    }

    pub fn to_filter(&self) -> Option<Filter> {
        if self.is_empty() {
            return None;
        }
        let mut conjuncts: Vec<Filter> = Vec::new();
        for kd in &self.in_ {
            conjuncts.push(Filter::IncludedBy(Box::new(kd.anchor(self.max_depth))));
        }
        if !self.in_any.is_empty() {
            conjuncts.push(Filter::Or(
                self.in_any
                    .iter()
                    .map(|kd| Filter::IncludedBy(Box::new(kd.anchor(self.max_depth))))
                    .collect(),
            ));
        }
        for kd in &self.not_in {
            conjuncts.push(Filter::Not(Box::new(Filter::IncludedBy(Box::new(
                kd.anchor(self.max_depth),
            )))));
        }
        Some(Filter::And(conjuncts))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindParams {
    #[schemars(description = "Fuzzy search query matching against document title and key")]
    pub query: Option<String>,
    #[schemars(description = "Only return documents that reference this key")]
    pub refs_to: Option<String>,
    #[schemars(description = "Only return documents referenced by this key")]
    pub refs_from: Option<String>,
    #[schemars(description = "Maximum number of results to return")]
    pub limit: Option<usize>,
    #[schemars(description = "Replacement projection (e.g. 'title,priority' or 'body=$content,parents=$includedBy'). Mutually exclusive with add_fields.")]
    pub project: Option<String>,
    #[schemars(description = "Additive projection: same grammar as project, extends defaults rather than replacing. Mutually exclusive with project.")]
    pub add_fields: Option<String>,
    #[serde(flatten)]
    pub selector: SelectorParams,
}

impl TryFrom<FindParams> for FindOptions {
    type Error = McpError;

    fn try_from(p: FindParams) -> Result<Self, Self::Error> {
        let project = match (p.project.as_deref(), p.add_fields.as_deref()) {
            (Some(_), Some(_)) => {
                return Err(McpError::invalid_params(
                    "project and add_fields are mutually exclusive".to_string(),
                    None,
                ))
            }
            (Some(s), None) => Some(
                parse_projection(s, ProjectionMode::Replace)
                    .map_err(|e| McpError::invalid_params(e, None))?,
            ),
            (None, Some(s)) => Some(
                parse_projection(s, ProjectionMode::Extend)
                    .map_err(|e| McpError::invalid_params(e, None))?,
            ),
            (None, None) => None,
        };
        Ok(FindOptions {
            query: p.query,
            refs_to: p.refs_to.map(|k| Key::name(&k)),
            refs_from: p.refs_from.map(|k| Key::name(&k)),
            filter: p.selector.to_filter(),
            limit: p.limit,
            sort: None,
            project,
        })
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RetrieveParams {
    #[schemars(description = "Document keys to retrieve. Can be empty when a structural selector is provided.")]
    #[serde(default)]
    pub keys: Vec<String>,
    #[schemars(description = "Levels of block references to expand (0 = document only, 1 = include direct sub-documents). Default: 1")]
    pub depth: Option<u8>,
    #[schemars(description = "Levels of parent documents to include. Default: 1")]
    pub context: Option<u8>,
    #[schemars(description = "Include inline-linked documents. Default: false")]
    pub links: Option<bool>,
    #[schemars(description = "Include incoming inline references. Default: true")]
    pub backlinks: Option<bool>,
    #[schemars(description = "Document keys to exclude from results")]
    pub exclude: Option<Vec<String>>,
    #[schemars(description = "Return metadata only without document content. Default: false")]
    pub no_content: Option<bool>,
    #[schemars(description = "Populate the `includes` array with child document edges. Default: false")]
    pub children: Option<bool>,
    #[serde(flatten)]
    pub selector: SelectorParams,
}

impl From<RetrieveParams> for RetrieveOptions {
    fn from(p: RetrieveParams) -> Self {
        RetrieveOptions {
            depth: p.depth.unwrap_or(1),
            context: p.context.unwrap_or(1),
            links: p.links.unwrap_or(false),
            backlinks: p.backlinks.unwrap_or(true),
            exclude: p
                .exclude
                .unwrap_or_default()
                .into_iter()
                .map(|k| Key::name(&k))
                .collect::<HashSet<_>>(),
            no_content: p.no_content.unwrap_or(false),
            children: p.children.unwrap_or(false),
            filter: p.selector.to_filter(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TreeParams {
    #[schemars(description = "Starting document keys. If empty and no selector, shows all root documents.")]
    pub keys: Option<Vec<String>>,
    #[schemars(description = "Maximum traversal depth. Default: 4")]
    pub depth: Option<u8>,
    #[serde(flatten)]
    pub selector: SelectorParams,
}

#[derive(Debug, Serialize)]
struct TreeNode {
    key: String,
    title: String,
    children: Vec<TreeNode>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatsParams {
    #[schemars(description = "Document key for per-document stats. Omit for aggregate graph statistics")]
    pub key: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SquashParams {
    #[schemars(description = "Root document key to expand")]
    pub key: String,
    #[schemars(description = "Levels of references to expand. Default: 2")]
    pub depth: Option<u8>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateParams {
    #[schemars(description = "Document title")]
    pub title: String,
    #[schemars(description = "Markdown content body (without the title heading)")]
    pub content: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateParams {
    #[schemars(description = "Document key to update")]
    pub key: String,
    #[schemars(description = "New full markdown content")]
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteParams {
    #[schemars(description = "Document key to delete")]
    pub key: String,
    #[schemars(description = "Preview changes without applying. Default: false")]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameParams {
    #[schemars(description = "Current document key")]
    pub old_key: String,
    #[schemars(description = "New document key")]
    pub new_key: String,
    #[schemars(description = "Preview changes without applying. Default: false")]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ChangesOutput {
    creates: Vec<ChangeEntry>,
    updates: Vec<ChangeEntry>,
    removes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ChangeEntry {
    key: String,
    content: String,
}

impl From<&Changes> for ChangesOutput {
    fn from(c: &Changes) -> Self {
        ChangesOutput {
            creates: c.creates.iter().map(|(k, v)| ChangeEntry { key: k.to_string(), content: v.clone() }).collect(),
            updates: c.updates.iter().map(|(k, v)| ChangeEntry { key: k.to_string(), content: v.clone() }).collect(),
            removes: c.removes.iter().map(|k| k.to_string()).collect(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractParams {
    #[schemars(description = "Source document key")]
    pub key: String,
    #[schemars(description = "Section title to extract (case-insensitive partial match)")]
    pub section: Option<String>,
    #[schemars(description = "Block number to extract (1-indexed, use list mode to discover)")]
    pub block: Option<usize>,
    #[schemars(description = "List all sections with block numbers instead of extracting. Default: false")]
    pub list: Option<bool>,
    #[schemars(description = "Preview changes without applying. Default: false")]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InlineParams {
    #[schemars(description = "Document key containing the block reference")]
    pub key: String,
    #[schemars(description = "Reference key or title to inline (partial match)")]
    pub reference: Option<String>,
    #[schemars(description = "Block number to inline (1-indexed, use list mode to discover)")]
    pub block: Option<usize>,
    #[schemars(description = "List all block references instead of inlining. Default: false")]
    pub list: Option<bool>,
    #[schemars(description = "Inline as blockquote instead of section. Default: false")]
    pub as_quote: Option<bool>,
    #[schemars(description = "Keep the target document after inlining. Default: false")]
    pub keep_target: Option<bool>,
    #[schemars(description = "Preview changes without applying. Default: false")]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Serialize)]
struct SectionEntry {
    block_number: usize,
    title: String,
}

#[derive(Debug, Serialize)]
struct ReferenceEntry {
    block_number: usize,
    key: String,
    title: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AttachParams {
    #[schemars(description = "Configured attach action(s) to attach to (e.g. 'today'). Pass one or more action names; the source is attached under each resolved target.")]
    #[serde(default)]
    pub to: Vec<String>,
    #[schemars(description = "Document key to attach as a block reference in the target(s)")]
    pub key: Option<String>,
    #[schemars(description = "List available attach actions instead of executing. Default: false")]
    pub list: Option<bool>,
    #[schemars(description = "Preview changes without applying. Default: false")]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Serialize)]
struct AttachActionEntry {
    name: String,
    title: String,
    target_key: String,
}

#[derive(Debug, Serialize)]
struct ConfigResource {
    markdown: MarkdownOptions,
    library: LibraryResourceView,
    completion: CompletionOptions,
    templates: HashMap<String, NoteTemplate>,
    actions: Vec<ActionResourceView>,
}

#[derive(Debug, Serialize)]
struct LibraryResourceView {
    date_format: Option<String>,
    default_template: Option<String>,
    frontmatter_document_title: Option<String>,
    locale: Option<String>,
}

#[derive(Debug, Serialize)]
struct ActionResourceView {
    name: String,
    action_type: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_key: Option<String>,
}

impl ConfigResource {
    fn from_config(config: &Configuration, server: &IweServer) -> Self {
        let actions = config
            .actions
            .iter()
            .map(|(name, action)| {
                let (action_type, title, target_key) = match action {
                    ActionDefinition::Transform(a) => ("transform", a.title.clone(), None),
                    ActionDefinition::Attach(a) => (
                        "attach",
                        a.title.clone(),
                        Some(server.render_key_template(&a.key_template)),
                    ),
                    ActionDefinition::Sort(a) => ("sort", a.title.clone(), None),
                    ActionDefinition::Inline(a) => ("inline", a.title.clone(), None),
                    ActionDefinition::Extract(a) => ("extract", a.title.clone(), None),
                    ActionDefinition::ExtractAll(a) => ("extract_all", a.title.clone(), None),
                    ActionDefinition::Link(a) => ("link", a.title.clone(), None),
                };
                ActionResourceView {
                    name: name.clone(),
                    action_type: action_type.to_string(),
                    title,
                    target_key,
                }
            })
            .collect();

        Self {
            markdown: config.markdown.clone(),
            library: LibraryResourceView {
                date_format: config.library.date_format.clone(),
                default_template: config.library.default_template.clone(),
                frontmatter_document_title: config.library.frontmatter_document_title.clone(),
                locale: config.library.locale.clone(),
            },
            completion: config.completion.clone(),
            templates: config.templates.clone(),
            actions,
        }
    }
}

fn op_error_to_mcp(e: OperationError) -> McpError {
    McpError::invalid_params(e.to_string(), None)
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReviewPromptArgs {
    #[schemars(description = "Document key to review")]
    pub key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RefactorPromptArgs {
    #[schemars(description = "Root document key to analyze for restructuring")]
    pub key: String,
}

#[derive(Clone)]
pub struct IweServer {
    graph: Arc<Mutex<Graph>>,
    base_path: Option<PathBuf>,
    config: Configuration,
    tool_router: ToolRouter<IweServer>,
    prompt_router: PromptRouter<IweServer>,
}

#[tool_router]
impl IweServer {
    #[tool(description = "Search and discover documents in the knowledge graph. Supports fuzzy text query (`query`), root filter (`roots`), direct-reference filters (`refs_to`, `refs_from`), and the structural set selector (`in` / `in_any` / `not_in` / `max_depth`) for transitive sub-document AND/OR/NOT queries with configurable depth.")]
    async fn iwe_find(
        &self,
        Parameters(params): Parameters<FindParams>,
    ) -> Result<CallToolResult, McpError> {
        let options: FindOptions = params.try_into()?;
        let graph = self.graph.lock().await;
        let finder = DocumentFinder::new(&graph);
        let output: FindOutput = finder.find(&options);
        to_json_result(&output.results)
    }

    #[tool(description = "Retrieve documents from the knowledge graph with configurable depth expansion, parent context, backlinks, and linked documents")]
    async fn iwe_retrieve(
        &self,
        Parameters(params): Parameters<RetrieveParams>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph.lock().await;
        let reader = DocumentReader::new(&graph);
        let keys: Vec<Key> = params.keys.iter().map(|k| Key::name(k)).collect();
        let options: RetrieveOptions = params.into();
        let output: RetrieveOutput = reader.retrieve_many(&keys, &options);
        to_json_result(&output.documents)
    }

    #[tool(description = "View the hierarchical tree structure of the knowledge graph showing how documents are connected via block references. Supports the structural set selector (in / in_any / not_in / max_depth) — when provided, the tree roots are restricted to (or selected from) that set.")]
    async fn iwe_tree(
        &self,
        Parameters(params): Parameters<TreeParams>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph.lock().await;

        let filter = params.selector.to_filter();
        let explicit_keys: Vec<Key> = params
            .keys
            .filter(|k| !k.is_empty())
            .map(|ks| ks.iter().map(|k| Key::name(k)).collect())
            .unwrap_or_default();

        let root_keys: Vec<Key> = if let Some(f) = filter {
            let selector_set: HashSet<Key> =
                query::evaluate(&f, &graph).into_iter().collect();
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
        } else if !explicit_keys.is_empty() {
            explicit_keys
        } else {
            let paths = graph.paths();
            let mut keys: Vec<Key> = paths
                .iter()
                .filter(|n| n.ids().len() == 1)
                .filter_map(|n| n.first_id())
                .map(|id| (&*graph).node(id).node_key())
                .collect();
            keys.sort();
            keys.dedup();
            keys
        };

        let max_depth = params.depth.unwrap_or(4);
        let mut trees: Vec<TreeNode> = Vec::new();
        for root_key in &root_keys {
            let mut visited: HashSet<Key> = HashSet::new();
            if let Some(node) = build_tree_node(&graph, root_key, max_depth, &mut visited) {
                trees.push(node);
            }
        }
        to_json_result(&trees)
    }

    #[tool(description = "Get comprehensive statistics about the knowledge graph including document counts, reference patterns, broken links, and most connected documents")]
    async fn iwe_stats(
        &self,
        Parameters(params): Parameters<StatsParams>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph.lock().await;
        if let Some(key) = params.key {
            let all_stats = KeyStatistics::from_graph(&graph);
            let stat = all_stats
                .into_iter()
                .find(|s| s.key == key)
                .ok_or_else(|| {
                    McpError::invalid_params(format!("Document '{}' not found", key), None)
                })?;
            to_json_result(&stat)
        } else {
            let stats = GraphStatistics::from_graph(&graph);
            to_json_result(&stats)
        }
    }

    #[tool(description = "Expand all block references into a single flat markdown document. Useful for export or generating a complete view of a document tree")]
    async fn iwe_squash(
        &self,
        Parameters(params): Parameters<SquashParams>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph.lock().await;
        let key = Key::name(&params.key);
        let depth = params.depth.unwrap_or(2);

        if (&*graph).get_node_id(&key).is_none() {
            return Err(McpError::invalid_params(
                format!("Document '{}' not found", params.key),
                None,
            ));
        }

        let squashed: Tree = (&*graph).squash(&key, depth);
        let mut patch = Graph::new();
        patch.build_key_from_iter(&key, TreeIter::new(&squashed));
        let content = patch.export_key(&key).unwrap_or_default();
        to_text_result(content)
    }

    #[tool(description = "Create a new document in the knowledge graph from a title and optional content")]
    async fn iwe_create(
        &self,
        Parameters(params): Parameters<CreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let slug = params
            .title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        let content_body = params.content.unwrap_or_default();
        let markdown = if content_body.is_empty() {
            format!("# {}\n", params.title)
        } else {
            format!("# {}\n\n{}\n", params.title, content_body)
        };

        let key = Key::name(&slug);
        let mut graph = self.graph.lock().await;

        if (&*graph).get_node_id(&key).is_some() {
            return Err(McpError::invalid_params(
                format!("Document '{}' already exists", slug),
                None,
            ));
        }

        graph.insert_document(key.clone(), markdown.clone());
        self.write_file(&key, &markdown);

        #[derive(Serialize)]
        struct CreateResult {
            key: String,
        }
        to_json_result(&CreateResult {
            key: slug,
        })
    }

    #[tool(description = "Update the full markdown content of an existing document")]
    async fn iwe_update(
        &self,
        Parameters(params): Parameters<UpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        let key = Key::name(&params.key);
        let mut graph = self.graph.lock().await;

        if (&*graph).get_node_id(&key).is_none() {
            return Err(McpError::invalid_params(
                format!("Document '{}' not found", params.key),
                None,
            ));
        }

        let previous_title = (&*graph)
            .get_key_title(&key)
            .unwrap_or_else(|| params.key.clone());

        graph.update_document(key.clone(), params.content.clone());
        self.write_file(&key, &params.content);

        let new_title = (&*graph)
            .get_key_title(&key)
            .unwrap_or_else(|| params.key.clone());

        #[derive(Serialize)]
        struct UpdateResult {
            key: String,
            previous_title: String,
            new_title: String,
        }
        to_json_result(&UpdateResult {
            key: params.key,
            previous_title,
            new_title,
        })
    }

    #[tool(description = "Delete a document from the knowledge graph. All block references and inline links to this document in other documents are cleaned up")]
    async fn iwe_delete(
        &self,
        Parameters(params): Parameters<DeleteParams>,
    ) -> Result<CallToolResult, McpError> {
        let key = Key::name(&params.key);
        let mut graph = self.graph.lock().await;
        let changes = op_delete(&graph, &key).map_err(op_error_to_mcp)?;

        if !params.dry_run.unwrap_or(false) {
            Self::apply_changes(&mut graph, &changes);
            self.write_changes(&changes);
        }

        to_json_result(&ChangesOutput::from(&changes))
    }

    #[tool(description = "Rename a document key. All block references and inline links across the entire graph are updated to point to the new key")]
    async fn iwe_rename(
        &self,
        Parameters(params): Parameters<RenameParams>,
    ) -> Result<CallToolResult, McpError> {
        let old_key = Key::name(&params.old_key);
        let new_key = Key::name(&params.new_key);
        let mut graph = self.graph.lock().await;
        let changes = op_rename(&graph, &old_key, &new_key).map_err(op_error_to_mcp)?;

        if !params.dry_run.unwrap_or(false) {
            Self::apply_changes(&mut graph, &changes);
            self.write_changes(&changes);
        }

        to_json_result(&ChangesOutput::from(&changes))
    }

    #[tool(description = "Extract a section from a document into a new standalone document. The original section is replaced with a block reference. Use list mode to discover sections first")]
    async fn iwe_extract(
        &self,
        Parameters(params): Parameters<ExtractParams>,
    ) -> Result<CallToolResult, McpError> {
        let source_key = Key::name(&params.key);
        let mut graph = self.graph.lock().await;

        if (&*graph).get_node_id(&source_key).is_none() {
            return Err(McpError::invalid_params(
                format!("Document '{}' not found", params.key),
                None,
            ));
        }

        let tree = (&*graph).collect(&source_key);
        let sections = collect_sections(&tree);

        if params.list.unwrap_or(false) {
            return to_json_result(&sections);
        }

        let selected = if let Some(ref title) = params.section {
            let matches: Vec<_> = sections
                .iter()
                .filter(|s| s.title.to_lowercase().contains(&title.to_lowercase()))
                .collect();
            if matches.is_empty() {
                return Err(McpError::invalid_params(
                    format!("No section matches '{}'", title),
                    None,
                ));
            }
            if matches.len() > 1 {
                return Err(McpError::invalid_params(
                    format!(
                        "Multiple sections match '{}': {}",
                        title,
                        matches.iter().map(|s| s.title.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                    None,
                ));
            }
            matches[0].block_number
        } else if let Some(block) = params.block {
            if block == 0 || block > sections.len() {
                return Err(McpError::invalid_params(
                    format!("Block number {} out of range (1-{})", block, sections.len()),
                    None,
                ));
            }
            block
        } else {
            return Err(McpError::invalid_params(
                "Must specify section, block, or list",
                None,
            ));
        };

        let section_id = tree
            .children
            .iter()
            .flat_map(|c| collect_section_ids(c))
            .nth(selected - 1)
            .ok_or_else(|| McpError::invalid_params("Section not found", None))?;

        let config = ExtractConfig::default();
        let changes = op_extract(&graph, &source_key, section_id, &config, std::time::SystemTime::now()).map_err(op_error_to_mcp)?;

        if !params.dry_run.unwrap_or(false) {
            Self::apply_changes(&mut graph, &changes);
            self.write_changes(&changes);
        }

        to_json_result(&ChangesOutput::from(&changes))
    }

    #[tool(description = "Replace a block reference with the actual content of the referenced document. Use list mode to discover block references first")]
    async fn iwe_inline(
        &self,
        Parameters(params): Parameters<InlineParams>,
    ) -> Result<CallToolResult, McpError> {
        let source_key = Key::name(&params.key);
        let mut graph = self.graph.lock().await;

        if (&*graph).get_node_id(&source_key).is_none() {
            return Err(McpError::invalid_params(
                format!("Document '{}' not found", params.key),
                None,
            ));
        }

        let tree = (&*graph).collect(&source_key);
        let refs = collect_block_refs(&tree);

        if params.list.unwrap_or(false) {
            return to_json_result(&refs);
        }

        let selected = if let Some(ref reference) = params.reference {
            let matches: Vec<_> = refs
                .iter()
                .filter(|r| {
                    r.title.to_lowercase().contains(&reference.to_lowercase())
                        || r.key.to_lowercase().contains(&reference.to_lowercase())
                })
                .collect();
            if matches.is_empty() {
                return Err(McpError::invalid_params(
                    format!("No reference matches '{}'", reference),
                    None,
                ));
            }
            if matches.len() > 1 {
                return Err(McpError::invalid_params(
                    format!(
                        "Multiple references match '{}': {}",
                        reference,
                        matches.iter().map(|r| r.key.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                    None,
                ));
            }
            matches[0].block_number
        } else if let Some(block) = params.block {
            if block == 0 || block > refs.len() {
                return Err(McpError::invalid_params(
                    format!("Block number {} out of range (1-{})", block, refs.len()),
                    None,
                ));
            }
            block
        } else {
            return Err(McpError::invalid_params(
                "Must specify reference, block, or list",
                None,
            ));
        };

        let ref_id = collect_ref_ids(&tree)
            .into_iter()
            .nth(selected - 1)
            .ok_or_else(|| McpError::invalid_params("Reference not found", None))?;

        let inline_type = if params.as_quote.unwrap_or(false) {
            liwe::model::config::InlineType::Quote
        } else {
            liwe::model::config::InlineType::Section
        };

        let config = InlineConfig {
            inline_type,
            keep_target: params.keep_target.unwrap_or(false),
        };

        let changes = op_inline(&graph, &source_key, ref_id, &config).map_err(op_error_to_mcp)?;

        if !params.dry_run.unwrap_or(false) {
            Self::apply_changes(&mut graph, &changes);
            self.write_changes(&changes);
        }

        to_json_result(&ChangesOutput::from(&changes))
    }

    #[tool(description = "Normalize all document formatting across the knowledge graph. Re-parses and re-writes all documents to ensure consistent formatting")]
    async fn iwe_normalize(&self) -> Result<CallToolResult, McpError> {
        let mut graph = self.graph.lock().await;
        let state = graph.export();
        let original_count = state.len();

        let mut changed = 0usize;
        for (key_str, original_content) in &state {
            let key = Key::name(key_str);
            let new_content = graph.to_markdown(&key);
            if new_content != *original_content {
                graph.update_document(key.clone(), new_content.clone());
                self.write_file(&key, &new_content);
                changed += 1;
            }
        }

        #[derive(Serialize)]
        struct NormalizeResult {
            total: usize,
            normalized: usize,
        }
        to_json_result(&NormalizeResult {
            total: original_count,
            normalized: changed,
        })
    }

    #[tool(description = "Attach a document as a block reference in one or more target documents determined by configured attach actions. Each target key is derived from the action's key_template (e.g. daily/{{today}}). The `to` field accepts a list of action names; the source is attached under each resolved target. Targets that already contain the source are silently skipped. Use list mode to discover available attach actions.")]
    async fn iwe_attach(
        &self,
        Parameters(params): Parameters<AttachParams>,
    ) -> Result<CallToolResult, McpError> {
        if params.list.unwrap_or(false) {
            let entries: Vec<AttachActionEntry> = self
                .config
                .actions
                .iter()
                .filter_map(|(name, action)| {
                    if let ActionDefinition::Attach(attach) = action {
                        Some(AttachActionEntry {
                            name: name.clone(),
                            title: attach.title.clone(),
                            target_key: self.render_key_template(&attach.key_template),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            return to_json_result(&entries);
        }

        if params.to.is_empty() {
            return Err(McpError::invalid_params(
                "'to' is required when not in list mode (pass one or more action names)".to_string(),
                None,
            ));
        }
        let source_key_str = params.key.as_deref().ok_or_else(|| {
            McpError::invalid_params("'key' is required when not in list mode".to_string(), None)
        })?;

        let mut graph = self.graph.lock().await;

        let source_key = Key::name(source_key_str);
        if (&*graph).get_node_id(&source_key).is_none() {
            return Err(McpError::invalid_params(
                format!("Document '{}' not found", source_key_str),
                None,
            ));
        }

        let reference_text = (&*graph)
            .get_key_title(&source_key)
            .unwrap_or_else(|| source_key_str.to_string());

        let mut combined = Changes::new();
        let markdown_options = graph.markdown_options();

        for action_name in &params.to {
            let attach = match self.config.actions.get(action_name) {
                Some(ActionDefinition::Attach(a)) => a,
                Some(_) => {
                    return Err(McpError::invalid_params(
                        format!("Action '{}' is not an attach action", action_name),
                        None,
                    ));
                }
                None => {
                    return Err(McpError::invalid_params(
                        format!("Action '{}' not found", action_name),
                        None,
                    ));
                }
            };

            let target_key = Key::name(&self.render_key_template(&attach.key_template));

            if (&*graph).get_node_id(&target_key).is_some() {
                let tree = (&*graph).collect(&target_key);
                if tree
                    .get_all_inclusion_edge_keys()
                    .contains(&source_key)
                {
                    continue;
                }
            }

            let reference = Tree {
                id: None,
                node: Node::Reference(Reference {
                    key: source_key.clone(),
                    text: reference_text.clone(),
                    reference_type: ReferenceType::Regular,
                }),
                children: vec![],
            };

            if (&*graph).get_node_id(&target_key).is_some() {
                let tree = (&*graph).collect(&target_key);
                let updated = tree.attach(reference);
                combined.add_update(
                    target_key.clone(),
                    updated
                        .iter()
                        .to_markdown(&target_key.parent(), &markdown_options),
                );
            } else {
                let content = reference
                    .iter()
                    .to_markdown(&target_key.parent(), &markdown_options);
                let document = self.render_document_template(
                    &attach.document_template,
                    &content,
                );
                combined.add_create(target_key.clone(), document);
            }
        }

        if !params.dry_run.unwrap_or(false) {
            Self::apply_changes(&mut graph, &combined);
            self.write_changes(&combined);
        }

        to_json_result(&ChangesOutput::from(&combined))
    }
}

fn build_tree_node(
    graph: &Graph,
    key: &Key,
    max_depth: u8,
    visited: &mut HashSet<Key>,
) -> Option<TreeNode> {
    graph.get_node_id(key)?;

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
        let ref_node_ids = graph.get_inclusion_edges_in(key);
        let mut refs: Vec<Key> = ref_node_ids
            .iter()
            .filter_map(|id| graph.graph_node(*id).ref_key())
            .collect();
        refs.sort();
        refs.into_iter()
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

use liwe::model::tree::Tree as ModelTree;
use liwe::model::NodeId;

fn collect_sections(tree: &ModelTree) -> Vec<SectionEntry> {
    let mut result = Vec::new();
    collect_sections_rec(tree, &mut result);
    result
}

fn collect_sections_rec(tree: &ModelTree, sections: &mut Vec<SectionEntry>) {
    if let Node::Section(inlines) = &tree.node {
        let title = inlines.iter().map(|i| i.plain_text()).collect::<String>();
        sections.push(SectionEntry {
            block_number: sections.len() + 1,
            title,
        });
    }
    for child in &tree.children {
        collect_sections_rec(child, sections);
    }
}

fn collect_section_ids(tree: &ModelTree) -> Vec<NodeId> {
    let mut ids = Vec::new();
    if tree.is_section() {
        if let Some(id) = tree.id {
            ids.push(id);
        }
    }
    for child in &tree.children {
        ids.extend(collect_section_ids(child));
    }
    ids
}

fn collect_block_refs(tree: &ModelTree) -> Vec<ReferenceEntry> {
    let mut result = Vec::new();
    collect_block_refs_rec(tree, &mut result);
    result
}

fn collect_block_refs_rec(tree: &ModelTree, refs: &mut Vec<ReferenceEntry>) {
    if let Node::Reference(reference) = &tree.node {
        refs.push(ReferenceEntry {
            block_number: refs.len() + 1,
            key: reference.key.to_string(),
            title: reference.text.clone(),
        });
    }
    for child in &tree.children {
        collect_block_refs_rec(child, refs);
    }
}

fn collect_ref_ids(tree: &ModelTree) -> Vec<NodeId> {
    let mut ids = Vec::new();
    if let Node::Reference(_) = &tree.node {
        if let Some(id) = tree.id {
            ids.push(id);
        }
    }
    for child in &tree.children {
        ids.extend(collect_ref_ids(child));
    }
    ids
}

#[prompt_router]
impl IweServer {
    #[prompt(
        name = "explore",
        description = "Start exploring the knowledge graph. Provides an overview of size, structure, root entry points, broken links, and orphaned documents"
    )]
    async fn explore(&self) -> Result<GetPromptResult, McpError> {
        let graph = self.graph.lock().await;
        let stats = GraphStatistics::from_graph(&graph);
        let stats_json =
            serde_json::to_string_pretty(&stats).unwrap_or_else(|_| "{}".to_string());

        let messages = vec![PromptMessage::new_text(
            PromptMessageRole::User,
            format!(
                "Here is an overview of the IWE knowledge graph.\n\n## Statistics\n\n```json\n{}\n```\n\nExplore the graph using iwe_retrieve to read documents, iwe_find to search, and iwe_tree to navigate the structure.",
                stats_json
            ),
        )];

        Ok(GetPromptResult::new(messages)
            .with_description("Overview of the IWE knowledge graph"))
    }

    #[prompt(
        name = "review",
        description = "Review a specific document within its graph context — its content, parents, children, and backlinks"
    )]
    async fn review(
        &self,
        Parameters(args): Parameters<ReviewPromptArgs>,
    ) -> Result<GetPromptResult, McpError> {
        let graph = self.graph.lock().await;
        let key = Key::name(&args.key);
        let reader = DocumentReader::new(&graph);
        let output = reader.retrieve(
            &key,
            &RetrieveOptions {
                depth: 2,
                context: 2,
                backlinks: true,
                ..Default::default()
            },
        );
        let json = serde_json::to_string_pretty(&output.documents)
            .unwrap_or_else(|_| "[]".to_string());

        let messages = vec![PromptMessage::new_text(
            PromptMessageRole::User,
            format!(
                "Review this document and its context in the knowledge graph:\n\n```json\n{}\n```\n\nConsider: Is it well-placed in the graph? Are there missing links? Is the content clear and well-structured? What sections might be extracted into separate documents?",
                json
            ),
        )];

        Ok(GetPromptResult::new(messages)
            .with_description(format!("Review of document '{}'", args.key)))
    }

    #[prompt(
        name = "refactor",
        description = "Analyze a section of the knowledge graph and suggest restructuring using extract, inline, and rename operations"
    )]
    async fn refactor(
        &self,
        Parameters(args): Parameters<RefactorPromptArgs>,
    ) -> Result<GetPromptResult, McpError> {
        let graph = self.graph.lock().await;
        let key = Key::name(&args.key);
        let reader = DocumentReader::new(&graph);
        let output = reader.retrieve(
            &key,
            &RetrieveOptions {
                depth: 3,
                context: 1,
                backlinks: true,
                ..Default::default()
            },
        );
        let json = serde_json::to_string_pretty(&output.documents)
            .unwrap_or_else(|_| "[]".to_string());

        let messages = vec![PromptMessage::new_text(
            PromptMessageRole::User,
            format!(
                "Analyze this document tree and suggest restructuring:\n\n```json\n{}\n```\n\nIdentify documents that are too large (should be extracted with iwe_extract), too small (should be inlined with iwe_inline), poorly named (should be renamed with iwe_rename), or missing connections. Propose a sequence of operations to improve the structure.",
                json
            ),
        )];

        Ok(GetPromptResult::new(messages)
            .with_description(format!("Refactoring analysis for '{}'", args.key)))
    }
}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for IweServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("iwe", env!("CARGO_PKG_VERSION")))
        .with_instructions(
            "IWE knowledge graph server. Tools: iwe_find, iwe_retrieve, iwe_tree, iwe_stats, iwe_squash, iwe_create, iwe_update, iwe_delete, iwe_rename, iwe_extract, iwe_inline, iwe_normalize, iwe_attach. Prompts: explore, review, refactor. Resources: iwe://documents/{key}, iwe://tree, iwe://stats, iwe://config."
                .to_string(),
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let graph = self.graph.lock().await;
        let mut resources = vec![
            RawResource::new("iwe://tree", "tree")
                .with_description("Full document tree structure")
                .with_mime_type("application/json")
                .no_annotation(),
            RawResource::new("iwe://stats", "stats")
                .with_description("Aggregate graph statistics")
                .with_mime_type("application/json")
                .no_annotation(),
            RawResource::new("iwe://config", "config")
                .with_description("Project configuration: markdown options, templates, actions")
                .with_mime_type("application/json")
                .no_annotation(),
        ];

        for key in graph.keys().iter().take(100) {
            let title = (&*graph)
                .get_key_title(key)
                .unwrap_or_else(|| key.to_string());
            resources.push(
                RawResource::new(format!("iwe://documents/{}", key), title)
                    .with_mime_type("text/markdown")
                    .no_annotation(),
            );
        }

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri = &request.uri;
        let graph = self.graph.lock().await;

        if uri == "iwe://tree" {
            let paths = graph.paths();
            let mut root_keys: Vec<Key> = paths
                .iter()
                .filter(|n| n.ids().len() == 1)
                .filter_map(|n| n.first_id())
                .map(|id| (&*graph).node(id).node_key())
                .collect();
            root_keys.sort();
            root_keys.dedup();

            let mut trees: Vec<TreeNode> = Vec::new();
            for root_key in &root_keys {
                let mut visited: HashSet<Key> = HashSet::new();
                if let Some(node) = build_tree_node(&graph, root_key, 4, &mut visited) {
                    trees.push(node);
                }
            }
            let json = serde_json::to_string_pretty(&trees)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                json,
                uri.clone(),
            )]));
        }

        if uri == "iwe://stats" {
            let stats = GraphStatistics::from_graph(&graph);
            let json = serde_json::to_string_pretty(&stats)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                json,
                uri.clone(),
            )]));
        }

        if uri == "iwe://config" {
            let config_view = ConfigResource::from_config(&self.config, self);
            let json = serde_json::to_string_pretty(&config_view)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                json,
                uri.clone(),
            )]));
        }

        if let Some(key_str) = uri.strip_prefix("iwe://documents/") {
            let key = Key::name(key_str);
            let content = graph
                .get_document(&key)
                .ok_or_else(|| {
                    McpError::resource_not_found(
                        format!("Document '{}' not found", key_str),
                        None,
                    )
                })?
                .to_string();
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                content,
                uri.clone(),
            )]));
        }

        Err(McpError::resource_not_found(
            format!("Unknown resource: {}", uri),
            None,
        ))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![
                RawResourceTemplate::new("iwe://documents/{key}", "document")
                    .with_description("A document in the knowledge graph by key")
                    .with_mime_type("text/markdown")
                    .no_annotation(),
            ],
            next_cursor: None,
            meta: None,
        })
    }
}

impl IweServer {
    pub fn new(base_path: &str, configuration: &Configuration) -> Self {
        let path = PathBuf::from_str(base_path).expect("valid path");
        let state = new_for_path(&path);
        let graph = Graph::from_state(
            state,
            false,
            configuration.markdown.clone().into(),
            configuration.library.frontmatter_document_title.clone(),
        );
        Self {
            graph: Arc::new(Mutex::new(graph)),
            base_path: Some(path),
            config: configuration.clone(),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    pub fn from_documents(documents: Vec<(&str, &str)>) -> Self {
        Self::from_documents_with_config(documents, Configuration::default())
    }

    pub fn from_documents_with_config(documents: Vec<(&str, &str)>, config: Configuration) -> Self {
        let state = new_from_hashmap(
            documents
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<String, String>>(),
        );
        let graph = Graph::from_state(state, true, MarkdownOptions::default(), None);
        Self {
            graph: Arc::new(Mutex::new(graph)),
            base_path: None,
            config,
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    fn apply_changes(graph: &mut Graph, changes: &Changes) {
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

    fn write_file(&self, key: &Key, content: &str) {
        if let Some(base_path) = &self.base_path {
            let file_path = base_path.join(format!("{}.md", key));
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&file_path, content).ok();
        }
    }

    fn write_changes(&self, changes: &Changes) {
        if let Some(base_path) = &self.base_path {
            for key in &changes.removes {
                let file_path = base_path.join(format!("{}.md", key));
                if file_path.exists() {
                    std::fs::remove_file(&file_path).ok();
                }
            }
            for (key, markdown) in &changes.creates {
                let file_path = base_path.join(format!("{}.md", key));
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::write(&file_path, markdown).ok();
            }
            for (key, markdown) in &changes.updates {
                let file_path = base_path.join(format!("{}.md", key));
                std::fs::write(&file_path, markdown).ok();
            }
        }
    }

    pub fn start_watching(&self) {
        if let Some(base_path) = &self.base_path {
            watcher::start(self.graph.clone(), base_path.clone());
        }
    }

    fn render_key_template(&self, template: &str) -> String {
        let now = Local::now();
        let date_format = self
            .config
            .library
            .date_format
            .as_deref()
            .unwrap_or(DEFAULT_KEY_DATE_FORMAT);
        let formatted = now.format(date_format).to_string();
        Environment::new()
            .template_from_str(template)
            .expect("valid key template")
            .render(context! {
                today => formatted,
                now => formatted,
            })
            .expect("key template to render")
    }

    fn render_document_template(&self, template: &str, content: &str) -> String {
        let now = Local::now();
        let date_format = self
            .config
            .markdown
            .date_format
            .as_deref()
            .unwrap_or("%b %d, %Y");
        let formatted = now.format(date_format).to_string();
        Environment::new()
            .template_from_str(template)
            .expect("valid document template")
            .render(context! {
                today => formatted,
                now => formatted,
                content => content,
            })
            .expect("document template to render")
    }
}
