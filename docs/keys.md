# Keys and Cross-References

## Document Identification

Each document is identified by a key — its path relative to the project root, without the `.md` extension (e.g. `folder/document`).

**Key features:**

- **Path-based**: Hierarchical organization support
- **Extension handling**: Automatic `.md` extension management
- **Relative linking**: Support for `../parent/document` syntax

## Reference Types

IWE supports three reference types:

1.  **Regular markdown links**: `[text](document.md)`
2.  **Wiki-style links**: `[[document]]`
3.  **Piped wiki links**: `[[document|display text]]`

Each reference type is preserved and can be normalized or converted as needed.

## Link Resolution

Every link is resolved to a `Key` when a document is read. How the link text maps to a key depends on the reference type.

### Markdown links

Markdown links resolve **relative to the document that contains them**. A link `[text](../shared/topic.md)` inside `projects/plan` resolves to the key `shared/topic`. The `.md` extension is dropped, and `..` segments walk up from the containing document's directory.

A link that starts with `/` resolves **from the library root** instead, regardless of where the linking document lives: `[text](/shared/topic.md)` resolves to the key `shared/topic` whether it is written from the root or from a nested document. A `#section` fragment is dropped before the key is computed, so `[text](/shared/topic.md#usage)` resolves to the same key as `[text](/shared/topic.md)`. Set `refs_path = "absolute"` in `[markdown]` to have write-time normalization emit every markdown link in this root-absolute form.

### Wiki links

Wiki links resolve **by path-suffix across the whole document set**, independent of where the link appears. The link text is matched against every document key, and a key matches when its path *ends with* the segments you wrote:

- `[[topic]]` matches any document whose file name is `topic`, regardless of directory.
- `[[shared/topic]]` matches a document whose path ends with `shared/topic`, e.g. `projects/shared/topic`.

This means you can write the short form `[[topic]]` from anywhere and it finds the right document, without tracking relative paths.

When more than one document matches the suffix, resolution is deterministic:

1.  The key with the **fewest path segments** wins.
2.  Ties break **lexicographically** by path.

Given documents `area-one/note` and `area-two/note`, the link `[[note]]` is ambiguous on its own — to point at a specific one you write a longer suffix, `[[area-one/note]]` or `[[area-two/note]]`. The `.md` extension is optional in wiki links and is stripped if present.

## Write-Time Normalization

When IWE writes a document, it normalizes every link so the stored form is canonical. This happens on formatting, normalization, rename, and any action that rewrites a document.

### Wiki link paths

How the path inside a wiki link (`[[…]]`) is written on normalization is controlled by `wiki_link_path` under `[markdown]`. It takes one of three values:

``` toml
[markdown]
wiki_link_path = "preserve"  # "preserve" | "full" | "short"
```

- `"preserve"` (default) — links are kept exactly as written. IWE does not rewrite the path; `[[topic]]` and `[[clippings/topic]]` are both left unchanged.
- `"full"` — every link is rewritten to its target's full key path, so `[[topic]]` becomes `[[clippings/topic]]`.
- `"short"` — every link is rewritten to the **shortest path suffix that still resolves to its target uniquely**.

With `"short"`, IWE starts from the bare file name and grows the suffix only as far as needed to disambiguate:

- A link to `clippings/topic` is written `[[topic]]` when no other document is named `topic`.
- With both `area-one/note` and `area-two/note` present, links are written `[[area-one/note]]` and `[[area-two/note]]` — the bare name `note` would be ambiguous, so one extra segment is added.
- Segments are added one at a time only as far as needed; the full path is the upper bound, used when no shorter suffix is unique.

A shortened link only ever resolves back to the exact document it was shortened from — if a link's target is not one of the indexed documents, it is left at its full path rather than collapsed onto an unrelated document that happens to share the file name.

Whichever value you choose, write-time normalization, completion, and the link actions all follow it. Resolution of existing links is unaffected — short, full, and as-typed links all resolve regardless of the setting.

### Markdown links and the reference extension

By default markdown links are written as paths **relative** to the containing document — a link from `guide/intro` to `reference/api` is written `../reference/api`. Set `refs_path = "absolute"` in `[markdown]` to instead write every markdown link as a root-absolute path from the library root, so the same link becomes `/reference/api`. This only changes how links are *written*; resolution is unaffected, and a leading `/` is always resolved from the library root regardless of the setting.

The two settings compose. `refs_extension` (for example `".md"`) appends an extension to written markdown links, so with `refs_path = "absolute"` and `refs_extension = ".md"` the link is written `/reference/api.md`. Wiki links never receive an extension. Fragment anchors (`#section`) are preserved.

See [Configuration](configuration.md) for `refs_extension`, `refs_path`, `wiki_link_path`, and the completion `link_format` option that controls which link style new links are created in.

## Caveats

With `wiki_link_path = "short"`, the shortest form of a wiki link depends on the whole document set, so a few behaviors are worth knowing:

- **Adding or removing a document can change the canonical form of links in other files.** Creating a second `note` makes the bare `[[note]]` ambiguous; existing `[[note]]` links elsewhere are only rewritten to the disambiguated form the next time those files are normalized.
- **A bare link that has become ambiguous still resolves without error** — it picks the match with the fewest path segments, breaking ties lexicographically. Adding a document that sorts ahead of the previous winner can therefore change which document an un-normalized bare link points to. Re-normalizing the referencing files settles them on explicit suffixes.
- **Round-tripping holds only for a fixed document set.** Within one snapshot, shortening and resolution are inverses; across edits to the corpus the shortest form can change. Keep the default `wiki_link_path = "preserve"` (or use `"full"`) if you prefer links that never change shape.
