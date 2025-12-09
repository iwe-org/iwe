# Configuration

IWE projects are configured through a `.iwe/config.toml` file in your project root. Below are all available configuration options.

## Basic Configuration

``` toml
[markdown]
refs_extension = ""
date_format = "%b %d, %Y"

[library]
path = ""
date_format = "%Y-%m-%d"
prompt_key_prefix = "prompts/"

[completion]
link_format = "markdown"
```

### Markdown Settings

- `refs_extension`: File extension for markdown references (default: empty, uses `.md`)
- `date_format`: Date format for markdown content display (default: `"%b %d, %Y"`, e.g., "Jan 15, 2024")

### Library Settings

- `path`: Subdirectory for markdown files relative to project root (default: empty, uses root)
- `date_format`: Date format for file key generation (default: `"%Y-%m-%d"`, e.g., "2024-01-15")
- `prompt_key_prefix`: Prefix for AI prompt keys (default: none)

### Completion Settings

- `link_format`: Format for auto-completed links (default: `"markdown"`)
  - `"markdown"`: Creates `[title](key)` style links
  - `"wiki"`: Creates `[[key]]` style WikiLinks

### Date Format Patterns

Date formats use [chrono format specifiers](https://docs.rs/chrono/latest/chrono/format/strftime/index.html):

- `%Y`: 4-digit year (2024)
- `%y`: 2-digit year (24)
- `%m`: Month as number (01-12)
- `%b`: Abbreviated month name (Jan)
- `%B`: Full month name (January)
- `%d`: Day of month (01-31)
- `%A`: Full weekday name (Monday)
- `%a`: Abbreviated weekday name (Mon)

## AI Models

Define LLM models for AI-powered actions:

``` toml
[models.default]
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com"
name = "gpt-4o"

[models.fast]
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com"
name = "gpt-4o-mini"
```

Each model requires:

- `api_key_env`: Environment variable containing API key
- `base_url`: API endpoint URL
- `name`: Model name

Optional parameters:

- `max_tokens`: Maximum tokens for input
- `max_completion_tokens`: Maximum tokens for completion
- `temperature`: Sampling temperature (0.0-1.0)

## AI Actions

IWE supports two types of actions for editor integration:

### Transform Actions

AI-powered text editing that modifies content in-place:

``` toml
[actions.rewrite]
type = "transform"
title = "Rewrite"
model = "default"
context = "Document"
prompt_template = """
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
- `model`: Reference to model configuration
- `context`: Context scope (`"Document"`)
- `prompt_template`: AI prompt with template variables

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

- `{{today}}`: Current date formatted using `library.date_format` (for keys) or `markdown.date_format` (for content)
- `{{content}}`: The content being attached

**Transform Actions** support:

- `{{context}}`: Document context
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
