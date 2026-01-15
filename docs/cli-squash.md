# IWE Squash

Creates consolidated documents by combining linked content into a single file.

## Usage

``` bash
iwe squash --key <KEY> [OPTIONS]
```

## Required

- `-k, --key <KEY>`: Starting document key/identifier to squash from

## Options

- `-d, --depth <DEPTH>`: How deep to traverse links (default: 2)
- `-v, --verbose <LEVEL>`: Verbosity level

## What it does

- Starts from the specified document
- Traverses linked documents up to specified depth
- Combines content into a single markdown document
- Converts block references to inline sections
- Maintains document structure and hierarchy

## Examples

``` bash
# Squash starting from document "project-overview"
iwe squash --key project-overview

# Squash with greater depth
iwe squash --key main-topic --depth 4

# With debug output
iwe squash --key research-notes --depth 3 -v 2
```

Example [PDF](https://github.com/iwe-org/iwe/blob/master/docs/book.pdf) generated using `squash` command and typst
