# Unique Features

IWE combines powerful knowledge management with developer-focused tooling, offering unique capabilities not found in other PKM solutions:

## Graph-based Transformations

- **Extract/embed notes operations**: Use LSP code actions to extract sections into separate notes or inline referenced content
- **Section-to-list and list-to-section conversions**: Transform document structure with a single click
- **Sub-sections extraction**: Break complex notes into manageable, linked components
- **Reference inlining**: Convert linked content to quotes or embed sections directly

## External Command Integration

- **Configurable command pipeline**: Connect to any CLI tool with custom templates
- **Block-level transformations**: Apply transformations to specific sections with full context awareness
- **Template-based input**: Customize command behavior for different content types
- **Context-aware processing**: Commands receive document structure and relationships

## Developer-Focused Architecture

- **Rust-powered performance**: Built with Rust for speed and reliability, handling thousands of files instantly
- **Shared core library**: CLI and LSP server share the same robust domain model
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
- **Flexible file organization**: Supports both flat Zettelkasten and hierarchical structures
- **Path-based navigation**: Multiple ways to reach the same content through different conceptual paths

## Cross-Editor LSP Integration

- **Native LSP support**: Works with VSCode, Neovim, Zed, Helix, and any LSP-compatible editor
- **Consistent experience**: Same features and performance across all editors
- **No vendor lock-in**: Switch editors without losing functionality

IWE also includes a comprehensive CLI utility for batch operations, document generation, and graph visualization.

The core differentiator is the shared library architecture between CLI and LSP components. This rich domain model enables easy construction of new graph transformations and ensures consistency across all interfaces. You can learn more in the [Data model](data-model.md) documentation.

## Detailed Comparisons

### IWE vs markdown-oxide

**markdown-oxide** is a solid PKM LSP server focused on basic knowledge management:

|Feature|IWE|markdown-oxide|
|-------|---|--------------|
|**Graph Transformations**|✅ Extract/embed sections, convert lists↔sections, inline references|❌ Basic linking only|
|**External Commands**|✅ Configurable CLI tools (supports AI agents, scripts, Unix tools)|❌ No external command features|
|**Performance**|✅ Rust-based, handles thousands of files instantly|✅ Good performance|
|**Batch Operations**|✅ CLI for bulk normalization and transformations|❌ LSP-only approach|
|**Editor Support**|✅ VSCode, Neovim, Zed, Helix|✅ VSCode, Neovim, Helix, Zed|
|**Auto-formatting**|✅ Comprehensive normalization on save|✅ Basic formatting|
|**Daily Notes**|✅ Supported using "attach" code action|✅ Dedicated daily notes support|
|**Backlinks**|✅ Via graph processing|✅ Native backlink support|

**IWE's advantage**: Advanced graph operations, external command integration, and comprehensive CLI tooling make it superior for complex knowledge work and developer workflows.

### IWE vs Obsidian

**Obsidian** is a popular GUI-based PKM tool with strong visualization capabilities and an extensive plugin ecosystem:

|Feature|IWE|Obsidian|
|-------|---|--------|
|**Editor Integration**|✅ Works with your preferred text editor (VSCode, Neovim, Zed, Helix)|❌ Proprietary editor only|
|**Cost**|✅ Completely free and open source|⚠️ Free for personal use, $8/month for sync, $16/month for publishing|
|**Performance**|✅ Rust-powered, instant operations on thousands of files|⚠️ Electron-based, can be slower with large vaults|
|**Graph Transformations**|✅ Automated extract/embed operations, section-to-list conversions|❌ Manual linking and organization|
|**External Commands**|✅ Configurable CLI tools (supports AI agents, scripts, Unix tools)|⚠️ Limited, requires third-party plugins (Text Generator, Smart Connections)|
|**Block References**|✅ Native support with automatic linking|⚠️ Available via plugins|
|**Auto-formatting**|✅ Comprehensive markdown normalization on save|⚠️ Basic formatting, requires plugins for advanced normalization|
|**Batch Operations**|✅ CLI for bulk transformations and normalization|❌ No batch operation capabilities|
|**Cross-platform**|✅ Consistent across all platforms|✅ Good cross-platform support|
|**Graph Visualization**|⚠️ CLI-based dot export (can generate visual graphs)|✅ Interactive graph view with customizable styling|
|**Plugin Ecosystem**|⚠️ Limited to LSP capabilities and CLI extensions|✅ Rich plugin marketplace with 1000+ community plugins|
|**Learning Curve**|⚠️ Requires basic terminal knowledge and editor setup|✅ GUI-friendly with intuitive interface|
|**Sync & Collaboration**|✅ Git-based sync (free), works with any Git hosting|⚠️ Obsidian Sync ($8/month) or manual Git setup|
|**Publishing**|✅ Export to various formats via CLI|⚠️ Obsidian Publish ($16/month) or manual export|
|**Mobile Support**|❌ Desktop/terminal only|✅ Native mobile apps with sync|

