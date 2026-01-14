# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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