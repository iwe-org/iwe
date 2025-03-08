
# About IWE

[IWE](https://iwe.md) is an open-source, local-first, markdown-based note-taking tool. It serves as a personal knowledge management (PKM) solution **designed for developers**.

IWE integrates seamlessly with popular developer text editors such as VSCode, Zed, Neovim, Helix, and others. It connects with your editor through the Language Server Protocol (LSP) to assist you in writing and maintaining your Markdown documents.

IWE offers powerful features such as **search**, **auto-complete**, **go to definition**, **find references**, **rename refactoring**, and more. In addition to standard Markdown, it also supports wiki-style links, tables, and other Markdown extensions.

The primary focus of IWE is to help you keep your notes tidy and structured. It understands the structure of your documents defined by **headers**, **lists**, and **links** and supports advanced refactorings, such as extract/embed note and many other via LSP **code actions**.

While IWE supports sub-directories and relative links, it also allows you to organize notes **hierarchically** using Map of Content ([MOC](https://github.com/iwe-org/iwe/wiki/Map-of-content)) documents.

> [!NOTE]
>
> The goal of the project is to make managing knowledge as seamless as managing code, enabling your PMK system to function like an IDE for Writing (IWE).

The main LSP features are:

- 🔍 **Search** through your notes
- 🧭 **Navigate** through markdown links
- ✨ **Auto-complete** links as you type
- 📥 **Extract** or **inline** sub-notes seamlessly
- 📝 **Format** the document and refresh link titles
- 🔄 **Rename** files and automatically update all the links
- 🔗 Search for **backlinks** to find references to the current document
- 💡 Display **inlay hints** with parent note references and link counts

You can learn more on the [LPS Features](https://github.com/iwe-org/iwe/wiki/LSP-features) page.

IWE also provides a CLI utility that allows you to process thousands of documents in just a second. With IWE, you can reformat documents and update link titles across your entire library. Additionally, you can use the CLI mode to combine multiple files into one comprehensive document. More information is available [here](https://github.com/iwe-org/iwe/wiki/CLI-features).

Quick Demos:

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

## How to install

You can find the installation instructions in the [Quick Start Guide](https://iwe.md/quick-start).

Check [usage guide](https://github.com/iwe-org/iwe/wiki/Usage) for more information.

## Get Involved

IWE fully depends on community support, which is essential for its growth and development. We encourage you to participate in [discussions](https://github.com/iwe-org/iwe/discussions) and report any [issues](https://github.com/iwe-org/iwe/issues) you encounter.

Contributions to the project [documentation](https://github.com/iwe-org/iwe/wiki) are also highly appreciated.

### Plugins / Packages

This repository is for Rust code and crates publishing only. Plugins and packages are in separate repositories. If you are willing to help with a non-listed package type, I'm happy to add a repo for it.

- VSCode plugin is avialbe [here](https://marketplace.visualstudio.com/items?itemName=IWE.iwe) ([repository](https://github.com/iwe-org/vscode-iwe))
- Zed plugin [repository](https://github.com/iwe-org/zed-iwe)

We're looking for a maintainer for the dedicated Neovim [plugin](https://github.com/iwe-org/iwe.nvim).

### Debug Mode

IWE includes a debug mode, which can be enabled by setting the `IWE_DEBUG` environment variable. In debug mode, IWE LSP will generate a detailed log file named `iwe.log` in the directory where you started it. Including logs with your [issue](https://github.com/iwe-org/iwe/issues) report will help us to resolve it faster.

```
export IWE_DEBUG=true; nvim
```

### Special thanks to

- A heartfelt thank you to [Sergej Podatelew](https://github.com/spodatelev) for his outstanding work on the VSCode plugin.
- Deep appreciation to [Daniel Fichtinger](https://github.com/ficcdaf) for his contributions to the project documentation and community.

### Inspired by many other open-source projects

- [pandoc](https://pandoc.org)
- [zk notes](https://github.com/zk-org/zk)
- [neuron](https://github.com/srid/neuron)
- [rust-analyzer](https://rust-analyzer.github.io)

### PS

A huge thank you to my wife, Iryna ❤️, for all her support and for giving me the time I needed to finish this over the weekends!

Thanks to the Rust community for creating such amazing software development tools. I've really enjoyed learning and using them in the process of building IWE.
