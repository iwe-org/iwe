# Key System and Cross-References

## Document Identification

Each document is identified by a `Key` - a path-based identifier:

``` rust
pub struct Key {
    pub relative_path: Arc<String>,  // e.g., "folder/document"
}
```

**Key features:**

- **Path-based**: Hierarchical organization support
- **Reference counting**: Arc enables efficient cloning
- **Extension handling**: Automatic .md extension management
- **Relative linking**: Support for ../parent/document syntax

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

### Wiki links are written in the shortest unambiguous form

A wiki link is always written as the **shortest path suffix that still resolves to its target uniquely**. IWE starts from the bare file name and grows the suffix only as far as needed to disambiguate:

- A link to `clippings/topic` is written `[[topic]]` when no other document is named `topic`.
- With both `area-one/note` and `area-two/note` present, links are written `[[area-one/note]]` and `[[area-two/note]]` — the bare name `note` would be ambiguous, so one extra segment is added.
- Segments are added one at a time only as far as needed; the full path is the upper bound, used when no shorter suffix is unique.

A shortened link only ever resolves back to the exact document it was shortened from — if a link's target is not one of the indexed documents, it is left at its full path rather than collapsed onto an unrelated document that happens to share the file name.

### Disabling shortening

Shortening is on by default. To keep wiki links exactly as written — at their full path — set `shorten_wiki_links = false` under `[markdown]`:

``` toml
[markdown]
shorten_wiki_links = false
```

With shortening off, write-time normalization, completion, and the link actions all emit the full key path (`[[clippings/topic]]`) instead of the shortest suffix. Resolution is unaffected — short links that already exist in your documents still resolve.

### Markdown links and the reference extension

Markdown links are written as paths relative to the containing document. By default no extension is added; set `refs_extension` in `[markdown]` (for example `".md"`) to append an extension to written markdown links. Wiki links never receive an extension. Fragment anchors (`#section`) are preserved.

See [Configuration](configuration.md) for `refs_extension`, `shorten_wiki_links`, and the completion `link_format` option that controls which link style new links are created in.

## Caveats

The shortest form of a wiki link depends on the whole document set, so a few behaviors are worth knowing:

- **Adding or removing a document can change the canonical form of links in other files.** Creating a second `note` makes the bare `[[note]]` ambiguous; existing `[[note]]` links elsewhere are only rewritten to the disambiguated form the next time those files are normalized.
- **A bare link that has become ambiguous still resolves without error** — it picks the match with the fewest path segments, breaking ties lexicographically. Adding a document that sorts ahead of the previous winner can therefore change which document an un-normalized bare link points to. Re-normalizing the referencing files settles them on explicit suffixes.
- **Round-tripping holds only for a fixed document set.** Within one snapshot, shortening and resolution are inverses; across edits to the corpus the shortest form can change. Set `shorten_wiki_links = false` if you prefer links that never change shape.
