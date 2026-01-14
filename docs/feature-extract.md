# Extract Actions

Extract actions enable the creation of new documents from markdown sections (headers). IWE provides two types of extract actions:

1. **Extract** - Extracts a single section into a new file
2. **Extract All** - Extracts all direct subsections of a section into separate files

Both operations:
- Create new files containing the selected content
- Add block reference links (like `[Section Title](new-file)`) to the newly created files
- Automatically adjust header levels to maintain proper document structure
- Support relative path preservation

The reverse operation, known as **inline**, allows you to:
1. Embed the content back into the document via the block reference link
2. Remove the link and the extracted file

## Extract Single Section

### Basic Configuration

The extract action is configured in `.iwe/config.toml` under the `[actions]` section:

```toml
[actions]
extract = { type = "extract", title = "Extract section", key_template = "{{id}}", link_type = "markdown" }
```

### Configuration Options

- **`type`**: Must be `"extract"` for single section extraction
- **`title`**: Display name shown in editor's code actions menu
- **`key_template`**: Template for generating the new file's key/name (see Template Variables below)
- **`link_type`**: Optional link format - `"markdown"` for `[text](key)` or `"wiki"` for `[[key]]`

### Example: Basic Section Extraction

**Source document** (`document.md`):
```markdown
# Main Document

## Important Section

This content will be extracted.

### Subsection

More content here.
```

**After extraction** (`document.md`):
```markdown
# Main Document

[Important Section](extracted-123)
```

**Extracted file** (`extracted-123.md`):
```markdown
# Important Section

This content will be extracted.

## Subsection

More content here.
```

## Extract All Subsections

### Configuration

```toml
[actions]
extract_all = { type = "extract_all", title = "Extract all subsections", key_template = "{{title}}", link_type = "markdown" }
```

### Configuration Options

- **`type`**: Must be `"extract_all"` for extracting all subsections
- **`title`**: Display name shown in editor's code actions menu
- **`key_template`**: Template for generating new file keys/names
- **`link_type`**: Optional link format - `"markdown"` for `[text](key)` or `"wiki"` for `[[key]]`

### Example: Extract All Subsections

**Source document** (`guide.md`):
```markdown
# User Guide

Introduction content here.

## Installation

How to install the software.

## Configuration

How to configure settings.

## Usage

How to use the application.
```

**After extract all** (`guide.md`):
```markdown
# User Guide

Introduction content here.

[Installation](Installation)

[Configuration](Configuration)

[Usage](Usage)
```

**Extracted files**:

`Installation.md`:
```markdown
# Installation

How to install the software.
```

`Configuration.md`:
```markdown
# Configuration

How to configure settings.
```

`Usage.md`:
```markdown
# Usage

How to use the application.
```

## Template Variables

The `key_template` supports several template variables for flexible file naming:

### Basic Variables
- **`{{id}}`**: Random unique identifier (e.g., `123`, `456`)
- **`{{today}}`**: Current date formatted using `date_format` from `[library]` section (default: `%Y-%m-%d`)
- **`{{title}}`**: The section title being extracted (automatically sanitized for filenames)
- **`{{slug}}`**: URL-friendly version of the title (lowercase, alphanumeric characters only, non-alphanumeric replaced with dashes, no consecutive dashes)

### Parent Section Variables
- **`{{parent.title}}`**: Title of the parent section containing the extracted section
- **`{{parent.slug}}`**: URL-friendly version of the parent section title (lowercase, alphanumeric characters only, non-alphanumeric replaced with dashes, no consecutive dashes)
- **`{{parent.key}}`**: Key of the parent document

### Source Document Variables
- **`{{source.key}}`**: Full key of the source document
- **`{{source.file}}`**: Filename portion of the source document key
- **`{{source.title}}`**: Title (first header) of the source document
- **`{{source.slug}}`**: URL-friendly version of the source document title (lowercase, alphanumeric characters only, non-alphanumeric replaced with dashes, no consecutive dashes)
- **`{{source.path}}`**: Directory path of the source document

## Relative Path Behavior

IWE preserves directory structure when extracting sections. The key generation follows these rules:

1. **Generated keys are relative to the source document's directory**
2. **`Key::combine(&key.parent(), &relative_key)` creates the final path**
3. **The parent directory is automatically preserved**

### Example: Relative Path Extraction

**Source document** (`docs/tutorial/basics.md`):
```markdown
# Basic Tutorial

## Getting Started

Content to extract.
```

**Configuration**:
```toml
extract = { type = "extract", title = "Extract", key_template = "extracted-{{title}}" }
```

