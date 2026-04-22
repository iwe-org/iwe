# Configuration

IWE projects are configured through a `.iwe/config.toml` file in your project root. Below are all available configuration options.

## Basic Configuration

``` toml
[markdown]
refs_extension = ""
date_format = "%b %d, %Y"
locale = "de_DE"

[library]
path = ""
date_format = "%Y-%m-%d"
locale = "en_US"
frontmatter_document_title = "title"

[completion]
link_format = "markdown"
min_prefix_length = 3
```

### Markdown Settings

- `refs_extension`: File extension for markdown references (default: empty, uses `.md`)
- `date_format`: Date format for markdown content display (default: `"%b %d, %Y"`, e.g., "Jan 15, 2024")
- `locale`: Locale for date formatting in document content (default: system locale). Allows different localization for content than for file keys.

### Library Settings

- `path`: Subdirectory for markdown files relative to project root (default: empty, uses root)
- `date_format`: Date format for file key generation (default: `"%Y-%m-%d"`, e.g., "2024-01-15")
- `locale`: Locale for date formatting (default: auto-detected from system). Affects day and month names when using `%A`, `%B`, etc.
- `frontmatter_document_title`: YAML frontmatter field to use as document title (default: none, uses first header)

### Completion Settings

- `link_format`: Format for auto-completed links (default: `"markdown"`)
  - `"markdown"`: Creates `[title](key)` style links
  - `"wiki"`: Creates `[[key]]` style WikiLinks
- `min_prefix_length`: Minimum number of characters typed before completions appear (default: `3`). Set to `0` to always show completions.

### Date Format Patterns

