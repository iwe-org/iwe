# IWE CLI

Command-line interface for IWE (IDE for Writing) - a local-first, markdown-based knowledge management tool designed for developers.

## Installation

Install from source:
```bash
cargo install --path .
```

Or build locally:
```bash
cargo build --release
```

## Quick Start

```bash
# Initialize a new workspace
mkdir my-notes && cd my-notes
iwe init

# Add some markdown files
echo "# Project Overview" > overview.md
echo "# Meeting Notes\n## Daily Standup" > meetings.md

# Normalize formatting
iwe normalize

# Generate table of contents
iwe contents

# Explore knowledge graph paths
iwe paths

# Export for visualization
iwe export dot > graph.dot
```

## Commands

### `init`
Initialize a new IWE workspace with configuration.

```bash
iwe init
```

Creates `.iwe/config.toml` with default settings for markdown processing.

### `normalize`
Format and normalize all markdown files in the workspace.

```bash
iwe normalize
```

Applies consistent formatting to headers, lists, links, and spacing according to configuration.

### `paths`
List knowledge graph paths through document structure.

```bash
iwe paths                    # Default depth: 4
iwe paths --depth 2         # Limit traversal depth
```

Shows navigation paths through your content hierarchy.

### `squash`
Combine content for a specific document key.

```bash
iwe squash --key document-name         # Default depth: 2
iwe squash --key project --depth 3     # Custom depth
```

Flattens hierarchical content into a single markdown document.

### `contents`
Generate a table of contents for the workspace.

```bash
iwe contents
```

Creates markdown links to all top-level documents.

### `export`
Export knowledge graph in various formats.

```bash
iwe export json                        # JSON format
iwe export dot                         # DOT format
iwe export json --key project          # Filter by key
iwe export dot --depth 3               # Limit depth
```

## Configuration

IWE uses `.iwe/config.toml` for workspace configuration:

```toml
[library]
path = ""  # Relative path to markdown files

[markdown]
normalize_headers = true
normalize_lists = true
```

## Features

- **Fast Processing**: Handle thousands of documents in seconds
- **Knowledge Graph**: Understand document relationships and structure
- **Flexible Export**: JSON and DOT formats for integration
- **Consistent Formatting**: Automatic markdown normalization
- **Hierarchical Organization**: Support for nested content structures
- **Link Management**: Automatic link title updates

## Examples

### Basic Workflow
```bash
# Set up workspace
iwe init

# Process and format files
iwe normalize

# Understand structure
iwe paths --depth 3
iwe contents

# Generate reports
iwe squash --key meetings > all-meetings.md
```

### Integration with Other Tools
```bash
# Visualize with Graphviz
iwe export dot | dot -Tpng > knowledge-graph.png

# Process with jq
iwe export json | jq '.[] | select(.title | contains("project"))'

# Batch processing
for file in *.md; do
    key=$(basename "$file" .md)
    iwe squash --key "$key" > "compiled-$key.md"
done
```

## Global Options

All commands support:
- `-v, --verbose <LEVEL>`: Set verbosity (0-2)
- `-h, --help`: Show help information
- `-V, --version`: Show version

## Use Cases

- **Documentation Management**: Keep technical docs organized and formatted
- **Research Notes**: Connect and explore related concepts
- **Meeting Records**: Combine distributed notes into comprehensive documents
- **Knowledge Base**: Build searchable, linked information systems
- **Content Publishing**: Generate clean, formatted output for sharing

## Performance

IWE is optimized for large document collections:
- Processes thousands of files in seconds
- Efficient graph traversal algorithms
- Minimal memory footprint
- Parallel processing where beneficial

## Integration

Works well with:
- **Git**: Track changes to normalized markdown
- **VSCode/Neovim**: Use with IWE LSP for full IDE experience
- **Static Site Generators**: Clean, consistent markdown output
- **Documentation Tools**: Export to various formats
- **Graph Visualization**: DOT/Graphviz, Gephi, etc.

## License

Apache-2.0

## Related Projects

- [IWE LSP Server](../iwes/) - Language server for editor integration
- [IWE Core Library](../liwe/) - Core functionality and graph processing
- [VSCode Extension](https://marketplace.visualstudio.com/items?itemName=IWE.iwe)
- [Zed Plugin](https://github.com/iwe-org/zed-iwe)

For more information, visit [iwe.md](https://iwe.md).
