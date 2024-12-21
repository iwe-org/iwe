# IWE - Markdown LSP server

![Demo](readme.iwe/demo.gif)

# About

IWE is a tool that helps you organize your notes. It treats notes like an interconnected graph, where each document acts as a sub-tree and the links are the edges connecting them. It supports various operations designed to assist with navigating and restructuring the graph.

- **[Block-reference](ndjveztj)** is a key concept and building block for the documents graph

  In markdown, it's a paragraph that contains one link to a note. Like this:

  [A block references](document-id)

  IWE uses these block references as if they're embedded notes. This lets you create a complex, layered document structure without having to mess with directories or overly large markdown files as well as 'reuse' a note in multiple context's. ([transclusion](https://en.wikipedia.org/wiki/Transclusion))

Once you have arranged your notes, IWE enables you to merge them into a single, cohesive document, streamlining the process of document creation. (see [readme.iwe/README](https://github.com/iwe-org/iwe/blob/readme/readme.iwe/README.md) which is source for this file)

The main focus of IWE is to help you to keep your notes organized. It works with the graph at the semantic level, understanding the **headers**, **lists** and **links** defined structure of the documents.

> [!NOTE]
>
> A simple analogy for software engineers would be an IDE for markdown notes.

IWE functions in two modes:

1.  **Editor Extension Mode** as LSP server

    It connect to your editor enabling documents navigation, searching, links auto-completion  and much more

2.  **Command Line Utility Mode**

    Allows you to bulk process thousands of documents in matter of a seconds

Thanks to robust underlying components, IWE can process thousands of files in just a second. It unpacks the files into in-memory graph structure to perform transformations and produced updated markdown back when needed.

## LSP features

### Extract/Inline Notes

The extract note action enables the creation of a new document from a list or a section (header) This involves:

1.  Creating a new file containing the selected content.
2.  Replacing the original selected content with a link to the newly created file.

The reverse operation, known as **inline**, allows you to:

- Inject the content back into the original document, replacing the link and removing the file.

Both operations automatically adjust the header levels as needed to maintain proper document structure.

### Navigation

IWE supports multiple way to navigate  your documents, including:

- **Links Navigation**: Implement as Go To Definition LSP command
- **Table of Contents**: Provided as Document Symbols to the editor
- **Backlinks List**: A backlinks list compiles references or citations linking back to the current document

### Search

Search is one of the key features. IWE, creates all possible document paths by considering the [Block-reference](ndjveztj)'s structure. This means it can come up with lists like:

```
Readme - Features
Readme - Features - Navigation
Readme - Features - Search
..ets
```

And provide this list to your text editor as Workspace Symbols.

This allows for context-aware fuzzy searching, making it easier for you to find what you need.

The search results are ordered by page-rank which is based on the number of references to the target note.

### Text structure normalization / formatting

LSP offers **auto-formatting**, which typically kicks in when you save your work. This feature helps tidy things up. Here's what gets cleaned up:

1.  Uprating link titles to the header of the linked document
2.  Adjusting header levels to ensure tree structure
3.  Cleaning up dead links by replacing them with title text
4.  Updating the numbering of the ordered lists
5.  Fixing newlines, indentations in lists, and much more

### Inlay hints

Inlay hints showing the number of references to the current document and the list of parent documents

### Auto-complete

IWE can suggest links as you type.

### Text manipulation

There is a LSP action to perform context aware transformation from list to headers and vice-verse

## CLI features

- **Normalize**: Standardizes the graph data and ensure consistency.
- **Paths**: Retrieves and displays all possible paths within the graph.
- **Squash**: Creates a document by simplifying the structure and embedding referenced documents

### Usage

You can run `iwe` using the following syntax:

``` sh
iwe [OPTIONS] <COMMAND>
```

#### Global Options

- `-v`, `--verbose <verbose>`: Sets the level of verbosity. Default is `0`.

#### Commands

- `iwe normalize`: Standardizes the graph data to ensure a uniform structure.
- `iwe paths`: Lists all the paths present in the graph.
- `iwe squash`: Traverse the graph to the specified  depth combining nodes into single markdown document
  - `-d`, `--depth <depth>`: Depth to squash to. Default is `2`.

> [!WARNING]
>
> Make sure that you have a copy of you files before you perform bulk-action such as `iwe normalize`.

##### Squash command

IWE can "project" the graph into a single document by changing [Block-reference](ndjveztj) into subsections (headers) and directly incorporating the [Block-reference](ndjveztj) into the parent document.

## How to install

### Prerequisites

- Rust and Cargo installed on your system. You can get them from [rustup.rs](https://rustup.rs).

### Installation

Clone the repository, navigate into the project directory, and build the project:

``` sh
git clone git@github.com:iwe-org/iwe.git
cd iwe
cargo build --release
```

This will create an executable located in the `target/release` directory.

### Editors

IWE can be used with any text editor with LSP support. IWE contains a special LSP binary called `iwes`.

#### VIM integration

To enable IWE LSP for markdown files in VIM you need to make sure that `iwes` binary is in your path and add this to your config:

``` lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'markdown',
  callback = function(args)
    vim.lsp.start({
      name = 'iwes',
      cmd = {'iwes'},
      root_dir = vim.fs.root(args.buf, {'.iwe' }),
      flags = {
        debounce_text_changes = 500
      }
    })
  end,
})

-- optional, enabled inlay hints
vim.lsp.inlay_hint.enable(not vim.lsp.inlay_hint.is_enabled())
```

And create `.iwe` directory as a marker in you notes root directory.

It works best with [render-markdown.nvim](https://github.com/MeanderingProgrammer/render-markdown.nvim/tree/main)

#### Visual Studio Code

Contributors are welcome.

## Configuration

IWE doesn't have any configuration options at the moment, but it does come with some sensible defaults.

For instance:

- Whenever a document is generated, it automatically gets a random file name made up of 8 alphanumeric characters.
- Links are generated without file extensions, with the default being `.md`.

If you'd like to tweak anything, feel free to open a pull request or an issue.

## Help needed

The IWE project is a work in progress and there's plenty of room for improvement. Check out the issues for specific areas that need attention. Help with documentation or editors integration is welcomed!

## Inspired by many other opens-source projects

- [zk notes](https://github.com/zk-org/zk)
- [neuron](https://github.com/srid/neuron)
- [rust-analyzer](https://rust-analyzer.github.io)

## PS

A huge thank you to my wife, Iryna ❤️, for all her support and for giving me the time I needed to finish this over the weekends!

Huge thanks to the Rust community for creating such amazing software development tools. I've really enjoyed learning and using them in the process of building IWE.
