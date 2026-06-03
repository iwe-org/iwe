# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1](https://github.com/iwe-org/iwe/compare/iwe-v0.3.0...iwe-v0.3.1) - 2026-06-03

Workspace version bump — no user-visible changes in this crate.

## [0.3.0](https://github.com/iwe-org/iwe/compare/iwe-v0.2.0...iwe-v0.3.0) - 2026-06-02

### Added

- `markdown.wiki_link_path` config option (`preserve` | `full` | `short`, default `preserve`) controls how `iwe normalize` and `iwe export` write the path inside a wiki link: `preserve` keeps each link as typed, `full` rewrites to the target's full key path, and `short` rewrites to the shortest unambiguous suffix. `iwe init` now writes the option in the generated config.

### Changed

- `iwe normalize` now recognizes task-list markers in list items (`- [ ]`, `- [x]`) and normalizes `[X]` to lowercase `[x]`
- List items are now a distinct node type rather than sections, so `iwe stats` no longer counts them toward the section total and `iwe extract` no longer lists them as extractable sections (section and `--block` numbers shift accordingly)

### Fixed

- Wiki link shortening no longer rewrites a link whose target is missing from the document set onto an unrelated document that shares the same file name; such links keep their full path.
## [0.2.0](https://github.com/iwe-org/iwe/compare/iwe-v0.1.10...iwe-v0.2.0) - 2026-06-02

### Added

- `markdown.formatting.ordered_list_content_indent` and `markdown.formatting.bullet_list_content_indent` config options set the minimum indentation for list item content and continuation lines (accepts `2`–`4`); set either to `4` for MkDocs-style alignment (`1.  item` / `-   item` with 4-space continuation) instead of the default single space after the marker

### Fixed

- `iwe normalize` now renders a list as loose (a blank line between items) when any item contains a code block, table, blockquote, or horizontal rule, so a following item is no longer glued directly under the preceding item's block (previously only items with multiple paragraphs triggered loose rendering)

## [0.1.10](https://github.com/iwe-org/iwe/compare/iwe-v0.1.9...iwe-v0.1.10) - 2026-05-30

### Fixed

- `iwe normalize` now inserts a blank line between a list item's text and an adjacent code block, table, blockquote, or horizontal rule (previously the block was glued directly under the item text)

## [0.1.9](https://github.com/iwe-org/iwe/compare/iwe-v0.1.8...iwe-v0.1.9) - 2026-05-27

Workspace version bump — no user-visible changes in this crate.

## [0.1.8](https://github.com/iwe-org/iwe/compare/iwe-v0.1.7...iwe-v0.1.8) - 2026-05-23

### Added

- `iwe normalize` honors three new `[markdown.formatting]` options: `wrap_column` wraps paragraphs at the configured column, `preserve_line_breaks` keeps hard line breaks instead of dropping them, and `line_break_style` (`"backslash"` | `"spaces"`, default `"backslash"`) selects how preserved breaks are emitted.

## [0.1.7](https://github.com/iwe-org/iwe/compare/iwe-v0.1.6...iwe-v0.1.7) - 2026-05-20

Workspace version bump — no user-visible changes in this crate.

## [0.1.6](https://github.com/iwe-org/iwe/compare/iwe-v0.1.5...iwe-v0.1.6) - 2026-05-17

Workspace version bump — no user-visible changes in this crate.

## [0.1.5](https://github.com/iwe-org/iwe/compare/iwe-v0.1.4...iwe-v0.1.5) - 2026-05-16

### Fixed

- `iwe normalize` preserves links to non-markdown files (e.g. `foo.html`, `foo.pdf`) instead of appending `.md` to them
- `iwe attach` writes the link with a path relative to the target file's directory and honours `markdown.refs_extension`

### Changed

- `iwe attach` creates new target documents from `document_template` (was a synthesised `# <action title>` heading)

## [0.1.4](https://github.com/iwe-org/iwe/compare/iwe-v0.1.3...iwe-v0.1.4) - 2026-05-15

### Added

- `iwe completions <SHELL>` subcommand — prints a shell completion script to stdout for `bash`, `elvish`, `fish`, `nushell`, `powershell`, or `zsh`

