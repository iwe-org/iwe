# IWE Schema

Bare `iwe schema` infers and displays the frontmatter schema across your workspace — it scans all documents (or a filtered subset) and reports field names, type distributions, coverage, distinct enumerable-value counts, and value breakdowns. The `iwe schema validate` subcommand instead checks documents against the [document schemas](document-schema.md) bound to them.

## Usage

``` bash
iwe schema [OPTIONS]
iwe schema validate [OPTIONS]
```

## Options

| Flag                            | Description                                                             | Default    |
| ------------------------------- | ----------------------------------------------------------------------- | ---------- |
| `-f, --format <FORMAT>`         | Output format: `markdown`, `json`, `yaml`                               | `markdown` |
| `--field <NAME>`                | Restrict output to a specific field and its children                    | none       |
| `--filter <EXPR>`               | Inline YAML filter expression. See [Query Language](query-language.md). | none       |
| `-k, --key <KEY>`               | Match by document key. Repeatable.                                      | none       |
| `--includes <KEY[:DEPTH]>`      | `$includes` anchor. Repeatable; anchors are ANDed.                      | none       |
| `--included-by <KEY[:DEPTH]>`   | `$includedBy` anchor. Repeatable; anchors are ANDed.                    | none       |
| `--references <KEY[:DIST]>`     | `$references` anchor. Repeatable; anchors are ANDed.                    | none       |
| `--referenced-by <KEY[:DIST]>`  | `$referencedBy` anchor. Repeatable; anchors are ANDed.                  | none       |
| `--roots`                       | Only documents with no incoming inclusion edges                         | false      |

The `--project` and `--add-fields` flags accepted by `find` and `tree` do not apply here -- schema operates on raw frontmatter.

## What it shows

For each frontmatter field found across the scanned documents:

- **Field** — dot-notation for nested fields (e.g., `engagement.upvotes`)
- **Types** — which YAML types appear and their percentage breakdown (string, number, boolean, null, date, datetime, array, object)
- **Coverage** — how many documents contain this field (count and percentage of total)
- **Distinct** — number of distinct enumerable values (see below)
- **Values** — value distribution for fields with at most 100 distinct enumerable values; otherwise empty

### Enumerable values

Only enumerable values participate in the `Distinct` count and the `Values` listing:

- `null`, booleans, and numbers
- strings made up entirely of `[A-Za-z0-9_.-/]`

Free-text strings (titles, prose, URLs containing `:`) are counted in `Coverage` and `Types` but do not appear in `Values`, and they do not increment `Distinct`. A field with only such values shows `Distinct: 0` and `Values: ---`.

## Examples

``` bash
# Full workspace schema
iwe schema

# Schema for posts only
iwe schema --filter 'type: post'

# Drill into a specific field
iwe schema --field status

# JSON output for scripting
iwe schema -f json

# Schema for a subtree
iwe schema --included-by ai-memory:0

# Nested field inspection
iwe schema --field engagement -f json
```

## Sample Markdown Output

``` markdown
| Field  | Types                    | Coverage | Distinct | Values |
| ------ | ------------------------ | -------- | -------- | --- |
| status | string (100%)            | 3 (100%) | 2        | draft (2), published (1) |
| type   | string (100%)            | 3 (100%) | 3        | external (1), hub (1), post (1) |
| url    | null (50%), string (50%) | 2 (67%)  | 1        | null (1) |
```

Fields with no enumerable values, or with more than 100 distinct enumerable values, show `---` in the `Values` column.

## JSON Format

JSON output is a top-level array of field objects:

``` json
[
  {
    "field": "status",
    "types": [
      { "type": "string", "count": 3, "percentage": 100.0 }
    ],
    "coverage": { "count": 3, "percentage": 100.0 },
    "distinct": 2,
    "values": [
      { "value": "draft", "count": 2 },
      { "value": "published", "count": 1 }
    ]
  }
]
```

YAML output has the same shape.

## Validate

`iwe schema validate` checks documents against the [document schemas](document-schema.md) bound to them in the `[schemas]` section of `.iwe/config.toml`. Each `[schemas]` entry names a schema file in `.iwe/schemas/` and a glob that binds it to document keys; a document is validated against every schema whose glob matches it.

``` bash
iwe schema validate [OPTIONS]
```

| Flag                    | Description                                                | Default |
| ----------------------- | ---------------------------------------------------------- | ------- |
| `-f, --format <FORMAT>` | Output format: `text`, `json`                              | `text`  |
| `--schema-file <PATH>`  | Validate against this schema file directly, ignoring the `[schemas]` bindings. | none    |
| `--filter <EXPR>`       | Inline YAML filter to scope which documents are validated. | none    |
| `-k, --key <KEY>`       | Match by document key. Repeatable.                         | none    |

The same structural anchor flags as `iwe schema` (`--includes`, `--included-by`, `--references`, `--referenced-by`, `--roots`) also apply; with no selector, every document is checked.

`--schema-file` runs an ad-hoc check: the selected documents are validated against that one file rather than the schemas bound to them in config, so a schema can be tried against a document without wiring up a `[schemas]` binding. Pair it with `-k` to check a single document — `iwe schema validate -k notes/intro --schema-file draft.yaml`. In `json` output the report's `schema` field is the file's stem (`draft.yaml` → `draft`).

### Output

`text` (default) reports one line per violation, `<key> › <breadcrumb>: <message>`, with an indented `hint:` line when the schema supplies one:

``` text
notes/intro: required section 'Summary' missing
  hint: every note opens with a summary
notes/intro › Tasks: header is 18 tokens (limit 12)
```

`json` emits an array of `{ key, schema, violations }` objects, one per `(document, matching schema)` pair with violations. Each violation carries `breadcrumb`, `message`, `hint`, `schemaPath` (a JSON Pointer into the schema file), and `keyword`:

``` json
[
  {
    "key": "notes/intro",
    "schema": "note",
    "violations": [
      {
        "breadcrumb": [],
        "message": "required section 'Summary' missing",
        "hint": "every note opens with a summary",
        "schemaPath": "/sections/0/minContains",
        "keyword": "minContains"
      }
    ]
  }
]
```

Clean documents produce no output.

### Exit codes

| Code | Meaning                                                              |
| ---- | ------------------------------------------------------------------- |
| `0`  | every validated document is clean (or no schema binds any document) |
| `1`  | at least one document has a violation                               |
| `2`  | a configuration or schema-file error (bad glob, missing or uncompilable schema file) — printed to stderr before any document is validated |

## See also

- [`iwe find`](cli-find.md) — search documents using the same filter language.
- [`iwe stats`](cli-stats.md) — structural statistics (sections, words, edges).
