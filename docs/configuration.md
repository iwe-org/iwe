# Configuration

IWE projects are configured through a `.iwe/config.toml` file in your project root. Below are all available configuration options.

## Basic Configuration

```toml
prompt_key_prefix = "prompt"

[markdown]
refs_extension = ""

[library]
path = ""
```

- `prompt_key_prefix`: Prefix for AI prompt keys (default: "prompt")
- `markdown.refs_extension`: File extension for markdown references (default: empty, uses `.md`)
- `library.path`: Subdirectory for markdown files relative to project root (default: empty, uses root)

## AI Models

Define LLM models for AI-powered actions:

```toml
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

Define custom AI-powered text editing actions:

```toml
[actions.rewrite]
title = "Rewrite"
model = "default"
context = "Document"
prompt_template = """
Here's a text that I'm going to ask you to edit...
"""
```

Each action requires:
- `title`: Display name in editor
- `model`: Reference to model name
- `context`: Context type ("Document")
- `prompt_template`: Prompt with `{{context}}`, `{{context_start}}`, `{{context_end}}`, `{{update_start}}`, `{{update_end}}` placeholders