### Fixed

- `iwe normalize` no longer corrupts links that contain a fragment anchor when `refs_extension` is set — the extension was being appended after the fragment, producing malformed URLs

## [0.1.3](https://github.com/iwe-org/iwe/compare/iwe-v0.1.2...iwe-v0.1.3) - 2026-05-05

Workspace version bump — no user-visible changes in this crate.

## [0.1.2](https://github.com/iwe-org/iwe/compare/iwe-v0.1.1...iwe-v0.1.2) - 2026-05-04

### Changed

- `--filter` accepts the natural form `{type: tracker, $or: [...]}` directly — bare field keys may be mixed with `$and`/`$or`/`$nor`/`$key`/graph operators at the filter root and inside logical-operator branches, combining via implicit AND (previously rejected; required the explicit `{$and: [{type: tracker}, {$or: [...]}]}` rewrite).
- `--not-in KEY` deprecation warning now points to `--filter '$nor: [{ $includedBy: ... }]'` (was: `--filter '$not: { $includedBy: ... }'`).

### Removed

- Top-level `$not` in `--filter` expressions. `$not` is now field-level only (matching MongoDB): `--filter 'priority: { $not: { $gt: 5 } }'` still works; `--filter '$not: { status: archived }'` is now a parse-time error and should be rewritten as `--filter '$nor: [{ status: archived }]'`. The error message points to `$nor`.

## [0.1.1](https://github.com/iwe-org/iwe/compare/iwe-v0.1.0...iwe-v0.1.1) - 2026-05-03

### Added

