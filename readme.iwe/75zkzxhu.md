# CLI features

- **Normalize**: Standardizes the graph data and ensure consistency.
- **Paths**: Retrieves and displays all possible paths within the graph.
- **Squash**: Creates a document by simplifying the structure and embedding referenced documents

## Usage

You can run `iwe` using the following syntax:

``` sh
iwe [OPTIONS] <COMMAND>
```

### Global Options

- `-v`, `--verbose <verbose>`: Sets the level of verbosity. Default is `0`.

### Commands

- `iwe normalize`: Standardizes the graph data to ensure a uniform structure.
- `iwe paths`: Lists all the paths present in the graph.
- `iwe squash`: Traverse the graph to the specified  depth combining nodes into single markdown document
  - `-d`, `--depth <depth>`: Depth to squash to. Default is `2`.

> [!WARNING]
>
> Make sure that you have a copy of you files before you perform bulk-action such as `iwe normalize`.

[Consolidated documents generation](bjeppfmk)
