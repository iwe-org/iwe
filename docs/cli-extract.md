# IWE Extract

Extract a section from a document into a new document with an [inclusion link](inclusion-links.md).

## Usage

```bash
iwe extract <KEY> [OPTIONS]
```

## Arguments

| Argument | Description |
|----------|-------------|
| `<KEY>` | Document key containing the section to extract |

## Options

| Flag | Description |
|------|-------------|
| `--section <TITLE>` | Section title to extract (case-insensitive) |
| `--block <N>` | Block number to extract (1-indexed) |
| `--list` | List all sections with block numbers |
| `--action <NAME>` | Action name from config to use for extraction |
| `--dry-run` | Preview changes without writing to disk |
| `--quiet` | Suppress progress output |
| `--keys` | Print affected document keys (one per line) |

## How It Works

The `extract` command creates a new document from an existing section:

1. **Select a section** - Use `--list` to see available sections, then select with `--section` or `--block`
2. **Create new document** - The section content is moved to a new document
3. **Add inclusion link** - The original section is replaced with a link to the new document
4. **Adjust headers** - Header levels are adjusted to maintain proper document structure

## Workflow

### Step 1: List Available Sections

```bash
$ iwe extract my-document --list
1: Introduction
2: Getting Started
3: Configuration
4: Advanced Topics
```

### Step 2: Extract by Title or Block Number

```bash
# By title (case-insensitive, partial match)
$ iwe extract my-document --section "configuration"
Extracting section 'Configuration' to 'configuration'
Done

# By block number (unambiguous)
$ iwe extract my-document --block 3
Extracting section 'Configuration' to 'configuration'
Done
```

## Configuration

The `--action` flag uses extraction settings from `.iwe/config.toml`:

```toml
[actions]
extract = { type = "extract", title = "Extract", key_template = "{{slug}}", link_type = "markdown" }
```

Without `--action`, the command uses the first `extract` action found in config, or defaults to:
- `key_template = "{{slug}}"` - URL-friendly version of the section title
- `link_type = "markdown"` - Standard markdown links

### Key Template Variables

See [feature-extract.md](feature-extract.md#template-variables) for all available template variables.

## Output Modes

### Default Output

Shows progress:

```bash
$ iwe extract my-document --section "Architecture"
Extracting section 'Architecture' to 'architecture'
Done
```

### Dry Run (`--dry-run`)

Preview what would happen:

```bash
$ iwe extract my-document --section "Design" --dry-run
Would extract section 'Design' to 'design'
Would update 'my-document'
```

### Keys Output (`--keys`)

Print affected document keys:

```bash
$ iwe extract my-document --block 2 --keys
my-document
getting-started
```

## Examples

```bash
# List all sections with block numbers
iwe extract notes/project --list

# Extract section by title
iwe extract notes/project --section "Architecture"

# Extract section by block number (unambiguous)
iwe extract notes/project --block 2

# Preview changes without writing
iwe extract notes/project --section "Notes" --dry-run

# Use a specific action from config
iwe extract notes/project --section "Design" --action "my-extract"

# Get keys for scripting
iwe extract notes/project --section "API" --keys
```

## Use Cases

### Breaking Down Large Documents

Extract sections to create a modular document structure:

```bash
# See what sections exist
iwe extract large-document --list

# Extract each major section
iwe extract large-document --section "Introduction"
iwe extract large-document --section "Implementation"
iwe extract large-document --section "Testing"
```

### Organizing by Topic

Move related content to dedicated documents:

```bash
# Preview the extraction
iwe extract project-notes --section "Authentication" --dry-run

# Extract to a dedicated document
iwe extract project-notes --section "Authentication"
```

### Scripting

Use with other commands:

```bash
# Extract and get the new document key
NEW_KEY=$(iwe extract doc --section "API" --keys | tail -1)
echo "Created: $NEW_KEY"
```

## Error Handling

The command fails with an error if:
- The document does not exist
- No section matches the provided title (with `--section`)
- Multiple sections match the title (use `--block` instead)
- The block number is out of range (with `--block`)
- Must specify `--section`, `--block`, or `--list`

## Key Collision Handling

When the generated key already exists, IWE automatically appends numeric suffixes:

1. First attempt: `architecture.md`
2. If exists: `architecture-1.md`
3. If exists: `architecture-2.md`

## Technical Notes

- Header levels are adjusted automatically (H2 in source becomes H1 in extracted document)
- The original section is replaced with an inclusion link
- The command uses the same extraction logic as the LSP code action
- Directory structure is preserved based on the source document location