- `iwe schema` command for frontmatter structure analysis — emits per-field type distribution, coverage, and distinct values; supports `-f markdown|json|yaml`, `--field NAME` to scope output, and the universal filter flags ([#274](https://github.com/iwe-org/iwe/pull/274))

## [0.1.0](https://github.com/iwe-org/iwe/compare/iwe-v0.0.70...iwe-v0.1.0) - 2026-05-01

### Added

- `iwe count` command — returns an integer count of matched documents, mirroring the `find` filter semantics
- Universal `--filter "<YAML>"` flag for inline query expressions on `find`, `count`, `retrieve`, `tree`, `export`, `delete`, and `update`
- Structural anchor flags — `-k/--key` (repeatable), `--includes`, `--included-by`, `--references`, `--referenced-by`, with `KEY[:DEPTH]` syntax
- `--max-depth` and `--max-distance` defaults applied to anchor flags lacking an explicit colon-suffix
- `--project f1,f2` and `-f json` on `find`, `tree`, and `retrieve` for projecting frontmatter fields into structured output
- `iwe update` command with body-overwrite (`-k -c`) and frontmatter mutation (`--filter` + `--set`/`--unset`) modes, plus `--dry-run`
- `retrieve --children` flag to populate the `includes` array independently of `--no-content`
- `retrieve --dry-run` honors `-f json|yaml` and emits a structured `{documents, lines}` object in those formats
- `tree --project f1,f2` to add user frontmatter fields to each tree node alongside `key`, `title`, `children`

### Changed

- Help text refreshed across `count`, `delete`, `extract`, `find`, `inline`, `rename`, `retrieve`, `stats`, `tree`, and `update`
- `find` JSON/YAML output is now a bare array of result objects (the `{query, limit, total, results}` envelope is removed)
- `retrieve` JSON/YAML output is now a bare array of document objects (the `{documents}` envelope is removed)
- `find` result objects flatten user frontmatter at the top level alongside `key`, `title`, `includedBy`; the nested `frontmatter` object is removed
- `retrieve` `includes` entries now carry `sectionPath` (unified `EdgeRef` shape with `includedBy` and `referencedBy`)
- `retrieve --no-content` no longer populates `includes` — use `--children` for that, and combine with `--no-content` for metadata-only output with edges
- `tree` JSON/YAML always emits `children: []` for leaf nodes (previously omitted)
- Markdown frontmatter rendered by `retrieve` uses `includedBy` / `referencedBy` instead of `parents` / `back-links`
- `stats -k KEY` rejects `-f markdown` and `-f csv` at parse time (was silently falling through to JSON)

### Removed

- `--roots` flag — removed

### Deprecated

- `--in`, `--in-any`, `--not-in`, `--refs-to`, `--refs-from` retained as hidden aliases for the new spec-named structural anchor flags

## [0.0.70](https://github.com/iwe-org/iwe/compare/iwe-v0.0.69...iwe-v0.0.70) - 2026-04-25

### Added

- Add --in structural set selector across read commands ([#269](https://github.com/iwe-org/iwe/pull/269))
- Add time format in addition to date format ([#268](https://github.com/iwe-org/iwe/pull/268))

### Other

- Update readme

## [0.0.68](https://github.com/iwe-org/iwe/compare/iwe-v0.0.67...iwe-v0.0.68) - 2026-04-22

### Fixed

- Index links inside the tables ([#255](https://github.com/iwe-org/iwe/pull/255))

## [0.0.66](https://github.com/iwe-org/iwe/compare/iwe-v0.0.65...iwe-v0.0.66) - 2026-04-04

### Added

- List broken links in the stats command output  ([#252](https://github.com/iwe-org/iwe/pull/252))

## [0.0.65](https://github.com/iwe-org/iwe/compare/iwe-v0.0.64...iwe-v0.0.65) - 2026-03-28

### Added

- Local dates and time components in the templates ([#245](https://github.com/iwe-org/iwe/pull/245))

## [0.0.63](https://github.com/iwe-org/iwe/compare/iwe-v0.0.62...iwe-v0.0.63) - 2026-03-20

### Added

- Search by document title, parent document titles and the document key instead of document path ([#231](https://github.com/iwe-org/iwe/pull/231))

### Other

- Removing unwarp's for stability and code style improvements ([#229](https://github.com/iwe-org/iwe/pull/229))

## [0.0.62](https://github.com/iwe-org/iwe/compare/iwe-v0.0.61...iwe-v0.0.62) - 2026-03-19

### Added

- [**breaking**] CLI tree command for documents hierarchy exploration ([#228](https://github.com/iwe-org/iwe/pull/228))
- CLI commands for graph transformations ([#227](https://github.com/iwe-org/iwe/pull/227))

## [0.0.61](https://github.com/iwe-org/iwe/compare/iwe-v0.0.60...iwe-v0.0.61) - 2026-03-16

### Other

- update Cargo.lock dependencies

## [0.0.59](https://github.com/iwe-org/iwe/compare/iwe-v0.0.58...iwe-v0.0.59) - 2026-01-10

### Other

- update Cargo.lock dependencies

## [0.0.58](https://github.com/iwe-org/iwe/compare/iwe-v0.0.57...iwe-v0.0.58) - 2026-01-10

### Added

- `iwe new` command ([#201](https://github.com/iwe-org/iwe/pull/201))

## [0.0.56](https://github.com/iwe-org/iwe/compare/iwe-v0.0.55...iwe-v0.0.56) - 2025-11-11

### Other

- Lint fixes ([#182](https://github.com/iwe-org/iwe/pull/182))
- Fix test on release only target ([#181](https://github.com/iwe-org/iwe/pull/181))

## [0.0.51](https://github.com/iwe-org/iwe/compare/iwe-v0.0.50...iwe-v0.0.51) - 2025-10-14

### Added

- Statistics in CSV and Markdown formats ([#166](https://github.com/iwe-org/iwe/pull/166))

## [0.0.46](https://github.com/iwe-org/iwe/compare/iwe-v0.0.45...iwe-v0.0.46) - 2025-09-20

### Other

- update Cargo.toml dependencies

## [0.0.44](https://github.com/iwe-org/iwe/compare/iwe-v0.0.43...iwe-v0.0.44) - 2025-09-07

### Added

- Honor .gitignore files ([#141](https://github.com/iwe-org/iwe/pull/141))
- Include/exclude headers structure in DOT exports ([#120](https://github.com/iwe-org/iwe/pull/120))

## [0.0.43](https://github.com/iwe-org/iwe/compare/iwe-v0.0.42...iwe-v0.0.43) - 2025-09-05

### Added

- Add --verbose flag for CLI and more debug logs ([#137](https://github.com/iwe-org/iwe/pull/137))

## [0.0.42](https://github.com/iwe-org/iwe/compare/iwe-v0.0.41...iwe-v0.0.42) - 2025-09-04

### Other

- Update Cargo.lock dependencies

## [0.0.41](https://github.com/iwe-org/iwe/compare/iwe-v0.0.40...iwe-v0.0.41) - 2025-09-01

### Fixed

- Do not remove extensions from local links ([#132](https://github.com/iwe-org/iwe/pull/132))

## [0.0.40](https://github.com/iwe-org/iwe/compare/iwe-v0.0.39...iwe-v0.0.39) - 2025-08-31

### Added

- Customizable "Attach" code action for documents linking ([#128](https://github.com/iwe-org/iwe/pull/128))

## [0.0.39](https://github.com/iwe-org/iwe/compare/iwe-v0.0.38...iwe-v0.0.39) - 2025-08-28

### Fixed

- Code action should not remove YAML metadata ([#127](https://github.com/iwe-org/iwe/pull/127))

## [0.0.37](https://github.com/iwe-org/iwe/compare/iwe-v0.0.36...iwe-v0.0.37) - 2025-08-27

### Added

- Include/exclude headers structure in DOT exports ([#120](https://github.com/iwe-org/iwe/pull/120))

### Fixed

- Ignore non alphanumeric chars in search ([#119](https://github.com/iwe-org/iwe/pull/119))

## [0.0.35](https://github.com/iwe-org/iwe/compare/iwe-v0.0.34...iwe-v0.0.35) - 2025-08-21

### Added

- DOT styles ([#114](https://github.com/iwe-org/iwe/pull/114))

## [0.0.34](https://github.com/iwe-org/iwe/compare/iwe-v0.0.33...iwe-v0.0.34) - 2025-08-18

### Added

- Graphviz DOT format export support ([#109](https://github.com/iwe-org/iwe/pull/109))

## [0.0.32](https://github.com/iwe-org/iwe/compare/iwe-v0.0.31...iwe-v0.0.32) - 2025-05-31

### Other

- update Cargo.toml dependencies

## [0.0.30](https://github.com/iwe-org/iwe/compare/iwe-v0.0.29...iwe-v0.0.30) - 2025-03-30

### Other

- update Cargo.lock dependencies

## [0.0.29](https://github.com/iwe-org/iwe/compare/iwe-v0.0.28...iwe-v0.0.29) - 2025-03-29

### Fixed

- List item with dual dash "- -" causing panic ([#92](https://github.com/iwe-org/iwe/pull/92))

## [0.0.28](https://github.com/iwe-org/iwe/compare/iwe-v0.0.27...iwe-v0.0.28) - 2025-03-30

### Added

- Custom LLM code actions support for context aware updates ([#90](https://github.com/iwe-org/iwe/pull/90))

## [0.0.27](https://github.com/iwe-org/iwe/compare/iwe-v0.0.26...iwe-v0.0.27) - 2025-03-08

### Added

- Tables support ([#77](https://github.com/iwe-org/iwe/pull/77))

## [0.0.25](https://github.com/iwe-org/iwe/compare/iwe-v0.0.24...iwe-v0.0.25) - 2025-02-24

### Added

- Sub-directories support (#71)

## [0.0.24](https://github.com/iwe-org/iwe/compare/iwe-v0.0.23...iwe-v0.0.24) - 2025-02-17

### Other

- update Cargo.lock dependencies

## [0.0.23](https://github.com/iwe-org/iwe/compare/iwe-v0.0.22...iwe-v0.0.23) - 2025-02-17

### Other

- update Cargo.lock dependencies

## [0.0.22](https://github.com/iwe-org/iwe/compare/iwe-v0.0.21...iwe-v0.0.22) - 2025-02-17

### Added

- Better search results ([#61](https://github.com/iwe-org/iwe/pull/61))

## [0.0.19](https://github.com/iwe-org/iwe/compare/iwe-v0.0.18...iwe-v0.0.19) - 2025-02-16

### Added

- wiki links support (#52)