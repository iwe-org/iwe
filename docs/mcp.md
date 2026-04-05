# MCP Server

IWE provides an MCP (Model Context Protocol) server that gives AI agents direct access to your knowledge graph. The MCP server exposes the same operations as the CLI — search, retrieve, create, refactor — through a standardized protocol that AI tools can use natively.

## Setup

The MCP server runs as a stdio process. You configure it by pointing your AI tool to the `iwec` binary and setting the working directory to your knowledge graph.

## Tools

The MCP server exposes 13 tools for reading, writing, and refactoring documents.

### Reading

| Tool           | Description                                                    |
| -------------- | -------------------------------------------------------------- |
| `iwe_find`     | Search documents with fuzzy matching and relationship filters  |
| `iwe_retrieve` | Fetch documents with depth expansion, parent context, backlinks |
| `iwe_tree`     | View hierarchical document structure                           |
| `iwe_stats`    | Get knowledge graph statistics and broken link reports         |
| `iwe_squash`   | Expand block references into a single flat document            |

### Writing

| Tool           | Description                                        |
| -------------- | -------------------------------------------------- |
| `iwe_create`   | Create a new document from title and content       |
| `iwe_update`   | Replace the full content of an existing document   |
| `iwe_delete`   | Delete a document and clean up all references      |

### Refactoring

| Tool             | Description                                                |
| ---------------- | ---------------------------------------------------------- |
| `iwe_rename`     | Rename a document key with automatic link updates          |
| `iwe_extract`    | Extract a section into a new document with block reference |
| `iwe_inline`     | Replace a block reference with the referenced content      |
| `iwe_normalize`  | Re-format all documents for consistent formatting          |
| `iwe_attach`     | Attach a document to a target using configured actions     |

All write and refactoring tools support a `dry_run` parameter to preview changes before applying them.

## Prompts

The server provides three built-in prompts that guide AI agents through common workflows:

| Prompt     | Description                                               |
| ---------- | --------------------------------------------------------- |
| `explore`  | Get an overview of the knowledge graph with key statistics |
| `review`   | Review a specific document with full context              |
| `refactor` | Analyze a document and suggest restructuring operations   |

## Resources

The server exposes knowledge graph data as MCP resources:

| URI                       | Description                            |
| ------------------------- | -------------------------------------- |
| `iwe://documents/{key}`   | Individual document content            |
| `iwe://tree`              | Full hierarchical document tree        |
| `iwe://stats`             | Aggregate knowledge graph statistics   |
| `iwe://config`            | Configuration with templates and actions |

## File watching

The MCP server watches the knowledge graph directory for changes. When you edit markdown files in your editor, the server automatically updates its in-memory graph. There is no need to restart the server after making changes.