Date formats use [chrono format specifiers](https://docs.rs/chrono/latest/chrono/format/strftime/index.html):

**Date specifiers:**

- `%Y`: 4-digit year (2024)
- `%y`: 2-digit year (24)
- `%m`: Month as number (01-12)
- `%b`: Abbreviated month name (Jan)
- `%B`: Full month name (January)
- `%d`: Day of month (01-31)
- `%A`: Full weekday name (Monday)
- `%a`: Abbreviated weekday name (Mon)

**Time specifiers:**

- `%H`: Hour in 24-hour format (00-23)
- `%M`: Minute (00-59)
- `%S`: Second (00-59)

**Combined examples:**

- `"%Y-%m-%d %H:%M"` → "2024-01-15 14:30"
- `"%b %d, %Y %H:%M:%S"` → "Jan 15, 2024 14:30:45"
- `"%Y%m%d%H%M"` → "202401151430" (useful for sortable file keys)

Textual specifiers (`%A`, `%a`, `%B`, `%b`) are localized based on the `locale` setting. For example, with `locale = "de_DE"` and `date_format = "%A, %d. %B %Y"`, dates display as "Freitag, 27. März 2026".

### Locale Settings

IWE supports separate locales for file keys and document content. By default, both use your system locale independently.

- **`library.locale`**: Controls the language for file key generation (e.g., `journal/Friday-March-27`)
- **`markdown.locale`**: Controls the language for document content (e.g., `# Freitag, 27. März 2026`)

``` toml
[library]
date_format = "%A-%B-%d"
locale = "en_US"

[markdown]
date_format = "%A, %d. %B %Y"
locale = "de_DE"
```

With this configuration:

- File keys use English day/month names: `journal/Friday-March-27`
- Document content uses German: `# Freitag, 27. März 2026`

The locale accepts both POSIX format (`de_DE`) and BCP47 format (`de-DE`). Encoding suffixes like `.UTF-8` are automatically stripped.

### Frontmatter Document Title

By default, IWE uses the first header in a document as its title for links, autocomplete suggestions, and search results. You can override this behavior by specifying a YAML frontmatter field to use instead:

``` toml
[library]
frontmatter_document_title = "title"
```

With this configuration, a document like:

``` markdown
---
title: My Custom Title
---

# Header (ignored for title)

Document content...
```

Will use "My Custom Title" as the document title instead of "Header (ignored for title)". This affects:

- Link text in auto-completed links: `[My Custom Title](document-key)`
- Link text normalization when references are updated
- Document titles in search results and workspace symbols

If the configured frontmatter field is missing or the document has no frontmatter, IWE falls back to using the first header as the title.

## Commands

Define CLI commands for text transformation actions. Commands receive input via stdin and output transformed content to stdout:

``` toml
[commands.claude]
run = "claude -p"
timeout_seconds = 120

[commands.uppercase]
run = "tr '[:lower:]' '[:upper:]'"
timeout_seconds = 5

[commands.custom_script]
run = "/path/to/my-script.sh"
timeout_seconds = 60
```

Each command requires:

- `run`: Command to execute (by default runs via `sh -c`)

Optional parameters:

- `args`: Array of arguments when using direct execution (only used when `shell = false`)
- `cwd`: Working directory for command execution
- `env`: Environment variables as key-value pairs (supports `$VAR` or `${VAR}` expansion from parent environment)
- `shell`: Execute via shell (`true`, default) or directly (`false`)
- `timeout_seconds`: Maximum execution time in seconds (default: 120)

Commands are executed with the processed input template piped to stdin. The command's stdout becomes the replacement content.

### Example Commands

**Using Claude CLI:**

``` toml
[commands.claude]
run = "claude -p"
timeout_seconds = 120
```

**Using a custom script:**

``` toml
[commands.rewriter]
run = "python ~/scripts/rewrite.py"
timeout_seconds = 30
```

**Simple text transformation:**

``` toml
[commands.uppercase]
run = "tr '[:lower:]' '[:upper:]'"
timeout_seconds = 5
```

**Direct execution with arguments (no shell):**

``` toml
[commands.claude_direct]
run = "claude"
args = ["-p", "--model", "sonnet"]
shell = false
timeout_seconds = 120
```

**With environment variables:**

``` toml
[commands.custom_api]
run = "my-api-tool"
env = { API_KEY = "$MY_API_KEY", DEBUG = "true" }
timeout_seconds = 60
```

**With custom working directory:**

``` toml
[commands.project_script]
run = "./scripts/process.sh"
cwd = "/path/to/project"
timeout_seconds = 30
```

## Transform Actions

Transform actions modify text content in-place using configured commands:

``` toml
[actions.rewrite]
type = "transform"
title = "Rewrite"
command = "claude"
input_template = """
Here's a text that I'm going to ask you to edit. The text is marked with {{context_start}}{{context_end}} tag.

The part you'll need to update is marked with {{update_start}}{{update_end}}.

{{context_start}}
{{context}}
{{context_end}}

Rewrite the given text to improve clarity and readability.
"""
```

Transform action parameters:

- `type`: Must be `"transform"`
- `title`: Display name in editor
- `command`: Reference to command configuration
- `input_template`: Template for preparing stdin input

### Attach Actions

Link content under cursor to another file, creating daily notes or collections:

``` toml
[actions.today]
type = "attach"
title = "Add to Today"
key_template = "{{today}}"
document_template = "# {{today}}\n\n{{content}}\n"

[actions.weekly_review]
type = "attach"
title = "Add to Weekly Review"
key_template = "weekly-{{today}}"
document_template = "# Weekly Review - {{today}}\n\n## Notes\n\n{{content}}\n\n## Action Items\n\n- [ ] \n"
```

Attach action parameters:

- `type`: Must be `"attach"`
- `title`: Display name in editor code actions
- `key_template`: Template for target file key (supports `{{today}}` variable)
- `document_template`: Template for new document content (supports `{{today}}` and `{{content}}` variables)

### Template Variables

**Attach Actions** support:

- `{{now}}`: Current date/time formatted using `library.date_format` (for keys) or `markdown.date_format` (for content). Supports both date and time specifiers.
- `{{today}}`: Alias for `{{now}}`
- `{{content}}`: The content being attached

**Transform Actions** support:

- `{{context}}`: Document context with the target block marked
- `{{context_start}}`, `{{context_end}}`: Context delimiters
- `{{update_start}}`, `{{update_end}}`: Update region delimiters

### Examples

**Daily Note Creation**

``` toml
[actions.daily]
type = "attach"
title = "Add to Daily Note"
key_template = "daily/{{today}}"
document_template = """# Daily Note - {{today}}

## Today's Focus

{{content}}

## Tasks
- [ ]

## Notes

"""
```

**Project Collection**

``` toml
[actions.project_ideas]
type = "attach"
title = "Add to Project Ideas"
key_template = "projects/ideas"
document_template = "# Project Ideas\n\n{{content}}\n"
```

**Text Transformation with Claude CLI**

``` toml
[commands.claude]
run = "claude -p"
timeout_seconds = 120

[actions.expand]
type = "transform"
title = "Expand"
command = "claude"
input_template = """
Here's a text that I'm going to ask you to edit. The text is marked with {{context_start}}{{context_end}} tag.

The part you'll need to update is marked with {{update_start}}{{update_end}}.

{{context_start}}
{{context}}
{{context_end}}

Expand the text you need to update, generate a couple paragraphs.
"""
```

**Simple Text Transformation**

``` toml
[commands.uppercase]
run = "tr '[:lower:]' '[:upper:]'"
timeout_seconds = 5

[actions.uppercase]
type = "transform"
title = "UPPERCASE"
command = "uppercase"
input_template = "{{context}}"
```

## Migration from Version 2

If you're upgrading from a configuration using the old `[models]` section, IWE will automatically migrate your configuration to version 3. The migration:

1.  Renames `[models]` section to `[commands]` with empty `run` values
2.  Renames `model` field to `command` in transform actions
3.  Renames `prompt_template` field to `input_template` in transform actions
4.  Removes the `context` field from transform actions

After migration, you'll need to manually update the `run` field in each command to specify the actual CLI command to execute.

**Before (version 2):**

``` toml
version = 2

[models.default]
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com"
name = "gpt-4o"

[actions.rewrite]
type = "transform"
title = "Rewrite"
model = "default"
prompt_template = "..."
context = "Document"
```

**After (version 3):**

``` toml
version = 3

[commands.default]
run = "claude -p"  # Update this to your preferred CLI command
timeout_seconds = 120

[actions.rewrite]
type = "transform"
title = "Rewrite"
command = "default"
input_template = "..."
```
