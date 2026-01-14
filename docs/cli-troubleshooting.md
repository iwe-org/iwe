# CLI Troubleshooting

Best practices and solutions to common issues when using the IWE CLI.

## Best practices

1. **Start small**: Test commands on a few files before processing large libraries
2. **Backup first**: Always backup before running `normalize` or other bulk operations
3. **Use debug mode**: Add `-v 2` to see detailed debug information
4. **Iterate gradually**: Use increasing depth values to explore graph complexity
5. **Visualize regularly**: Export graphs to understand document relationships
6. **Monitor root documents**: Use `contents` to track entry points as your library grows

## Common issues

| Issue | Solution |
|-------|----------|
| No changes after normalize | Check that files are properly formatted markdown |
| Export produces no output | Verify documents contain links and references |
| Squash fails | Ensure the specified key exists and is accessible |
| Command not found | Ensure IWE is installed and available in your PATH |
| Permission denied | Check file permissions in your project directory |

## Debugging

When encountering issues, use verbose mode to get more information:

``` bash
# INFO level logging
iwe -v 1 <command>

# DEBUG level logging
iwe -v 2 <command>
```

## Getting help

For any command, use the `--help` flag to see available options:

``` bash
iwe --help
iwe <command> --help
```
