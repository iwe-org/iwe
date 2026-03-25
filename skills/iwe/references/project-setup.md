# Project Setup

Use this reference when you need to confirm whether the current workspace is an IWE project and how the project is configured.

## Project marker

An IWE project is identified by:

```text
.iwe/config.toml
```

Running `iwe init` creates that directory and file.

## Important config fields

Read `.iwe/config.toml` before assuming file locations or defaults.

### `library.path`

- `""`: markdown files live at project root
- `"docs"` or another path: markdown files live in that subdirectory

### `library.default_template`

- Optional default template name for `iwe new`

### `markdown.refs_extension`

- Controls whether references use no extension or something like `.md`

### `completion.link_format`

- May indicate whether the project prefers markdown links or wiki links

## `iwe init`

Basic usage:

```bash
iwe init
```

What it creates:

```text
.iwe/
└── config.toml
```

Safe assumptions:

- Re-running `iwe init` in an existing project does not overwrite the config.
- The default config also defines extract, inline, and template behavior.

## Agent advice

- Check config before searching for note files manually.
- If `iwe` behavior seems surprising, inspect `.iwe/config.toml` before assuming a bug.
- When creating new notes, prefer `iwe new` so project templates and naming rules are respected.
