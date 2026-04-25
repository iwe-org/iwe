# IWE Update

Overwrite the full markdown content of an existing document.

## Usage

``` bash
iwe update -k <KEY> -c <CONTENT>
iwe update -k <KEY> -c -          # read content from stdin
```

## Options

| Flag                   | Description                                | Default |
| ---------------------- | ------------------------------------------ | ------- |
| `-k, --key <KEY>`      | Document key to update                     | (required) |
| `-c, --content <STR>`  | New full markdown content. Use `-` for stdin. | (required) |
| `--dry-run`            | Preview without writing                    | false   |
| `--quiet`              | Suppress progress output                   | false   |

## Behavior

`update` overwrites the document at `<KEY>` with the provided content. The graph is not mutated by `update` itself; the file change is picked up on the next read. Use `iwe normalize` afterward if you want to canonicalize the output.

## Examples

``` bash
# Replace a doc with a fixed string
iwe update -k notes/draft -c "# Draft\n\nNew content."

# Pipe content from another command
cat new-content.md | iwe update -k notes/draft -c -

# Preview without writing
iwe update -k notes/draft -c "..." --dry-run
```

## Relationship to MCP

This command mirrors the `iwe_update` MCP tool. Both take the same arguments and produce the same on-disk result.
