# IWE Stats

Generates comprehensive statistics about your knowledge graph.

## Usage

``` bash
iwe stats [OPTIONS]
```

## Options

- `-f, --format <FORMAT>`: Output format (default: markdown)
  - `markdown`: Human-readable formatted statistics
  - `csv`: Machine-readable CSV format with per-document statistics

## What it shows

The stats command provides detailed analytics across multiple dimensions:

### Overview

- Total documents in your knowledge base
- Total nodes (all content elements)
- Total paths through the graph

### Document Statistics

- Total sections/headers across all documents
- Average sections per document
- Top 10 documents by section count
- Paragraph counts per document

### Reference Statistics

- Block references (embedded documents)
- Inline references (wiki-links)
- Total references count
- Orphaned documents (no incoming references)
- Leaf documents (no outgoing references)
- Top 10 most referenced documents

### Lines Statistics

- Total lines across all documents
- Average lines per document
- Top 10 largest documents by line count

### Words Statistics

- Total words across all documents
- Average words per document
- Top 10 largest documents by word count

### Structure Statistics

- Root-level sections
- Maximum and average path depth
- Counts of bullet lists, ordered lists, code blocks, tables, and quotes

### Network Analysis

- Average references per document
- Top 10 most connected documents (by total incoming + outgoing references)

## Examples

``` bash
# Generate human-readable statistics (default)
iwe stats

# Export per-document statistics as CSV
iwe stats --format csv > stats.csv

# Analyze CSV data with standard tools
iwe stats -f csv | cut -d, -f1,2,3 | column -t -s,
```

## Sample Markdown Output

``` markdown
# Graph Statistics

## Overview

- **Total documents:** 39
- **Total nodes:** 1552
- **Total paths:** 302

## Document Statistics

- **Total sections:** 844
- **Average sections/doc:** 21.64

### Top Documents by Sections

1. **VS Code** (102 sections)
2. **Neovim** (89 sections)
3. **Extract Actions** (79 sections)
...

## Lines Statistics

- **Total lines:** 3404
- **Average lines/doc:** 87.28

### Top Documents by Lines

1. **Neovim** (429 lines)
2. **Extract Actions** (342 lines)
...

## Words Statistics

- **Total words:** 14204
- **Average words/doc:** 364.21

### Top Documents by Words

1. **Neovim** (1337 words)
2. **Extract Actions** (1200 words)
...
```

## CSV Format Details

The CSV format provides per-document statistics with the following columns:

- `key`: Document identifier/filename
- `title`: Document title (first heading)
- `sections`: Number of heading sections
- `paragraphs`: Number of paragraph blocks
- `lines`: Total line count
- `words`: Total word count
- `incoming_block_refs`: Block references pointing to this document
- `incoming_inline_refs`: Inline wiki-links pointing to this document
- `total_incoming_refs`: Total incoming references
- `outgoing_block_refs`: Block references in this document
- `outgoing_inline_refs`: Inline wiki-links from this document
- `total_connections`: Total references (incoming + outgoing)
- `bullet_lists`: Number of unordered lists
- `ordered_lists`: Number of numbered lists
- `code_blocks`: Number of code/raw blocks
- `tables`: Number of tables
- `quotes`: Number of quote blocks

## Using CSV Output

The CSV format enables programmatic analysis and integration with data tools:

``` bash
# Import into spreadsheet applications
iwe stats -f csv > knowledge-base-stats.csv

# Find most connected documents
iwe stats -f csv | tail -n +2 | sort -t, -k12 -nr | head -5

# Calculate total word count
iwe stats -f csv | tail -n +2 | cut -d, -f6 | paste -sd+ | bc

# Filter documents with many references
iwe stats -f csv | awk -F, '$9 > 5 {print $1, $2, $9}' OFS=,

# Generate reports with Python/pandas
import pandas as pd
df = pd.read_csv('stats.csv')
print(df.describe())
print(df.nlargest(10, 'total_connections'))
```
