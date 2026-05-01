# IWE Tree

Display document hierarchy as a tree structure.

## Usage

``` bash
iwe tree [OPTIONS]
```

## Options

| Option                          | Default    | Description                                                                          |
| ------------------------------- | ---------- | ------------------------------------------------------------------------------------ |
| `-f, --format <FORMAT>`         | `markdown` | Output format: `markdown`, `keys`, `json`, `yaml`.                                   |
| `-d, --depth <DEPTH>`           | `4`        | Maximum depth to traverse.                                                           |
| `--filter <EXPR>`               | -          | Inline YAML filter expression. See [Query Language](query-language.md).              |
| `-k, --key <KEY>`               | -          | Start tree from specific document(s); repeatable.                                    |
| `--includes <KEY[:DEPTH]>`      | -          | `$includes` anchor. Repeatable; anchors are ANDed.                                   |
| `--included-by <KEY[:DEPTH]>`   | -          | `$includedBy` anchor. Repeatable; anchors are ANDed.                                 |
| `--references <KEY[:DIST]>`     | -          | `$references` anchor. Repeatable; anchors are ANDed.                                 |
| `--referenced-by <KEY[:DIST]>`  | -          | `$referencedBy` anchor. Repeatable; anchors are ANDed.                               |
| `--max-depth <N>`               | `1`        | Session default for inclusion anchor flags without a colon-suffix. `0` = unbounded.  |
| `--max-distance <N>`            | `1`        | Session default for reference anchor flags without a colon-suffix. `0` = unbounded.  |
| `-v, --verbose <LEVEL>`         | `0`        | Verbosity level (1=info, 2=debug).                                                   |

When filter or anchor flags are provided, the selector resolves to a set of keys and those keys are used as the tree roots. Combining `-k` with the selector intersects the two — empty intersection yields an empty tree.


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
        "title": "Child Document",
        "children": []
      }
    ]
  }
]
```

`children` is always present (empty array on leaves). With `--project f1,f2`, the listed user-frontmatter fields are emitted alongside `key`, `title`, `children` per node:

``` bash
iwe tree --project pillar,status -f json
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
iwe tree -f yaml
iwe tree -k my-doc
iwe tree -k doc-a -k doc-b
iwe tree --depth 2
iwe tree | grep -i api
iwe tree -f keys | grep cli

# Roots inside an anchor's subtree
iwe tree --included-by projects/alpha:0

# Tree restricted to drafts
iwe tree --filter 'status: draft'
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
- Use `-f json` or `-f yaml` for structured data consumption
- Pipe through `grep` to filter results
- Use `-k` to explore documents involved in circular references
- Use `--depth 2` to quickly identify major topic areas

## Deprecated aliases

The following flags pre-date the query language and remain accepted for backward compatibility. Each invocation prints a one-line `warning: ... is deprecated` to stderr.

| Deprecated         | Use instead                                                                 |
| ------------------ | --------------------------------------------------------------------------- |
| `--in KEY[:N]`     | `--included-by KEY[:N]`                                                     |
| `--in-any K1 K2`   | `--filter '$or: [{ $includedBy: K1 }, { $includedBy: K2 }]'`                |
| `--not-in KEY`     | `--filter '$not: { $includedBy: KEY }'`                                     |
| `--refs-to KEY`    | `--references KEY` (legacy semantics: ORs `$includes` and `$references`)    |
| `--refs-from KEY`  | `--referenced-by KEY` (legacy semantics: ORs `$includedBy` and `$referencedBy`) |
