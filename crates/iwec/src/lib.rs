pub mod watcher;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use chrono::Local;
use diwe::config::{
    ActionDefinition, CompletionOptions, Configuration, MarkdownOptions, NoteTemplate,
    DEFAULT_KEY_DATE_FORMAT,
};
use diwe::find::{DocumentFinder, FindOptions, FindOutput};
use diwe::fs::{new_for_path, new_from_hashmap};
use diwe::retrieve::{DocumentReader, RetrieveOptions, RetrieveOutput};
use diwe::stats::{GraphStatistics, KeyStatistics};
use diwe::tokens::Truncation;
use liwe::graph::{Graph, GraphContext};
use liwe::model::node::NodePointer;
use liwe::model::tree::{Tree, TreeIter};
use liwe::model::{strip_doc_extension, Key};
use liwe::operations::{
    attach_reference, delete as op_delete, extract as op_extract, inline as op_inline, references,
    rename as op_rename, sections, select_reference, select_section, AttachTarget, Changes,
    ExtractConfig, InlineConfig, OperationError, SelectError,
};
use liwe::query::cli::parse_projection;
use liwe::query::{
    self, execute, parse_operation, strict_guard_violations, Filter, InclusionAnchor, Operation,
    OperationKind, Outcome, ProjectionBase,
};
use minijinja::{context, Environment};
use rmcp::handler::server::router::prompt::PromptRouter;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::schemars::JsonSchema;
use rmcp::service::RequestContext;
use rmcp::{prompt, prompt_handler, prompt_router, tool, tool_router, RoleServer};
use rmcp::{tool_handler, ErrorData as McpError, ServerHandler};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

fn to_json_result<T: Serialize>(output: &T) -> Result<CallToolResult, McpError> {
    let json =
        serde_json::to_string(output).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

#[derive(Serialize)]
struct TruncationNote<'a> {
    truncated: bool,
    emitted: usize,
    matched: usize,
    clipped: &'a [String],
    tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    budget: Option<usize>,
    hint: &'static str,
}

fn to_json_result_with_truncation<T: Serialize>(
    output: &T,
    truncation: &Truncation,
) -> Result<CallToolResult, McpError> {
    let json =
        serde_json::to_string(output).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let mut blocks = vec![Content::text(json)];
    if truncation.is_truncated() {
        blocks.push(Content::text(truncation_note(truncation)));
    }
    Ok(CallToolResult::success(blocks))
}

fn truncation_note(truncation: &Truncation) -> String {
    let note = TruncationNote {
        truncated: true,
        emitted: truncation.emitted,
        matched: truncation.matched,
        clipped: &truncation.clipped,
        tokens: truncation.tokens,
        budget: truncation.budget,
        hint: "Output was bounded. To see more, narrow the query, raise max_tokens/limit/max_document_tokens, or re-run excluding the returned keys.",
    };
    serde_json::to_string(&note).unwrap_or_else(|_| "{\"truncated\":true}".to_string())
}

fn to_json_result_with_warnings<T: Serialize>(
    output: &T,
    warnings: &[String],
) -> Result<CallToolResult, McpError> {
    let json =
        serde_json::to_string(output).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let mut blocks = vec![Content::text(json)];
    for warning in warnings {
        blocks.push(Content::text(format!("warning: {}", warning)));
    }
    Ok(CallToolResult::success(blocks))
}

