# IWE Export

Exports graph structure in various formats for visualization and analysis.

## Usage

``` bash
iwe export [OPTIONS] <FORMAT>
```

## Available formats

- `dot`: Graphviz DOT format for graph visualization

## Options

- `-k, --key <KEY>`: Filter to specific document and its connections (default: exports all root notes)
- `-d, --depth <DEPTH>`: Maximum depth to include (default: 0 = unlimited)
- `--include-headers`: Include section headers and create detailed subgraphs
- `-v, --verbose <LEVEL>`: Verbosity level

## DOT Export Examples

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

## Using DOT output

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
