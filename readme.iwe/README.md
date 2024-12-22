# IWE - Markdown LSP server

![Demo](readme.iwe/demo.gif)

# About

IWE is a tool that helps you organize your notes. It treats notes like an interconnected graph, where each document acts as a sub-tree and the links are the edges connecting them. It supports various operations designed to assist with navigating and restructuring the graph.

- **[Block-reference](https://github.com/iwe-org/iwe/blob/master/readme.iwe/block-reference.md)** is a key concept and building block for the documents graph

  In markdown, it's a paragraph that contains one link to a note. Like this:

  ``` markdown
  A paragraph...

  [Block-reference](block-reference)

  Another paragraph...
  ```

  IWE uses these block references as if they're embedded notes. This lets you create a complex, layered document structure without having to mess with directories or overly large markdown files as well as 'reuse' a note in multiple context's. ([transclusion](https://en.wikipedia.org/wiki/Transclusion))

Once you have arranged your notes, IWE enables you to merge them into a single, cohesive document, streamlining the process of document creation. (see [readme.iwe/README](https://github.com/iwe-org/iwe/blob/master/readme.iwe/README.md) which is source for this file)

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

[LSP features](fkkn9eju.md)

[CLI features](75zkzxhu.md)

[How to install](lfdpdruo.md)

[Configuration](oxzghxkb.md)

[Help needed](kpb7qxuz.md)

[Inspired by many other opens-source projects](r4vklxyb.md)

[PS](1pluyx2u.md)
