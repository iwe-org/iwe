# IWE CLI Output Format Spec — JSON & YAML

## 1. Overview

Scope of this file: the `json` and `yaml` structured output formats produced by the `iwe` CLI. JSON and YAML are isomorphic — same keys, same values, same nesting; only the surface syntax differs. They are the authoritative wire shape: when markdown disagrees, JSON/YAML wins.

Companion specs:

- `output-format-markdown-spec.md` — `markdown` and `keys` plaintext formats.
- `output-format-other-spec.md` — `dot`, `csv`, `count` integer, mutation prose.

This file is a companion to `query-cli-spec.md` (which defines the flag surface and which formats each command accepts) and `query-language-spec.md` (which defines the corpus model, reserved-prefix rule, and the operations that produce results).

Out of scope: see §9.

## 2. Format set and invariants

### 2.1 Per-command format matrix

| Command | `json` | `yaml` | Default |
|---|---|---|---|
| `iwe find` | ✓ | ✓ | `markdown` |
| `iwe retrieve` | ✓ | ✓ | `markdown` |
| `iwe tree` | ✓ | ✓ | `markdown` |
| `iwe stats` | ✓ | ✓ | `markdown` |

`iwe count` and the mutation commands have no `-f json|yaml` form (see `output-format-other-spec.md`).

### 2.2 Cross-format invariant

A given query MUST encode the same logical result data in JSON and YAML. JSON and YAML are isomorphic at the field level. Markdown is a human projection of the same data and may omit detail for readability, but MUST NOT contradict JSON/YAML. `keys` is a strict projection: one key per line, no header.

### 2.3 Field-name convention

| Surface | Convention |
|---|---|
| JSON keys | `camelCase` |
| YAML keys (top-level output and frontmatter inside markdown) | `camelCase` (same as JSON) |
| Markdown body text (headers, prose) | sentence case as authored |

Rationale: a single name per concept across all surfaces avoids the current `back-links` / `referencedBy` / `referenced_by` triple-naming. Pick one external name per field and use it everywhere.

> **Spec note:** the current `RetrieveRenderer` in `crates/iwe/src/render.rs:11-30` emits `parents` and `back-links` in markdown frontmatter, while JSON uses `includedBy` and `referencedBy`. Align on the JSON names in markdown frontmatter as well.

## 3. Common shapes

These shapes are referenced by the per-command specs in §4.

### 3.1 `KeyTitleRef`

```yaml
{ key: string, title: string }
```

Used wherever one document references another *by identity* without carrying any positional context.

### 3.2 `EdgeRef`

```yaml
{ key: string, title: string, sectionPath: [string] }
```

`KeyTitleRef` extended with `sectionPath`: the chain of section header texts (root-to-leaf) under which the edge appears in the source document. Empty array when the edge sits at the document root.

Used for every inclusion or reference edge surfaced in structured output: `includedBy`, `includes`, `referencedBy`.

> **Spec note:** the current `ChildDocumentInfo` struct in `crates/liwe/src/retrieve.rs:31-34` lacks `sectionPath`. Align all three edge types on `EdgeRef`.

### 3.3 User frontmatter merging

`find` and `tree` flatten user frontmatter into each result/node alongside system fields (`key`, `title`, etc.) — there is no nested `frontmatter` object. Reserved-prefix entries (`_`, `$`, `.`, `#`, `@` per `query-language-spec.md` §2.3) are stripped before merging and MUST NOT appear in any output. On collision between a user frontmatter key and a system field of the same name, the user value wins.

`--project f1,f2,...` restricts the result to the listed fields, in the listed order; system fields and user frontmatter fields are projectable interchangeably. Without `--project`, every system field plus every user frontmatter field is emitted.

### 3.4 Stable schema

All structured fields in JSON and YAML are emitted as their empty value (`{}`, `[]`, `null`, `""`) rather than omitted, unless §4 explicitly marks them conditional. Markdown frontmatter (a human surface) MAY omit empty optional fields. See §6 for the full rule.

