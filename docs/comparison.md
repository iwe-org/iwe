# Unique Features

IWE combines powerful knowledge management with developer-focused tooling, offering unique capabilities not found in other PKM solutions:

## Graph-based Transformations

- **Extract/embed notes operations**: Use LSP code actions to extract sections into separate notes or inline referenced content
- **Section-to-list and list-to-section conversions**: Transform document structure with a single click
- **Sub-sections extraction**: Break complex notes into manageable, linked components
- **Reference inlining**: Convert linked content to quotes or embed sections directly

## Query Language

- **MongoDB-style YAML filters**: Match documents by frontmatter fields with `$eq`, `$in`, `$gt`, `$exists`, `$all`, and other operators
- **Graph operators**: `$includes`, `$includedBy`, `$references`, `$referencedBy` walk inclusion and reference edges with bounded or unbounded depth
- **Bulk mutations**: `iwe update` and `iwe delete` apply filters across the whole workspace with `$set`/`$unset` operators
- **Projection, sort, and limit**: Shape results as JSON, YAML, or Markdown for piping into other tools

## MCP Server for AI Agents

- **13 native tools**: AI agents call `iwe_find`, `iwe_retrieve`, `iwe_create`, `iwe_extract`, `iwe_normalize`, and more via the Model Context Protocol
- **Built-in prompts**: `explore`, `review`, and `refactor` prompts guide agents through common workflows
- **Resources**: Documents, tree, stats, and config exposed as MCP resources
- **File watching**: In-memory graph stays in sync with editor edits — no restart needed
- **Dry-run support**: All write and refactoring tools preview changes before applying

## Schema Inference

- **Automatic frontmatter analysis**: `iwe schema` reports field names, type distributions, coverage, and value breakdowns across the workspace
- **Filter-aware**: Inspect schemas for any subset of documents using the query language
- **JSON/YAML output**: Pipe schema data into automation, dashboards, or validation pipelines

## External Command Integration

- **Configurable command pipeline**: Connect to any CLI tool with custom templates
- **Block-level transformations**: Apply transformations to specific sections with full context awareness
- **Template-based input**: Customize command behavior for different content types
- **Context-aware processing**: Commands receive document structure and relationships

## Developer-Focused Architecture

- **Rust-powered performance**: Built with Rust for speed and reliability, handling thousands of files instantly
- **Shared core library**: CLI, LSP server, and MCP server share the same robust domain model
- **Rich graph processing**: Advanced algorithms for document relationships and transformations
- **Cross-platform**: Works identically across all supported operating systems

## Advanced Markdown Normalization

- **Batch operations**: Normalize thousands of files in under a second
- **Auto-formatting on save**: Fix link titles, header levels, list numbering automatically
- **Header hierarchy management**: Maintain consistent document structure
- **Link title synchronization**: Keep link text in sync with target document titles

## Hierarchical Note Support

- **Context-aware search**: Find notes by understanding their position in the knowledge graph
- **Inlay hints**: See parent note context without leaving your current document
- **Hover preview**: Inspect linked notes inline via standard LSP hover, with frontmatter stripped
- **Flexible file organization**: Supports both flat Zettelkasten and hierarchical structures
- **Path-based navigation**: Multiple ways to reach the same content through different conceptual paths

## Cross-Editor LSP Integration

- **Native LSP support**: Works with VSCode, Neovim, Zed, Helix, and any LSP-compatible editor
- **Consistent experience**: Same features and performance across all editors
- **No vendor lock-in**: Switch editors without losing functionality

IWE also includes a comprehensive CLI utility for batch operations, document generation, querying, and graph visualization.

The core differentiator is the shared library architecture between CLI, LSP, and MCP components. This rich domain model enables easy construction of new graph transformations and ensures consistency across all interfaces. You can learn more in the [Data Model](data-model.md) documentation.

## Detailed Comparisons

### IWE vs markdown-oxide

**markdown-oxide** is an actively-maintained PKM Language Server for markdown:

