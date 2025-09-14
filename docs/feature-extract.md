# Extract Notes

The extract note action enables the creation of a new document from a section (header). This involves:

1.  Creating a new file containing the selected content.
2.  Adding a `[Block Reference](block-reference.md)` link to the newly created file.

The reverse operation, known as **inline**, allows you to:

1.  Embed the content into the document with [Block-reference](block-reference.md).
2.  Remove the link and injected file.

Both operations automatically adjust the header levels as needed to maintain proper document structure.

## Configuration

The extract action is configured in `.iwe/config.toml` under the `[actions]` section. Here's the basic structure:

```toml
[actions]
extract = { type = "extract", title = "Extract section", key_template = "{{id}}", link_type = "markdown" }
```

### Configuration Options

- **`type`**: Must be `"extract"` for extract actions
- **`title`**: The display name shown in the editor's code actions menu
- **`key_template`**: Template for generating the new file's key/name (see Template Variables below)
- **`link_type`**: Optional link format - `"markdown"` for `[text](key)` or `"wiki"` for `[[key]]`

### Template Variables

The `key_template` supports several template variables:

#### Basic Variables
- **`{{id}}`**: Random unique identifier
- **`{{today}}`**: Current date formatted using the `date_format` from `[library]` section (default: `%Y-%m-%d`)
- **`{{title}}`**: The section title being extracted (sanitized for filenames)

#### Parent Section Variables
- **`{{parent.title}}`**: Title of the parent section containing the extracted section

#### Source Document Variables
- **`{{source.key}}`**: Full key of the source document
- **`{{source.file}}`**: Filename portion of the source document key
- **`{{source.title}}`**: Title (first header) of the source document
- **`{{source.path}}`**: Directory path of the source document

### Example Configurations

#### Simple numeric keys
```toml
extract = { type = "extract", title = "Extract", key_template = "{{id}}" }
```

#### Date-based extraction
```toml
extract = { type = "extract", title = "Extract to today", key_template = "{{today}}" }
```

#### Title-based extraction
```toml
extract = { type = "extract", title = "Extract section", key_template = "{{title}}" }
```

#### Hierarchical extraction
```toml
extract = { type = "extract", title = "Extract with context", key_template = "{{parent.title}}-{{title}}" }
```

#### Source-aware extraction
```toml
extract = { type = "extract", title = "Extract from source", key_template = "{{source.file}}-{{title}}" }
```

#### Path-based extraction
```toml
extract = { type = "extract", title = "Extract to path", key_template = "{{source.path}}/extracted-{{title}}" }
```

#### Wiki-style links
```toml
extract = { type = "extract", title = "Extract (wiki)", key_template = "{{id}}", link_type = "wiki" }
```

### Key Collision Handling

If the generated key already exists, IWE automatically appends a numeric suffix (e.g., `extracted-1`, `extracted-2`) to ensure uniqueness.

### Date Formatting

The `{{today}}` variable uses the `date_format` setting from the `[library]` section:

```toml
[library]
date_format = "%Y-%m-%d"  # Results in: 2024-01-15
```

Special characters in titles are automatically sanitized to create valid filenames.
