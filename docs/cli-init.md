# IWE Init

Initializes the current directory as an IWE project.

## Usage

``` bash
iwe init
```

## What It Creates

Running `iwe init` creates:

```
.iwe/
└── config.toml    # Project configuration
```

The `.iwe/` directory marks the project root and stores configuration files.

## Default Configuration

The generated `config.toml` contains:

``` toml
version = 3

[markdown]
refs_extension = ""
date_format = "%b %d, %Y"

[library]
path = ""
date_format = "%Y-%m-%d"

[completion]

[commands]
[commands.default]
run = "claude -p"
timeout_seconds = 120

[actions]
[actions.extract]
type = "extract"
title = "Extract"
link_type = "markdown"
key_template = "{{id}}"

[actions.inline_section]
type = "inline"
title = "Inline section"
inline_type = "section"
keep_target = false

[actions.sort]
type = "sort"
title = "Sort A-Z"
reverse = false

[templates]
[templates.default]
key_template = "{{slug}}"
document_template = "# {{title}}\n\n{{content}}"
```

## Configuration Options

### Library Section

| Option | Default | Description |
|--------|---------|-------------|
| `path` | `""` | Subdirectory containing markdown files (empty = project root) |
| `date_format` | `%Y-%m-%d` | Format for dates in document keys |
| `default_template` | `null` | Template name for `iwe new` command |

### Markdown Section

| Option | Default | Description |
|--------|---------|-------------|
| `refs_extension` | `""` | File extension to append to references (e.g., `.md`) |
| `date_format` | `%b %d, %Y` | Format for dates displayed in documents |

### Completion Section

| Option | Default | Description |
|--------|---------|-------------|
| `link_format` | `null` | Link style for completions: `markdown` or `wiki` |

## Example

``` bash
cd ~/my-notes
iwe init
# Output: Creates .iwe/config.toml

# Verify initialization
ls -la .iwe/
# Output: config.toml
```

## Customization

After initialization, edit `.iwe/config.toml` to:

- Store markdown files in a subdirectory: set `library.path = "docs"`
- Use wiki-style links: set `completion.link_format = "wiki"`
- Define custom templates for document creation
- Configure AI-powered actions with custom commands

## Re-initialization

Running `iwe init` in an already-initialized project is safe - it will not overwrite an existing `config.toml`.