| Feature                   | IWE                                                                  | markdown-oxide                                |
| ------------------------- | -------------------------------------------------------------------- | --------------------------------------------- |
| **Graph Transformations** | ✅ Extract/embed sections, convert lists↔sections, inline references  | ❌ Basic linking only                          |
| **Query Language**        | ✅ MongoDB-style filters over frontmatter and graph edges             | ❌ None                                        |
| **MCP Server**            | ✅ Native MCP server (`iwec`) with 13 tools                           | ❌ None                                        |
| **External Commands**     | ✅ Configurable CLI tools (supports AI agents, scripts, Unix tools)   | ❌ No external command features                |
| **Performance**           | ✅ Rust-based, handles thousands of files instantly                   | ✅ Rust-based, good performance                |
| **Batch Operations**      | ✅ CLI for bulk normalization, update, delete with filters            | ❌ LSP-only approach                           |
| **Editor Support**        | ✅ VSCode, Neovim, Zed, Helix                                         | ✅ Neovim, VSCode, Zed, Helix, Kakoune         |
| **Code Lens**             | ⚠️ Inlay hints for parent context                                    | ✅ Reference counts shown inline               |
| **Auto-formatting**       | ✅ Comprehensive normalization on save                                | ✅ Basic formatting                            |
| **Daily Notes**           | ✅ Configurable via `attach` code action with templates               | ✅ Dedicated daily notes with natural language |
| **Backlinks**             | ✅ Via graph processing                                               | ✅ Files, headings, and blocks                 |
| **Hover Preview**         | ✅ LSP hover with frontmatter stripping                               | ✅ Hover support                               |

**IWE's advantage**: Advanced graph operations, query language, MCP integration, and comprehensive CLI tooling make it superior for complex knowledge work, automation, and AI-augmented workflows.

### IWE vs Obsidian

**Obsidian** is a popular GUI-based PKM tool with strong visualization, an extensive plugin ecosystem, and recent additions like Bases (typed database views), Canvas, and Web Clipper:

| Feature                                   | IWE                                                                   | Obsidian                                                                     |
| ----------------------------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| **Editor Integration**                    | ✅ Works with your preferred text editor (VSCode, Neovim, Zed, Helix)  | ❌ Proprietary editor only                                                    |
| **Cost**                                  | ✅ Completely free and open source                                     | ⚠️ Free for personal use; paid Sync, Publish, and Enterprise tiers           |
| **Performance**                           | ✅ Rust-powered, instant operations on thousands of files              | ⚠️ Electron-based, can be slower with large vaults                           |
| **Graph Transformations**                 | ✅ Automated extract/embed operations, section-to-list conversions     | ❌ Manual linking and organization                                            |
| **Query Language**                        | ✅ MongoDB-style YAML over frontmatter and graph edges                 | ✅ Bases (typed database views) and Dataview plugin                           |
| **MCP / AI Integration**                  | ✅ Native MCP server with 13 tools, prompts, and resources             | ⚠️ Requires third-party plugins (Text Generator, Smart Connections, etc.)    |
| **External Commands**                     | ✅ Configurable CLI tools (supports AI agents, scripts, Unix tools)    | ⚠️ Limited, requires third-party plugins                                     |
| **[Inclusion Links](inclusion-links.md)** | ✅ Native support with automatic linking                               | ⚠️ Available via plugins                                                     |
| **Auto-formatting**                       | ✅ Comprehensive markdown normalization on save                        | ⚠️ Basic formatting, requires plugins for advanced normalization             |
| **Batch Operations**                      | ✅ CLI for bulk transformations, update, delete with filters           | ❌ No batch operation capabilities                                            |
| **Cross-platform**                        | ✅ Consistent across all platforms                                     | ✅ Good cross-platform support                                                |
| **Graph Visualization**                   | ⚠️ CLI-based dot export (can generate visual graphs)                  | ✅ Interactive graph view with customizable styling                           |
| **Canvas / Whiteboard**                   | ❌ Not supported                                                       | ✅ Native infinite canvas                                                     |
| **Web Clipper**                           | ❌ Not supported                                                       | ✅ Native browser extension                                                   |
| **Plugin Ecosystem**                      | ⚠️ Limited to LSP capabilities, CLI extensions, and MCP tools         | ✅ Thousands of community plugins                                             |
| **Learning Curve**                        | ⚠️ Requires basic terminal knowledge and editor setup                 | ✅ GUI-friendly with intuitive interface                                      |
| **Sync & Collaboration**                  | ✅ Git-based sync (free), works with any Git hosting                   | ⚠️ Obsidian Sync (paid) with shared vaults, or manual Git setup              |
| **Publishing**                            | ✅ Export to various formats via CLI                                   | ⚠️ Obsidian Publish (paid) or manual export                                  |
| **Mobile Support**                        | ❌ Desktop/terminal only                                               | ✅ Native mobile apps with sync                                               |

