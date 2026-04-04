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

## Document Titles

By default, IWE uses the first header of a document as its title in completion suggestions. You can configure IWE to use a YAML frontmatter field instead:

``` toml
[library]
frontmatter_document_title = "title"
```

With this configuration, documents with frontmatter like:

``` markdown
---
title: Custom Document Title
---

# Header
```

Will appear in completions as "Custom Document Title" and insert `[Custom Document Title](key)` when selected. If the frontmatter field is missing, IWE falls back to using the first header.
