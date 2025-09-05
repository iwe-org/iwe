# Debug Mode

For the LSP server, set the `IWE_DEBUG` environment variable. In debug mode, IWE LSP will generate a detailed log file named `iwe.log` in the directory where you started it:

```bash
export IWE_DEBUG=true; nvim
```

## When to Use Debug Mode

Including debug logs with your [issue](https://github.com/iwe-org/iwe/issues) report will help us resolve problems faster. Debug mode is useful when:

- Troubleshooting CLI command behavior
- Understanding document processing steps
- Diagnosing LSP server issues
- Reporting bugs or unexpected behavior

Note: use `-v 2` argument to see debug logs from CLI command.
