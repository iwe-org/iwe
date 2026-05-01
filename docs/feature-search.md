# Notes Search

Notes search is a key feature in IWE. IWE allows you to organize documents hierarchy just by adding **[Inclusion Links](inclusion-links.md)**. Then you can search for the documents taking into account the hierarchy.

Search can be used via the LSP `Workspace Symbols` command.

For every note, IWE will generate full paths. And allow you to do a fuzzy matching to filter the search results. So you can find both entries just by typing `cappu`.

```
Journal, 2025      ⇒  Week 3 - Coffee week  ⇒  Jan 26, 2025 - Cappuccino

My Coffee Journey  ⇒  Week 3 - Coffee week  ⇒  Jan 26, 2025 - Cappuccino
```

Since `Week 3` is included in two notes it shown in both contexts.

Note that you don't have to deal with the file names at all, as everything is based on the headers from your notes!

## Custom Document Titles

By default, IWE uses the first header of each document as its title in search results. You can configure IWE to use a YAML frontmatter field instead by setting `frontmatter_document_title` in your configuration. See the [Configuration](configuration.md#frontmatter-document-title.md) documentation for details.

## Structured search via the CLI

The CLI [`iwe find`](cli-find.md) command pairs the same fuzzy matcher used by the LSP with a YAML-based filter language. Use `--filter` for frontmatter predicates (`status: draft`, `priority: { $gt: 3 }`), and the structural anchor flags (`--includes`, `--included-by`, `--references`, `--referenced-by`) to scope the search to a subtree or set of references. See the [Query Language](query-language.md) reference for the full syntax. [`iwe count`](cli-count.md) returns the integer count for the same filter shape.
