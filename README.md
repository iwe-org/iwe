# IWE - Personal Knowledge Management (PKM) system

![Demo](readme.iwe/demo.gif)

# About

IWE is a tool that helps you organize your markdown notes. It treats notes as an interconnected graph, where each document acts as a sub-tree and the links are the edges connecting them. It supports various operations designed to assist with navigating and restructuring the graph.

The main focus of IWE is to help you to keep your notes organized. It works with the graph at the semantic level, understanding the **headers**, **lists** and **links** defined structure of the documents.

> [!NOTE]
>
> A simple analogy for software engineers would be an IDE for markdown notes.

IWE functions in two modes:

1.  **Editor Extension Mode** (LSP server)

    IWE integrates seamlessly with your editor, helping you to:

    - Search through your notes
    - Navigate through markdown links
    - Auto-complete links as you type
    - Extract or inline sub-notes seamlessly
    - Format the document and refresh link titles
    - Rename files and automatically update all related links
    - Select backlinks to find references to the current document
    - Convert lists into headers and vice versa smoothly
    - Display inlay hints with parent note references and link counts

2.  **Command Line Utility Mode**

    This tool lets you process thousands of documents in just a second. With IWE, you can reformat documents and update link titles across your entire library. You can also use the CLI mode to combine multiple files into one extended document.

## Text editor extension features

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

Search is one of the key features. IWE, creates all possible document paths by considering the block-references structure. This means it can come up with lists like:

```
Readme - Features
Readme - Features - Navigation
Readme - Features - Search
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

IWE offers a range of actions to help you perform context-aware transformations on your notes. The actions can be called with "code actions" LSP menu of your editor. Some of the actions available are:

- Transforming list to headers/section
- Transforming subsequent of the same level to list
- Changing list type (bullet/ordered)

### Header levels normalization

IWE reads and understands nested structures based on headers. It identifies how sub-headers relationships. Markdown allows header structure where the nesting isn't clear, like:

``` markdown
## First Header

# Second Header
```

IWE automatically fixes the header levels to ensure they're nested correctly. So the example above corrects to:

``` markdown
# First Header

# Second Header
```

#### Removing unnecessary levels

IWE can normalize the headers structure dropping unnecessary header-levels, For example:

``` markdown
# First header

### Second header
```

Will be normalized into dropping unnecessary levels and will look like:

``` markdown
# First header

## Second header
```

### Renaming files

With IWE, you can rename the note file and automatically update all the references throughout your entire library using the `rename` LSP refactoring feature.

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

- `iwe init`: Initialize the directory as documents root by adding `.iwe` marker directory and creating default `config.json` in it
- `iwe normalize`: Standardizes the graph data to ensure a uniform structure
- `iwe paths`: Lists all the paths present in the graph
- `iwe squash`: Traverse the graph to the specified  depth combining nodes into single markdown document
  - `-d`, `--depth <depth>`: Depth to squash to. Default is `2`

> [!WARNING]
>
> Make sure that you have a copy of you files before you perform bulk-action such as `iwe normalize`.

#### Normalize command

This command will performs batch normalization of the entire library. Including:

- Uprating link titles to the header of the linked document
- Adjusting header levels to ensure tree structure
- Updating the numbering of the ordered lists
- Fixing newlines, indentations in Lists
- etc.

#### Squash command

IWE can "project" the graph into a single document by changing block-references into subsections (headers) and directly incorporating the block-references into the parent document.

## Nested documents

IWE has some cool features, like its support for nested documents through block-references. This is a type of [transclusion](https://en.wikipedia.org/wiki/Transclusion), where a sub-document is seamlessly incorporated into a parent document. Transclusion lets you reuse the same content across various contexts, making your work more efficient and interconnected.

With IWE, you can treat these block-references like embedded notes. This means you can build complex, layered document structures without having to deal with massive markdown files.

- **[Block-reference](https://github.com/iwe-org/iwe/blob/master/readme.iwe/block-reference.md)** is a key building block for the documents graph

  In markdown, it's a paragraph that contains one link to a note. Like this:

  ``` markdown
  A paragraph...

  [Block-reference](block-reference)

  Another paragraph...
  ```

After you've organized your notes, IWE lets you merge them into one cohesive document. It automatically adjusts the header levels of the embedded documents based on where they're referenced in the main document.

See [readme.iwe/README](https://github.com/iwe-org/iwe/blob/master/readme.iwe/README.md) which is source for this file.

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

#### Zed integration

IWE Zed [extension](https://github.com/iwe-org/zed-iwe) can be installed from the editor extensions menu.

The extension automatically fetches a pre-compiled binary of the LSP from a GitHub repository. If there is an LSP binary already installed on the system and it is accessible from the system's PATH, the extension will use that local binary instead of downloading a new one.

#### Helix integration

Make sure you have the `iwes` binary in your path, then add to your `languages.toml` (usually in `~/.config/helix`, create file if needed):

``` toml
[language-server.iwe]
command = "iwes"

[[language]]
name = "markdown"
language-servers = [ "iwe" ] # you might want more LSP's in here
auto-format = true # optional, enable format-on-save
```

Then run:

``` sh
hx --health markdown
```

To see configured language servers.

#### Visual Studio Code

Contributors are welcome.

## Configuration

IWE have limited configuration options at the moment, but it does come with sensible defaults.

For instance:

- Whenever a document is generated, it automatically gets a random file name made up of 8 alphanumeric characters.
- Links are generated without file extensions, with the default being `.md`.

If you'd like to tweak anything, feel free to open a pull request or an issue.

The only configuration options available lets you change the default extension for local links and the path where you want to keep the files (relative to current directory). For example:

``` json
{
  "markdown":{"refs_extension":".md"}
  "library":{"path":"readme.iwe"}
}
```

By default, IWE will add/keep `.md` extension to the local links.

## Help needed

The IWE project is still in development, and there's a lot of room for improvement.

Feel free to create issues for specific areas that need attention.

Contributions to documentation or integration with editors are welcome!

Please share your feedback and recommendations in the discussions section.

## Inspired by many other opens-source projects

- [zk notes](https://github.com/zk-org/zk)
- [neuron](https://github.com/srid/neuron)
- [rust-analyzer](https://rust-analyzer.github.io)

## PS

A huge thank you to my wife, Iryna ❤️, for all her support and for giving me the time I needed to finish this over the weekends!

Huge thanks to the Rust community for creating such amazing software development tools. I've really enjoyed learning and using them in the process of building IWE.