#### When to Choose IWE

- **Developer workflows**: You want to stay in your preferred code editor
- **Large repositories**: You need instant performance with thousands of markdown files
- **Automation-heavy workflows**: You want CLI-powered transformations and batch operations
- **AI-augmented workflows**: You want native MCP access for AI agents
- **Cost-conscious**: You want a completely free, open-source solution
- **Technical users**: You're comfortable with CLI tools and LSP setup
- **Git-based workflows**: You prefer version control over proprietary sync

#### When to Choose Obsidian

- **GUI preference**: You prefer visual interfaces over terminal-based tools
- **Plugin ecosystem**: You want access to thousands of community plugins
- **Mobile access**: You need to access notes on phones/tablets
- **Interactive visualization**: You want to explore your knowledge graph visually
- **Non-technical users**: You want a user-friendly setup without terminal configuration
- **Rich formatting**: You need advanced formatting, canvas features, or embedded media

**Key Philosophical Difference**: IWE is editor-agnostic and developer-focused, designed to integrate with existing technical workflows and AI agents. Obsidian is a comprehensive PKM environment with its own ecosystem, better suited for users who want an all-in-one solution with visual interfaces and extensive customization through plugins.

### IWE vs mdbase

**mdbase** is a specification (v0.2.1) for treating folders of markdown files as typed, queryable data collections. It has a TypeScript reference implementation and a Go library, but no LSP server, no editor extensions, and no CLI. It overlaps with IWE on querying and frontmatter typing — but not on graph transformations, editor integration, or AI tooling.

| Feature                   | IWE                                                                   | mdbase                                                              |
| ------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------- |
| **Project Maturity**      | ✅ Production tool: CLI, LSP, MCP shipping today                       | ⚠️ Pre-1.0 spec (v0.2.1) with reference library implementations     |
| **Editor Integration**    | ✅ LSP for VSCode, Neovim, Zed, Helix                                  | ❌ No LSP, no editor extensions                                      |
| **CLI**                   | ✅ `iwe` with find, retrieve, update, delete, normalize, schema, etc.  | ❌ Library-only (TypeScript / Go)                                    |
| **MCP / AI Integration**  | ✅ Native MCP server with 13 tools                                     | ❌ None                                                              |
| **Query Language**        | ✅ MongoDB-style filters over frontmatter and graph edges              | ✅ Expression language for filters, ordering, and link traversal     |
| **Frontmatter Schemas**   | ✅ Schema inference from existing documents (`iwe schema`)             | ✅ Explicit type definitions with validation and inheritance         |
| **Validation**            | ⚠️ Implicit via schema inference                                      | ✅ Configurable strictness (off / warn / error)                      |
| **Graph Transformations** | ✅ Extract, inline, attach, normalize, rename with link updates        | ⚠️ Rename updates references; no structural refactors               |
| **Inclusion Links**       | ✅ Native block-reference inclusion                                    | ❌ Wikilinks and markdown links only                                 |
| **Performance**           | ✅ Rust core, in-memory graph                                          | ⚠️ TypeScript reference impl with SQLite-backed cache               |
| **Batch Operations**      | ✅ `iwe update`/`delete` with filters                                  | ✅ `batchUpdate` / `batchDelete` library calls                       |
| **File Watching**         | ✅ Built into LSP and MCP servers                                      | ✅ Watch-mode simulation in reference impl                          |

**IWE's advantage**: A working LSP, MCP server, and CLI — not just a specification. Graph operations (extract, inline, attach) and inclusion links go beyond mdbase's flat document model. Schemas are inferred from existing content rather than authored upfront.

