# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `search` clause on `FindOp` (`SearchSpec { lexical, fuzzy }`) — a relevance stage that restricts membership to documents matching the query and supplies the default ordering, jointly with `filter`; `search` + `sort` keeps membership from search while `sort` supplies the order. The ranking logic (`query::search::ranked` / `matched`, RRF fusion) is now a shared stage used by both `DocumentFinder` and the query engine.
- `RetrieveOptions` expansion generalized to four edge-named depths — `includes`, `included_by`, `references`, `referenced_by` (`u32`, `0` = off, `UNBOUNDED` = no limit) — replacing the `depth` / `context` / `links` fields. Inbound-reference expansion (`referenced_by`) and transitive outbound-reference expansion (`references` > 1) are now expressible; `retrieve::expand_depth` maps an `--expand` value (`0` = unbounded) to a depth.

### Added
- `format::DocumentFormat` trait (`read` / `write` / `write_skip_frontmatter`) as the format boundary, with the built-in `MarkdownFormat` and (feature-gated) `DjotFormat` implementations and a `format::format_for` constructor.
- `query::QueryScores` and `query::execute_with_scores` — the query engine now ranks a `search` clause from caller-injected per-key relevance scores instead of computing them itself, keeping the kernel free of any search index.

### Changed
- Djot support is now behind an opt-in `djot` cargo feature; `jotdown` is an optional dependency and the `djot` module compiles only when the feature is enabled. Markdown remains built in. Enable `features = ["djot"]` to read and write `.dj` documents.
- The BM25 search index left the kernel: `Graph` no longer builds or holds one, so `Graph::search`, `has_search_index`, `search_scores`, and `lexical_query_has_terms` are removed, and `Graph::from_state` / `Graph::import` no longer take a `search_language` argument. The BM25 index and the `search` scoring (`ranked` / `matched`) moved to the `diwe` crate; `query::SearchSpec` stays as the DSL type.
- `Graph::has_search_index()` reports whether the graph carries a BM25 index; running a `find` with a `search` clause against a graph without one is an execution-time error (`EvalError::SearchIndexMissing`).
- `RetrieveOptions.limit` now caps the seed set before `DocumentReader::retrieve_many` expands (it was a post-expansion cap on the number of documents returned); the post-expansion cap moves to the new `RetrieveOptions.max_documents` field. Both are `Option<usize>`, `None` / `Some(0)` = unlimited.
- `EdgeRef` moved from `retrieve` to `query::edges`.

### Removed
- The engine modules (`find`, `retrieve`, `stats`, `tokens`, `fs`, `file`) and the `.iwe/config.toml` mapping (`Configuration`, `LibraryOptions`, `CompletionOptions`, `SearchOptions`, `Command`, `ActionDefinition` and its variants, `NoteTemplate`, `load_config`) moved to the new `diwe` crate; `liwe` keeps the document kernel and the format/option types (`Format`, `FormatOptions`, `MarkdownOptions`, `DjotOptions`, `FormattingOptions`, `LinkType`, `InlineType`, `TargetType`, `Operation`) in `liwe::model::config`.
- `Graph::from_path` — the filesystem-loading constructor moved to `diwe`; build a `State` from disk and call `Graph::from_state`.

## [0.10.0](https://github.com/iwe-org/iwe/compare/liwe-v0.9.0...liwe-v0.10.0) - 2026-07-09

### Added
- `RefsPath` enum and a `refs_path` field on `MarkdownOptions` / `DjotOptions` (default `RefsPath::Relative`), surfaced through `FormatOptions::refs_path()`; `RefsPath::Absolute` renders regular links as root-absolute paths (`/dir/note.md`) instead of paths relative to the linking document.
- `Key::link_url(relative_to, refs_path)` builds a regular link's path for the given `RefsPath`, so every link-writing path shares one implementation.

### Fixed
- `Key::from_rel_link_url` resolves a regular link with a leading `/` from the library root regardless of the linking document's directory (previously a leading `/` only resolved from a document at the root), and strips a trailing `#fragment` before computing the key so `note.md#section` resolves to the same key as `note.md`.

## [0.9.0](https://github.com/iwe-org/iwe/compare/liwe-v0.8.0...liwe-v0.9.0) - 2026-07-09

