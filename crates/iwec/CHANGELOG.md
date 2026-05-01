# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `iwe_retrieve` accepts `children: bool` to populate the `includes` array independently of `no_content`

### Changed

- `iwe_find` returns a bare array of result objects (the `{query, limit, total, results}` envelope is removed); each result flattens user frontmatter alongside `key`, `title`, `includedBy`
- `iwe_retrieve` returns a bare array of document objects (the `{documents}` envelope is removed); `includes` entries carry `sectionPath`
- `iwe_retrieve` `no_content` no longer implies child population; pass `children: true` for that
- `iwe_tree` always emits `children: []` for leaf nodes
- `review` and `refactor` prompts embed the new array-shaped retrieve JSON

## [0.1.0](https://github.com/iwe-org/iwe/compare/iwec-v0.0.70...iwec-v0.1.0) - 2026-05-01

### Changed

- `find`, `retrieve`, `stats`, and prompt assembly rewired onto the new query engine; tests updated for the new wire format

## [0.0.70](https://github.com/iwe-org/iwe/compare/iwec-v0.0.69...iwec-v0.0.70) - 2026-04-25

### Added

- Add --in structural set selector across read commands ([#269](https://github.com/iwe-org/iwe/pull/269))
- Add time format in addition to date format ([#268](https://github.com/iwe-org/iwe/pull/268))

### Other

- Update readme

## [0.0.68](https://github.com/iwe-org/iwe/compare/iwec-v0.0.67...iwec-v0.0.68) - 2026-04-22

### Fixed

- Index links inside the tables ([#255](https://github.com/iwe-org/iwe/pull/255))
# Changelog

All notable changes to this project will be documented in this file.
