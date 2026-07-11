# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `[schemas]` config binding — `config::SchemaBinding` and `config::Patterns` types and the `Configuration.schemas` map bind document schemas to document keys by glob (a single glob or a list). The new `schema` module resolves and runs them: `schema::SchemaBindings` matches a key to its schema names, and `schema::validate_documents` compiles the bound schema files, validates a set of documents, and returns a `schema::KeyReport` per `(key, schema)` with violations.
- `schema::validate_pending_documents` (and `schema::validate_pending_documents_in`, which takes an explicit schemas directory) validate a set of pending `(Key, content)` documents against their bound schemas by building a throwaway graph, so a change can be checked before it is written. `schema::pending_from_changes` collects the touched documents from a `Changes` set, `schema::render_reports_text` renders a `KeyReport` list as text, and `config::schemas_dir_in` resolves the schemas directory under a given base path.

## [0.11.0] - 2026-07-10

### Added
- `diwe` is the IWE engine library carved out of `liwe`. It carries the app-facing layer: `find` (BM25 / fuzzy search), `retrieve` (document expansion with token budgeting), `stats`, `tokens`, `fs` (filesystem / workspace loading), `graph_from_path`, and the `.iwe/config.toml` mapping (`config::Configuration`, `config::load_config`). It depends on `liwe` for the document kernel and re-exports `liwe`'s format/option types from `diwe::config`.
- `search` (the BM25 index) and `search_query` (BM25 + fuzzy resolvers, RRF fusion, `build_index`, `ranked` / `matched`, and an `execute` wrapper that resolves a query's `search` clause into scores and injects them into the `liwe` engine). `DocumentFinder::with_index` takes a caller-built index.
- `fs::apply_changes` — write a `Changes` set to a workspace (creates, updates, and removals), pruning any parent directories left empty by a removal.
