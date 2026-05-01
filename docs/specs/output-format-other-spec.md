# IWE CLI Output Format Spec — Other Formats

## 1. Overview

Scope of this file: output formats and shapes that aren't the structured (`json`/`yaml`) or human-text (`markdown`/`keys`) families:

- `dot` — graphviz output for `iwe export`.
- `csv` — per-document rows for `iwe stats` aggregate.
- the bare integer emitted by `iwe count`.
- the prose status lines emitted by single-shape mutation commands (`update`, `attach`, `new`, `init`, `normalize`, `squash`).

Companion specs:

- `output-format-markdown-spec.md` — `markdown` and `keys` plaintext formats.
- `output-format-json-yaml-spec.md` — JSON/YAML structured wire shapes.

Out of scope: see §6.

## 2. `iwe count` — single integer

```
25
```

A single integer followed by a newline. No format flag, no envelope. Empty corpus returns `0`. Stderr carries errors as usual; stdout is exactly the integer plus newline.

## 3. `iwe export` — `dot`

`-f dot` only. The output is a graphviz DOT document; its internal grammar is owned by graphviz. The envelope is whatever `dot_exporter::export_dot` produces. `--include-headers` switches to a denser variant (`dot_details_exporter::export_dot_with_headers`) but stays valid DOT.

This spec does not pin the internal DOT grammar. Consumers should pass the output to a DOT-aware tool unmodified.

## 4. `iwe stats` — `csv`

Aggregate mode (no `-k`) emits one row per document with `GraphStatistics::export_csv` headers.

Per-document mode (`-k KEY`) does not produce csv — it falls through to JSON in the current implementation; `output-format-json-yaml-spec.md` §4.4 proposes restricting per-doc output to `json|yaml` at parse time.

## 5. Mutation commands — single-shape prose

`update`, `attach`, `new`, `init`, `normalize`, `squash` (and the rest of the create-family) emit a fixed prose status line. They have no format flag because the operation has nothing structured to report.

```
Updated '<key>'
Updated N document(s)
Created '<key>'
Renamed '<old>' to '<new>'
```

`--quiet` suppresses the status line. `--dry-run` (where supported) prefixes with `Would `.

> **Spec note:** to keep the surface uniform, future work should extend `-f markdown|keys` to all mutation commands. Out of scope here.

## 6. Out of scope

- **Color, pagination, TTY rendering.** All output is plain bytes; downstream tools handle presentation.
- **The `dot` grammar.** Owned by graphviz.
- **Per-document `markdown` / `csv` for `iwe stats`** — see `output-format-json-yaml-spec.md` §4.4.
- **Structured (`-f json|yaml`) output for the mutation commands listed in §5.** Future work.
