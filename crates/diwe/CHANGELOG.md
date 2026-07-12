# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `config::RefsText` re-export and the `refs_text` field it sits on (`MarkdownOptions`/`DjotOptions`) — selects whether a markdown link's text is preserved (default) or normalized to the linked document's title.
- `stats` findings functions — `graph_findings` (whole-store orphan and dangling-link `Finding`s, discriminated by `Rule`) and `mutation_findings` (the same plus a similar-page check for the created/updated keys), with `orphan_keys` and the now-public `broken_links` behind them.
- `stats::SimilarityIndex` — a search index plus per-key token counts, built once per run (`SimilarityIndex::build`) and reused for `similar(key)` (near-identical pages for one key, as `stats::SimilarPage { key, score }`) and `pairs()` (every mutually-similar pair across the store, each once, computed concurrently). Duplicate detection uses mutual BM25 similarity with a token-size gate and a high threshold.
- `GraphStatistics.orphans` — the list of orphan keys behind the existing `orphaned_documents` count; `stats::KeyStatisticsReport` pairs a `KeyStatistics` with its similar pages. `index` pages (root `index` or any `<dir>/index`) are treated as intentional entry points and excluded from both the orphan list and the count.
- `search::Bm25Index` point-score API — `similar_to(key, floor)` (documents whose self-normalized score against `key`'s own embedding clears a floor, self excluded), `self_score(key)`, and `score_between(query_key, doc_key)`; `search_query::corpus_text` is now public.
- `[schemas]` config binding — `config::SchemaBinding` and `config::Patterns` types and the `Configuration.schemas` map bind document schemas to document keys by glob (a single glob or a list). The new `schema` module resolves and runs them: `schema::SchemaBindings` matches a key to its schema names, and `schema::validate_documents` compiles the bound schema files, validates a set of documents, and returns a `schema::KeyReport` per `(key, schema)` with violations.
- `schema::validate_pending_documents` (and `schema::validate_pending_documents_in`, which takes an explicit schemas directory) validate a set of pending `(Key, content)` documents against their bound schemas by building a throwaway graph, so a change can be checked before it is written. `schema::pending_from_changes` collects the touched documents from a `Changes` set, `schema::render_reports_text` renders a `KeyReport` list as text, and `config::schemas_dir_in` resolves the schemas directory under a given base path.
- `schema::validate_documents_against_file` — validate documents against one schema file directly, ignoring the `[schemas]` config bindings; reports are keyed by the file's stem.

### Changed
- `search` now orders tied scores deterministically (score descending, then key ascending); previously ties came back in arbitrary order.

## [0.11.0] - 2026-07-10

### Added
- `diwe` is the IWE engine library carved out of `liwe`. It carries the app-facing layer: `find` (BM25 / fuzzy search), `retrieve` (document expansion with token budgeting), `stats`, `tokens`, `fs` (filesystem / workspace loading), `graph_from_path`, and the `.iwe/config.toml` mapping (`config::Configuration`, `config::load_config`). It depends on `liwe` for the document kernel and re-exports `liwe`'s format/option types from `diwe::config`.
- `search` (the BM25 index) and `search_query` (BM25 + fuzzy resolvers, RRF fusion, `build_index`, `ranked` / `matched`, and an `execute` wrapper that resolves a query's `search` clause into scores and injects them into the `liwe` engine). `DocumentFinder::with_index` takes a caller-built index.
- `fs::apply_changes` — write a `Changes` set to a workspace (creates, updates, and removals), pruning any parent directories left empty by a removal.
