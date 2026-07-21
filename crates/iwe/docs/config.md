# Configuration

An IWE project is a directory containing a `.iwe/` marker directory. All
settings live in `.iwe/config.toml`; every section and field is optional,
and an absent field takes its default. `iwe init` creates the marker with a
starter config.

A complete configuration with every field at its default (unless noted):

```toml
version = 3
format = "markdown"

[markdown]
refs_extension = ""
refs_path = "relative"
refs_text = "preserve"
date_format = "%b %d, %Y"
time_format = "%b %d, %Y %H:%M"
locale = "en_US"
wiki_link_path = "preserve"

[markdown.formatting]
emphasis_token = "*"
strong_token = "**"
list_token = "-"
ordered_list_token = "."
code_block_token = "`"
code_block_token_count = 3
increment_ordered_list_bullets = true
ordered_list_content_indent = 4
bullet_list_content_indent = 4
rule_token = "-"
rule_token_count = 72
wrap_column = 80
preserve_line_breaks = false
line_break_style = "backslash"
preserve_newlines = false

[library]
path = ""
date_format = "%Y-%m-%d"
time_format = "%Y-%m-%d %H:%M"
locale = "en_US"
default_template = "default"
frontmatter_document_title = "title"

[completion]
link_format = "markdown"
min_prefix_length = 0
trigger_characters = ["["]

[search]
language = "english"

[templates.default]
key_template = "{{slug}}"
document_template = "# {{title}}\n\n{{content}}"

[commands.claude]
run = "claude -p"
timeout_seconds = 120

[actions.rewrite]
type = "transform"
title = "Rewrite"
command = "claude"
input_template = "{{context}}"

[schemas.note]
match = "notes/**"
```

## Top level

- `version`: config format version; current is `3`.
- `format`: source format for the library, `"markdown"` (default) or
  `"djot"`. With `"djot"`, a `[djot]` section mirrors `[markdown]` (same
  fields except `wiki_link_path`).

## `[markdown]`

- `refs_extension`: file extension written inside markdown links (default
  empty — links are written without an extension).
- `refs_path`: how the path inside a regular markdown link (`[…](…)`) is
  written (default `"relative"`). `"relative"` writes each link relative to
  the linking document's directory; `"absolute"` writes a root-absolute path
  from the library root (`/dir/note.md`). Resolution is unaffected — a link
  with a leading `/` always resolves from the library root regardless.
- `refs_text`: how the text of a regular markdown link is written (default
  `"preserve"`). `"preserve"` keeps the text as typed; `"normalize"`
  rewrites it to the linked document's title. Wiki links are unaffected.
- `date_format`: date format for document content and the `{{today}}`
  variable (default `"%b %d, %Y"`, e.g. "Jan 15, 2024").
- `time_format`: format for the `{{now}}` variable in content (default:
  falls back to `date_format`).
- `locale`: locale for date formatting in content (default: system locale).
  Accepts POSIX (`de_DE`) and BCP47 (`de-DE`) forms; encoding suffixes like
  `.UTF-8` are stripped.
- `wiki_link_path`: how the path inside a wiki link (`[[…]]`) is written
  (default `"preserve"`). `"preserve"` keeps each link as typed, `"full"`
  rewrites to the full key path, `"short"` rewrites to the shortest
  unambiguous suffix. Resolution of existing links is unaffected.

## `[markdown.formatting]`

Controls how documents are rendered on normalization and formatting. Invalid
values fall back to defaults.

- `emphasis_token`: `"*"` (default) or `"_"`.
- `strong_token`: `"**"` (default) or `"__"`.
- `list_token`: `"-"` (default), `"*"`, or `"+"`.
- `ordered_list_token`: `"."` (default, `1. item`) or `")"` (`1) item`).
- `code_block_token`: `` "`" `` (default) or `"~"`.
- `code_block_token_count`: minimum fence length (default `3`).
- `increment_ordered_list_bullets`: `true` (default) numbers items `1.`,
  `2.`, `3.`; `false` numbers every item `1.`.
- `ordered_list_content_indent` / `bullet_list_content_indent`: minimum
  column where list item content and continuation lines start. Accepts
  `2`–`4`; unset means content aligns one space after the marker. Set `4`
  for MkDocs-style alignment.
- `rule_token`: horizontal rule character, `"-"` (default), `"*"`, or `"_"`.
- `rule_token_count`: rule length (default `72`).
- `wrap_column`: wrap paragraphs at this column (default unset — no
  wrapping; minimum effective value `20`). Inline code, wiki links, math,
  and link URLs stay atomic.
