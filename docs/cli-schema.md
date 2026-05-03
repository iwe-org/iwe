# IWE Schema

Infer and display the frontmatter schema across your workspace. Scans all documents (or a filtered subset) and reports field names, type distributions, coverage, distinct enumerable-value counts, and value breakdowns.

## Usage

``` bash
iwe schema [OPTIONS]
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

## See also

- [`iwe find`](cli-find.md) — search documents using the same filter language.
- [`iwe stats`](cli-stats.md) — structural statistics (sections, words, edges).
