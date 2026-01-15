# Notes Search

Notes search is a key feature in IWE. IWE allows you to organize documents hierarchy just by adding **block-references**. Then you can search for the documents taking into account the hierarchy.

Search can be used via the LSP `Workspace Symbols` command.

For every note, IWE will generate full paths. And allow you to do a fuzzy matching to filter the search results. So you can find both entries just by typing `cappu`.

```
Journal, 2025      ⇒  Week 3 - Coffee week  ⇒  Jan 26, 2025 - Cappuccino

My Coffee Journey  ⇒  Week 3 - Coffee week  ⇒  Jan 26, 2025 - Cappuccino
```

Since `Week 3` is included in two notes it shown in both contexts.

Note that you don't have to deal with the file names at all, as everything is based on the headers from your notes!
