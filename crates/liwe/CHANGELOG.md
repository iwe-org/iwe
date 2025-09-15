# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.46](https://github.com/iwe-org/iwe/compare/liwe-v0.0.45...liwe-v0.0.46) - 2025-09-15

### Added

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