- `preserve_line_breaks`: keep hard line breaks (`  \n` or `\`-newline)
  instead of dropping them (default `false`).
- `line_break_style`: how preserved hard breaks are emitted —
  `"backslash"` (default) or `"spaces"`. Takes effect only with
  `preserve_line_breaks = true`.
- `preserve_newlines`: keep soft line breaks inside a paragraph instead of
  joining the lines (default `false`) — supports one-sentence-per-line
  authoring through normalization.

## `[library]`

- `path`: subdirectory holding the markdown files, relative to the project
  root (default empty — the root itself).
- `date_format`: date format for key generation and the `{{today}}` variable
  in key templates (default `"%Y-%m-%d"`).
- `time_format`: format for `{{now}}` in key templates (default: falls back
  to `date_format`).
- `locale`: locale for date formatting in keys (default: system locale).
  Separate from `markdown.locale`, so keys and content can use different
  languages.
- `default_template`: template used by `iwe new` when `--template` is not
  given.
- `frontmatter_document_title`: frontmatter field to use as the document
  title (default: none — the first header is the title). Affects link text,
  completion, and search results; falls back to the first header when the
  field is missing.

## `[completion]`

Editor completion via the LSP server (`iwes`).

- `link_format`: `"markdown"` (default, `[title](key)`) or `"wiki"`
  (`[[key]]`). Overridden by a typed `[` or `[[` prefix at the cursor.
- `min_prefix_length`: minimum characters typed before completions appear
  (default `0`).
- `trigger_characters`: characters that open the completion popup (default
  `["["]`).

## `[search]`

- `language`: stemmer language for BM25 full-text search (default
  `"english"`).

## `[templates]`

Document templates for `iwe new`. Each `[templates.<name>]` entry has:

- `key_template`: derives the document key.
- `document_template`: the initial document content.

Template variables: `{{title}}` (the title argument), `{{slug}}` (slugified
title), `{{today}}` (date, via `library.date_format` for keys and
`markdown.date_format` for content), `{{now}}` (date/time, via the
`time_format` fields), `{{id}}` (random 8-character alphanumeric), and
`{{content}}` (content from `-c` or stdin; document template only).

```toml
[templates.journal]
key_template = "journal/{{today}}"
document_template = "# {{today}}\n\n{{content}}"
```

## `[commands]`

External CLI commands used by transform actions. Input arrives on stdin;
stdout becomes the replacement content.

- `run`: command to execute (required; runs via `sh -c` by default).
- `args`: argument array for direct execution (used only with
  `shell = false`).
- `cwd`: working directory.
- `env`: environment variables (`$VAR` / `${VAR}` expand from the parent
  environment).
- `shell`: execute via shell (`true`, default) or directly (`false`).
- `timeout_seconds`: maximum execution time (default `120`).

```toml
[commands.rewriter]
run = "claude"
args = ["-p", "--model", "sonnet"]
shell = false
timeout_seconds = 120

[commands.uppercase]
run = "tr '[:lower:]' '[:upper:]'"
timeout_seconds = 5
```

## `[actions]`

Editor code actions served by the LSP server. Every entry has a `type` and a
`title` (the name shown in the editor); the remaining fields depend on the
type.

- `transform`: pipe a block through a command and replace it with the
  output. Fields: `command` (a `[commands]` entry name), `input_template`.
  Template variables: `{{context}}` (the document with the target block
  marked), `{{context_start}}`, `{{context_end}}`, `{{update_start}}`,
  `{{update_end}}`.
- `attach`: link the content under the cursor into another document,
  creating it from a template when missing. Fields: `key_template`,
  `document_template`. Template variables: `{{today}}`, `{{now}}`,
  `{{content}}`.
- `sort`: sort the list under the cursor. Field: `reverse` (optional bool).
- `inline`: inline the referenced document at the cursor. Fields:
  `inline_type` (`"section"` or `"quote"`), `keep_target` (optional bool —
  keep the referenced document instead of deleting it).
- `extract`: extract the section under the cursor into a new document,
  leaving a link behind. Fields: `key_template`, `link_type` (optional,
  `"markdown"` or `"wiki"`).
- `extract_all`: extract every subsection of the section under the cursor.
  Same fields as `extract`.
- `link`: turn the text under the cursor into a link to a new document.
  Same fields as `extract`.

The `extract`, `extract_all`, and `link` key templates support `{{title}}`,
`{{slug}}`, `{{today}}`, `{{id}}`, plus `{{parent.title}}`,
`{{parent.slug}}`, `{{parent.key}}` and `{{source.key}}`,
`{{source.title}}`, `{{source.slug}}`, `{{source.path}}`,
`{{source.file}}`.

```toml
[actions.today]
type = "attach"
title = "Add to Today"
key_template = "{{today}}"
document_template = "# {{today}}\n\n{{content}}\n"

[actions.sort_list]
type = "sort"
title = "Sort list"

[actions.inline_section]
type = "inline"
title = "Inline section"
inline_type = "section"

[actions.extract_section]
type = "extract"
title = "Extract section"
key_template = "{{slug}}"
```

## `[schemas]`

Bind document schemas to documents (see `iwe docs schema`). Each entry names
a schema file in `.iwe/schemas/` and a glob (or list of globs) matched
against document keys:

```toml
[schemas.person]
match = "people/**"

[schemas.session]
match = ["journal/*", "meetings/**"]
```

- The entry name is the schema name: `[schemas.person]` resolves to
  `.iwe/schemas/person.yaml`.
- `match` globs follow gitignore/globset syntax: `*` stays within a path
  segment, `**` crosses segments; a leading `/` is optional — patterns are
  anchored at the library root.
- Binding is order-free: a document is validated against **every** schema
  whose `match` hits. A document matching no entry is unvalidated.

Run `iwe schema validate` to check the store against these bindings.

## Date format patterns

Date and time formats use chrono strftime specifiers: `%Y` (2024), `%y`
(24), `%m` (01–12), `%b` (Jan), `%B` (January), `%d` (01–31), `%A`
(Monday), `%a` (Mon), `%H` (00–23), `%M` (00–59), `%S` (00–59). Textual
specifiers are localized by the `locale` setting.

Examples: `"%Y-%m-%d %H:%M"` → "2024-01-15 14:30";
`"%Y%m%d%H%M"` → "202401151430" (sortable keys).
