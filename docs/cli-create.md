# IWE Create

Creates a document in one of two explicit modes.

| Mode         | Selected by            | Who composes the document           | Key                                    |
| ------------ | ---------------------- | ----------------------------------- | -------------------------------------- |
| **Content**  | default                | you — `--content` is the whole file | required                               |
| **Template** | `--template` / `-t`    | iwe — a named template renders your variables | derived from `key_template`, or explicit |

The mode is never inferred from the shape of the input. `--content` and `--template` are mutually exclusive.

## Usage

``` bash
# Content mode -- the document is written exactly as passed
iwe create <KEY> --content '<DOCUMENT>'
cat doc.md | iwe create <KEY>

# Template mode
iwe create --template NAME --var title="..." [--var NAME=VALUE ...]
iwe create --template NAME --vars-yaml '<YAML mapping>'
iwe create --template NAME --vars-json '<JSON object>'
iwe create <KEY> --template NAME --set FIELD=VALUE [--set FIELD=VALUE ...]
```

## Options

| Flag                      | Description                                                                              | Mode     |
| ------------------------- | ---------------------------------------------------------------------------------------- | -------- |
| `<KEY>`                   | Document key. Required in content mode; optional in template mode.                        | both     |
| `-c, --content <STR>`     | The complete document, written verbatim. `-` reads it from stdin.                         | content  |
| `-t, --template <NAME>`   | Compose from the named template. The name is always required.                             | template |
| `--vars-yaml <YAML>`      | YAML mapping of template variables, values keeping their types.                            | template |
| `--vars-json <JSON>`      | JSON object of template variables, values keeping their types.                             | template |
| `--var <NAME=VALUE>`      | Set a single template variable to a verbatim string. Repeatable; overrides the bulk forms. | template |
| `--set <FIELD=VALUE>`     | Set a single frontmatter field, written above the rendered document. Repeatable.           | template |
| `-i, --if-exists <MODE>`  | `fail` / `skip`, plus `suffix` / `override` in template mode.                             | both     |
| `--strict`                | Validate against the configured document schema before writing.                           | both     |
| `-e, --edit`              | Open the created file in `$EDITOR`.                                                       | both     |

On success the absolute path of the created file is printed. `--if-exists skip` on an existing document prints nothing and exits successfully.

## Content mode

`--content` is the **complete document**: the YAML frontmatter block first (when there is one), then the markdown, normally starting with the title heading. The bytes are written to disk as given — nothing is added, nothing is moved.

``` bash
iwe create people/ada --content '---
type: person
tags: [pioneer]
---

# Ada Lovelace

First English computer programmer.'
```

``` markdown
---
type: person
tags: [pioneer]
---

# Ada Lovelace

First English computer programmer.
```

Piped input works with or without `--content -`:

``` bash
cat draft.md | iwe create projects/overview
cat draft.md | iwe create projects/overview --content -
```

`--content -` is an explicit request for stdin and is honoured at a terminal too — type the document and finish with Ctrl-D. Bare piped input is only read when stdin is not a terminal.

Because the key is explicit it *is* the document's identity, so `--if-exists` accepts only `fail` (default) and `skip`. There is no `suffix` — returning a different key than the one you asked for would be wrong — and no `override`; replacing an existing document is [`iwe update`](cli-update.md)'s job.

`--var`, `--vars-yaml`, `--vars-json` and `--set` all require `--template` — passing one without it is an argument error. In content mode the document you pass is already composed and already carries its own frontmatter, so there is nothing for them to do.

## Template mode

`--template NAME` composes the document from a template in `.iwe/config.toml`. The name is always required — nothing about template mode is implicit: `--var`, `--vars-yaml`, `--vars-json` and `--set` all require `--template`, and `--template` in turn requires a name. (`library.default_template` applies to [`iwe new`](cli-new.md) only.)

``` bash
# The stock template shipped in every configuration
iwe create --template default --var title="Standup notes"

# Named template, explicit key
iwe create meetings/2026-07-28 -t meeting --var title="Sync"
```

### Variables

`--var NAME=VALUE` sets one variable and the VALUE is used **verbatim, as a string** — never parsed. Markdown, punctuation and quotes all reach the template exactly as typed.

``` bash
iwe create -t meeting --var title=Sync --var body='## Notes'
```

Typed values — booleans, numbers, lists, nested maps — come from the bulk forms instead: `--vars-yaml '<YAML mapping>'` or `--vars-json '<JSON object>'`, at most one per command.

``` bash
iwe create -t meeting --vars-yaml 'title: Sync
attendees: [ada, alan]
draft: false'

iwe create -t meeting --vars-json '{"title": "Sync", "attendees": ["ada", "alan"], "draft": false}'
```

Those two produce identical documents: `draft` is a boolean for `{% if draft %}`, `attendees` a list for `{% for %}`.

The bulk mapping is applied first and every `--var` overrides it, **wherever the flags sit on the command line** — `--var title=A --vars-yaml 'title: B'` still yields `A`. Among `--var` flags alone the last one for a name wins. Values are replaced wholesale; there is no deep merge.

