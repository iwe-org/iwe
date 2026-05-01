# IWE CLI Output Format Spec — Markdown & Keys

## 1. Overview

Scope of this file: the `markdown` and `keys` output formats produced by the `iwe` CLI. These are the human-facing plaintext formats. They are projections of the underlying logical data; the structured forms in the JSON/YAML spec are authoritative when shapes disagree (see cross-format invariant in `output-format-json-yaml-spec.md` §2.2).

Companion specs:

- `output-format-json-yaml-spec.md` — structured JSON/YAML wire shapes.
- `output-format-other-spec.md` — `dot`, `csv`, `count` integer, mutation prose.

Out of scope: see §10.

## 2. Format set

Commands that accept `-f markdown` or `-f keys`:

| Command | `markdown` | `keys` | Default |
|---|---|---|---|
| `iwe find` | ✓ | ✓ | `markdown` |
| `iwe retrieve` | ✓ | ✓ | `markdown` |
| `iwe tree` | ✓ | ✓ | `markdown` |
| `iwe stats` (aggregate) | ✓ | — | `markdown` |
| `iwe delete`, `rename`, `extract`, `inline` | ✓ | ✓ | `markdown` |

Markdown is a human surface and may omit detail (e.g. counts, edge metadata) for readability — but it MUST NOT contradict the structured form for the same query. `keys` is a strict projection: one key per line, no header, no trailing blank.

## 3. Common shapes used by markdown frontmatter

`EdgeRef` inside markdown frontmatter:

- Always emits `key`, `title`.
- Emits `sectionPath` only when non-empty (the human surface tolerates omission; JSON/YAML always include it).

User frontmatter inside markdown documents has reserved-prefix entries (`_`, `$`, `.`, `#`, `@` per `query-language-spec.md` §2.3) already stripped — they MUST NOT appear in any output.

Full structured schemas (`KeyTitleRef`, `EdgeRef`, `Frontmatter`): see `output-format-json-yaml-spec.md` §3.

## 4. Read-side commands

### 4.1 `iwe find`

#### 4.1.1 Markdown

A header line, a blank line, then one line per result.

```
Found N results[ for "Q"][ (showing M)]:

<title>   #<key>
<title>   #<key>
...
```

`for "Q"` appears iff a positional query was given. `(showing M)` appears iff `--limit` truncated the result set. The separator between title and key is **three spaces**. Trailing newline after the last result line; no trailing blank.

> **Spec note:** the current `render_find_output` in `crates/iwe/src/main.rs:822-833` injects inline `↖<parent-title>` chips into the title field when `includedBy` is non-empty. The spec drops these chips — markdown stays one line per result; structured needs go through `-f json|yaml`.

#### 4.1.2 Keys

One key per line, no header, no blank lines, no trailing blank.

### 4.2 `iwe retrieve`

#### 4.2.1 Markdown

Each returned document is wrapped in a **four-backtick fenced code block** with the info string `markdown #<key>`. The whole stdout is therefore a valid meta-markdown document: every inner document appears as one fenced block, keyed by its identity. Inside the fence:

1. A YAML frontmatter block — flat, no wrapper key. The structure mirrors the JSON `DocumentOutput` shape (see `output-format-json-yaml-spec.md` §4.2.1), minus the fields lifted out: `key` lives in the fence info string, and the body lives below the frontmatter, not as a `content:` field.
2. A blank line.
3. The rendered markdown content.

The fence info string carries the key prefixed with `#` (e.g. `markdown #child`), matching the `#<key>` convention used by `iwe find` markdown output (§4.1.1). If the embedded content contains a four-or-more-backtick fence, use one more backtick for the outer fence so the inner fence cannot terminate it.

Frontmatter fields:

| Location | Field | Required? | Source |
|---|---|---|---|
| fence info | `key` | always | `DocumentOutput.key` |
| frontmatter | `title` | always | `DocumentOutput.title` |
| frontmatter | `includedBy` | omit when empty | `EdgeRef` list |
| frontmatter | `includes` | omit when empty | `EdgeRef` list |
| frontmatter | `referencedBy` | omit when empty | `EdgeRef` list |

`EdgeRef` inside frontmatter omits `sectionPath` when empty. There is no `document:` wrapper map; the frontmatter is always flat. Field naming matches JSON (no `parents` / `back-links` aliases).

`````
````markdown #child
---
title: Child Document
includedBy:
  - key: parent
    title: Parent Document
    sectionPath:
      - Overview
---

# Child Document

Child content.
````
`````

A multi-document stream concatenates blocks separated by **exactly one blank line** between the closing fence of one block and the opening fence of the next; no trailing blank line after the final block:

`````
````markdown #doc-a
---
title: Doc A
---

# Doc A

Body.
````

````markdown #doc-b
---
title: Doc B
---

# Doc B

Body.
````
`````

Consumers can split the stream on the four-backtick fence boundaries; the info string identifies the key.

> **Spec note:** the current renderer in `crates/iwe/src/render.rs:11-30, 47-54, 88-94` emits a bare frontmatter+body with `parents` / `back-links` keys, wraps fields under `document:`, and joins documents with no separator. The spec requires the four-backtick `markdown #<key>` fence wrapper, JSON-aligned field names (`includedBy`, `includes`, `referencedBy`), flat frontmatter with no wrapper map, and exactly one blank line between blocks.

