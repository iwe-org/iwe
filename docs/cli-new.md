# IWE New

Creates a new document from a template.

## Usage

``` bash
iwe new <TITLE> [OPTIONS]
```

## Arguments

- `<TITLE>`: Title for the new document (required)

## Options

- `-t, --template <NAME>`: Template name from config (default: "default")
- `-c, --content <CONTENT>`: Initial content for the document
- `-k, --key <KEY>`: Explicit document key, bypassing the template's key derivation. Subdirectory keys are allowed (e.g. `people/ada`). Omit the file extension.
- `-i, --if-exists <MODE>`: Behavior when file already exists (default: "suffix", or "fail" when `--key` is given)
  - `suffix`: Append `-1`, `-2`, etc. to filename until unique
  - `override`: Overwrite existing file
  - `skip`: Do nothing, exit successfully without output
  - `fail`: Report an error and exit with a non-zero status
- `-e, --edit`: Open created file in `$EDITOR` after creation

## Explicit keys

By default the filename is derived from the title (slugified) through the template's `key_template`. Pass `--key` to set the document key yourself and skip that derivation — the title still fills the document body. Use this when the key is a stable identifier drawn from metadata (an entity name, a session date) rather than the title wording.

Because an explicit key asserts an identity, `--key` defaults `--if-exists` to `fail`: creating a document whose key already exists reports an error instead of silently appending a `-1` suffix. Pass `--if-exists skip` or `--if-exists override` to opt into idempotent or forced re-creation.

## What it does

- Creates a new markdown file using the specified template
- Generates filename from the title (slugified)
- Supports content from command-line argument or stdin pipe
- Prints the absolute path of the created file to stdout
- Optionally opens the file in your configured editor

## Template Variables

Templates support the following variables:

- `{{title}}`: The provided title argument
- `{{slug}}`: Slugified title (kebab-case)
- `{{today}}`: Current date (uses `library.date_format` for key, `markdown.date_format` for content). Intended for date-only formatting.
- `{{now}}`: Current date/time (uses `library.time_format` for key, `markdown.time_format` for content). Falls back to `date_format` if `time_format` is not set. Supports both date specifiers (`%Y`, `%m`, `%d`) and time specifiers (`%H`, `%M`, `%S`).
- `{{id}}`: Random 8-character alphanumeric ID
- `{{content}}`: Content from `-c` option or stdin

## Examples

``` bash
# Create a new note with default template
iwe new "My New Note"
# Creates: my-new-note.md (or my-new-note-1.md if exists)

# Create with content
iwe new "Meeting Notes" --content "Discussed project timeline"

# Pipe content from clipboard (macOS)
pbpaste | iwe new "Clipboard Note"

# Create and open in editor
iwe new "Quick Idea" --edit

# Use a custom template
iwe new "Daily Journal" --template journal

# Overwrite existing file
iwe new "My Note" --if-exists override

# Skip if file exists (useful in scripts)
iwe new "My Note" --if-exists skip

# Create at an explicit key (fails if people/ada already exists)
iwe new "Ada Lovelace" --key people/ada

# Idempotent create at an explicit key
iwe new "Ada Lovelace" --key people/ada --if-exists skip
```

## Configuration

Templates are defined in `.iwe/config.toml`:

``` toml
[library]
default_template = "default"  # Optional: set default template

[templates.default]
key_template = "{{slug}}"
document_template = "# {{title}}\n\n{{content}}"

[templates.journal]
key_template = "journal/{{today}}"
document_template = "# {{today}}\n\n{{content}}"
```
