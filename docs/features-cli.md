# How to use in command line

IWE provides a powerful command-line interface for managing markdown-based knowledge graphs. The CLI enables you to initialize projects, normalize documents, explore connections, export visualizations, and create consolidated documents.

## Quick Start

1.  **Initialize a project**: `iwe init`
2.  **Normalize all documents**: `iwe normalize`
3.  **View document paths**: `iwe paths`
4.  **Export graph visualization**: `iwe export dot`

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

## Commands Reference

### `iwe init`

Initializes the current directory as an IWE project.

``` bash
iwe init
```

**What it does:**

- Creates `.iwe/` marker directory
- Generates default `config.toml` configuration
- Sets up the project structure for IWE operations

**Example:**

``` bash
cd ~/my-notes
iwe init
# Creates .iwe/config.toml with default settings
```

### `iwe normalize`

Performs comprehensive document normalization across all markdown files.

``` bash
iwe normalize
```

**Operations performed:**

- Updates link titles to match target document headers
- Adjusts header levels for consistent hierarchy
- Renumbers ordered lists
- Fixes markdown formatting (newlines, indentation)
- Standardizes list formatting
- Normalizes document structure

**Example:**

``` bash
# Basic normalization
iwe normalize

# With debug output (global verbose option)
iwe -v 2 normalize
```

**⚠️ Important:** Always backup your files before running normalization, especially the first time.

### `iwe paths`

Displays all possible navigation paths in your document graph.

``` bash
iwe paths [OPTIONS]
```

**Options:**

- `-d, --depth <DEPTH>`: Maximum path depth to explore (default: 4)
- `-v, --verbose <LEVEL>`: Verbosity level

**Output format:**Shows hierarchical paths through your documents, revealing connection patterns and document relationships.

**Example:**

``` bash
# Show paths up to depth 4
iwe paths

# Show deeper paths
iwe paths --depth 6

# With debug output
iwe paths -v 2 --depth 3
```

### `iwe contents`

Lists root documents (notes without parent references).

``` bash
iwe contents
```

**Purpose:**Identifies entry points in your knowledge graph - documents that aren't referenced by others, potentially serving as main topics or starting points.

**Example:**

``` bash
iwe contents
```

### `iwe squash`

Creates consolidated documents by combining linked content into a single file.

``` bash
iwe squash --key <KEY> [OPTIONS]
```

**Required:**

- `-k, --key <KEY>`: Starting document key/identifier to squash from

**Options:**

- `-d, --depth <DEPTH>`: How deep to traverse links (default: 2)
- `-v, --verbose <LEVEL>`: Verbosity level

**What it does:**

- Starts from the specified document
- Traverses linked documents up to specified depth
- Combines content into a single markdown document
- Converts block references to inline sections
- Maintains document structure and hierarchy

**Examples:**

``` bash
# Squash starting from document "project-overview"
iwe squash --key project-overview

# Squash with greater depth
iwe squash --key main-topic --depth 4

# With debug output
iwe squash --key research-notes --depth 3 -v 2
```

Example [PDF](https://github.com/iwe-org/iwe/blob/master/docs/book.pdf) generated using `squash` command and typst

### `iwe export`

Exports graph structure in various formats for visualization and analysis.

``` bash
iwe export [OPTIONS] <FORMAT>
```

**Available formats:**

- `dot`: Graphviz DOT format for graph visualization

**Options:**

- `-k, --key <KEY>`: Filter to specific document and its connections (default: exports all root notes)
- `-d, --depth <DEPTH>`: Maximum depth to include (default: 0 = unlimited)
- `--include-headers`: Include section headers and create detailed subgraphs
- `-v, --verbose <LEVEL>`: Verbosity level

**DOT Export Examples:**

``` bash
# Export entire graph
iwe export dot

# Export specific document and connections
iwe export dot --key "project-main"

# Include section headers for detailed view
iwe export dot --include-headers

# Export with depth limit and headers
iwe export dot --key "research" --depth 3 --include-headers
```

**Using DOT output:**

``` bash
# Generate PNG visualization
iwe export dot > graph.dot
dot -Tpng graph.dot -o graph.png

# Generate SVG for web use
iwe export dot --include-headers > detailed.dot
dot -Tsvg detailed.dot -o detailed.svg

# Interactive visualization
iwe export dot | dot -Tsvg | firefox /dev/stdin
```

## Workflow Examples

### Daily Maintenance

``` bash
# Update all document formatting and links
iwe normalize

# Check document structure
iwe paths --depth 5
```

### Content Analysis

``` bash
# Find entry points
iwe contents

# Visualize specific topic area
iwe export dot --key "machine-learning" --include-headers > ml.dot
dot -Tpng ml.dot -o ml-graph.png
```

### Document Consolidation

``` bash
# Create comprehensive document from research notes
iwe squash --key "research-index" --depth 4 > consolidated-research.md

# Generate presentation material
iwe squash --key "project-summary" --depth 2 > project-overview.md
```

### Large Library Management

``` bash
# Process with debug information
iwe normalize -v 2

# Analyze complex relationships with debug output
iwe paths --depth 8 -v 2

# Export detailed visualization
iwe export dot --include-headers --depth 5 > full-graph.dot
```

## Configuration

Commands respect settings in `.iwe/config.toml`:

``` toml
[library]
path = ""  # Subdirectory containing markdown files

[markdown]
normalize_headers = true
normalize_lists = true
```

## Best Practices

1.  **Start Small**: Test commands on a few files before processing large libraries
2.  **Backup First**: Always backup before running `normalize` or other bulk operations
3.  **Use Debug Mode**: Add `-v 2` to see detailed debug information about operations being performed
4.  **Iterate Gradually**: Use increasing depth values to explore graph complexity
5.  **Visualize Regularly**: Export graphs to understand document relationships
6.  **Monitor Root Documents**: Use `contents` to track entry points as your library grows

## Troubleshooting

- **No changes after normalize**: Check that files are properly formatted markdown
- **Export produces no output**: Verify documents contain links and references
- **Squash fails**: Ensure the specified key exists and is accessible
