# Auto-Complete

IWE can suggest links as you type using the standard LSP code completion feature.

## Link Format

By default, completions insert Markdown-style links `[title](key)`. You can configure IWE to use WikiLinks `[[key]]` instead:

``` toml
[completion]
link_format = "wiki"
```

Available options:
- `"markdown"` (default): Creates `[title](key)` style links
- `"wiki"`: Creates `[[key]]` style WikiLinks
