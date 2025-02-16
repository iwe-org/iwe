# About IWE

[IWE](https://iwe.md) is a Markdown notes assistant for your favorite text editor. It's a tool that helps you organize your Markdown notes. It treats notes as an interconnected graph, where each document acts as a sub-tree and the links are the edges connecting them. It supports various operations designed to assist with navigating and restructuring the graph.

The main focus of IWE is to help you keep your notes tidy and structured. It works with the graph at the semantic level, understanding the **headers**, **lists** and **links** defined structure of the documents.

IWE allows you to organize notes **hierarchically** using block-references, all without relying on a folder structure, directly within your favorite editor.

> [!NOTE]
>
> A simple analogy for software engineers would be an IDE for Markdown notes.

IWE functions in two modes, as editor extension and CLI utility

## Editor Extension Mode

IWE integrates seamlessly with your editor, helping you to:

- **Search** through your notes
- **Navigate** through markdown links
- **Auto-complete** links as you type
- **Extract** or **inline** sub-notes seamlessly
- **Format** the document and refresh link titles
- **Rename** files and automatically update all related links
- Select **backlinks** to find references to the current document
- **Convert** lists into headers and vice versa
- Display **inlay hints** with parent note references and link counts

Please visit [IWE.md](https://iwe.md) for more information and [quick start guide](https://iwe.md/quick-start/).

### Demos

<details>
<summary>Notes search</summary>

![Demo](https://iwe.md/images/search.gif)

</details>

<details>
<summary>Auto-formatting</summary>

![Demo](https://iwe.md/images/normalization.gif)

</details>

<details>
<summary>Extract note</summary>

![Demo](https://iwe.md/images/extract.gif)

</details>

## Command Line Utility Mode

This tool lets you process thousands of documents in just a second. With IWE, you can reformat documents and update link titles across your entire library. You can also use the CLI mode to combine multiple files into one extended document.

## Installation

Installation instructions are below. Editor integration is covered in the [quick start](https://iwe.md/quick-start) section.

### From The AUR

Arch Linux users can install the `iwe-bin` or `iwe-git` packages. They include the `iwe` and `iwes` binaries.

```bash
# You can use your favorite AUR helper.
paru -S iwe-bin
paru -S iwe-git
```

### From Crates.IO

- Rust and Cargo must be installed on your system. You can get them from [rustup.rs](https://rustup.rs).

IWE is available at [crates.io](https://crates.io/crates/iwe). You can install IWE using cargo (and [iwes](https://crates.io/crates/iwes) for LSP server)

```sh
cargo install iwe
cargo install iwes
```

The binaries will be installed to `$HOME/.cargo/bin`. You may need to add it to your `$PATH`.

### From Source

Clone the repository, navigate into the project directory, and build the project:

```sh
git clone git@github.com:iwe-org/iwe.git
cd iwe
cargo build --release
```

This will create executables located in the `target/release` directory.

## Get Involved

IWE fully depends on community support, which is essential for its growth and development. We encourage you to participate in [discussions](https://github.com/iwe-org/iwe/discussions) and report any [issues](https://github.com/iwe-org/iwe/issues) you encounter.

## Debug Mode

IWE includes a debug mode, which can be enabled by setting the `IWE_DEBUG` environment variable. In debug mode, IWE LSP will generate a detailed log file named `iwe.log` in the directory where you started it. Including logs with your [issue](https://github.com/iwe-org/iwe/issues) report will help us to resolve it faster.

```
export IWE_DEBUG=true; nvim
```

## Inspired by many other open-source projects

- [zk notes](https://github.com/zk-org/zk)
- [neuron](https://github.com/srid/neuron)
- [rust-analyzer](https://rust-analyzer.github.io)

## PS

A huge thank you to my wife, Iryna ❤️, for all her support and for giving me the time I needed to finish this over the weekends!

Thanks to the Rust community for creating such amazing software development tools. I've really enjoyed learning and using them in the process of building IWE.