**Result**:
- Source: `docs/tutorial/basics.md`
- Extracted file: `docs/tutorial/extracted-Getting Started.md`
- Link: `[Getting Started](extracted-Getting Started)`

The extracted file is created in the same directory (`docs/tutorial/`) as the source document.

## Advanced Configuration Examples

### 1. Simple Numeric Keys
```toml
extract = { type = "extract", title = "Extract", key_template = "{{id}}" }
```
Creates files like: `123.md`, `456.md`

### 2. Date-based Extraction
```toml
extract = { type = "extract", title = "Extract to today", key_template = "{{today}}" }
```
Creates files like: `2024-01-15.md`

### 3. Title-based Extraction
```toml
extract = { type = "extract", title = "Extract section", key_template = "{{title}}" }
```
Creates files like: `Getting Started.md`, `Configuration.md`

### 4. Slug-based Extraction (URL-friendly)
```toml
extract = { type = "extract", title = "Extract as slug", key_template = "{{slug}}" }
```
Creates files like: `getting-started.md`, `configuration.md`
From titles like "Getting Started", "User's Guide/Setup", "API*Reference" → `getting-started.md`, `users-guide-setup.md`, `api-reference.md`

### 5. Hierarchical Extraction
```toml
extract = { type = "extract", title = "Extract with context", key_template = "{{parent.title}}-{{title}}" }
```
From a document with structure:
```markdown
# User Guide
## Installation
```
Creates: `User Guide-Installation.md`

### 6. Hierarchical Extraction (URL-friendly)
```toml
extract = { type = "extract", title = "Extract with slug context", key_template = "{{parent.slug}}-{{slug}}" }
```
From a document with structure:
```markdown
# User Guide & Setup
## Installation/Configuration
```
Creates: `user-guide-setup-installation-configuration.md`

### 7. Source-aware Extraction
```toml
extract = { type = "extract", title = "Extract from source", key_template = "{{source.file}}-{{title}}" }
```
From `user-guide.md`:
```markdown
## Installation
```
Creates: `user-guide-Installation.md`

### 8. Source-aware Extraction (URL-friendly)
```toml
extract = { type = "extract", title = "Extract with source slug", key_template = "{{source.slug}}-{{slug}}" }
```
From `User Guide & Manual.md`:
```markdown
# User Guide & Manual
## Installation/Setup
```
Creates: `user-guide-manual-installation-setup.md`

### 9. Path-based Organization
```toml
extract = { type = "extract", title = "Extract to subfolder", key_template = "extracted/{{title}}" }
```
From `docs/guide.md`, creates: `docs/extracted/Installation.md`

### 10. Wiki-style Links
```toml
extract = { type = "extract", title = "Extract (wiki)", key_template = "{{id}}", link_type = "wiki" }
```
Creates links like: `[[123]]` instead of `[Section Title](123)`

### 11. Complex Template Example
```toml
extract = { type = "extract", title = "Extract with full context", key_template = "{{source.path}}/{{today}}-{{parent.title}}-{{title}}" }
```
From `docs/tutorials/advanced.md` on 2024-01-15:
```markdown
# Advanced Tutorial
## Complex Features
```
Creates: `docs/tutorials/2024-01-15-Advanced Tutorial-Complex Features.md`

## Key Collision Handling

When the generated key already exists, IWE automatically appends numeric suffixes:

1. First attempt: `extracted-section.md`
2. If exists: `extracted-section-1.md`
3. If exists: `extracted-section-2.md`
4. And so on...

### Example: Handling Collisions

**Multiple sections with same title**:
```markdown
# Document

## Section
Content 1

## Section
Content 2

## Section
Content 3
```

**With `key_template = "{{title}}"`, extract all creates**:
- `Section.md` (first section)
- `Section-1.md` (second section)
- `Section-2.md` (third section)

## Date Formatting

The `{{today}}` variable uses the `date_format` setting from the `[library]` section:

```toml
[library]
date_format = "%Y-%m-%d"  # Results in: 2024-01-15

[actions]
extract = { type = "extract", title = "Daily extract", key_template = "{{today}}-{{title}}" }
```

Common date formats:
- `%Y-%m-%d`: `2024-01-15`
- `%Y%m%d`: `20240115`
- `%b %d, %Y`: `Jan 15, 2024`
- `%A, %B %d, %Y`: `Monday, January 15, 2024`

## Filename Sanitization

Special characters in titles are automatically sanitized using the `sanitize_filename` crate:

- `Section/With*Special:Chars` → `SectionWithSpecialChars`
- `"Quoted Section"` → `Quoted Section`
- `Section | With | Pipes` → `Section  With  Pipes`