Because `--var` values are strings, `--var draft=false` is the **string** `"false"`, which `{% if draft %}` reads as true. Write `draft: false` in `--vars-yaml` when you mean the boolean. Shell quoting is stripped before iwe sees the argument, so a value-typing rule on `--var` could only ever guess; strings-only removes the guess.

- `body` is the prose slot by convention. `{{content}}` is a legacy alias for `{{body}}` in templates — `content` names the same variable, and setting both is an error.
- Template mode never reads stdin — piped input belongs to content mode. Every variable arrives through a flag; a variable no flag sets renders as empty.
- `slug`, `today`, `now` and `id` are computed by iwe and cannot be set as variables, in any of the three forms.
- The key derivation and `{{slug}}` read the `title` variable. `title` and `body` are ordinary variables by convention, not privileged parameters — a template that uses neither simply ignores them.
- `--vars-yaml ''` and `--vars-json ''` are errors; `--vars-yaml '{}'` is a valid no-op. A `--var` argument without `=` that looks like a mapping points you at `--vars-yaml`.
- A null variable value (`body: null`, `body: ~`, or a bare `body:`) is rejected — the template would render it as the text `none`. Pass `''` for an empty value.

#### Passing multiline prose

`--var` values are verbatim, so a quoted multiline argument works as-is. In a `--vars-yaml` mapping, use a block scalar:

``` bash
iwe create -t note --vars-yaml 'title: Release
body: |
  ## Notes

  Shipped.'
```

### Frontmatter

`--set FIELD=VALUE` sets one frontmatter field, written **above** the rendered document. This is metadata prepended to the render, as distinct from the variable flags, which feed the render. Unlike `--var`, `--set` values *are* parsed as YAML — variables are render text, frontmatter is data.

There is deliberately no bulk frontmatter flag. Per-field flags (`--set`, `--var`) are the CLI's way of naming one thing; bulk structured input only comes in the format-named forms (`--vars-yaml`, `--vars-json`), and frontmatter has no such pair — `iwe update --set` doesn't either. A whole mapping is repeated `--set`.

``` bash
iwe create people/ada --template default \
  --var title="Ada Lovelace" \
  --set type=person \
  --set tags='[pioneer]' \
  --set status=draft
```

``` markdown
---
type: person
tags:
- pioneer
status: draft
---

# Ada Lovelace
```

- `--set` is repeatable, its VALUE is parsed as YAML, and the last `--set` for a field wins. YAML typing means `--set version=1.10` stores the number `1.1` and `--set note=~` stores null; quote to force a string, `--set version='"1.10"'`.
- Fields are applied in command-line order. A new field is appended; a repeated field is replaced in place, keeping its position. Values are replaced wholesale — there is no deep merge.
- Fields starting with `_`, `$`, `.`, `#` or `@` are reserved by IWE and are dropped.
- When the template already emits its own frontmatter, passing `--set` is an error. Drop the flag, or drop the frontmatter from the template.

### Mode boundaries

Variable values are trusted as passed — template mode does not inspect them. One check keeps composition honest: `--set` against a template that emits its own frontmatter is rejected, since the document would carry two frontmatter blocks.

If you already hold the complete document — frontmatter, title heading and all — pass it to `--content` and skip composition entirely.

### Keys

Without a `<KEY>` the key comes from the template's `key_template`. With a `<KEY>` that derivation is skipped and the title still fills the document body.

`--if-exists` defaults to `suffix` for a derived key (quick capture appends `-1`, `-2`, …) and to `fail` for an explicit key, since an explicit key asserts an identity.

## Template variables

| Variable      | Value                                                                                         |
| ------------- | --------------------------------------------------------------------------------------------- |
| `{{title}}`   | The `title` variable                                                                            |
| `{{body}}`    | The `body` variable. `{{content}}` is a legacy alias                                            |
| `{{slug}}`    | URL-safe form of the `title` variable                                                           |
| `{{today}}`   | Current date — `library.date_format` for the key, `markdown.date_format` for the document       |
| `{{now}}`     | Current date and time — `library.time_format` / `markdown.time_format`, each falling back to the matching `date_format` |
| `{{id}}`      | Random 8-character alphanumeric ID                                                              |

Any other name passed with `--var`, `--vars-yaml` or `--vars-json` is available under that name.

## Configuration

Templates are defined in `.iwe/config.toml`:

``` toml
[templates.default]
key_template = "{{slug}}"
document_template = "# {{title}}\n\n{{body}}"

[templates.journal]
key_template = "journal/{{today}}"
document_template = "# {{today}}\n\n{{body}}"
```

## Examples

``` bash
# Write a document you already have
iwe create projects/overview --content "$(cat overview.md)"

# Idempotent create for scripts and agents
cat note.md | iwe create notes/inbox --if-exists skip

# Quick capture through the stock template
iwe create --template default --var title="Random idea"

# Named template with structured variables
iwe create -t meeting --vars-json '{"title": "Sync", "attendees": ["ada", "alan"]}'

# Reject the write when it breaks the document schema
iwe create docs/one --content "$(cat one.md)" --strict
```

## See also

- [`iwe update`](cli-update.md) — replace or mutate an existing document
- [`iwe new`](cli-new.md) — title-first quick capture with `library.default_template`
