# [IWE](https://iwe.md) - Personal Knowledge Management (PKM) system

![Demo](https://iwe.md/docs/demo.gif)

# About

[IWE](https://iwe.md) is a tool that helps you organize your markdown notes. It treats notes as an interconnected graph, where each document acts as a sub-tree and the links are the edges connecting them. It supports various operations designed to assist with navigating and restructuring the graph.

The main focus of [IWE](https://iwe.md) is to help you keep your notes organized. It works with the graph at the semantic level, understanding the **headers**, **lists** and **links** defined structure of the documents.

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

Please visit [IWE.md](https://iwe.md) for more information.

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
