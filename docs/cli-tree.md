# IWE Tree

Display document hierarchy as a tree structure.

## Usage

``` bash
iwe tree [OPTIONS]
```

## Options

| Option                  | Default    | Description                                           |
| ----------------------- | ---------- | ----------------------------------------------------- |
| `-f, --format <FORMAT>` | `markdown` | Output format: markdown, keys, json                   |
| `-k, --key <KEY>`       | -          | Start tree from specific document(s), can be repeated |
| `-d, --depth <DEPTH>`   | `4`        | Maximum depth to traverse                             |
| `-v, --verbose <LEVEL>` | `0`        | Verbosity level (1=info, 2=debug)                     |


## Output Formats

### Markdown (default)

Nested list with links:

```
- [Main Document](main)
  - [Child Document](child)
    - [Nested Item](nested)
- [Another Root](another)
```

### Keys

Document keys only with tab indentation:

```
main
	child
		nested
another
```

### JSON

Nested JSON array structure:

``` json
[
  {
    "key": "main",
    "title": "Main Document",
    "children": [
      {
        "key": "child",
        "title": "Child Document"
      }
    ]
  }
]
```

## Starting from Specific Documents

Use `-k` to start the tree from specific document(s):

``` bash
iwe tree -k my-doc
iwe tree -k doc-a -k doc-b
```

This is essential for documents involved in circular references that have no natural root.

## Handling Circular References

When documents form circular references (A→B→C→A), they have no natural root and won't appear in the default tree output. Use `-k` to start from any document in the cycle:

``` bash
iwe tree -k doc-a
```

Output shows the cycle:

```
- [Doc A](doc-a)
  - [Doc B](doc-b)
    - [Doc C](doc-c)
      - [Doc A](doc-a)
```

## Examples

``` bash
iwe tree
iwe tree -f keys
iwe tree -f json
iwe tree -k my-doc
iwe tree -k doc-a -k doc-b
iwe tree --depth 2
iwe tree | grep -i api
iwe tree -f keys | grep cli
```

## Depth Impact

| Depth | Shows                           |
| ----- | ------------------------------- |
| 1     | Root documents only             |
| 2     | Roots and their direct children |
| 3     | Up to grandchildren             |
| 4+    | Deeper nested relationships     |


## AI Agent Tips

- Use `tree` to understand the overall structure of a knowledge base
- Use `-f keys` for programmatic processing
- Use `-f json` for structured data consumption
- Pipe through `grep` to filter results
- Use `-k` to explore documents involved in circular references
- Use `--depth 2` to quickly identify major topic areas
