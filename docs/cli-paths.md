# IWE Paths

Displays all possible navigation paths in your document graph.

## Usage

``` bash
iwe paths [OPTIONS]
```

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `-d, --depth <DEPTH>` | `4` | Maximum path depth to explore |
| `-v, --verbose <LEVEL>` | `0` | Verbosity level (1=info, 2=debug) |

## Output Format

Paths are displayed with nodes separated by ` • ` (bullet separator), showing how documents connect through the graph:

```
Project Overview
Project Overview • Goals
Project Overview • Goals • Q1 Objectives
Project Overview • Tasks
Project Overview • Tasks • Backend
Project Overview • Tasks • Frontend
Research Topics
Research Topics • Machine Learning
Research Topics • Machine Learning • Neural Networks
```

Each line represents a navigation path from a root document to a specific section or referenced document.

## Understanding Paths

A path represents the hierarchical journey through your knowledge graph:

- **Single-node paths** (e.g., `Project Overview`) are root documents
- **Multi-node paths** show the chain of connections from root to leaf
- **Path depth** indicates how many links you follow to reach a document

## Examples

``` bash
# Show paths up to default depth (4)
iwe paths

# Show shallow paths only (direct children)
iwe paths --depth 2

# Show deeper relationships
iwe paths --depth 6

# With debug output
iwe paths -v 2 --depth 3
```

## Depth Impact

| Depth | Shows |
|-------|-------|
| 1 | Root documents only |
| 2 | Roots and their direct children |
| 3 | Up to grandchildren |
| 4+ | Deeper nested relationships |

Higher depths reveal more of your knowledge structure but produce more output.

## AI Agent Tips

- Use `paths` to understand the overall structure of a knowledge base
- Analyze path depth distribution to identify shallow vs deep topics
- Look for common prefixes to find thematic clusters
- Compare path counts at different depths to assess knowledge density
- Use `--depth 2` to quickly identify major topic areas