### Added
- `query::block` module with the `BlockPredicate` grammar for addressing blocks inside a document: text and regex predicates, `$within` / `$contains` axes, per-type operators (`$section`, `$header`, `$paragraph`, `$item`, `$list`, `$quote`, `$code`, `$table`, `$ref`, `$hr`), `$references`, and `$and` / `$or` / `$nor` composition.
- Block-addressed projection sources: `{ $content: PREDICATE }` renders the selected blocks, `$blocks` reports each selected block as `type` / `path` / `text`, and `{ $matches: REGEX }` greps matching lines with their section paths — all evaluated through the new `query::block_eval::BlockIndex`.
- `IntoBlockPredicate` trait accepting scalar shorthands wherever a block predicate is expected.
- `FindOp::add_fields` sets an additive (`addFields`) projection; `CountOp` and `DeleteOp` implement `From<FindOp>`, reinterpreting a built find with its projection dropped.
- `project` accepts a `$`-prefixed block predicate mapping (`project: { $header: {} }`), lowering it to a `key` field and a `content` field carrying the narrowed body.
- Block update operators in the `update` document — `$replace`, `$replaceText`, `$insertBefore`, `$insertAfter`, `$append`, `$delete` — each pairing a block predicate selector with its payload and an optional `expect` guard, validated and applied atomically; represented by `BlockUpdate`, `BlockUpdateOp`, and `Expect` on `Update::block_ops`.
- Unit block operators act on their target as selected: a `$header` node covers its heading line alone (`$delete` dissolves the section, a heading `$replace` retitles it) while a `$section` covers the whole tree.
- `UpdateOp` and `DeleteOp` carry an optional `expect` guard asserting the number of matched documents; on violation nothing is written and the error lists each matched document.
- `$content` filter operator (`Filter::Content`) matches documents holding at least one block satisfying a block predicate; it composes with every other filter clause.
- `query::strict_guard_violations` names the mutating applications that lack an `expect` guard.

### Changed
- `query::execute` returns `Result<Outcome, block_update::EvalError>` (was `Outcome`) so a failed block update reports its validation error instead of writing; `find`, `count`, and `delete` always return `Ok`.
- `Update` carries `block_ops: Vec<BlockUpdate>` alongside the frontmatter `operators`.
- `Projection` carries a `base: ProjectionBase` (`Empty` / `Frontmatter` / `Document`) in place of `mode: ProjectionMode`, so `FindOp::project` is a plain `Projection` (was `Option<Projection>`); `Projection::document_fields()` replaces `Projection::default_for_find()`.
- `ProjectionContext` is constructed with `ProjectionContext::new(graph, key)` (was a public struct literal).

### Fixed
- A mis-typed `project` mapping now reports a parse error instead of being silently read as a comma list of frontmatter field names and yielding `null`.

### Removed
- `query::prelude` module (with its `WithFilter` trait) — the Rust builder functions moved into the test suite; construct `Operation`, `FindOp`, and `Filter` directly.
- `query::project::apply_projection_or_default` — `apply_projection` covers the no-projection case.

## [0.8.0](https://github.com/iwe-org/iwe/compare/liwe-v0.7.0...liwe-v0.8.0) - 2026-07-07

### Added
- `search::Bm25Index` plus `Graph::search` and `Graph::search_scores` provide BM25 full-text ranking over document title and body; the index is built and kept in sync by the ingestion pipeline (`insert_document` / `update_document` / `remove_document`).
- `Graph::to_plain_text` renders a document to plain text (markup stripped, link display text kept, code and table cells included); `Node::plain_text` now also covers table cells.
- `[search]` configuration table with a `language` field (one of 17 stemming languages, default `english`), exposed through `Configuration::search_language`.
- `search::rrf_weight` and `search::RRF_K` for Reciprocal Rank Fusion of ranked result lists.
- `Graph::lexical_query_has_terms` (backed by `Bm25Index::has_query_terms`) reports whether a lexical query keeps any searchable terms after stop-word removal and stemming.

### Changed
- `Graph::from_state` and `Graph::from_path` take an extra `Option<Language>` argument that enables search indexing when set (`None` skips it); `Graph::import` is unchanged and keeps indexing off.
- `FindOptions` carries separate `fuzzy` and `lexical` query fields (was a single `query`); when both are set the finder fuses the two rankings with Reciprocal Rank Fusion.

