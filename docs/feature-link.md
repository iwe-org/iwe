# Creating links from text

The "link" code action allows you to quickly convert text under your cursor or selected text into a link to a new note. This feature streamlines the process of creating new notes while maintaining connections in your knowledge graph.

## How It Works

When you place your cursor on a word or select text and trigger a code action, IWE can:

1. **Create a new note** based on the text at your cursor or selection
2. **Replace the text** with a link to the newly created note
3. **Use templates** to control where the new note is created and how it's named

The link action works on a single line - you can either place your cursor on a word (and IWE will detect the word boundaries) or select a specific range of text.

## Configuration

Configure link actions in your `.iwe/config.toml`:

```toml
[actions.link]
type = "link"
title = "Link word"
key_template = "{{id}}"           # Template for the new note's key
link_type = "Markdown"            # Optional: "Markdown" or "WikiLink"
```

### Configuration Options

- **title**: The name displayed in your editor's code action menu
- **key_template**: Template for generating the new note's key (supports template variables)
- **link_type**: Optional link format
  - `"Markdown"`: Creates `[text](key)` style links (default)
  - `"WikiLink"`: Creates `[[key]]` style links

## Common Use Cases

### Simple ID-Based Links

Create notes with auto-generated numeric IDs:

```toml
[actions.link]
type = "link"
title = "Link word"
key_template = "{{id}}"
```

**Workflow:**
1. Place cursor on "important" or select "important concept"
2. Trigger code action and select "Link word"
3. Text becomes `[important](2)` and a new note `2.md` is created with `# important`
4. Or if you selected "important concept", it becomes `[important concept](2)` with `# important concept`

### Slug-Based Links

Use slugified versions of the text as the note key:

```toml
[actions.link]
type = "link"
title = "Link word"
key_template = "{{slug}}"
```

**Example:**
- Cursor on "Important Concept" → `[Important Concept](important-concept)` → `important-concept.md`
- Selected "My Idea" → `[My Idea](my-idea)` → `my-idea.md`

### Title-Based Links

Use the exact text (sanitized for filenames) as the key:

```toml
[actions.link]
type = "link"
title = "Link word"
key_template = "{{title}}"
```

**Example:**
- Cursor on "MyWord" → `[MyWord](MyWord)` → `MyWord.md`
- Selected "Project Ideas" → `[Project Ideas](Project Ideas)` → `Project Ideas.md`

### WikiLink Format

Use WikiLink style instead of Markdown:

```toml
[actions.wiki_link]
type = "link"
title = "Link word (wiki)"
key_template = "{{id}}"
link_type = "WikiLink"
```

**Example:**
- Cursor on "concept" → `[[2]]` → `2.md`
- Selected "important idea" → `[[2]]` → `2.md` with `# important idea`

## Text Selection vs Cursor

The link action supports two modes:

### Cursor Mode (Word Detection)
- Place your cursor anywhere within a word
- IWE automatically detects word boundaries
- Words can include alphanumeric characters, underscores, hyphens, and Unicode characters
- Examples:
  - Cursor on "wo|rd" → detects "word"
  - Cursor on "multi-w|ord" → detects "multi-word"
  - Cursor on "some_|function" → detects "some_function"

### Selection Mode
- Select any text range on a single line
- IWE uses exactly what you selected
- Perfect for phrases with spaces or partial words
- Examples:
  - Select "important concept" → creates link for "important concept"
  - Select "very important" → creates link for "very important"
  - Select part of a sentence → creates link for that specific text

## Template Variables

Link actions support several template variables:

- **{{id}}**: Auto-generated unique numeric ID
- **{{slug}}**: Slugified version of the text (lowercase, hyphens instead of spaces)
- **{{title}}**: Sanitized version of the text (safe for filenames)
- **{{today}}**: Current date formatted using your configured date format from `library.date_format` (default: `"%Y-%m-%d"`)

## Key Collision Handling

If a note with the generated key already exists, IWE automatically appends a numeric suffix:

**Example:**
- First link: `concept` → `concept.md`
- Second link: `concept` → `concept-1.md`
- Third link: `concept` → `concept-2.md`

### Use Multiple Link Actions

Configure different link actions for different purposes:

```toml
[actions.link]
type = "link"
title = "Quick Link"
key_template = "{{id}}"

[actions.concept_link]
type = "link"
title = "Link Concept"
key_template = "concepts/{{slug}}"

[actions.daily_link]
type = "link"
title = "Link to Today"
key_template = "daily/{{today}}"
```
