# IWE Query Projection Spec

## 1. Overview

This document specifies projection — the part of the query language that shapes *which fields* a `find` (or, by extension, `retrieve`) result carries, *under which output names*. It uses MongoDB-style `$project` semantics: the **left-hand side is the output field name**, the **right-hand side is the source**. It is a companion to:

- `query-language-spec.md` — defines `project` as one of the operation-document keys (§3.2, §5).
- `query-cli-spec.md` — defines the `--project` CLI lowering (§4.2).
- `output-format-json-yaml-spec.md` — defines the structured wire shape of the result (with `output-format-markdown-spec.md` and `output-format-other-spec.md` as companions).

This spec **replaces** the projection rules in those three documents — the inclusion-list algebra in `query-language-spec.md` §5 (e.g. `title: 1, status: 1`) is dropped in favor of an output-name-to-source mapping (e.g. `title: $title, status: 1`). It also adds the structural-source extension. The other specs should cross-reference §3 and §5 here for projection semantics.

The motivation: today, `find` returns metadata (key, title, counts, parents, frontmatter) and `retrieve` returns content (key, title, body, parents, children, backlinks). They share most of the structural plumbing, but a caller who wants "matching docs *with content*" has to either run two commands and join, or call `retrieve` with a filter. MongoDB-style projection collapses the two into one query+shape pipeline AND lets the caller pick the output names — `body`, `parents`, `links` — instead of being stuck with engine-internal names.

> **Deprecation note.** `retrieve` is on a deprecation path. New callers SHOULD use `find` with explicit `--project` (or the unified default projection in §3.2). The `retrieve` examples in this spec describe current behavior; they will be removed when `retrieve` is.

## 2. Corpus model — structural pseudo-fields (sources)

`query-language-spec.md` §2.3 reserves field names whose first character is `$`, `_`, `.`, `#`, or `@`. This spec defines a concrete set of `$`-prefixed **pseudo-field source selectors** that are addressable as projection sources (and only as projection sources — they are not addressable in `filter`, `sort`, or `update`).

The `$`-prefix is a **source-side marker**, not an output-side marker. It says "this name resolves against the engine, not against user frontmatter." Output names are always bare.

| Source selector | Type | Meaning |
|---|---|---|
| `$key` | string | The document's key. |
| `$title` | string | The document's title. |
| `$titleSlug` | string | Slugified form of `$title`: lowercase, ASCII, whitespace and non-alphanumerics replaced with `-`, leading/trailing `-` trimmed. Derived deterministically from `$title`; no separate corpus storage. |
| `$content` | string | Rendered markdown body, frontmatter stripped. |
| `$frontmatter` | mapping | Full user frontmatter, reserved entries already stripped per §2.3. |
| `$includedBy` | `[EdgeRef]` | Inbound inclusion edges. |
| `$includes` | `[EdgeRef]` | Outbound inclusion edges (= child documents). |
| `$referencedBy` | `[EdgeRef]` | Inbound reference edges (= backlinks). |
| `$references` | `[EdgeRef]` | Outbound reference edges. |
| `$includedByCount` | int | `len($includedBy)`. |
| `$includesCount` | int | `len($includes)`. |
| `$referencedByCount` | int | `len($referencedBy)`. |
| `$referencesCount` | int | `len($references)`. |

`EdgeRef` is the shape `{ key, title, sectionPath: [string] }`. `EdgeRef` sub-fields are unprefixed — they are produced by projection (engine output), not addressed as sources. Sub-field projection within `EdgeRef` is reserved for v2 (§8).

These source selectors are reserved permanently. User frontmatter fields whose names start with `$` are already forbidden (`query-language-spec.md` §2.3), so there is no collision risk between source names and user data.

The selector set is closed in v1. Future additions are additive; consumers MUST tolerate unknown fields in result documents (per the schema-evolution rule in `output-format-json-yaml-spec.md` §9).

### 2.1 Wire naming

**All output keys are bare identifiers chosen by the projection author.** No `$`-prefix on output, ever. The `$`-prefix lives on the right-hand side (source selectors), not on the left (output keys).

This applies recursively: `EdgeRef` sub-fields are bare (`key`, `title`, `sectionPath`), every level of the result document uses bare keys.

