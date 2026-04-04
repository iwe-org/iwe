# CLI Troubleshooting

Best practices and solutions to common issues when using the IWE CLI.

## Best practices

1.  **Start small**: Test commands on a few files before processing large libraries
2.  **Backup first**: Always backup before running `normalize` or other bulk operations
3.  **Use debug mode**: Add `-v 2` to see detailed debug information
4.  **Iterate gradually**: Use increasing depth values to explore graph complexity
5.  **Visualize regularly**: Export graphs to understand document relationships
6.  **Monitor root documents**: Use `tree` to track entry points as your library grows

## Common issues

| Issue                      | Solution                                           |
| -------------------------- | -------------------------------------------------- |
| No changes after normalize | Check that files are properly formatted markdown   |
| Export produces no output  | Verify documents contain links and references      |
| Squash fails               | Ensure the specified key exists and is accessible  |
| Command not found          | Ensure IWE is installed and available in your PATH |
| Permission denied          | Check file permissions in your project directory   |


## Debugging

When encountering issues, use verbose mode to get more information:

``` bash
# INFO level logging
iwe -v 1 <command>

# DEBUG level logging
iwe -v 2 <command>
```

## Edge Cases

### Empty Knowledge Base

When no markdown files exist:

| Command      | Behavior             |
| ------------ | -------------------- |
| `tree`       | No output            |
| `find`       | No matches found     |
| `stats`      | Shows zero counts    |
| `export dot` | Produces empty graph |


### Missing Referenced Documents

When a document links to a non-existent file:

- **Normalize**: Updates link title to empty string
- **Retrieve**: Skips missing references in expansion
- **Squash**: Skips missing linked documents
- **Export**: Excludes edges to missing documents

### Circular References

IWE handles circular references gracefully:

- **Retrieve**: Expands each document once, avoiding infinite loops
- **Squash**: Includes each document once at first encounter
- **Tree**: Use `-k` to start from any document in a cycle
- **Export**: Renders cycles as valid graph edges

### Large Knowledge Bases

For repositories with thousands of files:

- Use `--depth` limits to constrain exploration
- Use `find` with filters for targeted searches
- Use `tree -k` to explore specific subtrees
- Consider exporting subgraphs with `--key` filter

## Getting help

For any command, use the `--help` flag to see available options:

``` bash
iwe --help
iwe <command> --help
```
