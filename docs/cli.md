# How to Use in Command Line

IWE provides a powerful command-line interface for managing markdown-based knowledge graphs. The CLI enables you to initialize projects, normalize documents, explore connections, export visualizations, and create consolidated documents.

## Quick Start

1.  **Initialize a project**: `iwe init`
2.  **Create a new document**: `iwe new "My Note"`
3.  **Retrieve a document with context**: `iwe retrieve -k my-note`
4.  **Find and search documents**: `iwe find "search term"`
5.  **Count matching documents**: `iwe count --filter 'status: draft'`
6.  **Normalize all documents**: `iwe normalize`
7.  **View document hierarchy**: `iwe tree`
8.  **Analyze your knowledge base**: `iwe stats`
9.  **Export graph visualization**: `iwe export -f dot`
10. **Rename a document**: `iwe rename old-key new-key`
11. **Delete a document**: `iwe delete document-key`
12. **Bulk delete by filter**: `iwe delete --filter 'status: archived'`
13. **Extract a section**: `iwe extract document --section "Title"`
14. **Inline a reference**: `iwe inline document --reference "other-doc"`
15. **Overwrite a document body**: `iwe update -k document-key -c "new content"`
16. **Mutate frontmatter**: `iwe update --filter 'status: draft' --set reviewed=true`
17. **Attach via configured action**: `iwe attach --to today -k document-key`

## Installation & Setup

Before using the CLI, ensure IWE is installed and available in your PATH. Initialize any directory as an IWE project:

``` bash
cd your-notes-directory
iwe init
```

This creates a `.iwe/` directory with configuration files.

## Global Usage

``` bash
iwe [OPTIONS] <COMMAND>
```

### Global Options

- `-V`, `--version`: Display version information
- `-v`, `--verbose <LEVEL>`: Set verbosity level (default: 0)
  - `1`: Minimal output (INFO level messages to stderr)
  - `2` or higher: Debug-level information to stderr
- `-h`, `--help`: Show help information

## Configuration

Commands respect settings in `.iwe/config.toml`:

``` toml
[library]
path = ""  # Subdirectory containing markdown files

[markdown]
normalize_headers = true
normalize_lists = true
```

## Querying

`find`, `count`, `update`, and `delete` accept the same YAML-based filter language. Read-only commands (`retrieve`, `tree`, `export`) accept the filter flags as a selector to narrow what they operate on.

The two entry points:

- `--filter "EXPR"` — inline YAML filter document.
- Structural anchor flags — `-k`, `--includes`, `--included-by`, `--references`, `--referenced-by` (all repeatable, ANDed at the top level), with optional `KEY[:DEPTH]` / `KEY[:DIST]` colon-suffix.

See the [Query Language](query-language.md) reference for the YAML syntax, operators, and examples.

## Command Categories

### Document Management

| Command     | Description                                                  | Documentation                     |
| ----------- | ------------------------------------------------------------ | --------------------------------- |
| `init`      | Initialize a new IWE project                                 | [IWE Init](cli-init.md)           |
| `new`       | Create a new document                                        | [IWE New](cli-new.md)             |
| `update`    | Overwrite a document body, or mutate frontmatter via filter  | [IWE Update](cli-update.md)       |
| `normalize` | Normalize all documents                                      | [IWE Normalize](cli-normalize.md) |


### Document Retrieval

| Command    | Description                              | Documentation                   |
| ---------- | ---------------------------------------- | ------------------------------- |
| `retrieve` | Retrieve document with context           | [IWE Retrieve](cli-retrieve.md) |
| `find`     | Search and discover documents            | [IWE Find](cli-find.md)         |
| `count`    | Count documents matching a filter        | [IWE Count](cli-count.md)       |
| `tree`     | Display document hierarchy               | [IWE Tree](cli-tree.md)         |


### Refactoring Operations

| Command   | Description                               | Documentation                 |
| --------- | ----------------------------------------- | ----------------------------- |
| `rename`  | Rename a document and update references   | [IWE Rename](cli-rename.md)   |
| `delete`  | Delete a document and clean up references | [IWE Delete](cli-delete.md)   |
| `extract` | Extract a section to a new document       | [IWE Extract](cli-extract.md) |
| `inline`  | Inline a referenced document              | [IWE Inline](cli-inline.md)   |


### Analysis & Export

| Command  | Description                       | Documentation               |
| -------- | --------------------------------- | --------------------------- |
| `stats`  | Analyze knowledge base statistics | [IWE Stats](cli-stats.md)   |
| `export` | Export graph visualization        | [IWE Export](cli-export.md) |
| `squash` | Squash documents                  | [IWE Squash](cli-squash.md) |


## Exit Codes

| Code | Meaning                                                    |
| ---- | ---------------------------------------------------------- |
| `0`  | Success - command completed without errors                 |
| `1`  | Error - invalid arguments, missing files, operation failed |


All commands return exit code `0` on success. On error, commands print a message to stderr and return exit code `1`.

``` bash
# Check exit code
iwe find "nonexistent-query"
echo $?  # Returns 0 (empty result is not an error)

iwe retrieve --key "missing-doc"
echo $?  # Returns 1 (document not found is an error)
```