The visual rule: `$X` in a projection document is a *reference* to engine-side data; bare keys are output names that ship to the consumer.

> **Spec note:** `output-format-json-yaml-spec.md` §3.1, §3.2, §4.1.1, §4.2.1, §4.3.1 currently mandate specific unprefixed structural keys (`key`, `title`, `content`, `includedBy`, `sectionPath`) as fixed wire shapes. This spec re-casts those keys as **defaults** producible by §3.2's default projection — they remain the wire names you see when you don't pass `--project`, but explicit projection lets the caller rename them. The output-format spec must be amended to reflect the re-casting: those keys are no longer fixed wire names, they are outputs of the default projection.

## 3. Projection document

A projection document is a YAML mapping. Each entry has the shape:

```
<outputName>: <source>
```

Where:

- **`outputName`** — a bare identifier. The key under which the field appears in the result document. MUST NOT start with `$` (reserved for source selectors). MUST NOT contain `.` (output is always flat at the top level — nesting is determined by the source's value type). Casing is preserved as written; the default projections (§3.2) use camelCase per `output-format-json-yaml-spec.md` §2.3 convention, but user projections may use any casing.
- **`source`** — one of the forms below.

### 3.1 Source forms

| RHS form | Meaning |
|---|---|
| `1` | Include a frontmatter field whose name equals `outputName`. Shorthand for `<outputName>: <outputName>`. |
| `$<selector>` | Include the named structural pseudo-field source. |
| `path.to.fm.field` | Include a frontmatter value at a dotted path (per `query-language-spec.md` §4.4). |
| `{ $<selector>: { <options> } }` | Reserved syntax for selectors that take options. No options are defined in v1; see §8 for v2 candidates. |

Examples:

```yaml
project:
  status: 1                           # frontmatter.status → status
  priority: metadata.priority         # frontmatter.metadata.priority → priority
  body: $content                      # $content → body
  parents: $includedBy                # $includedBy → parents
  links: $references                  # $references → links
  fm: $frontmatter                    # full user frontmatter mapping → fm
```

Result document for the projection above:

```yaml
status: draft
priority: 5
body: "# Doc One\n\n..."
parents:
  - key: parent
    title: Parent Doc
    sectionPath: [Overview]
links: []
fm:
  status: draft
  metadata:
    priority: 5
```

`key` and `title` are absent because the projection does not select them. Add `key: $key` and `title: $title` (or rely on the default projection in §3.2) to include them.

### 3.2 Default projection

When the operation document omits `project`, the command applies the **default projection** below. `find` and `retrieve` share the same default:

```yaml
project:
  key: $key
  title: $title
  references: $references
  includes: $includes
  referencedBy: $referencedBy
  includedBy: $includedBy
```

The four edge fields cover both inclusion directions (`includes` / `includedBy`) and both reference directions (`references` / `referencedBy`). User frontmatter is **not** wrapped under a `frontmatter:` key in the default — it is flat-merged at the top level of the result per `output-format-json-yaml-spec.md` §3.3. Callers who want the whole frontmatter map under a single key can project it explicitly with `addFields: { fm: $frontmatter }` or `project: { ..., frontmatter: $frontmatter }`.

`count` returns an integer and has no projection.

**Frontmatter precedence on name collision.** A default projection entry that maps a structural source onto an output name (e.g. `key: $key`, `title: $title`) does not overwrite a user frontmatter field of the same name. If the document's frontmatter already has `key`, `title`, etc., the frontmatter value wins; the structural value is suppressed. This rule applies to the default projection only — under explicit projection, the caller chose the output name and is responsible for any collision.

When `project` is set explicitly, it **replaces** the default — there is no merge. A user who wants to extend the default writes the full set:

```yaml
# default + body
project:
  key: $key
  title: $title
  references: $references
  includes: $includes
  referencedBy: $referencedBy
  includedBy: $includedBy
  body: $content                # added
```

### 3.3 Identity fields

`key` and `title` are not implicit. They appear in the result only when the projection — default or explicit — selects them. The default projection in §3.2 includes both, so callers who do not pass `--project` see them without effort. A caller who passes `--project status,priority` gets exactly `status` and `priority`; `key` and `title` are absent unless added.

A projection that maps `$key`, `$title`, or `$titleSlug` under a different output name (e.g. `slug: $titleSlug`, `heading: $title`) emits the alias only — there is no automatic duplicate `key` / `title` field. If the caller wants both the canonical name and an alias, both must be projected. Example:

```yaml
project:
  slug: $titleSlug
  heading: $title

# result:
slug: doc-one
heading: Doc One
```

### 3.4 Conditional structural sources

Some pseudo-field sources require auxiliary graph computation. On `find` (the supported path), projecting any of `$content`, `$includes`, `$includedBy`, `$references`, `$referencedBy` *implies* the corresponding compute — no flags needed. The implied depth for `$includes` is 1 (immediate children only); deeper traversal is currently only available on `retrieve`.

On `retrieve` (deprecated — see §1), the legacy flag set (`-b`, `-c`, `-l`, `-d`, `--no-content`) still gates these computations. When the projection asks for a source whose backing flag is not set, the field is emitted with its empty value (`[]`, `""`):

- `parents: $referencedBy` without `-b` → `parents: []`.
- `body: $content` with `--no-content` → `body: ""`.
- `kids: $includes` with `-d 0` → `kids: []`.

The empty form preserves stable schema (`output-format-json-yaml-spec.md` §6).

### 3.5 `addFields` — additive projection

For callers who want to *augment* the default projection rather than replace it, the operation document accepts an alternative key, `addFields`, alongside `project`.

`addFields` follows the same grammar as `project` (§3.1). It does **not** replace the default — it extends it. The baseline that `addFields` augments is exactly the default projection from §3.2:

```yaml
key: $key
title: $title
references: $references
includes: $includes
referencedBy: $referencedBy
includedBy: $includedBy
```

Combine rule:

- Each entry in `addFields` is appended to the six default entries above.
- Output names absent from the default are appended.
- Output names that collide with a default-projection output name overwrite the default value with the projected value. For example, `addFields: { title: $key }` replaces the default `title: $title` mapping in the result.

`project` and `addFields` are mutually exclusive within a single operation document. Setting both is a parse-time error.

The conditional-source rule (§3.4) applies identically. A structural pseudo-field appearing under `addFields` on `find` implies the corresponding compute, just as it would under `project`.

The frontmatter-precedence rule from §3.2 also applies: when an `addFields` entry maps a structural source onto an output name (e.g. `title: $title`) and the document's frontmatter already carries that name, the frontmatter value wins.

Example — `find` with the body added:

```yaml
addFields:
  body: $content
```

Result document carries the full `find` default projection (§3.2), the document's user frontmatter flat-merged at the top level (per `output-format-json-yaml-spec.md` §3.3), plus the `body` field:

```yaml
key: doc-1
title: Doc One
references: []
includes: []
referencedBy: []
includedBy: []
status: draft        # user frontmatter, flat-merged
priority: 5          # user frontmatter, flat-merged
body: "# Doc One\n\n..."
```

## 4. Output shape under projection

Projection shapes the per-document fields. The wire shape is **a flat array of projected documents** — no envelope. This matches `output-format-json-yaml-spec.md` §4.1.1 (`find`) and §4.2.1 (`retrieve`).

```yaml
[<projected-doc>]
```

Each element is a projected document per §3.1. The shapes the output-format spec calls `FindResult` and `DocumentOutput` are now the **default-projection** results for `find` and `retrieve` respectively (the same shape, per §3.2); explicit projection produces whatever shape the projection document specifies.

`retrieve` is deprecated (§1); new callers should use `find`.

### 4.1 Cross-command convergence

Once projection is unified, `find` and `retrieve` (deprecated) differ only in **selection vocabulary**: `find` accepts a positional fuzzy `QUERY`; `retrieve` accepts `-k KEY` (and graph-walk flags like `-d`, `-c`, `-l`). Default projection is the same on both (§3.2), and the wire shape is the same flat array.

A `find` invocation with `--project 'body=$content,parents=$includedBy'` produces the same per-document shape — and the same outer shape — as `retrieve --project 'body=$content,parents=$includedBy'`.

## 5. CLI lowering (`--project`, `--add-fields`)

Replaces and supersedes `query-cli-spec.md` §4.2 for `--project`. Adds `--add-fields`.

### 5.1 Grammar

Both flags accept either a comma-separated item list **or** an inline YAML mapping:

```
--project    ITEM[,ITEM]...    |    --project    'YAML-MAPPING'
--add-fields ITEM[,ITEM]...    |    --add-fields 'YAML-MAPPING'
```

The argument is parsed as YAML first. If the parsed value is a mapping, it is used directly as the projection document — this mirrors the `--filter` lowering in `query-cli-spec.md` §3.1. Otherwise the argument is treated as a comma-separated `ITEM` list and lowered per the table below.

`--project` populates the operation document's `project` key; `--add-fields` populates `addFields` (§3.5).

**Comma-list form.** Each `ITEM` lowers to a single `<outputName>: <source>` entry:

| ITEM form | Lowered entry | Notes |
|---|---|---|
| `name` | `name: 1` | Frontmatter field, output as `name`. |
| `name=path.to.fm` | `name: path.to.fm` | Frontmatter at dotted path, output as `name`. |
| `name=$selector` | `name: $selector` | Pseudo-field source, output as `name`. |
| `$selector` | `selector: $selector` | Pseudo-field, output name = selector minus `$`. Convenience form. |

**YAML form.** The argument is a YAML mapping written in the same shape as the `project` / `addFields` document body:

```bash
--add-fields 'body: $content'
--add-fields '{body: $content, parents: $includedBy}'
--project    '{key: $key, title: $title, body: $content}'
```

The two forms are interchangeable; pick whichever reads better at the call site. The comma-list form is shorter for trivial cases; the YAML form scales to larger projections without quoting gymnastics and matches the document body verbatim.

Order of items is preserved (matters for human-facing rendering; structurally JSON/YAML mappings are unordered, but rendering implementations SHOULD preserve insertion order).

`--project` and `--add-fields` are mutually exclusive on a single invocation. Passing both is a CLI parse error, mirroring the document-level rule in §3.5.

**Shell quoting.** The `$`-prefix in source selectors triggers shell variable expansion in unquoted form. Quote `--project` and `--add-fields` arguments with single quotes: `--add-fields 'body=$content,parents=$includedBy'` or `--add-fields 'body: $content'`. Bash, zsh, fish, and PowerShell all preserve `$` inside single quotes.

### 5.2 Examples

```bash
# Default projection — current find behavior
iwe find --filter 'status: draft'

# Two frontmatter fields
iwe find --filter 'status: draft' --project title,priority
# → each array element = { title, priority }

# Pseudo-fields with default names (selector minus $)
iwe find --filter 'status: draft' --project '$content,$includedBy'
# → each array element = { content, includedBy }

# Pseudo-fields renamed
iwe find --filter 'status: draft' --project 'body=$content,parents=$includedBy,status'
# → each array element = { body, parents, status }

# Include key/title alongside other fields by naming them explicitly
iwe find --filter 'status: draft' --project 'key=$key,title=$title,body=$content'
# → each array element = { key, title, body }

# All structural fields, renamed for human readers
# (projection implies the compute on `find`; no extra flags needed — see §3.4)
iwe find --filter 'status: draft' \
  --project 'body=$content,parents=$includedBy,kids=$includes,backlinks=$referencedBy'

# retrieve with a slim, renamed projection
iwe retrieve -k notes/foo --project 'parents=$includedBy,backlinks=$referencedBy' -b

# Default projection plus an extra field — no need to re-list every default
iwe find --filter 'status: draft' --add-fields body=$content
# → each array element = default fields (§3.2) + { body }

# Default projection plus several structural fields
iwe find --add-fields 'body=$content,parents=$includedBy' -f json

# YAML form — same projection, written as YAML
iwe find --filter 'status: draft' --add-fields 'body: $content'
iwe find --add-fields '{body: $content, parents: $includedBy}' -f json

# YAML form for --project too
iwe find --project '{key: $key, title: $title, body: $content}'
```

## 6. Markdown rendering under projection

`output-format-markdown-spec.md` §4.1.1 / §4.2.1 define the markdown shapes for default projection. With explicit projection:

- **`find` markdown:** always emits one four-backtick fenced `markdown #<key>` block per result. The frontmatter inside the block contains only the projected fields under their **output names**, with two omissions: `key` is hoisted to the fence info string (never duplicated inside frontmatter), and any field whose source is `$content` is rendered as the body rather than inside frontmatter. The document body is always rendered below the frontmatter, regardless of whether `$content` is projected. If, after the two omissions, no fields remain, the `---` markers are dropped and the block is fence + body + closing fence.
- **`retrieve` markdown:** the frontmatter block contains only the fields the projection requested, under their **output names**. Omitting a `$content` projection emits the frontmatter block with no body.

The cross-format invariant in `output-format-json-yaml-spec.md` §2.2 still applies — markdown MUST NOT contradict JSON/YAML, but MAY abbreviate or omit fields for readability.

## 7. Migration from current behavior

This spec is a forward-compatible extension where possible and a clean break where projection semantics change.

| Current behavior | Under this spec |
|---|---|
| `find` with no `--project` emits user frontmatter under a nested `frontmatter:` key (`crates/liwe/src/find.rs:175-183`). | User frontmatter is flat-merged at the top level of every result per `output-format-json-yaml-spec.md` §3.3. The default projection (§3.2) no longer carries a `frontmatter:` entry. **Breaking** for callers that read `result.frontmatter.X`; they should now read `result.X` directly, or opt into the nested form with `--add-fields frontmatter=$frontmatter`. |
| `find --project f1,f2` emits *both* the default fields *and* the projected frontmatter subset. | `--project f1,f2` lowers to `{f1: 1, f2: 1}` and replaces defaults — the result carries only `f1` and `f2`. To keep `key`/`title`, project them explicitly, or use `--add-fields f1,f2` to keep the full default projection (§3.5). **Breaking.** |
| `retrieve` always emits `content`, even when callers don't want it. | `retrieve --project 'parents=$includedBy'` emits only the requested fields. **Breaking** for callers that depended on `content` always being present, but they can project it explicitly. |
| `find` and `retrieve` have separate default projections. | Unified default projection — same shape from both commands when neither passes `--project` (§3.2). |
| `key` and `title` always emitted regardless of projection. | `key` and `title` appear only when the projection selects them (default projection includes both, so the no-`--project` case is unchanged). **Breaking** for callers that pass `--project` and rely on `key`/`title` still being present. |
| `find` cannot return content. | `find --project '$content'` (or `body=$content`) does. New capability. |
| `--project` lowering only knows frontmatter paths. | `--project` accepts frontmatter paths, `$`-prefixed source selectors, and `name=source` aliases. **Additive.** |
| Output keys are fixed by the engine. | Output keys are chosen by the projection. Default projection preserves existing names (`key`, `title`, `includedBy`, etc.) for backward compatibility. **Additive — callers who do not pass `--project` see no rename.** |
| `EdgeRef` sub-fields named `key`, `title`, `sectionPath`. | Same. Unchanged. |

The two breaking changes (default-replacement semantics for `find` and `retrieve`) are the price of a coherent projection model. Callers who relied on the previous "default + projected subset" merge should switch to `--add-fields` (§3.5, §5.1), which is the explicit additive form.

## 8. Out of scope

- **Exclusion projection** (`field: 0`). Reserved for v2.
- **Sub-field projection within `EdgeRef`** (e.g. emit only `key` and `title` from each `$includedBy` entry). Reserved for v2 — currently `$includedBy` is all-or-nothing.
- **Computed/derived sources** (e.g. `$wordCount`, `$age`, MongoDB-style `{ $concat: [...] }`). Out of scope; the source set in §2 is closed for v1.
- **Folding `find` and `retrieve` into a single command.** `retrieve` is on a deprecation path (§1); the v2 surface will be `find` only.
- **`$content` rendering modes** (`outline`, `first-paragraph`, `summary`, `headers`, length-capped or windowed forms, structured-outline variants like `[{depth, text}]`) and the `:MODE` shorthand grammar. The object form `{ $content: { ... } }` (§3.1) leaves room for the option set; the option fields are v2.
- **Nested output paths** (e.g. `meta.priority: $referencesCount` to nest into a sub-object). v1 keeps the projected output flat; nesting is determined by the source's natural shape.
