# Graph Visualization

IWE provides powerful graph visualization capabilities through DOT format export, allowing you to create visual representations of your knowledge graph structure. This helps you understand the relationships between documents, sections, and references in your markdown collection.

## Export Command

The `iwe export dot` command generates graph data in DOT format, which can be processed by Graphviz and other visualization tools.

### Basic Usage

``` bash
# Export all root documents
iwe export dot

# Export specific document by key
iwe export dot --key project-notes

# Export with depth limit
iwe export dot --depth 3
```

### Advanced Visualization with Headers

Use the `--include-headers` flag to create detailed visualizations that show document structure with sections grouped in colored subgraphs:

``` bash
# Include sections and subgraphs
iwe export dot --include-headers

# Detailed view of specific document
iwe export dot --key documentation --include-headers

# Combined with depth limit
iwe export dot --key meetings --depth 2 --include-headers
```

## Visualization Modes

### Basic Mode (Default)

Shows document-to-document relationships with clean node styling:

``` dot
digraph G {
  rankdir=LR
  fontname=Verdana
  
  1[label="Project Notes",fillcolor="#ffeaea",fontsize=16,shape=note,style=filled]
  2[label="Meeting Notes",fillcolor="#f6e5ee",fontsize=16,shape=note,style=filled]
  1 -> 2 [color="#38546c66",arrowhead=normal,penwidth=1.2]
}
```

### Detailed Mode (--include-headers)

Shows document structure with sections grouped in colored subgraphs:

``` dot
digraph G {
  rankdir=LR
  
  1[label="Project Notes",shape=note,style=filled]
  2[label="Introduction",shape=plain]
  3[label="Requirements",shape=plain]
  
  subgraph cluster_0 {
    labeljust="l"
    style=filled
    color="#fff9de"
    fillcolor="#fff9de"
    2
    3
  }
  
  2 -> 1 [arrowhead="empty",style="dashed"]
  3 -> 1 [arrowhead="empty",style="dashed"]
}
```

## Key Features

- **Color Coding**: Each document key gets a unique, consistent color scheme
- **Shape Differentiation**: Documents use `note` shape, sections use `plain` shape
- **Subgraph Clustering**: Sections are grouped in colored clusters with document keys
- **Edge Styles**: Different styles for document vs section relationships
- **Automatic Layout**: Left-to-right layout optimized for readability

## Integration with Graphviz

### Generate PNG Images

``` bash
# Basic visualization
iwe export dot | dot -Tpng -o knowledge-graph.png

# Detailed with sections
iwe export dot --include-headers | dot -Tpng -o detailed-graph.png

# Focus on specific topic
iwe export dot --key project --include-headers | dot -Tpng -o project-structure.png
```

### Generate SVG for Web

``` bash
# Scalable vector graphics
iwe export dot | dot -Tsvg -o interactive-graph.svg

# With better layout for complex graphs
iwe export dot --include-headers | neato -Tsvg -o network-view.svg
```

### Different Layout Engines

``` bash
# Hierarchical layout (default)
iwe export dot | dot -Tpng -o hierarchical.png

# Force-directed layout
iwe export dot | neato -Tpng -o network.png

# Circular layout
iwe export dot | circo -Tpng -o circular.png

# Spring-based layout
iwe export dot | fdp -Tpng -o spring.png
```

## Filtering and Focusing

### By Document Key

``` bash
# Show only documents related to 'meetings'
iwe export dot --key meetings --include-headers

# Multiple levels of related documents
iwe export dot --key architecture --depth 2
```

### By Content Depth

``` bash
# Show only immediate relationships
iwe export dot --depth 1

# Show deeper connections
iwe export dot --depth 3 --include-headers
```

## Workflow Examples

### Daily Documentation Review

``` bash
#!/bin/bash
# Generate today's knowledge graph
iwe export dot --include-headers > today.dot
dot -Tpng today.dot -o daily-review.png
open daily-review.png  # macOS
```

### Project Structure Analysis

``` bash
#!/bin/bash
# Analyze specific project structure
iwe export dot --key $PROJECT_NAME --include-headers | \
  dot -Tsvg -o "project-${PROJECT_NAME}.svg"
```

### Knowledge Base Overview

``` bash
#!/bin/bash
# Create multiple views of your knowledge base
iwe export dot > overview.dot
iwe export dot --include-headers > detailed.dot

# Generate both views
dot -Tpng overview.dot -o overview.png
dot -Tpng detailed.dot -o detailed.png
```

## Customization Tips

### Layout Optimization

For large graphs, experiment with different Graphviz engines:

- **`dot`**: Best for hierarchical structures
- **`neato`**: Good for network-like relationships
- **`fdp`**: Spring model, useful for clustered data
- **`circo`**: Circular layout for cyclic structures

### Output Formats

Graphviz supports many output formats:

- **PNG/JPG**: For presentations and documents
- **SVG**: For interactive web displays
- **PDF**: For high-quality prints
- **DOT**: For further processing or debugging

### Performance Considerations

- Use `--depth` limits for large knowledge bases
- Filter by `--key` to focus on specific areas
- Use `--include-headers` for detailed structure visualization when needed

## Troubleshooting

### Large Graphs

``` bash
# Reduce complexity with depth limits
iwe export dot --depth 2 | dot -Tpng -o simplified.png

# Use different layout engine
iwe export dot | fdp -Tpng -o alternative-layout.png
```

### Missing Graphviz

Install Graphviz on your system:

``` bash
# macOS
brew install graphviz

# Ubuntu/Debian
sudo apt install graphviz

# Windows
winget install graphviz
```

### Complex Layouts

For complex graphs, try different approaches:

``` bash
# Increase node separation
iwe export dot | dot -Tpng -Gnodesep=1.0 -o spaced.png

# Adjust DPI for clarity
iwe export dot | dot -Tpng -Gdpi=200 -o high-res.png
```

The visualization feature makes IWE's knowledge management capabilities tangible, helping you understand and navigate your documentation structure at a glance.
