# CLI Workflow Examples

Practical examples of using IWE CLI commands for common tasks.

## Daily Maintenance

``` bash
# Update all document formatting and links
iwe normalize

# Check document structure
iwe paths --depth 5
```

## Content Analysis

``` bash
# Find entry points
iwe contents

# Visualize specific topic area
iwe export dot --key "machine-learning" --include-headers > ml.dot
dot -Tpng ml.dot -o ml-graph.png
```

## Document Consolidation

``` bash
# Create comprehensive document from research notes
iwe squash --key "research-index" --depth 4 > consolidated-research.md

# Generate presentation material
iwe squash --key "project-summary" --depth 2 > project-overview.md
```

## Large Library Management

``` bash
# Process with debug information
iwe normalize -v 2

# Analyze complex relationships with debug output
iwe paths --depth 8 -v 2

# Export detailed visualization
iwe export dot --include-headers --depth 5 > full-graph.dot
```
