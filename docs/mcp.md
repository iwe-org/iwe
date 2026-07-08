# MCP Server

IWE provides an MCP (Model Context Protocol) server that gives AI agents direct access to your knowledge graph. The MCP server exposes the same operations as the CLI — search, retrieve, create, refactor — through a standardized protocol that AI tools can use natively.

## Setup

You configure the server by pointing your AI tool to the `iwec` binary and setting the working directory to your knowledge graph. By default it communicates over stdio, which is what most editor and agent integrations expect.

### Transport options

`iwec` accepts two flags that control how it serves the protocol:

| Flag                        | Default     | Description                                                  |
| --------------------------- | ----------- | ------------------------------------------------------------ |
| `--transport <stdio\|http>` | `stdio`     | Serve over stdio, or over HTTP                               |
| `--host <HOST>`             | `127.0.0.1` | Address to bind to (only used with `--transport http`)       |
| `--port <PORT>`             | `8000`      | Port to listen on (only used with `--transport http`)        |

With `--transport http` the server listens for Streamable HTTP connections at `http://<host>:<port>/mcp`:

```bash
iwec --transport http --port 8000
```

By default the HTTP server binds to `127.0.0.1`, so it only accepts connections from the local machine. To accept connections from other machines, bind to a reachable address:

```bash
iwec --transport http --host 0.0.0.0 --port 8000
```

The server speaks plain HTTP, so put a reverse proxy in front of it for TLS or authentication when exposing it beyond localhost.

## Tools

The MCP server exposes 14 tools for reading, writing, querying, and refactoring documents.

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

### Query

| Tool        | Description                                                          |
| ----------- | -------------------------------------------------------------------- |
| `iwe_query` | Run a [Query Language](query-language.md) operation document verbatim |

`iwe_query` takes an `operation` kind (`find`, `count`, `update`, or `delete`) and the operation `document` as a YAML string, plus an optional `dry_run` for the mutating kinds. It exposes the full query surface: frontmatter and graph filters, the `$content` block-membership operator, the `$content` / `$blocks` / `$matches` projection sources, and the block update operators (`$replace`, `$replaceText`, `$insertBefore`, `$insertAfter`, `$append`, `$delete`). `find` and `count` read; `update` applies frontmatter and block edits atomically per document; `delete` removes documents with reference cleanup.

The tool is **always strict**: every mutating application must carry an `expect` guard — the document-level `expect` on `update` / `delete`, plus one per block operator — or the operation is refused with the missing guards named. Use `find` with `$blocks` / `$matches` to locate targets and learn the counts before mutating. See [Strict mode](query-language.md#strict-mode).

### Refactoring

| Tool             | Description                                                |
| ---------------- | ---------------------------------------------------------- |
| `iwe_rename`     | Rename a document key with automatic link updates          |
| `iwe_extract`    | Extract a section into a new document with block reference |
| `iwe_inline`     | Replace a block reference with the referenced content      |
| `iwe_normalize`  | Re-format all documents for consistent formatting          |
| `iwe_attach`     | Attach a document to a target using configured actions     |

All write and refactoring tools support a `dry_run` parameter to preview changes before applying them.

### Selector parameters

`iwe_find`, `iwe_retrieve`, and `iwe_tree` accept a structural selector embedded in their tool input: `in`, `in_any`, `not_in`, and `max_depth`. Each entry is either a bare key or `{ key, depth }`. These are a convenience for the most common selection patterns; the full query surface — `--filter`-style documents, `$`-prefixed graph operators, block predicates, frontmatter and block mutation — is `iwe_query`, documented in the [Query Language](query-language.md) reference.

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
