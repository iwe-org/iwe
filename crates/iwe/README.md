# IWE CLI

Command-line interface for IWE - memory system for you and your AI agents.

## Installation

Install from source:
```bash
cargo install --path .
```

Or build locally:
```bash
cargo build --release
```

## Quick start

```bash
mkdir my-notes && cd my-notes
iwe init

iwe new "Project Overview"
iwe new "Meeting Notes" --template meeting

iwe find
iwe find "project"

iwe retrieve -k project-overview
iwe tree --depth 3

iwe normalize
iwe stats
```

## Commands

### `init`
Initialize a new IWE workspace with configuration.

```bash
iwe init
```

Creates `.iwe/config.toml` with default settings for markdown processing.

### `new`
Create a new document in the workspace.

```bash
iwe new "Document Title"
iwe new "Daily Note" --template daily
iwe new "Import" --content "# Imported content"
echo "piped content" | iwe new "From Stdin"
iwe new "Maybe Duplicate" --if-exists skip
```

Options:
- `-t, --template <NAME>` - template name from config
- `-c, --content <TEXT>` - document content
- `-i, --if-exists <MODE>` - behavior when file exists: `suffix` (default), `override`, `skip`
- `-e, --edit` - open created file in `$EDITOR`

### `find`
Search and discover documents.

```bash
iwe find                           # list all documents
iwe find "search query"            # fuzzy match on title and key
iwe find --roots                   # only root documents (no incoming refs)
iwe find --refs-to project         # documents referencing "project"
iwe find --refs-from project       # documents referenced by "project"
iwe find -l 10                     # limit results
iwe find -f json                   # output as JSON
```

Options:
- `--roots` - only root documents (no incoming block refs)
- `--refs-to <KEY>` - documents that reference this key
- `--refs-from <KEY>` - documents referenced by this key
- `-l, --limit <N>` - maximum results
- `-f, --format <FORMAT>` - output format: `markdown` (default), `keys`, `json`

### `retrieve`
Retrieve document content with configurable expansion.

```bash
iwe retrieve -k my-document
iwe retrieve -k doc1 -k doc2       # multiple documents
iwe retrieve -k doc -d 2           # expand refs 2 levels deep
iwe retrieve -k doc -c 0           # no parent context
iwe retrieve -k doc --links        # include inline references
iwe retrieve -k doc -e other-doc   # exclude specific documents
iwe retrieve -k doc -f json        # JSON output
iwe retrieve -k doc --dry-run      # show document count and lines only
iwe retrieve -k doc --no-content   # metadata only
```

Options:
- `-k, --key <KEY>` - document key(s) to retrieve (repeatable)
- `-d, --depth <N>` - follow block refs down N levels (default: 1)
- `-c, --context <N>` - include N levels of parent context (default: 1)
- `-l, --links` - include inline references
- `-b, --backlinks` - include incoming references (default: true)
- `-e, --exclude <KEY>` - exclude document key(s) (repeatable)
- `-f, --format <FORMAT>` - output format: `markdown` (default), `keys`, `json`
- `--dry-run` - show document count and total lines without content
- `--no-content` - exclude document content from results (metadata only)

### `tree`
Display the document hierarchy.

```bash
iwe tree                           # full tree, depth 4
iwe tree -d 2                      # limit depth
iwe tree -k project                # subtree from specific document(s)
iwe tree -f json                   # JSON output
```

Options:
- `-k, --key <KEY>` - filter to paths starting from specific document(s) (repeatable)
- `-d, --depth <N>` - maximum depth to traverse (default: 4)
- `-f, --format <FORMAT>` - output format: `markdown` (default), `keys`, `json`

### `normalize`
Format and normalize all markdown files in the workspace.

```bash
iwe normalize
```

Applies consistent formatting to headers, lists, links, and spacing according to configuration.

### `squash`
Combine content for a specific document key.

```bash
iwe squash document-name
iwe squash project --depth 3
```

Flattens hierarchical content into a single markdown document by expanding block references.

### `stats`
Generate statistics about the knowledge graph.

```bash
iwe stats
iwe stats --format csv
iwe stats -f csv > stats.csv
```

Options:
- `-f, --format <FORMAT>` - output format: `markdown` (default), `csv`

### `export`
Export knowledge graph in DOT format for visualization.

```bash
iwe export dot
iwe export dot --key project       # filter by key
iwe export dot --depth 3           # limit depth
iwe export dot --include-headers   # include section headers as subgraphs
```

Options:
- `-k, --key <KEY>` - filter nodes by specific key
- `-d, --depth <N>` - limit traversal depth
- `--include-headers` - include section headers with colored subgraphs

### `rename`
Rename a document key, updating all references across the graph.

```bash
iwe rename old-key new-key
iwe rename old-key new-key --dry-run
iwe rename old-key new-key --keys  # print affected document keys
```

Options:
- `--dry-run` - preview changes without writing to disk
- `--quiet` - suppress progress output
- `--keys` - print affected document keys (one per line)

### `delete`
Delete a document from the knowledge graph.

```bash
iwe delete document-key
iwe delete document-key --force    # skip confirmation
iwe delete document-key --dry-run  # preview only
```

Options:
- `--dry-run` - preview changes without writing to disk
- `--quiet` - suppress progress output
- `--keys` - print affected document keys (one per line)
- `--force` - skip confirmation prompt

### `extract`
Extract a section from a document into a new standalone document.

```bash
iwe extract my-doc --list                  # list sections with block numbers
iwe extract my-doc --section "Overview"    # extract by section title
iwe extract my-doc --block 2              # extract by block number
iwe extract my-doc --section "Notes" --action my-action
```

Options:
- `--section <TITLE>` - section title to extract (case-insensitive)
- `--block <N>` - block number to extract (1-indexed)
- `--list` - list all sections with block numbers
- `--action <NAME>` - action name from config
- `--dry-run` - preview changes without writing to disk
- `--quiet` - suppress progress output
- `--keys` - print affected document keys (one per line)

### `inline`
Replace a block reference with the content of the referenced document.

```bash
iwe inline my-doc --list                    # list block references
iwe inline my-doc --reference "other-doc"   # inline by reference
iwe inline my-doc --block 1                 # inline by block number
iwe inline my-doc --block 1 --as-quote      # inline as blockquote
iwe inline my-doc --block 1 --keep-target   # keep target document
```

Options:
- `--reference <KEY>` - reference key or title to inline
- `--block <N>` - block number to inline (1-indexed)
- `--list` - list all block references with numbers
- `--action <NAME>` - action name from config
- `--as-quote` - inline as blockquote instead of section
- `--keep-target` - keep the target document after inlining
- `--dry-run` - preview changes without writing to disk
- `--quiet` - suppress progress output
- `--keys` - print affected document keys (one per line)

## Configuration

IWE uses `.iwe/config.toml` for workspace configuration:

```toml
[library]
path = ""

[markdown]
refs_extension = false
```

## Global options

All commands support:
- `-v, --verbose <LEVEL>` - set verbosity (0-2)
- `-h, --help` - show help information
- `-V, --version` - show version

## License

Apache-2.0

## Related projects

- [IWE LSP Server](../iwes/) - Language server for editor integration
- [IWE MCP Server](../iwec/) - MCP server for AI tool integration
- [IWE Core Library](../liwe/) - Core functionality and graph processing
- [VSCode Extension](https://marketplace.visualstudio.com/items?itemName=IWE.iwe)
- [Zed Plugin](https://github.com/iwe-org/zed-iwe)

For more information, visit [iwe.md](https://iwe.md).
