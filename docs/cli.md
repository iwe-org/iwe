# How to Use in Command Line

IWE provides a powerful command-line interface for managing markdown-based knowledge graphs. The CLI enables you to initialize projects, normalize documents, explore connections, export visualizations, and create consolidated documents.

## Quick Start

1.  **Initialize a project**: `iwe init`
2.  **Create a new document**: `iwe new "My Note"`
3.  **Normalize all documents**: `iwe normalize`
4.  **View document paths**: `iwe paths`
5.  **Analyze your knowledge base**: `iwe stats`
6.  **Export graph visualization**: `iwe export dot`

## Installation & Setup

Before using the CLI, ensure IWE is installed and available in your PATH. Initialize any directory as an IWE project:

``` bash
cd your-notes-directory
iwe init
```

This creates a `.iwe/` directory with configuration files.

## Global Usage

``` bash
iwe [OPTIONS] <COMMAND>
```

### Global Options

- `-V`, `--version`: Display version information
- `-v`, `--verbose <LEVEL>`: Set verbosity level (default: 0)
  - `1`: Minimal output (INFO level messages to stderr)
  - `2` or higher: Debug-level information to stderr
- `-h`, `--help`: Show help information

## Configuration

Commands respect settings in `.iwe/config.toml`:

``` toml
[library]
path = ""  # Subdirectory containing markdown files

[markdown]
normalize_headers = true
normalize_lists = true
```