fn to_text_result(text: String) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum KeyDepthParam {
    Bare(String),
    Qualified { key: String, depth: Option<u8> },
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
    #[schemars(description = "Exclude candidates that are sub-documents of ANY listed key (NOT).")]
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
            conjuncts.push(Filter::Nor(vec![Filter::IncludedBy(Box::new(
                kd.anchor(self.max_depth),
            ))]));
        }
        Some(Filter::And(conjuncts))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindParams {
    #[schemars(description = "Fuzzy match on document title and key")]
    pub fuzzy: Option<String>,
    #[schemars(description = "Lexical (BM25) full-text match on title and body")]
    pub lexical: Option<String>,
    #[schemars(description = "Only return documents that reference this key")]
    pub refs_to: Option<String>,
    #[schemars(description = "Only return documents referenced by this key")]
    pub refs_from: Option<String>,
    #[schemars(
        description = "Maximum number of results to return. Unlimited if omitted (0 also = unlimited)."
    )]
    pub limit: Option<usize>,
    #[schemars(
        description = "Cap total projected `$content` tokens across all results. Unlimited if omitted (0 also = unlimited)."
    )]
    pub max_tokens: Option<usize>,
    #[schemars(
        description = "Cap projected `$content` tokens per result. Unlimited if omitted (0 also = unlimited)."
    )]
    pub max_document_tokens: Option<usize>,
    #[schemars(
        description = "Replacement projection (e.g. 'title,priority' or 'body=$content,parents=$includedBy'). Mutually exclusive with add_fields."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "Additive projection: same grammar as project, extends defaults rather than replacing. Mutually exclusive with project."
    )]
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
                parse_projection(s, ProjectionBase::Empty)
                    .map_err(|e| McpError::invalid_params(e, None))?,
            ),
            (None, Some(s)) => Some(
                parse_projection(s, ProjectionBase::Document)
                    .map_err(|e| McpError::invalid_params(e, None))?,
            ),
            (None, None) => None,
        };
        Ok(FindOptions {
            fuzzy: p.fuzzy,
            lexical: p.lexical,
            refs_to: p.refs_to.map(|k| Key::name(&k)),
            refs_from: p.refs_from.map(|k| Key::name(&k)),
            filter: p.selector.to_filter(),
            limit: p.limit,
            sort: None,
            project,
            max_tokens: p.max_tokens,
            max_document_tokens: p.max_document_tokens,
        })
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ExpandParams {
    #[schemars(description = "Levels of inclusion descendants to pull in (0 = unbounded).")]
    pub includes: Option<u64>,
    #[schemars(description = "Levels of inclusion ancestors to pull in (0 = unbounded).")]
    #[serde(rename = "includedBy")]
    pub included_by: Option<u64>,
    #[schemars(description = "Hops of outbound reference links to follow (0 = unbounded).")]
    pub references: Option<u64>,
    #[schemars(description = "Hops of inbound reference links to follow (0 = unbounded).")]
    #[serde(rename = "referencedBy")]
    pub referenced_by: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RetrieveParams {
    #[schemars(
        description = "Document keys to retrieve, or the candidate set searched within when `search`/`fuzzy` is present. Can be empty when a structural selector is provided."
    )]
    #[serde(default)]
    pub keys: Vec<String>,
    #[schemars(
        description = "Seed search: BM25 full-text query over title and body. Present → the tool searches the candidate set and reads the ordered seeds."
    )]
    pub search: Option<String>,
    #[schemars(
        description = "Seed search: fuzzy query over title and key. Fuses with `search` (RRF)."
    )]
    pub fuzzy: Option<String>,
    #[schemars(
        description = "Expansion directions to follow out from each seed: object over includes / includedBy / references / referencedBy → integer depths (0 = unbounded, omitted = not followed). Expansion is doc-only when omitted."
    )]
    pub expand: Option<ExpandParams>,
    #[schemars(description = "DEPRECATED: use `expand: { includes: N }`.")]
    pub depth: Option<u8>,
    #[schemars(description = "DEPRECATED: use `expand: { includedBy: N }`.")]
    pub context: Option<u8>,
    #[schemars(description = "DEPRECATED: use `expand: { references: 1 }`.")]
    pub links: Option<bool>,
    #[schemars(description = "Include incoming inline references. Default: true")]
    pub backlinks: Option<bool>,
    #[schemars(description = "Document keys to exclude from results")]
    pub exclude: Option<Vec<String>>,
    #[schemars(
        description = "Populate the `includes` array with child document edges. Default: false"
    )]
    pub children: Option<bool>,
    #[schemars(
        description = "Cap the number of seed documents kept before expansion — top-N by relevance when searching, the first N of the selection otherwise. Unlimited if omitted (0 also = unlimited)."
    )]
    pub limit: Option<usize>,
    #[schemars(
        description = "Cap the number of documents returned after expansion, trimming periphery documents first. Unlimited if omitted (0 also = unlimited)."
    )]
    pub max_documents: Option<usize>,
    #[schemars(
        description = "Cap total content tokens across all documents. Unlimited if omitted (0 also = unlimited)."
    )]
    pub max_tokens: Option<usize>,
    #[schemars(
        description = "Cap content tokens per document. Unlimited if omitted (0 also = unlimited)."
    )]
    pub max_document_tokens: Option<usize>,
    #[serde(flatten)]
    pub selector: SelectorParams,
}

