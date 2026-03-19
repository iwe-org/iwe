# How to Use in Command Line

IWE provides a powerful command-line interface for managing markdown-based knowledge graphs. The CLI enables you to initialize projects, normalize documents, explore connections, export visualizations, and create consolidated documents.

## Quick Start

1.  **Initialize a project**: `iwe init`
2.  **Create a new document**: `iwe new "My Note"`
3.  **Retrieve a document with context**: `iwe retrieve -k my-note`
4.  **Find and search documents**: `iwe find "search term"`
5.  **Normalize all documents**: `iwe normalize`
6.  **View document hierarchy**: `iwe tree`
7.  **Analyze your knowledge base**: `iwe stats`
8.  **Export graph visualization**: `iwe export dot`
9.  **Rename a document**: `iwe rename old-key new-key`
10. **Delete a document**: `iwe delete document-key`
11. **Extract a section**: `iwe extract document --section "Title"`
12. **Inline a reference**: `iwe inline document --reference "other-doc"`

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

## Command Categories

### Document Management

| Command | Description | Documentation |
|---------|-------------|---------------|
| `init` | Initialize a new IWE project | [cli-init](cli-init.md) |
| `new` | Create a new document | [cli-new](cli-new.md) |
| `normalize` | Normalize all documents | [cli-normalize](cli-normalize.md) |

### Document Retrieval

| Command | Description | Documentation |
|---------|-------------|---------------|
| `retrieve` | Retrieve document with context | [cli-retrieve](cli-retrieve.md) |
| `find` | Search and discover documents | [cli-find](cli-find.md) |
| `tree` | Display document hierarchy | [cli-tree](cli-tree.md) |

### Refactoring Operations

| Command | Description | Documentation |
|---------|-------------|---------------|
| `rename` | Rename a document and update references | [cli-rename](cli-rename.md) |
| `delete` | Delete a document and clean up references | [cli-delete](cli-delete.md) |
| `extract` | Extract a section to a new document | [cli-extract](cli-extract.md) |
| `inline` | Inline a referenced document | [cli-inline](cli-inline.md) |

### Analysis & Export

| Command | Description | Documentation |
|---------|-------------|---------------|
| `stats` | Analyze knowledge base statistics | [cli-stats](cli-stats.md) |
| `export` | Export graph visualization | [cli-export](cli-export.md) |
| `squash` | Squash documents | [cli-squash](cli-squash.md) |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success - command completed without errors |
| `1` | Error - invalid arguments, missing files, operation failed |

All commands return exit code `0` on success. On error, commands print a message to stderr and return exit code `1`.

``` bash
# Check exit code
iwe find "nonexistent-query"
echo $?  # Returns 0 (empty result is not an error)

iwe retrieve --key "missing-doc"
echo $?  # Returns 1 (document not found is an error)
```
