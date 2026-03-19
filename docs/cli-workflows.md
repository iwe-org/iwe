# CLI Workflow Examples

Practical examples of using IWE CLI commands for common tasks.

## Daily Maintenance

``` bash
iwe normalize

iwe tree --depth 5
```

## Content Analysis

``` bash
iwe tree

iwe tree -f keys | grep api

iwe export dot --key "machine-learning" --include-headers > ml.dot
dot -Tpng ml.dot -o ml-graph.png
```

## Document Consolidation

``` bash
# Create comprehensive document from research notes
iwe squash research-index --depth 4 > consolidated-research.md

# Generate presentation material
iwe squash project-summary --depth 2 > project-overview.md
```

## Large Library Management

``` bash
iwe normalize -v 2

iwe tree --depth 8 -v 2

iwe export dot --include-headers --depth 5 > full-graph.dot
```