## 4. Per-command shapes

### 4.1 `iwe find`

#### 4.1.1 Shape

The top-level value is an **array** of `FindResult` — no envelope. Pagination and query-echo metadata (the original positional query, effective `--limit`, total-before-limit) are out of scope for v1; a future revision may add an `--envelope` flag that wraps the array under a metadata header. Until then, callers that need `total` should re-run `iwe count` with the same filter.

```yaml
[FindResult]
```

A `FindResult` is a flat mapping. There is no nested `frontmatter` object: user frontmatter fields are siblings of `key`, `title`, and the system-derived counts and edge arrays. Without `--project` the result carries the full set of system fields plus all user frontmatter fields. With `--project` the result carries only the listed fields, in the listed order; system fields and user frontmatter fields are projectable interchangeably.

System fields (the full set emitted without `--project`):

```yaml
FindResult:
  key:           string
  title:         string
  includedBy:    [EdgeRef]   # always present; [] when none
  # ... plus every user frontmatter field, merged at top level
```

Reserved-prefix entries (`_`, `$`, `.`, `#`, `@` per `query-language-spec.md` §2.3) are stripped from user frontmatter before merging and MUST NOT appear in output. On collision between a system field and a user frontmatter field of the same name (e.g. when `frontmatter_document_title = "title"` populates `title` from user frontmatter), the user frontmatter value wins; system synthesis fills only when the user field is absent.

> **Spec note:** the current `FindResult` in `crates/liwe/src/find.rs:21-33` emits a nested `frontmatter` object. The spec requires flattening: drop the `frontmatter` key and emit user fields at top level.

#### 4.1.2 JSON

```json
[
  {
    "key": "doc1",
    "title": "Document One",
    "includedBy": []
  }
]
```

Trailing newline after the closing `]`.

#### 4.1.3 YAML

```yaml
- key: doc1
  title: Document One
  includedBy: []
```

### 4.2 `iwe retrieve`

> **Status:** `retrieve` is slated for deprecation. The shape below documents current behavior; new consumers should prefer `find` (see §4.2.4 for the migration map). No new fields will be added here.

#### 4.2.1 Shape

The top-level value is an **array** of `DocumentOutput` — no envelope, matching `find` (§4.1.1).

```yaml
DocumentOutput:
  key:           string
  title:         string
  content:       string         # rendered markdown body only — source-file YAML frontmatter is always stripped, since those fields are surfaced at the top level of the result; "" when --no-content
  includedBy:    [EdgeRef]      # always present; [] when none
  includes:      [EdgeRef]      # always present; populated when --children (§5); [] otherwise
  referencedBy:  [EdgeRef]      # always present; populated when --backlinks; [] otherwise
```

> **Spec note:** the current code populates `includes` only when `--no-content` is set (`crates/liwe/src/retrieve.rs:199-203`), conflating two features under one flag. The spec separates them: `--no-content` blanks `content`, and a dedicated `--children` flag controls `includes`. See §5 for the flag table.

#### 4.2.2 JSON

```json
[
  {
    "key": "test-doc",
    "title": "Test Document",
    "content": "# Test Document\n\nContent here.\n",
    "includedBy": [],
    "includes": [],
    "referencedBy": []
  }
]
```

#### 4.2.3 YAML

```yaml
- key: test-doc
  title: Test Document
  content: |
    # Test Document

    Content here.
  includedBy: []
  includes: []
  referencedBy: []
```

#### 4.2.4 Migrating to `find`

Most of what `retrieve` produces is reachable from `find` plus filesystem reads of the source `.md` files. Equivalences:

