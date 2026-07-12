# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `iwe_create`, `iwe_update`, and `iwe_query` (`update` / `delete`) now return non-blocking stats warnings alongside a successful result — orphan pages, dangling links, and (on create/update) near-identical pages — as `warning:` content blocks, each finding reported once per session. They never block a write; schema validation remains the only hard reject.
- `iwe_stats` with a `key` now returns a `similarPages` array of documents near-identical to that page.

### Changed
- Markdown link text in rendered document content is kept as written by default; set the `refs_text` markdown option to `normalize` to rewrite each link's text to the linked document's title as before.
- Mutating tools (`iwe_create`, `iwe_update`, `iwe_delete`, `iwe_query`, `iwe_rename`, `iwe_extract`, `iwe_inline`, `iwe_attach`) now reject a change that would leave a touched document violating its bound schema, returning a detailed error carrying the readable report and a structured `violations` payload; nothing is written and the in-memory graph is left untouched. `iwe_normalize` and `iwe_squash` are unaffected.

## [0.11.0](https://github.com/iwe-org/iwe/compare/iwec-v0.10.0...iwec-v0.11.0) - 2026-07-10

### Added
- `iwe_create` gains an optional `key` parameter — create a document at an explicit key instead of a title-derived slug. Derive it from stable metadata (entity name, session date); subdirectory keys (e.g. `people/ada`) are allowed; omit the file extension. Creation fails if a document with that key already exists.
- `iwe_query` `find` operations accept a `search` clause (`search: { lexical, fuzzy }`) — relevance selection that restricts and orders results; a `lexical` query with no searchable terms returns an empty array plus a warning content block.
- `iwe_retrieve` gains `search` / `fuzzy` (seed queries), `expand` (object over `includes` / `includedBy` / `references` / `referencedBy` → integer depths, `0` = unbounded), and `max_documents` (cap the documents returned after expansion). With a search query the tool finds seed documents within the candidate set (`keys` + selector) and expands the graph around the ordered seeds.

### Changed
- `iwe_retrieve` `limit` now caps the seed documents before expansion (top-N by relevance when searching, the first N of the selection otherwise); use `max_documents` for the post-expansion document cap.
- `iwe_retrieve` no longer expands by default — omit `expand` and it returns the requested document(s) only (previously the implicit behavior was one level of children and parents).
- `iwe_delete` and `iwe_query` deletes now also remove any parent directory left empty by a removed document, matching the CLI (previously empty directories were kept).

### Deprecated
- `iwe_retrieve` `depth` / `context` / `links` — retained as aliases for `expand`'s `includes` / `includedBy` / `references`; passing `expand` together with any of them is an error.

## [0.10.0](https://github.com/iwe-org/iwe/compare/iwec-v0.9.0...iwec-v0.10.0) - 2026-07-09

Workspace version bump — no user-visible changes in this crate.

## [0.9.0](https://github.com/iwe-org/iwe/compare/iwec-v0.8.0...iwec-v0.9.0) - 2026-07-09

### Added
- `iwe_query` tool runs an IWE query/block-selection operation document — `operation` is `find` / `count` / `update` / `delete` and `document` is the operation as a YAML string. It exposes the `$content` membership filter, the `$content` / `$blocks` / `$matches` projection sources, and the block update operators (`$replace`, `$replaceText`, `$insertBefore`, `$insertAfter`, `$append`, `$delete`). `find` and `count` read; `update` applies frontmatter and block edits; `delete` removes documents with reference cleanup. The tool is always strict: every mutating application must carry an `expect` guard or the operation is refused with the missing guards named. `update` / `delete` accept `dry_run` to preview without writing.

## [0.8.0](https://github.com/iwe-org/iwe/compare/iwec-v0.7.0...iwec-v0.8.0) - 2026-07-07

### Changed
- `iwe_find` replaces its `query` parameter with explicit `fuzzy` (match on document title and key) and `lexical` (BM25 full-text over title and body) parameters; supplying both fuses the results with Reciprocal Rank Fusion. Set the stemming language for lexical search with `[search] language` in the configuration.