## [0.7.0](https://github.com/iwe-org/iwe/compare/liwe-v0.6.1...liwe-v0.7.0) - 2026-07-03

### Added
- `tokens` module (`count_tokens`, `truncate_to_tokens`) backed by `tiktoken-rs`.
- `RetrieveOptions` gains `limit`, `max_tokens`, `max_document_tokens`; `FindOptions` gains `max_tokens`, `max_document_tokens`; `RetrieveOutput` / `FindOutput` gain a `truncation` summary.

### Removed
- `RetrieveOptions::no_content` field — removed; `DocumentReader` always populates `content`.

## [0.6.1](https://github.com/iwe-org/iwe/compare/liwe-v0.6.0...liwe-v0.6.1) - 2026-07-03

### Fixed

- `DocumentInline::key_range` now returns the target range for wiki links (`[[target]]` and `[[target|label]]`); it previously assumed `[text](url)` syntax and returned an empty range at the wrong offset.
- The markdown reader now offsets line positions by the stripped empty frontmatter (`---` / `---`), so block and inline ranges point at the original document lines instead of being two lines too low.
- Sorting a field that mixes value types (e.g. some documents with `priority: 3` and others with `priority: high`) now produces a stable, deterministic order: values are grouped by type before being compared, so cross-type pairs no longer collapse to "equal" and scramble the result.
- Numeric filters compare integers exactly instead of routing everything through 64-bit floats, so large whole numbers past 2^53 (like `id: 9007199254740993`) are no longer treated as equal to their neighbours.
- A `not-a-number` filter target (`.nan`) no longer compares as equal to every number, so range operators like `$gte: .nan` match nothing instead of matching everything.
- Datetime detection in query filters no longer crashes on a value with multibyte characters in the timezone tail (for example `2026-04-26T10:30:00+0é9`).
- Code blocks whose content contains a run of the fence token are now written with a longer fence, so the block no longer terminates early and leaks its content into the surrounding text.
- A paragraph that starts with an escaped `\*` keeps its backslash when written, so it is not re-parsed as a bullet list on the next pass.
- A document key strips only one file extension, so a file named `note.md.md` maps to the key `note.md` instead of collapsing onto `note` and being duplicated on write.
- Deleting a table node now releases its header and row lines back to the arena; repeatedly re-parsing a document that contains a table no longer leaks those line slots and grows memory without bound.

## [0.6.0](https://github.com/iwe-org/iwe/compare/liwe-v0.5.0...liwe-v0.6.0) - 2026-06-27

### Added
- `FormattingOptions` gains a `preserve_newlines` field (default `false`); when enabled, a soft line break inside a paragraph is read as `DocumentInline::SoftBreak` and written back as a plain newline instead of being collapsed into a space, so the source line layout survives normalization.

### Changed
- `Inline::Math` now carries a `MathType` (`Inline::Math(MathType, String)`), distinguishing inline from display math.

### Fixed
- The djot writer now leaves a blank line before a nested list or any second block inside a list item, so list items with sub-lists or extra paragraphs round-trip instead of collapsing into the item's first line.
- The djot reader no longer panics on a document that contains a reference link definition or a definition list; the orphaned text is dropped instead of crashing the parser.
- A hard line break now becomes a space when line breaks aren't preserved, instead of running the words on either side together.
- Djot task list items (`- [ ]` / `- [x]`) round-trip instead of having the checkbox escaped and the item text split onto a separate line.
- Djot display math (`$$`) is no longer written back as inline math (`$`).
- Djot autolinks (`<url>`) round-trip instead of being expanded to a full `[url](url)` link.

## [0.5.0](https://github.com/iwe-org/iwe/compare/liwe-v0.4.0...liwe-v0.5.0) - 2026-06-23

