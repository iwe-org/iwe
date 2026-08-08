# Open Knowledge Format

[OKF](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) is an open format for agent-maintained knowledge, published by Google Cloud: a directory of markdown files with YAML frontmatter, where provenance (`sources`, `generated`), trust (`verified`), and lifecycle (`status`, `stale_after`) are first-class frontmatter families. No SDK, no registry — if you can `cat` a file, you can read OKF; if you can `git clone` a repo, you can ship it.

That is also a precise description of an IWE workspace. IWE has no OKF mode because it doesn't need one: markdown, frontmatter, and links are its native data model, so OKF bundles are readable, queryable, and refactorable with the standard tools.

## Scaffold a bundle

``` bash
iwe init --okf
```

This initializes a workspace with links written in OKF's resolvable form (`refs_extension = ".md"`), a bundle-root `index.md` carrying `okf_version: "0.2"`, and three conformance schemas wired into validation:

| Schema | Checks |
|---|---|
| `okf.yaml` | every concept document carries a non-empty `type`; the optional OKF families (`status`, `stale_after`, `generated`, `verified`, `sources`) are well-formed when present |
| `okf-index.yaml` | reserved `index.md` files carry no frontmatter (bar the bundle root's `okf_version`) and keep the SPEC §8 shape — sections of link bullets |
| `okf-log.yaml` | reserved `log.md` files keep the SPEC §9 shape — date-grouped history under ISO 8601 headings |

## Make an existing workspace conformant

OKF conformance (SPEC §11) is three rules: every non-reserved `.md` file has parseable frontmatter, every frontmatter block has a non-empty `type`, and `index.md` / `log.md` follow their prescribed shapes. To adopt it in a workspace you already have:

1. Set `refs_extension = ".md"` in `.iwe/config.toml` and run `iwe normalize` so every internal link resolves for consumers outside IWE.
2. Add `type` to any document missing it.
3. Copy the three schemas from a scaffolded project (or [`schematter/examples/okf/`](https://github.com/iwe-org/schematter/tree/main/examples/okf)) into `.iwe/schemas/` and bind them in `.iwe/config.toml`.

A binding for the concept schema needs to skip the reserved files, which is what `!` negation is for — the last matching pattern decides, gitignore-style:

``` toml
[schemas.okf]
match = ["**", "!index", "!**/index", "!log", "!**/log"]

[schemas.okf-index]
match = ["index", "**/index"]

[schemas.okf-log]
match = ["log", "**/log"]
```

All matching schemas apply to a document, so `okf.yaml` stacks on top of whatever per-type schemas you already have.

## Validate

``` bash
iwe schema validate
```

Validation is mechanical and CI-ready — it exits non-zero on any violation and names the exact document, field, and rule. Because document schemas describe body structure as well as frontmatter, this covers all three conformance criteria, including the index and log shapes a frontmatter-only check can't see.

## Query OKF frontmatter

OKF fields are ordinary frontmatter, so `iwe find` filters on them directly:

``` bash
iwe find --filter '{type: Playbook}'
iwe find --filter '{status: draft}' -f keys
iwe find --filter '{type: Metric, status: deprecated}'
```

## Reference bundles

Both IWE workspace templates ship as conformant OKF v0.2 bundles, validated in CI — clone one to see every family in use, or browse them online:

- [marketing-workspace](https://github.com/iwe-org/marketing-workspace) — campaign memory for a marketing agent ([browse](https://iwe.md/templates/marketing-workspace/data/))
- [dev-workspace](https://github.com/iwe-org/dev-workspace) — project memory for a coding agent ([browse](https://iwe.md/templates/dev-workspace/data/))
