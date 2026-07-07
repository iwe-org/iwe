# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
