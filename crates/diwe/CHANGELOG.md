# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `diwe` is the IWE engine library carved out of `liwe`. It carries the app-facing layer: `find` (BM25 / fuzzy search), `retrieve` (document expansion with token budgeting), `stats`, `tokens`, `fs` (filesystem / workspace loading), `loader::from_path`, and the `.iwe/config.toml` mapping (`config::Configuration`, `config::load_config`). It depends on `liwe` for the document kernel and re-exports `liwe`'s format/option types from `diwe::config`.
- `search` (the BM25 index) and `search_query` (BM25 + fuzzy resolvers, RRF fusion, `build_index`, `ranked` / `matched`, and an `execute` wrapper that resolves a query's `search` clause into scores and injects them into the `liwe` engine). `DocumentFinder::with_index` takes a caller-built index.
