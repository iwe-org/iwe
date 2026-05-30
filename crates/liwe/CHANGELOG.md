# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fragment-only links like `[text](#header)` no longer produce `[text](.md#header)` when `refs_extension` is set
- Ordered list items use single space after marker (`1. item` instead of `1.  item`)
- Tables no longer produce extra trailing blank line

## [0.1.9](https://github.com/iwe-org/iwe/compare/liwe-v0.1.8...liwe-v0.1.9) - 2026-05-27

### Fixed

- `InlineRange` character positions now use character offsets instead of byte offsets, fixing `link_at`/`url_at` position lookups in documents containing multi-byte characters
- `line_starts` now counts actual `\n` byte positions instead of using `str::lines().len() + 1`, fixing line offset calculations for `\r\n` line endings

## [0.1.8](https://github.com/iwe-org/iwe/compare/liwe-v0.1.7...liwe-v0.1.8) - 2026-05-23

### Added

- `markdown.formatting.wrap_column: Option<usize>` — wraps `Para`/`Plain` blocks emitted by `Graph::to_markdown` at word boundaries; inline code, wiki links, math, and link/image URLs stay atomic while inline-link / image text wraps at spaces. List and blockquote indents are subtracted from the effective width via the new `GraphBlock::to_markdown_indented` API.
- `markdown.formatting.preserve_line_breaks: Option<bool>` — when `true`, `MarkdownEventsReader` preserves hard line breaks (`  \n`, `\\\n`) instead of dropping them, emitting them in the configured `line_break_style` on output.
- `markdown.formatting.line_break_style: Option<LineBreakStyle>` (default `Backslash`) with variants `Backslash`, `Spaces` — controls how `GraphInline::LineBreak` is rendered. `FormattingOptions::line_break_marker()` exposes the configured marker string.
- `GraphBlock::to_markdown_indented`, `blocks_to_markdown_and_indented`, and `blocks_to_markdown_sparce_indented` — indent-aware variants used internally to thread list/blockquote prefix width into paragraph wrap calculations.

## [0.1.7](https://github.com/iwe-org/iwe/compare/liwe-v0.1.6...liwe-v0.1.7) - 2026-05-20

### Added

- `CompletionOptions::trigger_characters: Option<Vec<String>>` field on the configuration model.

## [0.1.6](https://github.com/iwe-org/iwe/compare/liwe-v0.1.5...liwe-v0.1.6) - 2026-05-17

Workspace version bump — no user-visible changes in this crate.

## [0.1.5](https://github.com/iwe-org/iwe/compare/liwe-v0.1.4...liwe-v0.1.5) - 2026-05-16

### Fixed

- `append_refs_extension` no longer adds the configured `refs_extension` to link URLs that already carry a file extension (`.pdf`, `.html`, `.txt`, …), so serialization preserves links to non-markdown assets instead of mangling `foo.html` into `foo.html.md`

## [0.1.4](https://github.com/iwe-org/iwe/compare/liwe-v0.1.3...liwe-v0.1.4) - 2026-05-15

### Fixed

- `normalize_url` splits the URL on `#` before stripping `refs_extension`, and link emission re-attaches `refs_extension` to the path portion (before the fragment) instead of to the end of the URL, so links containing a fragment anchor round-trip correctly

## [0.1.3](https://github.com/iwe-org/iwe/compare/liwe-v0.1.2...liwe-v0.1.3) - 2026-05-05

Workspace version bump — no user-visible changes in this crate.

## [0.1.2](https://github.com/iwe-org/iwe/compare/liwe-v0.1.1...liwe-v0.1.2) - 2026-05-04

### Changed

- Filter parser allows mixing bare field keys with document-level operators (`$and`, `$or`, `$nor`, `$key`, `$includes`, `$includedBy`, `$references`, `$referencedBy`) at document-matching positions (filter root, branches of `$and`/`$or`/`$nor`, graph-anchor `match` clauses); they combine via implicit AND (was rejected with `cannot mix operator keys ($...) and bare keys`). Mixing inside a field-value mapping (e.g. `author: { $eq: alice, name: alice }`) and inside a field-level `$not` body remains rejected.
- `MixedDollarAndBare` error message names the offending position ("inside a field-value mapping at '<path>'") and suggests the fix.

### Removed

- Top-level `$not` operator. `$not` is now field-level only (matching MongoDB), e.g. `priority: { $not: { $gt: 5 } }`. For document-level negation use `$nor: [filter]`. The `Filter::Not` AST variant is removed; internal callers that previously constructed `Filter::Not(Box::new(inner))` now use `Filter::Nor(vec![inner])` (semantically identical). The `not()` constructor in `query::prelude` is removed; use `nor(vec![filter])` instead.

