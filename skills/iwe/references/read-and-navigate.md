# Read And Navigate

Use this reference for agent-facing read workflows: discovery, hierarchy inspection, targeted retrieval, and context building.

## `iwe find`

Use `find` for discovery.

```bash
iwe find [QUERY] [OPTIONS]
```

Best uses:

- Fuzzy-search document titles and keys
- List root documents with `--roots`
- Find reference relationships with `--refs-to` and `--refs-from`

Useful examples:

```bash
iwe find
iwe find authentication
iwe find --roots
iwe find --refs-to authentication
iwe find --refs-from index
iwe find -f keys
iwe find -f json
```

Agent pattern:

1. Start with `iwe find "topic"`.
2. If results are broad, inspect roots or references.
3. Pass the chosen key into `retrieve`.

## `iwe tree`

Use `tree` for hierarchy inspection.

```bash
iwe tree [OPTIONS]
```

Useful examples:

```bash
iwe tree --depth 2
iwe tree -k my-doc
iwe tree -f keys
iwe tree -f json
```

Agent advice:

- `--depth 2` is a good default for orienting on major topic areas.
- Use `-k` when a document is in a cycle or when the user asked about one subtree.

## `iwe retrieve`

Use `retrieve` for context building.

```bash
iwe retrieve [OPTIONS]
```

Key flags:

- `-k, --key <KEY>`: target document key, repeatable
- `-d, --depth <N>`: child expansion depth
- `-c, --context <N>`: parent context depth
- `-l, --links`: include inline referenced documents
- `-e, --exclude <KEY>`: skip documents already loaded
- `-f, --format <FMT>`: `markdown`, `keys`, or `json`
- `--no-content`: metadata only
- `--dry-run`: show document and line counts first

Practical defaults:

- Focused read: `iwe retrieve -k topic -d 1 -c 1`
- Minimal read: `iwe retrieve -k topic -d 0 -c 0`
- Broader context: `iwe retrieve -k topic -d 2 -c 2`
- Programmatic chaining: `iwe retrieve -k topic -f keys`

Agent advice:

- Use `--dry-run` before deeper retrievals.
- Increase depth gradually; avoid loading too much graph context at once.
- Use `-l` only when inline references are likely important.
- Use `-e` to avoid duplicate context across repeated retrievals.

## `iwe squash`

Use `squash` when you want one combined markdown artifact.

```bash
iwe squash <KEY> [OPTIONS]
```

Useful examples:

```bash
iwe squash project-overview
iwe squash project-overview --depth 4
```

Best uses:

- Produce one linear document for export or review
- Create a compact artifact to send to another model
- Build a standalone snapshot of linked content

Tradeoff:

- `squash` gives merged structure
- `retrieve` gives graph metadata such as parents and backlinks

Use whichever matches the task.