**mdbase's advantage**: Explicit, vendor-neutral schema definitions with strict validation are well-suited to teams that want to enforce frontmatter contracts across tools. If you primarily need typed records rather than a knowledge graph, mdbase's typed-collection model may fit better.

### IWE vs zk and telekasten.nvim

**zk** (zk-org) is a CLI-driven Zettelkasten tool with an LSP server and integrations for Neovim, VSCode, and Emacs. **telekasten.nvim** is a Neovim-only plugin with calendar, image paste, and Telescope integration:

| Feature                   | IWE                                                                | zk                                              | telekasten.nvim                |
| ------------------------- | ------------------------------------------------------------------ | ----------------------------------------------- | ------------------------------ |
| **Editor Support**        | ✅ VSCode, Neovim, Zed, Helix, any LSP-compatible editor            | ✅ Neovim, VSCode, Emacs via LSP                 | ❌ Neovim only                  |
| **Graph Transformations** | ✅ Automated extract/embed, structural changes                      | ❌ Basic note creation and linking               | ❌ Basic note creation          |
| **Query Language**        | ✅ MongoDB-style YAML filters over frontmatter and graph edges      | ⚠️ Filter flags for tags, links, mentions       | ❌ None                         |
| **MCP / AI Integration**  | ✅ Native MCP server with 13 tools                                  | ❌ None                                          | ❌ None                         |
| **External Commands**     | ✅ Configurable CLI tools (supports AI agents, scripts, Unix tools) | ⚠️ Aliases and shell automation                 | ❌ Manual workflows only        |
| **Performance**           | ✅ Rust-powered LSP                                                 | ✅ Go-based CLI                                  | ⚠️ Lua-based, editor-dependent |
| **Batch Operations**      | ✅ CLI for bulk operations with filters                             | ⚠️ Limited (notebook housekeeping)              | ❌ One-note-at-a-time workflow  |
| **Auto-formatting**       | ✅ Built-in normalization                                           | ❌ Requires external tools                       | ❌ Requires external tools      |
| **Note Templates**        | ✅ Note templates via `attach` command                              | ✅ Template-based note creation                  | ✅ Static templates             |
| **Daily / Periodic Notes**| ✅ Configurable via `attach` action                                 | ⚠️ Via templates                                | ✅ Daily/weekly/monthly/yearly  |
| **Search Integration**    | ✅ LSP-based with any picker                                        | ✅ Built-in `fzf` browser                        | ✅ Telescope/fzf integration    |
| **Calendar View**         | ❌ Not supported                                                    | ❌ Not supported                                 | ✅ Calendar with note highlights|
| **Image / Media Paste**   | ❌ Not supported                                                    | ❌ Not supported                                 | ✅ Clipboard image paste        |
| **Installation**          | ✅ Single binary + editor extension                                 | ✅ Single binary + LSP                           | ⚠️ Complex Neovim plugin setup |

**IWE's advantage**: Works across all editors, provides graph-level automation, query language, and native MCP integration. zk is a strong choice for Zettelkasten purists who want a CLI-first workflow with a simpler data model. telekasten.nvim is best for Neovim users who want a polished journaling experience with calendar and image-paste workflows.

## Why Choose IWE?

IWE is the **only tool** that combines:

- 🚀 **Performance**: Rust-powered speed that handles thousands of files instantly
- 🤖 **AI-Native**: Built-in MCP server gives AI agents structured access to your knowledge graph
- 🔍 **Query Language**: MongoDB-style filters over frontmatter and graph edges
- 🔧 **Flexibility**: Works with VSCode, Neovim, Zed, Helix, and any LSP-compatible editor
- ⚡ **Power**: Advanced graph transformations, batch operations, and schema inference
- 👨‍💻 **Developer Focus**: CLI + LSP + MCP architecture designed for technical workflows

IWE is powerful enough for complex knowledge work, fast enough for large repositories, and flexible enough to adapt to any workflow or editor preference. Whether you're a researcher managing thousands of notes, a developer documenting complex systems, or a writer organizing interconnected ideas — and now, whether you're working alongside AI agents that need structured access to your knowledge.