#### 4.2.2 Keys

One key per line, in the order documents appear in the envelope. No header.

### 4.3 `iwe tree`

#### 4.3.1 Markdown

Nested unordered list, two-space indent per depth level, each entry as a markdown link `[<title>](<key>)`:

```
- [AI Agent Memory](ai-memory)
  - [Post One](post-1)
  - [Post Two](post-2)
```

#### 4.3.2 Keys

One key per line, **tab-indented** by depth (root has zero tabs). Order matches a depth-first walk of the tree.

```
ai-memory
    post-1
    post-2
```

### 4.4 `iwe stats` (aggregate, markdown)

Aggregate `markdown` is a human-readable report of corpus-level statistics. Per-document stats (`-k KEY`) does not produce markdown — see `output-format-json-yaml-spec.md` §4.4 for the per-doc shape and `output-format-other-spec.md` §4 for csv.

## 5. Mutation commands — selection-style

`delete`, `rename`, `extract`, `inline` produce a status report and accept `-f markdown|keys`.

### 5.1 `markdown` — status report

A human-prose report of what changed. Each command picks its own header line(s), but the structure is consistent:

```
<verb-ing> '<source-key>'[ to '<target-key>']
Updated N document(s)
```

Or, with `--dry-run`:

```
Would <verb> '<source-key>'[ to '<target-key>']
Would update N document(s)
  <key>
  <key>
```

`--quiet` suppresses both forms.

### 5.2 `keys` — affected-keys list

One key per line, no header. The list contains every key that was modified — the operation's primary target plus every document whose references were rewritten. With `--dry-run`, the list is the keys that *would* be modified.

## 6. Flag effects on shape

Flags that change the *shape* (not just the selection) of the markdown/keys output. Selection-only flags (filter, sort, limit, anchors) live in `query-cli-spec.md`.

### 6.1 `iwe find`

| Flag | Effect on shape |
|---|---|
| `--project f1,f2,...` | No effect on `markdown` (titles only) or `keys` (keys only). For structured-format effects, see `output-format-json-yaml-spec.md` §5.1. |
| `--add-fields f1,f2,...` | Same. The flag is additive over the default projection in structured output (`query-projection-spec.md` §3.5); `markdown` and `keys` still render as titles-only / keys-only. The block-vs-line decision in `query-projection-spec.md` §6 applies whether structural sources arrive via `--project` or `--add-fields`. |

### 6.2 `iwe retrieve`

| Flag | Effect on shape |
|---|---|
| `--no-content` | Document body is empty; the frontmatter block remains. |
| `--children` | When non-empty, populates the `includes:` list inside frontmatter. Independent of `--no-content`. |
| `-b`, `--backlinks` | When non-empty, populates the `referencedBy:` list inside frontmatter. |
| `--dry-run` | Replaces normal output with the prose form `documents: N\nlines: N`. |

> **Spec note:** the current `--dry-run` path in `crates/iwe/src/main.rs:691-701` always prints the prose form and ignores `-f`. The structured form is documented in `output-format-json-yaml-spec.md` §5.2.

### 6.3 Mutation commands

| Flag | Effect on shape |
|---|---|
| `-f keys` | Switches from prose status to one-key-per-line. |
| `--dry-run` | Prefixes prose with `Would …`; for `keys`, lists the keys that *would* be affected. Suppresses writeback. |
| `--quiet` | Suppresses prose-form output. Has no effect on `-f keys`. |

## 7. Stable-schema rule (markdown is exempt)

The stable-schema rule in `output-format-json-yaml-spec.md` §6 requires every declared field to appear in every emission. Markdown is a human surface and is exempt:

- `RetrieveRenderer` MAY omit `includedBy` / `includes` / `referencedBy` from frontmatter when empty.
- `EdgeRef` inside frontmatter MAY omit `sectionPath` when empty.
- `key` is always carried in the fence info string; `title` is always in frontmatter.

`keys` output is a strict projection — one key per line, no envelope — so the rule does not apply.

## 8. Error output

All commands write errors and progress to stderr. Stdout carries only the format-determined payload. An error path MUST NOT print partial output to stdout — if the command fails before producing a complete result, stdout is empty and the process exits non-zero.

Error message form is free-text, prefixed `Error: ` (or `error: ` for parse-stage failures, matching the language spec).

## 9. Examples

```
iwe find --filter 'pillar: ai-memory'                            # markdown (default)
iwe find --filter 'pillar: ai-memory' -f keys

iwe retrieve -k ai-memory -d 1                                   # markdown
iwe retrieve -k ai-memory --no-content                           # blank body, no children
iwe retrieve -k ai-memory --dry-run                              # documents: N\nlines: N

iwe tree -k ai-memory -d 2                                       # nested list
iwe tree -k ai-memory -d 2 -f keys                               # tab-indented keys
```

## 10. Out of scope

- **Color, pagination, TTY rendering.** Markdown is plain markdown; downstream tools handle presentation.
- **Streaming output.** All output is a single document.
- **Schema versioning.** Future field additions are additive; consumers MUST tolerate unknown frontmatter keys.
