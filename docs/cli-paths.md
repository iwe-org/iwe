# IWE Paths

Displays all possible navigation paths in your document graph.

## Usage

``` bash
iwe paths [OPTIONS]
```

## Options

- `-d, --depth <DEPTH>`: Maximum path depth to explore (default: 4)
- `-v, --verbose <LEVEL>`: Verbosity level

## Output format

Shows hierarchical paths through your documents, revealing connection patterns and document relationships.

## Examples

``` bash
# Show paths up to depth 4
iwe paths

# Show deeper paths
iwe paths --depth 6

# With debug output
iwe paths -v 2 --depth 3
```