## [0.7.0](https://github.com/iwe-org/iwe/compare/iwec-v0.6.1...iwec-v0.7.0) - 2026-07-03

### Added
- `iwe_retrieve` and `iwe_find` accept `max_tokens` / `max_document_tokens` (and `iwe_retrieve` a `limit`) to bound output; all are unlimited unless set. When a limit trims the output, the tool appends a second content block with a JSON truncation summary (`truncated`, `emitted`, `matched`, `clipped`, `tokens`, `budget`, `hint`) alongside the unchanged primary JSON.

### Removed
- `iwe_retrieve` tool `no_content` parameter — removed; the tool always returns content.

## [0.6.1](https://github.com/iwe-org/iwe/compare/iwec-v0.6.0...iwec-v0.6.1) - 2026-07-03

### Fixed
- `iwe_normalize` now actually reformats documents on disk: it compares each document's normalized form against the file's current contents and rewrites the ones that differ, reporting an accurate `normalized` count instead of always returning `0`.
- `iwe_create` rejects a title with no alphanumeric characters (e.g. `"!!!"`) instead of writing an empty-named file.
- `iwe_attach` and the `iwe://config` resource return an error instead of crashing the server when an attach action's template is malformed.
- The file watcher maps a file named `note.md.md` to the key `note.md` instead of `note`, matching how the graph loads documents from disk.
- Repeatedly saving a watched document that contains a table no longer leaks memory as the server re-parses it.

## [0.6.0](https://github.com/iwe-org/iwe/compare/iwec-v0.5.0...iwec-v0.6.0) - 2026-06-27

Workspace version bump — no user-visible changes in this crate.

## [0.5.0](https://github.com/iwe-org/iwe/compare/iwec-v0.4.0...iwec-v0.5.0) - 2026-06-23