## [0.1.1](https://github.com/iwe-org/iwe/compare/liwe-v0.1.0...liwe-v0.1.1) - 2026-05-03

### Added

- `liwe::schema` module — `FieldSchema`, `TypeCount`, `ValueCount`, `Coverage` types plus `infer_schema` for frontmatter type/coverage analysis ([#274](https://github.com/iwe-org/iwe/pull/274))
- `YamlType` derives `Hash` and implements `Display`

### Changed

- `Graph::from_state` now takes `&State` instead of `State` (caller no longer transfers ownership of the parsed state)

## [0.1.0](https://github.com/iwe-org/iwe/compare/liwe-v0.0.70...liwe-v0.1.0) - 2026-05-01

### Added

- Query language engine over frontmatter — filter, project, sort, limit, update with `$eq`, `$ne`, `$gt`, `$gte`, `$lt`, `$lte`, `$in`, `$nin`, `$exists`, `$and`, `$or`, `$not`, `$regex`, plus update operators `$set` and `$unset`
- Graph filter operators for cross-document selection — `$includes`, `$includedBy`, `$references`, `$referencedBy`, each supporting bounded depth/distance
- Graph `walk` traversal module for bounded ancestor/descendant iteration
- Reserved frontmatter prefixes (`_`, `$`, `.`, `#`, `@`) — engine-only namespaces, invisible to user-facing queries and stripped from `update` writeback
- `EdgeRef { key, title, sectionPath }` — single canonical shape for inclusion / reference edges in `retrieve` and `find` output
- `RetrieveOptions.children: bool` — controls whether `DocumentOutput.includes` is populated, independently of `no_content`

### Changed

- `find`, `retrieve`, and `stats` rewritten on top of the query engine
- `FindResult` is now a `serde_yaml::Mapping` with system fields (`key`, `title`, `includedBy`) merged with user frontmatter at the top level; the nested `frontmatter` field and the four count fields are removed
- `ChildDocumentInfo`, `ParentDocumentInfo`, `BacklinkInfo` retired in favor of `EdgeRef`; `get_child_documents` now computes `section_path`
- `RetrieveOptions.no_content` only controls content blanking; child edges require `children: true`

### Removed

- Legacy `selector` module — superseded by `query`

## [0.0.70](https://github.com/iwe-org/iwe/compare/liwe-v0.0.69...liwe-v0.0.70) - 2026-04-25

### Added

- Add --in structural set selector across read commands ([#269](https://github.com/iwe-org/iwe/pull/269))
- Add time format in addition to date format ([#268](https://github.com/iwe-org/iwe/pull/268))

### Other

- Update readme

## [0.0.69](https://github.com/iwe-org/iwe/compare/liwe-v0.0.68...liwe-v0.0.69) - 2026-04-23

### Added

- Custom Markdown formatting ([#266](https://github.com/iwe-org/iwe/pull/266))

### Fixed

- Relative parent path normalization ([#264](https://github.com/iwe-org/iwe/pull/264))

## [0.0.68](https://github.com/iwe-org/iwe/compare/liwe-v0.0.67...liwe-v0.0.68) - 2026-04-22

### Added

- Add min prefix length for completions ([#262](https://github.com/iwe-org/iwe/pull/262))

### Fixed

- Relative inline links ([#263](https://github.com/iwe-org/iwe/pull/263))
- Index links inside the tables ([#255](https://github.com/iwe-org/iwe/pull/255))

## [0.0.66](https://github.com/iwe-org/iwe/compare/liwe-v0.0.65...liwe-v0.0.66) - 2026-04-04

### Added

- List broken links in the stats command output  ([#252](https://github.com/iwe-org/iwe/pull/252))
- Go to definition for external URL's ([#247](https://github.com/iwe-org/iwe/pull/247))

## [0.0.65](https://github.com/iwe-org/iwe/compare/liwe-v0.0.64...liwe-v0.0.65) - 2026-03-28

### Added

- Local dates and time components in the templates ([#245](https://github.com/iwe-org/iwe/pull/245))

## [0.0.64](https://github.com/iwe-org/iwe/compare/liwe-v0.0.63...liwe-v0.0.64) - 2026-03-25

### Added

- Add LSP folding ranges ([#235](https://github.com/iwe-org/iwe/pull/235))

## [0.0.63](https://github.com/iwe-org/iwe/compare/liwe-v0.0.62...liwe-v0.0.63) - 2026-03-20

### Added

- Search by document title, parent document titles and the document key instead of document path ([#231](https://github.com/iwe-org/iwe/pull/231))

### Other

- Removing unwarp's for stability and code style improvements ([#229](https://github.com/iwe-org/iwe/pull/229))

## [0.0.62](https://github.com/iwe-org/iwe/compare/liwe-v0.0.61...liwe-v0.0.62) - 2026-03-19

### Added

- CLI commands for graph transformations ([#227](https://github.com/iwe-org/iwe/pull/227))

### Fixed

- Ignore hidden files and directories ([#225](https://github.com/iwe-org/iwe/pull/225))

### Other

- release v0.0.61 ([#224](https://github.com/iwe-org/iwe/pull/224))

## [0.0.61](https://github.com/iwe-org/iwe/compare/liwe-v0.0.60...liwe-v0.0.61) - 2026-03-16

### Fixed

- Ignore hidden files and directories ([#225](https://github.com/iwe-org/iwe/pull/225))

## [0.0.60](https://github.com/iwe-org/iwe/compare/liwe-v0.0.59...liwe-v0.0.60) - 2026-01-14

### Added

- Preview linked note with LSP hover ([#207](https://github.com/iwe-org/iwe/pull/207))

## [0.0.59](https://github.com/iwe-org/iwe/compare/liwe-v0.0.58...liwe-v0.0.59) - 2026-01-10

### Added

- Align table columns width ([#203](https://github.com/iwe-org/iwe/pull/203))

### Fixed

- Honor soft break ([#204](https://github.com/iwe-org/iwe/pull/204))

## [0.0.58](https://github.com/iwe-org/iwe/compare/liwe-v0.0.57...liwe-v0.0.58) - 2026-01-10

### Added

- `iwe new` command ([#201](https://github.com/iwe-org/iwe/pull/201))

## [0.0.57](https://github.com/iwe-org/iwe/compare/liwe-v0.0.56...liwe-v0.0.57) - 2025-12-09

### Added

- Add wiki style links completion ([#199](https://github.com/iwe-org/iwe/pull/199))

### Other

- Move functionality search from library to server ([#188](https://github.com/iwe-org/iwe/pull/188))

## [0.0.56](https://github.com/iwe-org/iwe/compare/liwe-v0.0.55...liwe-v0.0.56) - 2025-11-11

### Fixed

- Rename operation should keep the title of the link ([#184](https://github.com/iwe-org/iwe/pull/184))

### Other

- Lint fixes ([#182](https://github.com/iwe-org/iwe/pull/182))

## [0.0.54](https://github.com/iwe-org/iwe/compare/liwe-v0.0.53...liwe-v0.0.54) - 2025-10-17

### Added

- Remove files from the index on delete ([#170](https://github.com/iwe-org/iwe/pull/170))

## [0.0.49](https://github.com/iwe-org/iwe/compare/liwe-v0.0.48...liwe-v0.0.49) - 2025-10-13

### Added

- Link the word under cursor ([#160](https://github.com/iwe-org/iwe/pull/160))

## [0.0.48](https://github.com/iwe-org/iwe/compare/liwe-v0.0.47...liwe-v0.0.48) - 2025-10-05

### Fixed

- Use default config if config doesn't exits ([#158](https://github.com/iwe-org/iwe/pull/158))

## [0.0.46](https://github.com/iwe-org/iwe/compare/liwe-v0.0.45...liwe-v0.0.46) - 2025-09-20

### Added

- Extract all config support ([#151](https://github.com/iwe-org/iwe/pull/151))
- Extract code action config ([#149](https://github.com/iwe-org/iwe/pull/149))

## [0.0.45](https://github.com/iwe-org/iwe/compare/liwe-v0.0.44...liwe-v0.0.45) - 2025-09-13

### Added

- Add Inline code action config with optional removal of the inlined file and references to it ([#145](https://github.com/iwe-org/iwe/pull/145))

### Fixed

- Panic when a key is not found during code action lookup ([#146](https://github.com/iwe-org/iwe/pull/146))

## [0.0.44](https://github.com/iwe-org/iwe/compare/liwe-v0.0.43...liwe-v0.0.44) - 2025-09-07

### Added

- Honor .gitignore files ([#141](https://github.com/iwe-org/iwe/pull/141))
- Delete note updating all references ([#140](https://github.com/iwe-org/iwe/pull/140))
- Add sort code action for lists sorting ([#139](https://github.com/iwe-org/iwe/pull/139))
- Include/exclude headers structure in DOT exports ([#120](https://github.com/iwe-org/iwe/pull/120))

## [0.0.43](https://github.com/iwe-org/iwe/compare/liwe-v0.0.42...liwe-v0.0.43) - 2025-09-05

### Added

- Add --verbose flag for CLI and more debug logs ([#137](https://github.com/iwe-org/iwe/pull/137))

## [0.0.42](https://github.com/iwe-org/iwe/compare/liwe-v0.0.41...liwe-v0.0.42) - 2025-09-04

### Fixed

- Inlay hints request for non existent file crashes the server ([#135](https://github.com/iwe-org/iwe/pull/135))

## [0.0.41](https://github.com/iwe-org/iwe/compare/liwe-v0.0.40...liwe-v0.0.41) - 2025-09-01

### Fixed

- Do not remove extensions from local links ([#132](https://github.com/iwe-org/iwe/pull/132))

## [0.0.40](https://github.com/iwe-org/iwe/compare/liwe-v0.0.39...liwe-v0.0.40) - 2025-08-31

### Added

- Customizable "Attach" code action for documents linking ([#128](https://github.com/iwe-org/iwe/pull/128))

## [0.0.39](https://github.com/iwe-org/iwe/compare/liwe-v0.0.38...liwe-v0.0.39) - 2025-08-28

### Fixed

- Code action should not remove YAML metadata ([#127](https://github.com/iwe-org/iwe/pull/127))

## [0.0.38](https://github.com/iwe-org/iwe/compare/liwe-v0.0.37...liwe-v0.0.38) - 2025-08-28

### Fixed

- Inline links extension formatting bug fix ([#123](https://github.com/iwe-org/iwe/pull/123))

## [0.0.37](https://github.com/iwe-org/iwe/compare/liwe-v0.0.36...liwe-v0.0.37) - 2025-08-27

### Added

- Include/exclude headers structure in DOT exports ([#120](https://github.com/iwe-org/iwe/pull/120))

### Fixed

- Ignore non alphanumeric chars in search ([#119](https://github.com/iwe-org/iwe/pull/119))

## [0.0.36](https://github.com/iwe-org/iwe/compare/liwe-v0.0.35...liwe-v0.0.36) - 2025-08-24

### Added

- Backlinks inlay hints ([#117](https://github.com/iwe-org/iwe/pull/117))

## [0.0.35](https://github.com/iwe-org/iwe/compare/liwe-v0.0.34...liwe-v0.0.35) - 2025-08-21

### Added

- DOT styles ([#114](https://github.com/iwe-org/iwe/pull/114))

## [0.0.34](https://github.com/iwe-org/iwe/compare/liwe-v0.0.33...liwe-v0.0.34) - 2025-08-18

### Added

- Graphviz DOT format export support ([#109](https://github.com/iwe-org/iwe/pull/109))

## [0.0.33](https://github.com/iwe-org/iwe/compare/liwe-v0.0.32...liwe-v0.0.33) - 2025-06-07

### Fixed

- Fix panic in case of quote in list item ([#105](https://github.com/iwe-org/iwe/pull/105))

## [0.0.31](https://github.com/iwe-org/iwe/compare/liwe-v0.0.30...liwe-v0.0.31) - 2025-04-06

### Added

- Triggering LLM queries using LSP completions ([#97](https://github.com/iwe-org/iwe/pull/97))

## [0.0.29](https://github.com/iwe-org/iwe/compare/liwe-v0.0.28...liwe-v0.0.29) - 2025-03-29

### Fixed

- List item with dual dash "- -" causing panic ([#92](https://github.com/iwe-org/iwe/pull/92))

## [0.0.28](https://github.com/iwe-org/iwe/compare/liwe-v0.0.27...liwe-v0.0.28) - 2025-03-30

### Added

- Custom LLM code actions support for context aware updates ([#90](https://github.com/iwe-org/iwe/pull/90))

## [0.0.27](https://github.com/iwe-org/iwe/compare/liwe-v0.0.26...liwe-v0.0.27) - 2025-03-08

### Added

- Tables support ([#77](https://github.com/iwe-org/iwe/pull/77))

## [0.0.26](https://github.com/iwe-org/iwe/compare/liwe-v0.0.25...liwe-v0.0.26) - 2025-02-25

### Fixed

- Use relative paths in code actions ([#73](https://github.com/iwe-org/iwe/pull/73))

## [0.0.25](https://github.com/iwe-org/iwe/compare/liwe-v0.0.24...liwe-v0.0.25) - 2025-02-24

### Added

- Sub-directories support ([#71](https://github.com/iwe-org/iwe/pull/71))

## [0.0.22](https://github.com/iwe-org/iwe/compare/liwe-v0.0.21...liwe-v0.0.22) - 2025-02-17

### Added

- Better search results ([#61](https://github.com/iwe-org/iwe/pull/61))

## [0.0.21](https://github.com/iwe-org/iwe/compare/liwe-v0.0.20...liwe-v0.0.21) - 2025-02-17

### Added

- better search results (#61)

## [0.0.20](https://github.com/iwe-org/iwe/compare/liwe-v0.0.19...liwe-v0.0.20) - 2025-02-17

### Added

- LSP search with fuzzy matching and page-rank (#56)

## [0.0.19](https://github.com/iwe-org/iwe/compare/liwe-v0.0.18...liwe-v0.0.19) - 2025-02-16

### Added

- wiki links support (#52)