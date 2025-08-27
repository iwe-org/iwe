# IWE CLI Quick Reference

A concise reference for all IWE CLI commands and their common usage patterns.

## Installation & Setup

```bash
# Build from source
cargo build --release

# Initialize workspace
iwe init
```

## Command Summary

| Command | Purpose | Key Options |
|---------|---------|-------------|
| `init` | Initialize workspace | - |
| `normalize` | Format markdown files | - |
| `paths` | List knowledge paths | `--depth <N>` |
| `squash` | Combine content | `--key <KEY>` `--depth <N>` |
| `contents` | Generate TOC | - |
| `export` | Export graph data | `dot` `--key <KEY>` `--depth <N>` `--include-headers` |

## Commands

### `iwe init`
```bash
iwe init                    # Initialize current directory
iwe init --verbose 1        # With debug output
```
**Creates:** `.iwe/config.toml` with default settings

### `iwe normalize`
```bash
iwe normalize               # Format all markdown files
iwe normalize --verbose 1   # With processing details
```
**Effect:** Standardizes headers, lists, links, spacing

### `iwe paths`
```bash
iwe paths                   # Show paths (depth 4)
iwe paths --depth 2         # Limit to depth 2
iwe paths --depth 0         # No depth limit
```
**Output:** Navigation routes through content hierarchy

### `iwe squash`
```bash
iwe squash --key filename           # Combine content (depth 2)
iwe squash --key project --depth 3  # Custom depth
```
**Output:** Flattened markdown combining related sections

### `iwe contents`
```bash
iwe contents                # Generate table of contents
iwe contents > TOC.md       # Save to file
```
**Output:** Markdown links to all top-level documents

### `iwe export`
```bash
# DOT format (default - exports all root notes)
iwe export dot                                  # All root documents
iwe export dot --key project-notes             # Filter by specific key
iwe export dot --depth 3                       # Limit depth
iwe export dot --include-headers                # Include sections and subgraphs
iwe export dot --key docs --include-headers     # Detailed view of specific key
iwe export dot > graph.dot                     # Save for visualization
```

## Common Workflows

### Setup New Project
```bash
mkdir knowledge-base && cd knowledge-base
iwe init
echo "# Welcome" > index.md
iwe normalize
```

### Maintain Documentation
```bash
iwe normalize              # Format files
iwe contents > README.md   # Update TOC
git add . && git commit -m "Update docs"
```

### Generate Reports
```bash
iwe squash --key meetings > all-meetings.md
iwe squash --key project --depth 3 > project-summary.md
```

### Visualize Structure
```bash
# Basic structure
iwe export dot > graph.dot
dot -Tpng graph.dot -o structure.png

# Detailed structure with sections
iwe export dot --include-headers > detailed.dot
dot -Tpng detailed.dot -o detailed-structure.png

# Focus on specific topic
iwe export dot --key project --include-headers > project.dot
dot -Tpng project.dot -o project-structure.png
```

### Analyze Content
```bash
iwe paths --depth 2                    # See relationships
iwe export dot --key topic             # Export specific key
iwe export dot --include-headers        # Detailed structure analysis
```

## Configuration

### Default `.iwe/config.toml`
```toml
[library]
path = ""                  # Markdown files location

[markdown]
normalize_headers = true   # Format headers
normalize_lists = true     # Format lists
```

### Custom Library Path
```toml
[library]
path = "docs"             # Look in ./docs/ directory
```

## Global Options

All commands support:
- `-v, --verbose <LEVEL>` - Verbosity (0-2)
- `-h, --help` - Show help
- `-V, --version` - Show version

## Tips

- **File Organization**: Use descriptive filenames (they become keys for squashing)
- **Regular Normalization**: Run `iwe normalize` before commits or on save
- **Depth Control**: Use `--depth` to limit processing for large repositories
- **Key Matching**: Keys match filenames without `.md` extension (not document titles)
- **Link Management**: Normalize updates link titles automatically
- **Detailed Exports**: Use `--include-headers` to show document structure with sections in colored subgraphs

## Integration Examples

### With Git Hooks
```bash
# .git/hooks/pre-commit
#!/bin/bash
iwe normalize
git add -u
```

### With CI/CD
```yaml
# .github/workflows/docs.yml
- name: Format docs
  run: iwe normalize
- name: Generate TOC
  run: iwe contents > TABLE_OF_CONTENTS.md
```

### With Static Sites
```bash
iwe normalize              # Ensure clean markdown
hugo build                 # Or your generator
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Command not found | Check PATH, rebuild with `cargo build` |
| Permission denied | Ensure write access to workspace |
| Key not found | Use exact filename without `.md` |
| Large output | Use `--depth` to limit processing |
| Config missing | Run `iwe init` in workspace root |

## Exit Codes

- `0` - Success
- `1` - Error (missing files, invalid args)
- `101` - Panic (nonexistent key, etc.)

---

For detailed documentation, see `CLI_COMMANDS.md` or visit [iwe.md](https://iwe.md).