| `retrieve` flag / field | `find` equivalent |
|---|---|
| `-k KEY` (single doc) | `find --key KEY` |
| `-k KEY -d N` (subtree, N levels) | `find --included-by KEY:N` |
| `-k KEY -c N` (ancestors, N levels) | `find --includes KEY:N` |
| `-k KEY -l` (outbound refs) | `find --references KEY` |
| `-b`, `--backlinks` (inbound refs) | `find --referenced-by KEY` |
| `DocumentOutput.key`, `title` | system fields on `FindResult` (§4.1.1) |
| `DocumentOutput.includedBy` | system field on `FindResult` |
| `DocumentOutput.includes` (with `--children`) | second `find` query: `find --included-by KEY:1` |
| `DocumentOutput.referencedBy` (with `--backlinks`) | second `find` query: `find --referenced-by KEY:1` |
| `DocumentOutput.content` | read the source file at `<key>.md` directly; the on-disk markdown is authoritative |
| Multi-doc envelope | array of `FindResult` is already an array |

The one capability `find` does not replicate is the rendered, frontmatter-stripped `content` field. Callers that needed this should read the source `.md` from disk and strip frontmatter at the consumer; for use cases that just need the body for an LLM context window, `iwe squash` produces a consolidated rendering.

### 4.3 `iwe tree`

#### 4.3.1 Envelope

The top-level value is an **array** of root nodes (not wrapped in an object):

```yaml
TreeNode:
  key:      string
  title:    string
  children: [TreeNode]   # always present in spec; [] when leaf
  # ... plus any user frontmatter fields requested via --project
```

Each `TreeNode` is a flat mapping with the same projection semantics as a `FindResult` (§4.1.1). Without `--project`, only the system fields (`key`, `title`, `children`) are emitted. With `--project`, the listed user frontmatter fields are added as siblings of `key` and `title`, in projection order; `children` is always present regardless of projection so the tree shape remains traversable.

> **Spec note:** the current `TreeNode` in `crates/iwe/src/main.rs:271-278` omits `children` when empty (`skip_serializing_if = "Vec::is_empty"`) and has no projection support. Per §6, emit `[]` instead, and wire `--project` through to per-node frontmatter projection.

#### 4.3.2 JSON

```json
[
  {
    "key": "ai-memory",
    "title": "AI Agent Memory",
    "children": [
      {
        "key": "post-1",
        "title": "Post One",
        "children": []
      }
    ]
  }
]
```

#### 4.3.3 YAML

```yaml
- key: ai-memory
  title: AI Agent Memory
  children:
    - key: post-1
      title: Post One
      children: []
```

### 4.4 `iwe stats`

Two modes: aggregate (no `-k`) and per-document (`-k KEY`).

**Aggregate** — `json` and `yaml` serialize the `GraphStatistics` struct directly. Aggregate `markdown` is in `output-format-markdown-spec.md` §4.4; aggregate `csv` is in `output-format-other-spec.md` §4.

**Per-document** — `-k KEY` always emits a single object/document. With `-f yaml`, YAML; with any other (`json`, `markdown`, `csv`), JSON. (Per-doc `markdown` and `csv` fall through to JSON in the current implementation.)

> **Spec note:** the current `stats_command` in `crates/iwe/src/main.rs:1233-1251` silently treats `markdown` and `csv` as JSON in per-doc mode. Either restrict the per-doc format set to `json|yaml` and reject the others at parse time, or implement real per-doc markdown/csv output. The spec requires the parse-time restriction unless someone needs the other formats.

## 5. Flag effects on shape

Per command, the flags that change the *shape* (not just the selection) of structured output. Selection-only flags (filter, sort, limit, anchors) live in `query-cli-spec.md`.

### 5.1 `iwe find`

| Flag | Effect on shape |
|---|---|
| `--project f1,f2,...` | Each `FindResult` carries only the listed fields, in the listed order. System fields (`key`, `title`, `includedBy`) and user frontmatter fields are projectable interchangeably. With no `--project`: every system field plus every user frontmatter field, all merged at top level. |
| `--add-fields f1,f2,...` | Each `FindResult` carries the default projection (`query-projection-spec.md` §3.2) **plus** the listed fields — additive, not replacing. Mutually exclusive with `--project`. See `query-projection-spec.md` §3.5. |

