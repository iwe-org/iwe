# IWE Squash

Creates consolidated documents by combining linked content into a single markdown file.

## Usage

``` bash
iwe squash <KEY> [OPTIONS]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<KEY>` | Document key to squash |

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `-d, --depth <DEPTH>` | `2` | How deep to traverse links |
| `-v, --verbose <LEVEL>` | `0` | Verbosity level |

## What It Does

1. Starts from the specified document
2. Traverses [inclusion links](inclusion-links.md) up to specified depth
3. Combines content into a single markdown document
4. Converts linked sections to inline sections
5. Adjusts header levels to maintain hierarchy

## Output Format

Given this document structure:

`project-overview.md`:
``` markdown
# Project Overview

Introduction to the project.

- [Goals](goals)
- [Architecture](architecture)
```

`goals.md`:
``` markdown
# Goals

Our main objectives are:

- Improve performance
- Add new features
```

Running `iwe squash project-overview` outputs:

``` markdown
# Project Overview

Introduction to the project.

## Goals

Our main objectives are:

- Improve performance
- Add new features

## Architecture

...
```

Note that linked document headers become sub-headers (# → ##) to preserve hierarchy.

## Examples

``` bash
# Squash starting from document "project-overview"
iwe squash project-overview

# Squash with greater depth
iwe squash main-topic --depth 4

# Save output to file
iwe squash research-notes --depth 3 > output.md

# With debug output
iwe squash research-notes --depth 3 -v 2
```

## Header Level Adjustment

When documents are inlined, their header levels are adjusted:

| Original Level | Becomes | Reason |
|----------------|---------|--------|
| `#` (h1) | `##` (h2) | Linked document becomes section |
| `##` (h2) | `###` (h3) | Preserves relative hierarchy |
| `###` (h3) | `####` (h4) | And so on... |

This ensures the squashed document maintains a logical structure with the root document as the top-level heading.

## Use Cases

- **Export to PDF**: Create a single document for typesetting with tools like Typst
- **Sharing**: Generate a standalone file from interconnected notes
- **Backup**: Consolidate project knowledge into a portable format
- **Context building**: Create comprehensive documents for review

## AI Agent Tips

- Use `squash` to build comprehensive context from related documents
- Combine with `retrieve` for maximum context: squash provides structure, retrieve provides backlinks
- Adjust depth based on scope: shallow (2) for focused topics, deep (4+) for comprehensive overviews
- Squashed output is ideal for sending to language models as context

Example [PDF](https://github.com/iwe-org/iwe/blob/master/docs/book.pdf) generated using `squash` command and Typst.
