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
iwe find --fuzzy project

iwe retrieve -k project-overview
iwe tree --depth 3

iwe normalize
iwe stats
```

## Commands

### `init`
Initialize an IWE workspace with configuration.

```bash
iwe init
iwe init --dry-run
iwe init -y
```

Detects the conventions of markdown files already in the directory and proposes a matching `.iwe/config.toml`.

Options:
- `-y, --auto` - write the detected configuration without prompting
- `--dry-run` - print the proposed configuration and evidence, write nothing
- `--defaults` - write the static default template without detection
- `--json` - print a machine-readable report

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
- `-k, --key <KEY>` - explicit document key, bypassing the template's key derivation
- `-i, --if-exists <MODE>` - behavior when file exists: `suffix` (default), `override`, `skip`, `fail` (default when `--key` is given)
- `-e, --edit` - open created file in `$EDITOR`

### `find`
Search and discover documents.

```bash
iwe find                           # list all documents
iwe find --fuzzy project           # fuzzy match on title and key
iwe find --lexical "search query"  # full-text match on title and body
iwe find --filter 'status: draft'  # frontmatter filter
iwe find --roots                   # only root documents (no incoming refs)
iwe find --references project      # documents that link to "project"
iwe find --referenced-by project   # documents "project" links to
iwe find -l 10                     # limit results
iwe find -f json                   # output as JSON
```

Options:
- `--fuzzy <QUERY>` - fuzzy match on document title and key
- `--lexical <QUERY>` - full-text (BM25) match on title and body
- `--filter <EXPR>` - filter expression (inline YAML)
- `--roots` - only root documents (no incoming inclusion links)
- `--references <KEY>` - documents that reference this key
- `--referenced-by <KEY>` - documents this key references
- `-l, --limit <N>` - maximum results (0 = unlimited)
- `-f, --format <FORMAT>` - output format: `markdown` (default), `keys`, `json`, `yaml`

### `retrieve`
Retrieve document content with configurable expansion.

```bash
iwe retrieve -k my-document
iwe retrieve -k doc1 -k doc2                 # multiple documents
iwe retrieve -k doc --expand-includes 2      # expand into children 2 levels deep
iwe retrieve -k doc --expand-included-by 1   # include parent context
iwe retrieve -k doc --expand-references 1    # follow outbound reference links
iwe retrieve --lexical "query" --limit 5     # seed by full-text search
iwe retrieve -k doc -e other-doc             # exclude specific documents
iwe retrieve -k doc -f json                  # JSON output
iwe retrieve -k doc --children               # populate the includes edge array
```

Options:
- `-k, --key <KEY>` - document key(s) to retrieve (repeatable)
- `--expand-includes <N>` - expand into child documents to depth N (0 = unbounded)
- `--expand-included-by <N>` - expand into parent documents to depth N (0 = unbounded)
- `--expand-references <N>` - follow outbound reference links to depth N (0 = unbounded)
- `--expand-referenced-by <N>` - follow inbound reference links to depth N (0 = unbounded)
- `--lexical <QUERY>` - seed search: full-text query on title and body
- `--fuzzy <QUERY>` - seed search: fuzzy query on title and key
- `--limit <N>` - cap seed documents kept before expansion (0 = unlimited)
- `-e, --exclude <KEY>` - exclude document key(s) (repeatable)
- `-b, --backlinks` - include incoming references (default: true)
- `-f, --format <FORMAT>` - output format: `markdown` (default), `keys`, `json`, `yaml`
- `--children` - populate the `includes` array with child document edges

### `count`
Count documents matching a filter.

```bash
iwe count
iwe count --filter 'status: draft'
```

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
- `-f, --format <FORMAT>` - output format: `markdown` (default), `keys`, `json`, `yaml`

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
iwe stats -k my-document           # per-document stats
iwe stats similarity               # near-identical page pairs
iwe stats -f csv > stats.csv
```

Options:
- `-k, --key <KEY>` - document key for per-document stats
- `-f, --format <FORMAT>` - output format: `markdown` (default), `csv`, `json`, `yaml`