#### When to Choose IWE

- **Developer workflows**: You want to stay in your preferred code editor
- **Large repositories**: You need instant performance with thousands of markdown files
- **Automation-heavy workflows**: You want CLI-powered transformations and batch operations
- **Cost-conscious**: You want a completely free, open-source solution
- **Technical users**: You're comfortable with CLI tools and LSP setup
- **Git-based workflows**: You prefer version control over proprietary sync

#### When to Choose Obsidian

- **GUI preference**: You prefer visual interfaces over terminal-based tools
- **Plugin ecosystem**: You want access to hundreds of community plugins
- **Mobile access**: You need to access notes on phones/tablets
- **Interactive visualization**: You want to explore your knowledge graph visually
- **Non-technical users**: You want a user-friendly setup without terminal configuration
- **Rich formatting**: You need advanced formatting, canvas features, or embedded media

**Key Philosophical Difference**: IWE is editor-agnostic and developer-focused, designed to integrate with existing technical workflows. Obsidian is a comprehensive PKM environment with its own ecosystem, better suited for users who want an all-in-one solution with visual interfaces and extensive customization through plugins.

### IWE vs zk.nvim/telekasten.nvim

**zk.nvim** and **telekasten.nvim** are Neovim-specific Zettelkasten solutions:

|Feature|IWE|zk.nvim/telekasten|
|-------|---|------------------|
|**Editor Support**|✅ VSCode, Neovim, Zed, Helix, others|❌ Neovim only|
|**Graph Transformations**|✅ Automated extract/embed, structural changes|❌ Basic note creation and linking|
|**External Commands**|✅ Configurable CLI tools (supports AI agents, scripts, Unix tools)|❌ Manual workflows only|
|**Performance**|✅ Rust-powered LSP|⚠️ Lua-based, editor-dependent|
|**Batch Operations**|✅ CLI for bulk operations|❌ One-note-at-a-time workflow|
|**Auto-formatting**|✅ Built-in normalization|❌ Requires external tools|
|**Note Templates**|✅ Note templates supported via "Attach" command|✅ Static templates|
|**Search Integration**|✅ LSP-based with any picker|✅ Telescope/fzf integration|
|**Installation**|✅ Single binary + editor extension|⚠️ Complex Neovim plugin setup|

**IWE's advantage**: Works across all editors, provides powerful automation, and offers external command integration. zk.nvim/telekasten are better for Neovim purists who prefer simple, manual workflows.

## Why Choose IWE?

IWE is the **only tool** that combines:

- 🚀 **Performance**: Rust-powered speed that handles thousands of files instantly
- 🤖 **Extensibility**: External command integration with contextual templates for enhanced workflows
- 🔧 **Flexibility**: Works with VSCode, Neovim, Zed, Helix, and any LSP-compatible editor
- ⚡ **Power**: Advanced graph transformations and batch operations
- 👨‍💻 **Developer Focus**: CLI + LSP architecture designed for technical workflows

IWE is powerful enough for complex knowledge work, fast enough for large repositories, and flexible enough to adapt to any workflow or editor preference. Whether you're a researcher managing thousands of notes, a developer documenting complex systems, or a writer organizing interconnected ideas.
