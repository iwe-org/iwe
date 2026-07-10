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
| `iwe_retrieve` | Fetch documents with search seeds and graph expansion          |
| `iwe_tree`     | View hierarchical document structure                           |
| `iwe_stats`    | Get knowledge graph statistics and broken link reports         |
| `iwe_squash`   | Expand block references into a single flat document            |

### Writing

| Tool           | Description                                                 |
| -------------- | ---------------------------------------------------------- |
| `iwe_create`   | Create a new document from title and content, or an explicit key |
| `iwe_update`   | Replace the full content of an existing document           |
| `iwe_delete`   | Delete a document and clean up all references              |

`iwe_create` derives the document key from the title (slugified) unless you pass an explicit `key`. Give a `key` when the identity is a stable value drawn from metadata (an entity name, a session date) rather than the title wording; subdirectory keys such as `people/ada` are allowed; omit the file extension. Creation always fails if the key already exists.

### Query

| Tool        | Description                                                          |
| ----------- | -------------------------------------------------------------------- |
| `iwe_query` | Run a [Query Language](query-language.md) operation document verbatim |

`iwe_query` takes an `operation` kind (`find`, `count`, `update`, or `delete`) and the operation `document` as a YAML string, plus an optional `dry_run` for the mutating kinds. It exposes the full query surface: frontmatter and graph filters, the `$content` block-membership operator, the [`search`](query-language.md#search-find-only) stage on `find` (`search: { lexical, fuzzy }`), the `$content` / `$blocks` / `$matches` projection sources, and the block update operators (`$replace`, `$replaceText`, `$insertBefore`, `$insertAfter`, `$append`, `$delete`). `find` and `count` read; `update` applies frontmatter and block edits atomically per document; `delete` removes documents with reference cleanup.

The tool is **always strict**: every mutating application must carry an `expect` guard — the document-level `expect` on `update` / `delete`, plus one per block operator — or the operation is refused with the missing guards named. Use `find` with `$blocks` / `$matches` to locate targets and learn the counts before mutating. See [Strict mode](query-language.md#strict-mode).

### `iwe_retrieve` search and expansion

`iwe_retrieve` assembles reading context in one call. Beyond the selector parameters and token budgets, it accepts:

| Parameter | Description |
| --------- | ----------- |
| `search`  | BM25 full-text seed query (lexical). Present → the tool searches the candidate set (`keys` / selector) and reads the ordered seeds. |
| `fuzzy`   | Fuzzy seed query on title + key. Combine with `search` to fuse (RRF). |
| `expand`  | Object over `includes` / `includedBy` / `references` / `referencedBy` → integer depths (`0` = unbounded, omitted key = not followed). Follows those edges out from each seed. Expansion is doc-only when omitted. |
| `limit`   | Cap the number of seed documents kept **before** expansion — top-N by relevance when searching, the first N of the selection otherwise (`0` = unlimited). |
| `max_documents` | Cap the number of documents returned **after** expansion, trimming periphery documents first (`0` = unlimited). |

Output is seeds first (relevance order), then expansion. The edge-list toggles (`backlinks`, `children`) are unchanged. The pre-existing `depth`, `context`, and `links` parameters are **deprecated** aliases for `expand`'s `includes` / `includedBy` / `references`; passing `expand` together with any of them is an error.

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