impl RetrieveParams {
    fn searching(&self) -> bool {
        self.search.is_some() || self.fuzzy.is_some()
    }

    fn validate_expand(&self) -> Result<(), String> {
        if self.expand.is_some()
            && (self.depth.is_some() || self.context.is_some() || self.links.unwrap_or(false))
        {
            return Err(
                "`expand` cannot be combined with the deprecated `depth` / `context` / `links` aliases"
                    .to_string(),
            );
        }
        Ok(())
    }

    fn expansion(&self) -> (u32, u32, u32, u32) {
        use diwe::retrieve::expand_depth;
        if let Some(e) = &self.expand {
            return (
                e.includes.map(expand_depth).unwrap_or(0),
                e.included_by.map(expand_depth).unwrap_or(0),
                e.references.map(expand_depth).unwrap_or(0),
                e.referenced_by.map(expand_depth).unwrap_or(0),
            );
        }
        (
            self.depth.map(u32::from).unwrap_or(0),
            self.context.map(u32::from).unwrap_or(0),
            if self.links.unwrap_or(false) { 1 } else { 0 },
            0,
        )
    }

    fn base_options(&self) -> RetrieveOptions {
        let (includes, included_by, references, referenced_by) = self.expansion();
        RetrieveOptions {
            includes,
            included_by,
            references,
            referenced_by,
            backlinks: self.backlinks.unwrap_or(true),
            exclude: self
                .exclude
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(|k| Key::name(&k))
                .collect::<HashSet<_>>(),
            children: self.children.unwrap_or(false),
            filter: None,
            limit: self.limit,
            max_documents: self.max_documents,
            max_tokens: self.max_tokens,
            max_document_tokens: self.max_document_tokens,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TreeParams {
    #[schemars(
        description = "Starting document keys. If empty and no selector, shows all root documents."
    )]
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
    #[schemars(
        description = "Document key for per-document stats. Omit for aggregate graph statistics"
    )]
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
    #[schemars(
        description = "Explicit document key. Derive it from stable metadata (entity name, session date), not the title wording. Subdirectory keys allowed (e.g. people/ada); do not include a file extension. Omit to derive a slug from the title. Creation fails if a document with this key already exists."
    )]
    pub key: Option<String>,
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

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum QueryKind {
    Find,
    Count,
    Update,
    Delete,
}

impl From<QueryKind> for OperationKind {
    fn from(kind: QueryKind) -> Self {
        match kind {
            QueryKind::Find => OperationKind::Find,
            QueryKind::Count => OperationKind::Count,
            QueryKind::Update => OperationKind::Update,
            QueryKind::Delete => OperationKind::Delete,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryParams {
    #[schemars(
        description = "Operation kind: find (read documents), count (count documents), update (mutate frontmatter and/or blocks), or delete (remove documents)."
    )]
    pub operation: QueryKind,
    #[schemars(
        description = "The operation document as YAML. Uses the IWE query + block-selection language: `filter` (with $content block membership), `project`/`addFields` ($content narrowing, $blocks, $matches), `sort`, `limit` for reads; `filter` + `update` (with block operators $replace, $replaceText, $insertBefore, $insertAfter, $append, $delete) for update; `filter` + `expect` for delete. This surface is always strict: every mutating application must carry an `expect` guard (document-level `expect`, and one per block operator)."
    )]
    pub document: String,
    #[schemars(
        description = "Preview mutations without writing to disk (update/delete only). Default: false."
    )]
    #[serde(default)]
    pub dry_run: Option<bool>,
}