### `export`
Export knowledge graph in DOT format for visualization.

```bash
iwe export
iwe export -k project              # filter by key
iwe export -d 3                    # limit depth
iwe export --include-headers       # include section headers as subgraphs
```

Options:
- `-f, --format <FORMAT>` - output format: `dot` (default)
- `-k, --key <KEY>` - filter nodes by specific key
- `-d, --depth <N>` - limit traversal depth
- `--include-headers` - include section headers with colored subgraphs

### `rename`
Rename a document key, updating all references across the graph.

```bash
iwe rename old-key new-key
iwe rename old-key new-key --dry-run
iwe rename old-key new-key -f keys # print affected document keys
```

Options:
- `--dry-run` - preview changes without writing to disk
- `--quiet` - suppress progress output
- `-f, --format <FORMAT>` - output format: `markdown` (default), `keys` (affected document keys, one per line)

### `delete`
Delete documents from the knowledge graph.

```bash
iwe delete document-key
iwe delete --filter 'status: archived' --expect 3
iwe delete document-key --dry-run  # preview only
```

Options:
- `--filter <EXPR>` - filter expression (inline YAML); required if positional KEY omitted
- `--expect <ARG>` - assert the number of matched documents before deleting
- `--strict` - require the `--expect` guard
- `--dry-run` - preview changes without writing to disk
- `--quiet` - suppress progress output
- `-f, --format <FORMAT>` - output format: `markdown` (default), `keys`

### `update`
Apply frontmatter and block edits, or overwrite a document body.

```bash
iwe update -k my-document --set status=done
iwe update --filter 'status: draft' --set status=review --dry-run
iwe update -k my-document --content -        # new body from stdin
```

Options:
- `-k, --key <KEY>` - match by document key (repeatable)
- `--filter <EXPR>` - filter expression (inline YAML)
- `--set FIELD=VALUE` / `--unset FIELD` - frontmatter edits
- `--replace`, `--replace-text`, `--insert-before`, `--insert-after`, `--append`, `--delete` - block edit operators
- `-c, --content <TEXT>` - new full markdown content (`-` reads stdin)
- `--expect <ARG>` - assert the number of matched documents
- `--strict` - require an expect guard on every mutating application
- `--dry-run` - preview without writing
- `--quiet` - suppress progress output

### `extract`
Extract a section from a document into a new standalone document.

```bash
iwe extract my-doc --list                  # list sections with block numbers
iwe extract my-doc --section "Overview"    # extract by section title
iwe extract my-doc --block 2               # extract by block number
iwe extract my-doc --section "Notes" --action my-action
```

Options:
- `--section <TITLE>` - section title to extract (case-insensitive)
- `--block <N>` - block number to extract (1-indexed)
- `--list` - list all sections with block numbers
- `--action <NAME>` - action name from config
- `--dry-run` - preview changes without writing to disk
- `--quiet` - suppress progress output
- `-f, --format <FORMAT>` - output format: `markdown` (default), `keys`

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
- `-f, --format <FORMAT>` - output format: `markdown` (default), `keys`

### `attach`
Attach a document to configured targets.

```bash
iwe attach --list
iwe attach -k my-document --to my-action
```

Options:
- `--to <ACTION>` - configured attach action(s) (repeatable)
- `-k, --key <KEY>` - source document key
- `--list` - list configured attach actions
- `--dry-run` - preview without writing
- `--quiet` - suppress progress output

### `schema`
Print inferred document schemas, or validate documents against configured schemas.

```bash
iwe schema
iwe schema validate
iwe schema validate --schema-file my-schema.yaml
```

### `docs`
Print built-in reference documentation.

```bash
iwe docs query
iwe docs config
iwe docs schema
```

### `completions`
Generate shell completions.

```bash
iwe completions zsh
```

Supported shells: `bash`, `elvish`, `fish`, `nushell`, `powershell`, `zsh`.

## Configuration

IWE uses `.iwe/config.toml` for workspace configuration:

```toml
[library]
path = ""

[markdown]
refs_extension = ""
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
