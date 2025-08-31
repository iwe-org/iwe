# Unique Features

IWE combines powerful knowledge management with developer-focused tooling, offering unique capabilities not found in other PKM solutions:

## Graph-based Transformations

- **Extract/embed notes operations**: Use LSP code actions to extract sections into separate notes or inline referenced content
- **Section-to-list and list-to-section conversions**: Transform document structure with a single click
- **Sub-sections extraction**: Break complex notes into manageable, linked components
- **Reference inlining**: Convert linked content to quotes or embed sections directly

## AI-Powered Contextual Commands

- **Configurable LLM integration**: Connect to any LLM provider with custom templates
- **Block-level AI actions**: Apply AI transformations to specific sections with full context awareness
- **Template-based prompts**: Customize AI behavior for different content types and use cases
- **Context-aware processing**: AI commands understand document structure and relationships

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
|**AI Integration**|✅ Configurable LLM with contextual templates|❌ No AI features|
|**Performance**|✅ Rust-based, handles thousands of files instantly|✅ Good performance|
|**Batch Operations**|✅ CLI for bulk normalization and transformations|❌ LSP-only approach|
|**Editor Support**|✅ VSCode, Neovim, Zed, Helix|✅ VSCode, Neovim, Helix, Zed|
|**Auto-formatting**|✅ Comprehensive normalization on save|✅ Basic formatting|
|**Daily Notes**|✅ Supported using "attach" code action|✅ Dedicated daily notes support|
|**Backlinks**|✅ Via graph processing|✅ Native backlink support|

**IWE's advantage**: Advanced graph operations, AI integration, and comprehensive CLI tooling make it superior for complex knowledge work and developer workflows.

### IWE vs Obsidian

**Obsidian** is a popular GUI-based PKM tool with strong visualization:

|Feature|IWE|Obsidian|
|-------|---|--------|
|**Editor Integration**|✅ Works with your preferred text editor|❌ Proprietary editor only|
|**Cost**|✅ Completely free and open source|⚠️ Free for personal use, $8/month for sync|
|**Performance**|✅ Rust-powered, instant operations|⚠️ Electron-based, can be slower|
|**Graph Transformations**|✅ Automated extract/embed operations|❌ Manual linking and organization|
|**AI Integration**|✅ Configurable LLM providers|⚠️ Limited, requires plugins|
|**Cross-platform**|✅ Consistent across all platforms|✅ Good cross-platform support|
|**Graph Visualization**|⚠️ CLI-based dot export|✅ Interactive graph view|
|**Plugin Ecosystem**|⚠️ Limited to LSP capabilities|✅ Rich plugin marketplace|
|**Learning Curve**|⚠️ Requires basic terminal knowledge|✅ GUI-friendly|

**IWE's advantage**: Better for developers who want to stay in their preferred editor, need powerful automation, or want completely free sync via Git. Obsidian is better for users who prefer GUIs and interactive visualizations.

### IWE vs zk.nvim/telekasten.nvim

**zk.nvim** and **telekasten.nvim** are Neovim-specific Zettelkasten solutions:

|Feature|IWE|zk.nvim/telekasten|
|-------|---|------------------|
|**Editor Support**|✅ VSCode, Neovim, Zed, Helix, others|❌ Neovim only|
|**Graph Transformations**|✅ Automated extract/embed, structural changes|❌ Basic note creation and linking|
|**AI Integration**|✅ Configurable LLM with templates|❌ Manual workflows only|
|**Performance**|✅ Rust-powered LSP|⚠️ Lua-based, editor-dependent|
|**Batch Operations**|✅ CLI for bulk operations|❌ One-note-at-a-time workflow|
|**Auto-formatting**|✅ Built-in normalization|❌ Requires external tools|
|**Note Templates**|✅ Note templates supported via "Attach" command|✅ Static templates|
|**Search Integration**|✅ LSP-based with any picker|✅ Telescope/fzf integration|
|**Installation**|✅ Single binary + editor extension|⚠️ Complex Neovim plugin setup|

**IWE's advantage**: Works across all editors, provides powerful automation, and offers AI-enhanced workflows. zk.nvim/telekasten are better for Neovim purists who prefer simple, manual workflows.

## Why Choose IWE?

IWE is the **only tool** that combines:

- 🚀 **Performance**: Rust-powered speed that handles thousands of files instantly
- 🤖 **Intelligence**: Integrated AI with contextual templates for enhanced workflows
- 🔧 **Flexibility**: Works with VSCode, Neovim, Zed, Helix, and any LSP-compatible editor
- ⚡ **Power**: Advanced graph transformations and batch operations
- 👨‍💻 **Developer Focus**: CLI + LSP architecture designed for technical workflows

IWE is powerful enough for complex knowledge work, fast enough for large repositories, and flexible enough to adapt to any workflow or editor preference. Whether you're a researcher managing thousands of notes, a developer documenting complex systems, or a writer organizing interconnected ideas.