### Added
- `--host` flag sets the address the HTTP transport binds to (default `127.0.0.1`); pass `--host 0.0.0.0` to accept connections from other machines.
- `format = "djot"` in the configuration makes the server read, write, and watch [djot](https://djot.net/) `.dj` documents (default remains `markdown` with `.md`).

### Fixed
- The server no longer leaks memory while watching a folder; repeatedly saving documents used to grow memory without bound and could exhaust RAM and swap over a long session.

## [0.4.0](https://github.com/iwe-org/iwe/compare/iwec-v0.3.2...iwec-v0.4.0) - 2026-06-22

### Added

- `--transport` flag selects how the server is served: `stdio` (default, unchanged) or `http`. With `--transport http` the server listens for Streamable HTTP connections at `http://127.0.0.1:<port>/mcp`, with `--port` setting the port (default `8000`). The server speaks plain HTTP and binds to localhost only; put a reverse proxy in front of it for TLS or remote access.

## [0.3.2](https://github.com/iwe-org/iwe/compare/iwec-v0.3.1...iwec-v0.3.2) - 2026-06-05

Workspace version bump — no user-visible changes in this crate.

## [0.3.1](https://github.com/iwe-org/iwe/compare/iwec-v0.3.0...iwec-v0.3.1) - 2026-06-03

Workspace version bump — no user-visible changes in this crate.

## [0.3.0](https://github.com/iwe-org/iwe/compare/iwec-v0.2.0...iwec-v0.3.0) - 2026-06-02

### Changed

- The `iwe_normalize` tool now recognizes task-list markers in list items (`- [ ]`, `- [x]`) and normalizes `[X]` to lowercase `[x]`
- List items are now a distinct node type rather than sections, so `iwe_stats` no longer counts them toward the section total and `iwe_extract` no longer lists them as extractable sections (block numbers shift accordingly)

## [0.2.0](https://github.com/iwe-org/iwe/compare/iwec-v0.1.10...iwec-v0.2.0) - 2026-06-02

Workspace version bump — no user-visible changes in this crate.

## [0.1.10](https://github.com/iwe-org/iwe/compare/iwec-v0.1.9...iwec-v0.1.10) - 2026-05-30

### Fixed

- File-watcher document keys join relative path components with `/` so they use forward-slash separators on all platforms (previously preserved Windows backslashes)

## [0.1.9](https://github.com/iwe-org/iwe/compare/iwec-v0.1.8...iwec-v0.1.9) - 2026-05-27

Workspace version bump — no user-visible changes in this crate.

## [0.1.8](https://github.com/iwe-org/iwe/compare/iwec-v0.1.7...iwec-v0.1.8) - 2026-05-23

Workspace version bump — no user-visible changes in this crate.

## [0.1.7](https://github.com/iwe-org/iwe/compare/iwec-v0.1.6...iwec-v0.1.7) - 2026-05-20

Workspace version bump — no user-visible changes in this crate.

## [0.1.6](https://github.com/iwe-org/iwe/compare/iwec-v0.1.5...iwec-v0.1.6) - 2026-05-17

Workspace version bump — no user-visible changes in this crate.

## [0.1.5](https://github.com/iwe-org/iwe/compare/iwec-v0.1.4...iwec-v0.1.5) - 2026-05-16

Workspace version bump — no user-visible changes in this crate.

## [0.1.4](https://github.com/iwe-org/iwe/compare/iwec-v0.1.3...iwec-v0.1.4) - 2026-05-15

Workspace version bump — no user-visible changes in this crate.

## [0.1.3](https://github.com/iwe-org/iwe/compare/iwec-v0.1.2...iwec-v0.1.3) - 2026-05-05

Workspace version bump — no user-visible changes in this crate.

## [0.1.2](https://github.com/iwe-org/iwe/compare/iwec-v0.1.1...iwec-v0.1.2) - 2026-05-04

### Changed

- Filter expressions in `iwe_find` / `iwe_update` / `iwe_delete` accept the natural form `{type: tracker, $or: [...]}` — bare field keys may be mixed with `$and`/`$or`/`$nor`/`$key`/graph operators at document-matching positions, combining via implicit AND (previously rejected).

### Removed

- Top-level `$not` in MCP query filters. `$not` is now field-level only (matching MongoDB); use `$nor: [filter]` for document-level negation. Top-level `$not` returns a parse-time error pointing to `$nor`.

## [0.1.1](https://github.com/iwe-org/iwe/compare/iwec-v0.1.0...iwec-v0.1.1) - 2026-05-03

Workspace version bump — no user-visible changes in this crate.

## [0.1.0](https://github.com/iwe-org/iwe/compare/iwec-v0.0.70...iwec-v0.1.0) - 2026-05-01

### Added

- `iwe_retrieve` accepts `children: bool` to populate the `includes` array independently of `no_content`

### Changed

- `find`, `retrieve`, `stats`, and prompt assembly rewired onto the new query engine; tests updated for the new wire format
- `iwe_find` returns a bare array of result objects (the `{query, limit, total, results}` envelope is removed); each result flattens user frontmatter alongside `key`, `title`, `includedBy`
- `iwe_retrieve` returns a bare array of document objects (the `{documents}` envelope is removed); `includes` entries carry `sectionPath`
- `iwe_retrieve` `no_content` no longer implies child population; pass `children: true` for that
- `iwe_tree` always emits `children: []` for leaf nodes
- `review` and `refactor` prompts embed the new array-shaped retrieve JSON

## [0.0.70](https://github.com/iwe-org/iwe/compare/iwec-v0.0.69...iwec-v0.0.70) - 2026-04-25

### Added

- Add --in structural set selector across read commands ([#269](https://github.com/iwe-org/iwe/pull/269))
- Add time format in addition to date format ([#268](https://github.com/iwe-org/iwe/pull/268))

### Other

- Update readme

## [0.0.68](https://github.com/iwe-org/iwe/compare/iwec-v0.0.67...iwec-v0.0.68) - 2026-04-22

### Fixed

- Index links inside the tables ([#255](https://github.com/iwe-org/iwe/pull/255))
