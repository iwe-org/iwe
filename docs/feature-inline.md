# Inline Content Action

The `inline` action is a powerful feature for embedding content from one document directly into another. It replaces a link to a document with the actual content of that document. This is useful for consolidating notes, embedding sources, or restructuring your knowledge base.

There are two primary ways to inline content, configured via the `inline_type` property:

1.  **`section`**: Inlines the entire content of the linked document into the current document's section that contains the link.
2.  **`quote`**: Replaces the link with the content of the linked document, formatted as a markdown blockquote.

## Configuration

You can define custom `inline` actions in your `config.toml` file within the `.iwe` directory.

### Example Configuration:

Here is an example of how to configure two different inline actions: one for inlining as a section and another for inlining as a quote.

```toml
[actions]

# Inlines the content of the linked document and deletes the original file.
inline_section = { type = "inline", title = "Inline Section", inline_type = "section", keep_target = false }

# Inlines the content as a blockquote and keeps the original file.
inline_quote = { type = "inline", title = "Inline as Quote", inline_type = "quote", keep_target = true }
```

### Configuration Keys:

-   `type`: Must be set to `"inline"`.
-   `title`: The text that will appear in the code action menu in your editor (e.g., "Inline Section").
-   `inline_type`: Determines how the content is embedded.
    -   `"section"`: Embeds the full content.
    -   `"quote"`: Wraps the content in a blockquote.
-   `keep_target` (optional, defaults to `false`):
    -   If `false`, the original source file of the link will be deleted after its content is inlined. The action will also clean up any other references to the deleted file across your workspace.
    -   If `true`, the original source file is left untouched.
