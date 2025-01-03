# IWE - Personal Knowledge Managemnt (PKM) system

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

[Text editor extension features](lsp.md)

[CLI features](cli.md)

[Nested documents](nested-doucments.md)

[How to install](how-to-install.md)

[Configuration](configuration.md)

[Help needed](help-needed.md)

[Inspired by many other opens-source projects](inspired-by.md)

[PS](ps.md)