### 5.2 `iwe retrieve`

| Flag | Effect on shape |
|---|---|
| `--no-content` | `content` becomes `""`. Does **not** populate `includes`. (Note: `content` never contains the source file's YAML frontmatter — those fields are always returned at the top level of the result, so `content` is body-only by definition.) |
| `--children` (new) | `includes` populated with `EdgeRef` entries for child documents. Independent of `--no-content`. |
| `-b`, `--backlinks` | `referencedBy` populated with `EdgeRef` entries for inbound reference edges. |
| `-d N` | Adds N levels of descendants to the top-level array (selection, not shape — affects which docs appear, not their internal shape). |
| `-c N` | Same, for ancestors. |
| `-l`, `--links` | Same, for outbound reference targets. |
| `--dry-run` | Replaces normal output. With `-f json` or `-f yaml`: the structured form `{ documents: N, lines: N }` (JSON) or the equivalent YAML mapping — this is the one place a wrapper object is emitted, since the dry-run summary is not per-document. |

> **Spec note:** the current `--dry-run` path in `crates/iwe/src/main.rs:691-701` ignores `-f` entirely and always prints the prose form. Honoring `-f` is required.

> **Spec note:** the current code combines suppress-content and emit-children under one flag (`--no-content`, `crates/liwe/src/retrieve.rs:192-203`). The spec splits them. The `--children` name is provisional; `--with-children` is acceptable if it reads better in `--help`.

### 5.3 `iwe tree`

| Flag | Effect on shape |
|---|---|
| `--project f1,f2,...` | Each `TreeNode` carries the listed user frontmatter fields, in the listed order, alongside the system fields (`key`, `title`, `children`). `children` is always present regardless of projection. With no `--project`: only system fields. |

## 6. Stable-schema rule

Every field this spec lists for JSON/YAML output MUST be present in every emission of that command, with its declared type. Empty values are encoded explicitly:

| Type | Empty encoding |
|---|---|
| array | `[]` |
| mapping | `{}` |
| string | `""` |
| nullable scalar | `null` |

A consumer MUST NOT need to test for key presence to handle the empty case. This applies to `find` (`includedBy: []`), `retrieve` (`includedBy: []`, `includes: []`, `referencedBy: []`), `tree` (`children: []`), and any future structured output. Note that with `--project`, only the listed fields are emitted — the stable-schema rule applies to fields the spec declares for the command, not to fields the user excluded by projection.

Markdown is a human surface and is exempt — see `output-format-markdown-spec.md` §7.

## 7. Error output

All commands write errors and progress to stderr. Stdout carries only the format-determined payload. An error path MUST NOT print partial structured output to stdout — if the command fails before producing a complete result, stdout is empty and the process exits non-zero.

Error message form is free-text, prefixed `Error: ` (or `error: ` for parse-stage failures, matching the language spec). This spec does not pin error wording.

## 8. Examples

```
iwe find --filter 'pillar: ai-memory' -f json
iwe find --filter 'pillar: ai-memory' -f yaml

iwe retrieve -k ai-memory -d 1 -f json
iwe retrieve -k ai-memory -d 1 -f yaml
iwe retrieve -k ai-memory --children -f json                     # children populated
iwe retrieve -k ai-memory --no-content --children -f json        # both, independently
iwe retrieve -k ai-memory --dry-run -f json                      # { documents: N, lines: N }

iwe tree -k ai-memory -d 2 -f json                               # array of TreeNode
```

## 9. Out of scope

- **Streaming output (NDJSON, JSON Lines, chunked YAML).** All structured output is a single document. A future revision may add a streaming mode for very large result sets.
- **Pagination cursors.** The wire shape has no cursor field; clients use `--limit` and re-query.
- **Schema versioning.** This spec is v1; future field additions are additive (consumers MUST tolerate unknown keys), but no `schemaVersion` field is emitted.
- **Structured (`-f json|yaml`) output for the mutation commands** (see `output-format-other-spec.md` §5). Future work.
