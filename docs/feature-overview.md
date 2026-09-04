# Feature Overview

IWE turns a directory of markdown files into a knowledge graph — a connected structure you browse from your editor and your AI queries from the command line. This page is the catalogue: the capabilities that do not exist in a markdown language server at all, and where each one is documented.

For how IWE stacks up against marksman, markdown-oxide, Obsidian, zk, and mdbase — including where IWE is weaker — see [How IWE Compares](comparison.md).

## Graph Transformations

- **[Extract](feature-extract.md) and [inline](feature-inline.md) notes**: LSP code actions that split a section into its own document or merge a referenced document back in, rewriting every affected link
- **Section-to-list and list-to-section conversions**: restructure a document without retyping it
- **Sub-section extraction**: break a long note into linked components, one action at a time
- **Reference inlining**: turn a link into a quote, or embed the target section directly

## Query Language

- **MongoDB-style YAML filters**: match documents by frontmatter with `$eq`, `$in`, `$gt`, `$exists`, `$all`, and the rest of the operator set
- **Graph operators**: `$includes`, `$includedBy`, `$references`, `$referencedBy` walk inclusion and reference edges at bounded or unbounded depth
- **Bulk mutations**: `iwe update` and `iwe delete` apply the same filters across the whole workspace with `$set` / `$unset`
- **Projection, sort, and limit**: shape results as JSON, YAML, or Markdown for piping into other tools

See [Query Language](query-language.md) and the [formal specification](spec.md).

## MCP Server for AI Agents

- **14 native tools**: agents call `iwe_find`, `iwe_retrieve`, `iwe_create`, `iwe_extract`, `iwe_normalize`, and more over the Model Context Protocol
- **Built-in prompts**: `explore`, `review`, and `refactor` guide agents through common workflows
- **Resources**: documents, tree, stats, and config exposed as MCP resources
- **File watching**: the in-memory graph stays in sync with editor edits — no restart needed
- **Dry-run support**: every write and refactoring tool can preview its changes before applying

See [MCP Server](mcp.md).

## Search and Ranking

- **Titles and path context, not filenames**: a note called `20260903-a7f.md` is found by its heading and its position in the graph
- **Two rankers, fused**: skim fuzzy matching over title and key, BM25 lexical relevance over title and body, combined with Reciprocal Rank Fusion
- **One ranking everywhere**: the CLI, the LSP workspace symbols, and the MCP tools read the same index
- **Incremental**: the index is maintained per edit, so long editor sessions stay current

See [Notes Search](feature-search.md) and [Search Ranking](search-ranking.md).

## Schema Inference

- **Automatic frontmatter analysis**: `iwe schema` reports field names, type distributions, coverage, and value breakdowns across the workspace
- **Filter-aware**: inspect the schema of any subset of documents using the query language
- **JSON/YAML output**: pipe schema data into automation, dashboards, or validation pipelines

See [IWE Schema](cli-schema.md) and [Document Schema](document-schema.md).

## External Command Integration

- **Configurable command pipeline**: connect any CLI tool through a custom template
- **Block-level transformations**: apply a command to one section with the surrounding structure as context
- **Template-based input**: tune command behavior per content type

See [Custom Text Commands](feature-ai-commands.md).

## Markdown Normalization

- **Batch operations**: normalize thousands of files in under a second
- **Auto-formatting on save**: link titles, header levels, list numbering, and wrapping fixed in place
- **Header hierarchy management**: keep document structure consistent as it grows
- **Link title synchronization**: keep link text in step with the target document's title

See [Text Structure Normalization](feature-auto-format.md) and [Header Levels Normalization](feature-normalization.md).

## Hierarchical Notes

- **[Inclusion links](inclusion-links.md)**: the same note can belong to several parents without copying the file
- **Context-aware search**: results carry the note's position in the graph, not just its name
- **[Inlay hints](feature-inlay-hints.md)**: see parent context without leaving the document you are reading
- **[Hover preview](feature-hover-preview.md)**: inspect a linked note inline, frontmatter stripped
- **Flexible organization**: flat Zettelkasten and deep directory trees both work
- **Path-based navigation**: reach the same content through different conceptual paths

## Cross-Editor LSP Integration

- **Native LSP**: documented setup for [VS Code](vscode.md), [Neovim](neovim.md), [Zed](zed.md), and [Helix](helix.md); any LSP-compatible editor can be wired up
- **Consistent behavior**: the same features and the same performance in every client
- **No lock-in**: switch editors without losing anything

## Developer-Focused Architecture

- **Rust core**: [20,000 files in under a second](benchmark.md)
- **Shared core library**: the CLI, the LSP server, and the MCP server are thin shells over one domain model, so a new graph operation appears in all three at once
- **Rich graph processing**: the transformations above are graph traversals, not text munging
- **Cross-platform**: identical behavior on every supported operating system

The shared library is the reason the rest of this page is possible. See [Data Model](data-model.md) for how documents become a graph.
