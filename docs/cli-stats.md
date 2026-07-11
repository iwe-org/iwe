# IWE Stats

Generates comprehensive statistics about your knowledge graph.

## Usage

``` bash
iwe stats [OPTIONS]
iwe stats similarity
```

## Options

- `-f, --format <FORMAT>`: Output format (default: `markdown`)
  - `markdown`: Human-readable formatted statistics
  - `csv`: Machine-readable CSV format with per-document statistics
  - `json`: JSON output (aggregate stats only; ignored when `-k` is given — per-key stats always serialize as JSON)
- `-k, --key <KEY>`: Document key for per-document statistics (always JSON output). Omit for aggregate graph statistics.

## Subcommands

- `similarity`: list pages that have a near-identical, mutually-similar counterpart elsewhere in the store (see [Detecting similar pages](#detecting-similar-pages)).

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

- Inclusion links (embedded documents)
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

### Orphans

- A list of every document with no incoming references (inclusion or inline), by key. `index` pages (the root `index` or any `<dir>/index`) are treated as intentional entry points and are never reported as orphans. The markdown, JSON, and YAML outputs carry this list; the aggregate count also appears under Reference Statistics.

### Network Analysis

- Average references per document
- Top 10 most connected documents (by total incoming + outgoing references)

### Per-document Similar pages

When you pass `-k <KEY>`, the output also lists **similar pages** — other documents whose content is near-identical to this one (see [Detecting similar pages](#detecting-similar-pages) for how the match is decided). The markdown output appends a `- **Similar page:** <key> (<score>)` line per match; the JSON/YAML output carries a `similarPages` array of `{ key, score }`.

## Detecting similar pages

`iwe stats similarity` scans the whole store and reports pages that duplicate one another. A page is only reported when the match is a genuine duplicate, not merely a related page:

- **Mutual.** The similarity must hold in both directions — the two pages must each be mostly made of the other's content. This rules out a short page that is wholly contained in a longer one.
- **Comparable size.** Both pages must be at least ~50 tokens and within ~2× the length of each other. Real duplicates are the same content, so they are the same size.
- **Near-identical.** The bar is set high (a self-normalized BM25 score of at least `0.85`), so the same page saved under two keys is caught while related-but-distinct pages are not.

Forward matches are computed once per page and run concurrently, so the scan stays fast on large stores. Output lists each mutually-similar pair once, tab-separated, in alphabetical order:

``` text
people/ada-and-kai	people/kai-and-ada
notes/2019-budget	notes/2019-budget-copy
```

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
- `includedByCount`: Documents that include this one
- `referencedByCount`: Inline references pointing to this document
- `incomingEdgesCount`: Total incoming references
- `includesCount`: Documents included by this one
- `referencesCount`: Inline references from this document
- `totalEdgesCount`: Total references (incoming + outgoing)
- `bulletLists`: Number of unordered lists
- `orderedLists`: Number of numbered lists
- `codeBlocks`: Number of code/raw blocks
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
print(df.nlargest(10, 'totalEdgesCount'))
```