### Added
- `FormatOptions` (`Markdown(MarkdownOptions)` | `Djot(DjotOptions)`) bundles the document format with its formatting options; it is what the graph reads and writes through. `Graph::new_with_options`, `from_state`, `from_path`, and `import` accept `impl Into<FormatOptions>`. `DjotReader`/`DjotWriter` parse and serialize [djot](https://djot.net/) documents so the graph round-trips a `.dj` document back to djot, and `Configuration` gains a top-level `format` selector, a `djot: DjotOptions` table, and `Configuration::format_options()`.
- `Inline` and `DocumentInline` gain `Span`, `Mark`, `Insert`, `Delete`, and `Symbol` variants, and an `inline::Attributes` type, so djot's bracketed attribute spans, highlight/insert/delete marks, and symbols round-trip losslessly through the graph.

### Changed
- `NodeIter::to_text(parent, &FormatOptions)` replaces the markdown-specific `to_markdown`, serializing to whichever format the `FormatOptions` carries.
- `Key::to_path` and the `fs` discovery and write helpers (`walk_md_paths`, `new_for_path`, `write_file`, `write_store_at_path`) now take a `Format` so document files use the configured extension (`.md` or `.dj`).

### Fixed
- Updating or removing a document now reclaims the graph nodes, lines, and reference-index entries that belonged to its previous version, so a long-lived `Graph` no longer grows without bound as the same documents are edited over and over.

## [0.4.0](https://github.com/iwe-org/iwe/compare/liwe-v0.3.2...liwe-v0.4.0) - 2026-06-22

### Fixed
- `MarkdownReader` now normalizes Windows line endings before parsing, so documents with `\r\n` line endings keep their frontmatter and report correct positions (previously frontmatter was dropped and positions drifted one column per line).
- The Markdown writer now re-escapes special characters in text, so escaped literals such as `\*text\*`, a leading `\#`, `\[label\](url)`, and `1\.` survive normalization instead of turning back into emphasis, headings, links, or list markers; inline code containing a backtick is fenced with enough backticks to render intact, and a list item written as an escaped `\[ \]` is no longer mistaken for a task checkbox.

## [0.3.2](https://github.com/iwe-org/iwe/compare/liwe-v0.3.1...liwe-v0.3.2) - 2026-06-05

### Fixed
- `MarkdownReader` parsing of large documents no longer runs in quadratic time; source offsets are mapped to line/column positions with a binary search over the line table and a single UTF-16 count per endpoint, instead of rescanning every preceding line for each inline element.

## [0.3.1](https://github.com/iwe-org/iwe/compare/liwe-v0.3.0...liwe-v0.3.1) - 2026-06-03

Workspace version bump — no user-visible changes in this crate.

## [0.3.0](https://github.com/iwe-org/iwe/compare/liwe-v0.2.0...liwe-v0.3.0) - 2026-06-02

### Added
- `Node::Item(Option<bool>, Inlines)` represents a list item as a first-class node carrying task-checkbox state (`- [ ]` → `Some(false)`, `- [x]`/`- [X]` → `Some(true)`, plain item → `None`); checkboxes are detected when the tree is collected and re-emitted (normalized to lowercase `[x]`) when rendering markdown.
- `Reference.url: String` carries the wiki link target exactly as written, and `Reference.display_url: Option<String>` holds the pre-resolved wiki display URL, both populated when the tree is collected so markdown rendering is self-contained.
- `MarkdownOptions.wiki_link_path: WikiLinkPath` (`Full` | `Short` | `Preserve`, default `Preserve`) controls how the path inside a wiki link is rendered: `Preserve` keeps `Reference.url` as written, `Full` uses the target's full key path, and `Short` uses the shortest unambiguous suffix. `Graph::wiki_display(&self, &Key, &str) -> String` applies the setting, exposed to the inline-resolution pass through `InlinesContext::wiki_display` (replacing `InlinesContext::shorten_wiki`). `KeyIndex::wiki_target(&self, &Key, WikiLinkPath) -> String` computes the form for newly created links.

### Changed
- The markdown model types moved out of the now-removed `model::graph` module: `GraphInline`/`GraphInlines` became `Inline`/`Inlines` in `model::inline`, and `GraphBlock` became `Block` in `model::writer` (`Blocks` and the `blocks_to_markdown*` helpers move with it). `NodeIter` and `NodePointer` move to `model::node_iter` and `model::node_pointer`, and `TreeIter` to `model::tree_iter`.
- `Projector::project`, `NodeIter::to_markdown` / `to_markdown_skip_frontmatter`, and `Graph::to_markdown` / `to_markdown_skip_frontmatter` no longer take a `KeyIndex` and render markdown purely from the collected tree; the `*_indexed` `NodeIter` variants are removed. Wiki links are resolved to their display form (per `wiki_link_path`) when the tree is collected (was at render time) and the result is carried on `Reference.display_url`.

### Fixed
- Wiki link shortening no longer shortens a target absent from the document set onto an unrelated document that shares its file name (previously a suffix matching zero keys was accepted, so a link could be rewritten to a shorter form resolving elsewhere); the shortened form now resolves only to the exact target it was derived from, otherwise the full path is kept.
## [0.2.0](https://github.com/iwe-org/iwe/compare/liwe-v0.1.10...liwe-v0.2.0) - 2026-06-02

### Added
- `markdown.formatting.ordered_list_content_indent: Option<usize>` and `markdown.formatting.bullet_list_content_indent: Option<usize>` set the minimum column where list item content and continuation lines start (accepts `2`–`4`; other values are ignored). When unset, content aligns one space after the marker as before; set to `4` for MkDocs-style alignment (`1.  item` / `-   item` with 4-space continuation). The natural marker width is always respected. `FormattingOptions` gains the `ordered_list_content_indent()` / `bullet_list_content_indent()` getters.

### Fixed
- List rendering now treats a list as loose when any item contains a block requiring blank-line separation (code block, table, blockquote, horizontal rule), inserting a blank line between items, so a following item is no longer glued directly under the preceding item's block (previously only multi-paragraph items triggered loose rendering)
- Wiki links (`[[name]]`) now resolve by path-suffix across the whole document set instead of relative to the linking file's directory: a bare name matches any document with that basename, and a partial path (`[[folder/name]]`) matches any document whose path ends with those segments, with ambiguity resolved deterministically (fewest path segments, then lexicographic). Markdown link resolution is unchanged, and wiki link backlink edges are keyed by the resolved target.

### Changed
- Wiki link references are now stored fully resolved in the graph — resolved to their canonical `Key` when the document is built, the same way markdown links are — so reference and inclusion edges carry the resolved target and no longer need to be re-resolved at query time. The shortest path-suffix form is computed only when rendering markdown (document content and completion).
- `Graph` caches a `KeyIndex` built from its keys and exposes it via `Graph::key_index(&self) -> &KeyIndex` (previously this method built and returned an owned index per call); the cache is kept in sync as documents are added and removed.
- `KeyIndex` gains `insert`, `remove`, and `resolve_link_key`, and derives `Clone`/`Default`; wiki links are rendered/normalized as the shortest path-suffix that uniquely identifies the target via `KeyIndex::shorten_wiki`. `NodeIter::to_markdown_indexed` / `to_markdown_skip_frontmatter_indexed` and `Projector::project` take an optional `KeyIndex`.
- `to_graph_inlines`, `DocumentInline::to_graph_inline`, and `SectionsBuilder::new` take a `&KeyIndex` and resolve each reference to its canonical `Key` as the document is built (markdown links relative to the document, wiki links by path-suffix), so the graph no longer stores the raw as-written wiki target.
## [0.1.10](https://github.com/iwe-org/iwe/compare/liwe-v0.1.9...liwe-v0.1.10) - 2026-05-30

### Fixed

- `DocumentBlock::child_inlines` now returns the inlines of table header and body cells, so `Document::link_at`/`Parser::url_at` resolve links inside table cells (previously returned nothing for tables).
- `MarkdownReader::read` appends a trailing newline to input that lacks one before parsing, so the last block's `LineRange` covers its final line; a multi-line block (such as a table) at the end of a newline-less document previously had its last line excluded from the range.
- `InlineRange` character offsets are now UTF-16 code units instead of Unicode scalar counts, matching LSP position semantics; `Document::link_at` and `DocumentInline::key_range` were previously off by one column per preceding astral-plane character (such as an emoji).
- Fragment-only links like `[text](#header)` no longer produce `[text](.md#header)` when `refs_extension` is set
- Ordered list items use single space after marker (`1. item` instead of `1.  item`)
- Tables no longer produce extra trailing blank line
- Code blocks, tables, blockquotes, and horizontal rules inside a list item are now separated from an adjacent block by a blank line (previously rendered with no separator, e.g. a fenced code block glued directly under the item text)
- `walk_md_paths` joins relative path components with `/` so document keys use forward-slash separators on all platforms (previously preserved Windows backslashes, leaving nested-document keys unmatchable against URI-derived keys)

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