#[derive(Debug, Serialize)]
struct QueryUpdateOutput {
    dry_run: bool,
    changed: Vec<ChangeEntry>,
}

#[derive(Debug, Serialize)]
struct QueryCountOutput {
    count: usize,
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
            creates: c
                .creates
                .iter()
                .map(|(k, v)| ChangeEntry {
                    key: k.to_string(),
                    content: v.clone(),
                })
                .collect(),
            updates: c
                .updates
                .iter()
                .map(|(k, v)| ChangeEntry {
                    key: k.to_string(),
                    content: v.clone(),
                })
                .collect(),
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
    #[schemars(
        description = "List all sections with block numbers instead of extracting. Default: false"
    )]
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
    #[schemars(
        description = "Configured attach action(s) to attach to (e.g. 'today'). Pass one or more action names; the source is attached under each resolved target."
    )]
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
    fn from_config(config: &Configuration, server: &IweServer) -> Result<Self, String> {
        let actions = config
            .actions
            .iter()
            .map(|(name, action)| {
                let (action_type, title, target_key) = match action {
                    ActionDefinition::Transform(a) => ("transform", a.title.clone(), None),
                    ActionDefinition::Attach(a) => (
                        "attach",
                        a.title.clone(),
                        Some(server.render_key_template(&a.key_template)?),
                    ),
                    ActionDefinition::Sort(a) => ("sort", a.title.clone(), None),
                    ActionDefinition::Inline(a) => ("inline", a.title.clone(), None),
                    ActionDefinition::Extract(a) => ("extract", a.title.clone(), None),
                    ActionDefinition::ExtractAll(a) => ("extract_all", a.title.clone(), None),
                    ActionDefinition::Link(a) => ("link", a.title.clone(), None),
                };
                Ok(ActionResourceView {
                    name: name.clone(),
                    action_type: action_type.to_string(),
                    title,
                    target_key,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(Self {
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
        })
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
    #[tool(
        description = "Search and discover documents in the knowledge graph. Supports fuzzy text query (`query`), root filter (`roots`), direct-reference filters (`refs_to`, `refs_from`), and the structural set selector (`in` / `in_any` / `not_in` / `max_depth`) for transitive sub-document AND/OR/NOT queries with configurable depth."
    )]
    async fn iwe_find(
        &self,
        Parameters(params): Parameters<FindParams>,
    ) -> Result<CallToolResult, McpError> {
        let options: FindOptions = params.try_into()?;
        let graph = self.graph.lock().await;
        let index = (options.lexical.is_some() || options.fuzzy.is_some())
            .then(|| diwe::search_query::build_index(&graph, self.config.search_language()));
        let finder = match &index {
            Some(index) => DocumentFinder::with_index(&graph, index),
            None => DocumentFinder::new(&graph),
        };
        let output: FindOutput = finder.find(&options);
        to_json_result_with_truncation(&output.results, &output.truncation)
    }

    #[tool(
        description = "Retrieve documents from the knowledge graph. Reads the given `keys` (or search seeds) and expands the graph around them via `expand` (includes / includedBy / references / referencedBy). With `search`/`fuzzy`, seeds are found by relevance within the candidate set (keys + selector); `limit` caps the seeds before expansion and `max_documents` caps the documents returned after expansion."
    )]
    async fn iwe_retrieve(
        &self,
        Parameters(params): Parameters<RetrieveParams>,
    ) -> Result<CallToolResult, McpError> {
        params
            .validate_expand()
            .map_err(|e| McpError::invalid_params(e, None))?;
        let graph = self.graph.lock().await;
        let reader = DocumentReader::new(&graph);
        let mut options = params.base_options();

        let output: RetrieveOutput = if params.searching() {
            let key_filter = (!params.keys.is_empty()).then(|| {
                Filter::Key(query::KeyOp::In(
                    params.keys.iter().map(|k| Key::name(k)).collect(),
                ))
            });
            let selector_filter = params.selector.to_filter();
            let candidate_filter = match (selector_filter, key_filter) {
                (Some(a), Some(b)) => Some(Filter::And(vec![a, b])),
                (Some(f), None) | (None, Some(f)) => Some(f),
                (None, None) => None,
            };
            let candidates: Vec<Key> = match &candidate_filter {
                None => graph.keys(),
                Some(f) => query::evaluate(f, &graph),
            };
            let spec = query::SearchSpec::new(params.search.clone(), params.fuzzy.clone());
            let index = diwe::search_query::build_index(&graph, self.config.search_language());
            let seeds = diwe::search_query::ranked(&graph, &index, &candidates, &spec);
            reader.retrieve_many(&seeds, &options)
        } else {
            options.filter = params.selector.to_filter();
            let keys: Vec<Key> = params.keys.iter().map(|k| Key::name(k)).collect();
            reader.retrieve_many(&keys, &options)
        };
        to_json_result_with_truncation(&output.documents, &output.truncation)
    }

    #[tool(
        description = "View the hierarchical tree structure of the knowledge graph showing how documents are connected via block references. Supports the structural set selector (in / in_any / not_in / max_depth) — when provided, the tree roots are restricted to (or selected from) that set."
    )]
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
            let selector_set: HashSet<Key> = query::evaluate(&f, &graph).into_iter().collect();
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

    #[tool(
        description = "Get comprehensive statistics about the knowledge graph including document counts, reference patterns, broken links, and most connected documents"
    )]
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

    #[tool(
        description = "Expand all block references into a single flat markdown document. Useful for export or generating a complete view of a document tree"
    )]
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

    #[tool(
        description = "Create a new document from a title and optional content. Pass an explicit `key` to control the document's stable identity — derive it from stable metadata (entity name, session date), not the title wording; creation fails if that key already exists."
    )]
    async fn iwe_create(
        &self,
        Parameters(params): Parameters<CreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let key_name = match &params.key {
            Some(k) => {
                if strip_doc_extension(k) != k.as_str() {
                    return Err(McpError::invalid_params(
                        format!("Key '{}' must not include a file extension", k),
                        None,
                    ));
                }
                let key = Key::name(k);
                if key.relative_path.is_empty() {
                    return Err(McpError::invalid_params(
                        "Key must not be empty".to_string(),
                        None,
                    ));
                }
                key.to_string()
            }
            None => {
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

                if slug.is_empty() {
                    return Err(McpError::invalid_params(
                        "Title must contain at least one alphanumeric character".to_string(),
                        None,
                    ));
                }
                slug
            }
        };

        let content_body = params.content.unwrap_or_default();
        let markdown = if content_body.is_empty() {
            format!("# {}\n", params.title)
        } else {
            format!("# {}\n\n{}\n", params.title, content_body)
        };

        let key = Key::name(&key_name);
        let mut graph = self.graph.lock().await;

        if (&*graph).get_node_id(&key).is_some() {
            return Err(McpError::invalid_params(
                format!("Document '{}' already exists", key_name),
                None,
            ));
        }

        graph.insert_document(key.clone(), markdown.clone());
        self.write_file(&key, &markdown);

        #[derive(Serialize)]
        struct CreateResult {
            key: String,
        }
        to_json_result(&CreateResult { key: key_name })
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

    #[tool(
        description = "Delete a document from the knowledge graph. All block references and inline links to this document in other documents are cleaned up"
    )]
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

    #[tool(
        description = "Run an IWE query/block-selection operation document. `find` and `count` read; `update` mutates frontmatter and blocks (operators $replace, $replaceText, $insertBefore, $insertAfter, $append, $delete); `delete` removes documents. Membership uses the `$content` filter operator; reads project `$content` narrowing, `$blocks`, and `$matches`. Always strict: every mutating application must carry an `expect` guard (document-level `expect` plus one per block operator). Use `find` with `$blocks`/`$matches` to locate targets and learn counts before mutating."
    )]
    async fn iwe_query(
        &self,
        Parameters(params): Parameters<QueryParams>,
    ) -> Result<CallToolResult, McpError> {
        let kind: OperationKind = params.operation.into();
        let op = parse_operation(&params.document, kind)
            .map_err(|e| McpError::invalid_params(format!("invalid operation: {}", e), None))?;

        let violations = strict_guard_violations(&op);
        if !violations.is_empty() {
            return Err(McpError::invalid_params(
                format!(
                    "MCP block operations run strict: every mutating application must carry an `expect` guard; missing: {}. \
                     State the expected count — 1 for a precision edit, {{ min: 1 }} for a bulk edit that must match, {{ min: 0 }} when zero is acceptable.",
                    violations.join(", ")
                ),
                None,
            ));
        }

        let dry_run = params.dry_run.unwrap_or(false);
        let mut graph = self.graph.lock().await;

        let index = match &op {
            Operation::Find(find) if find.search.is_some() => Some(
                diwe::search_query::build_index(&graph, self.config.search_language()),
            ),
            _ => None,
        };

        match &op {
            Operation::Find(find) => {
                let outcome = diwe::search_query::execute(&op, &graph, index.as_ref())
                    .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let Outcome::Find { matches } = outcome else {
                    unreachable!("find operation yields a find outcome")
                };
                let documents: Vec<_> = matches.into_iter().map(|m| m.document).collect();
                let mut warnings = Vec::new();
                if let Some(spec) = &find.search {
                    if index
                        .as_ref()
                        .map(|idx| diwe::search_query::lexical_has_no_terms(idx, spec))
                        .unwrap_or(false)
                    {
                        warnings.push(diwe::search_query::no_terms_warning(spec));
                    }
                }
                to_json_result_with_warnings(&documents, &warnings)
            }
            Operation::Count(_) => {
                let outcome = execute(&op, &graph)
                    .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let Outcome::Count(count) = outcome else {
                    unreachable!("count operation yields a count outcome")
                };
                to_json_result(&QueryCountOutput { count })
            }
            Operation::Update(_) => {
                let outcome = execute(&op, &graph)
                    .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let Outcome::Update { changes } = outcome else {
                    unreachable!("update operation yields an update outcome")
                };
                let changed: Vec<ChangeEntry> = changes
                    .iter()
                    .map(|(key, content)| ChangeEntry {
                        key: key.to_string(),
                        content: content.clone(),
                    })
                    .collect();
                if !dry_run {
                    for (key, content) in &changes {
                        graph.update_document(key.clone(), content.clone());
                        self.write_file(key, content);
                    }
                }
                to_json_result(&QueryUpdateOutput { dry_run, changed })
            }
            Operation::Delete(_) => {
                let outcome = execute(&op, &graph)
                    .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
                let Outcome::Delete { removed } = outcome else {
                    unreachable!("delete operation yields a delete outcome")
                };
                let mut combined = Changes::default();
                for key in &removed {
                    let changes = op_delete(&graph, key).map_err(op_error_to_mcp)?;
                    combined.merge(changes);
                }
                if !dry_run {
                    Self::apply_changes(&mut graph, &combined);
                    self.write_changes(&combined);
                }
                to_json_result(&ChangesOutput::from(&combined))
            }
        }
    }

    #[tool(
        description = "Rename a document key. All block references and inline links across the entire graph are updated to point to the new key"
    )]
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

    #[tool(
        description = "Extract a section from a document into a new standalone document. The original section is replaced with a block reference. Use list mode to discover sections first"
    )]
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

        if params.list.unwrap_or(false) {
            let sections: Vec<SectionEntry> = sections(&tree)
                .into_iter()
                .map(|section| SectionEntry {
                    block_number: section.number,
                    title: section.title,
                })
                .collect();
            return to_json_result(&sections);
        }

        let section = match select_section(&tree, params.section.as_deref(), params.block) {
            Ok(section) => section,
            Err(SelectError::NotFound(query)) => {
                return Err(McpError::invalid_params(
                    format!("No section matches '{}'", query),
                    None,
                ))
            }
            Err(SelectError::Ambiguous(query, matches)) => {
                return Err(McpError::invalid_params(
                    format!(
                        "Multiple sections match '{}': {}",
                        query,
                        matches
                            .iter()
                            .map(|section| section.title.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    None,
                ))
            }
            Err(SelectError::OutOfRange(block, len)) => {
                return Err(McpError::invalid_params(
                    format!("Block number {} out of range (1-{})", block, len),
                    None,
                ))
            }
            Err(SelectError::NoSelector) => {
                return Err(McpError::invalid_params(
                    "Must specify section, block, or list",
                    None,
                ))
            }
        };

        let section_id = section.id;

        let config = ExtractConfig::default();
        let changes = op_extract(
            &graph,
            &source_key,
            section_id,
            &config,
            std::time::SystemTime::now(),
        )
        .map_err(op_error_to_mcp)?;

        if !params.dry_run.unwrap_or(false) {
            Self::apply_changes(&mut graph, &changes);
            self.write_changes(&changes);
        }

        to_json_result(&ChangesOutput::from(&changes))
    }

    #[tool(
        description = "Replace a block reference with the actual content of the referenced document. Use list mode to discover block references first"
    )]
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

        if params.list.unwrap_or(false) {
            let refs: Vec<ReferenceEntry> = references(&tree)
                .into_iter()
                .map(|reference| ReferenceEntry {
                    block_number: reference.number,
                    key: reference.key.to_string(),
                    title: reference.title,
                })
                .collect();
            return to_json_result(&refs);
        }

        let reference = match select_reference(&tree, params.reference.as_deref(), params.block) {
            Ok(reference) => reference,
            Err(SelectError::NotFound(query)) => {
                return Err(McpError::invalid_params(
                    format!("No reference matches '{}'", query),
                    None,
                ))
            }
            Err(SelectError::Ambiguous(query, matches)) => {
                return Err(McpError::invalid_params(
                    format!(
                        "Multiple references match '{}': {}",
                        query,
                        matches
                            .iter()
                            .map(|reference| reference.key.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    None,
                ))
            }
            Err(SelectError::OutOfRange(block, len)) => {
                return Err(McpError::invalid_params(
                    format!("Block number {} out of range (1-{})", block, len),
                    None,
                ))
            }
            Err(SelectError::NoSelector) => {
                return Err(McpError::invalid_params(
                    "Must specify reference, block, or list",
                    None,
                ))
            }
        };

        let ref_id = reference.id;

        let inline_type = if params.as_quote.unwrap_or(false) {
            diwe::config::InlineType::Quote
        } else {
            diwe::config::InlineType::Section
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

    #[tool(
        description = "Normalize all document formatting across the knowledge graph. Re-parses and re-writes all documents to ensure consistent formatting"
    )]
    async fn iwe_normalize(&self) -> Result<CallToolResult, McpError> {
        let graph = self.graph.lock().await;
        let state = graph.export();
        let original_count = state.len();

        let mut changed = 0usize;
        if self.base_path.is_some() {
            for (key_str, normalized_content) in &state {
                let key = Key::name(key_str);
                if self.read_file(&key).as_deref() != Some(normalized_content.as_str()) {
                    self.write_file(&key, normalized_content);
                    changed += 1;
                }
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

    #[tool(
        description = "Attach a document as a block reference in one or more target documents determined by configured attach actions. Each target key is derived from the action's key_template (e.g. daily/{{today}}). The `to` field accepts a list of action names; the source is attached under each resolved target. Targets that already contain the source are silently skipped. Use list mode to discover available attach actions."
    )]
    async fn iwe_attach(
        &self,
        Parameters(params): Parameters<AttachParams>,
    ) -> Result<CallToolResult, McpError> {
        if params.list.unwrap_or(false) {
            let mut entries: Vec<AttachActionEntry> = Vec::new();
            for (name, action) in &self.config.actions {
                if let ActionDefinition::Attach(attach) = action {
                    let target_key =
                        self.render_key_template(&attach.key_template)
                            .map_err(|e| {
                                McpError::invalid_params(format!("action '{}': {}", name, e), None)
                            })?;
                    entries.push(AttachActionEntry {
                        name: name.clone(),
                        title: attach.title.clone(),
                        target_key,
                    });
                }
            }
            return to_json_result(&entries);
        }

        if params.to.is_empty() {
            return Err(McpError::invalid_params(
                "'to' is required when not in list mode (pass one or more action names)"
                    .to_string(),
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

            let target_key = Key::name(&self.render_key_template(&attach.key_template).map_err(
                |e| McpError::invalid_params(format!("action '{}': {}", action_name, e), None),
            )?);

            match attach_reference(&graph, &target_key, &source_key, &reference_text) {
                AttachTarget::AlreadyAttached => continue,
                AttachTarget::Update(content) => {
                    combined.add_update(target_key.clone(), content);
                }
                AttachTarget::Create(body) => {
                    let document = self
                        .render_document_template(&attach.document_template, &body)
                        .map_err(|e| {
                            McpError::invalid_params(
                                format!("action '{}': {}", action_name, e),
                                None,
                            )
                        })?;
                    combined.add_create(target_key.clone(), document);
                }
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

#[prompt_router]
impl IweServer {
    #[prompt(
        name = "explore",
        description = "Start exploring the knowledge graph. Provides an overview of size, structure, root entry points, broken links, and orphaned documents"
    )]
    async fn explore(&self) -> Result<GetPromptResult, McpError> {
        let graph = self.graph.lock().await;
        let stats = GraphStatistics::from_graph(&graph);
        let stats_json = serde_json::to_string_pretty(&stats).unwrap_or_else(|_| "{}".to_string());

        let messages = vec![PromptMessage::new_text(
            PromptMessageRole::User,
            format!(
                "Here is an overview of the IWE knowledge graph.\n\n## Statistics\n\n```json\n{}\n```\n\nExplore the graph using iwe_retrieve to read documents, iwe_find to search, and iwe_tree to navigate the structure.",
                stats_json
            ),
        )];

        Ok(GetPromptResult::new(messages).with_description("Overview of the IWE knowledge graph"))
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
                includes: 2,
                included_by: 2,
                backlinks: true,
                ..Default::default()
            },
        );
        let json =
            serde_json::to_string_pretty(&output.documents).unwrap_or_else(|_| "[]".to_string());

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
                includes: 3,
                included_by: 1,
                backlinks: true,
                ..Default::default()
            },
        );
        let json =
            serde_json::to_string_pretty(&output.documents).unwrap_or_else(|_| "[]".to_string());

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
            let config_view = ConfigResource::from_config(&self.config, self)
                .map_err(|e| McpError::internal_error(e, None))?;
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
                    McpError::resource_not_found(format!("Document '{}' not found", key_str), None)
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
        let state = new_for_path(&path, configuration.format);
        let graph = Graph::from_state(
            &state,
            false,
            configuration.format_options(),
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
        let graph = Graph::from_state(&state, true, MarkdownOptions::default(), None);
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
            let extension = self.config.format.extension();
            let file_path = base_path.join(format!("{}.{}", key, extension));
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&file_path, content).ok();
        }
    }

    fn read_file(&self, key: &Key) -> Option<String> {
        let base_path = self.base_path.as_ref()?;
        let extension = self.config.format.extension();
        let file_path = base_path.join(format!("{}.{}", key, extension));
        std::fs::read_to_string(&file_path).ok()
    }

    fn write_changes(&self, changes: &Changes) {
        if let Some(base_path) = &self.base_path {
            let _ = diwe::fs::apply_changes(changes, base_path, self.config.format);
        }
    }

    pub fn start_watching(&self) {
        if let Some(base_path) = &self.base_path {
            watcher::start(self.graph.clone(), base_path.clone(), self.config.format);
        }
    }

    fn render_key_template(&self, template: &str) -> Result<String, String> {
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
            .map_err(|e| format!("invalid key template: {}", e))?
            .render(context! {
                today => formatted,
                now => formatted,
            })
            .map_err(|e| format!("key template rendering failed: {}", e))
    }

    fn render_document_template(&self, template: &str, content: &str) -> Result<String, String> {
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
            .map_err(|e| format!("invalid document template: {}", e))?
            .render(context! {
                today => formatted,
                now => formatted,
                content => content,
            })
            .map_err(|e| format!("document template rendering failed: {}", e))
    }
}
