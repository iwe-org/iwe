# Why It Exists

I've always been a big fan of modern text editors like Neovim and Zed, and I've longed to manage my Markdown notes in a way similar to how I write code. I wanted features like "Go To Definition" for diving into details, "Extract note" refactoring for breaking down complex documents into smaller more manageable notes, and autocomplete notes linking.

All modern editors support the Language Server Protocol (LSP), which enhances text editors with IDE-like capabilities. This was exactly what I wanted for my Markdown notes.

So, I developed an LSP called IWE. It includes essential features such as note search, link navigation, autocomplete, backlink search, and some unique capabilities like:

- Creating a nested notes hierarchy.
- Extract/inline refactoring for improved note management.
- Code actions for various text transformations.
- And more

IWE allows you to build a notes library that can support basic journaling as well as GTD, Zettelkasten, PARA, you name it methods of note-taking. IWE does not enforce any structure on you notes library. It doesn't care about your file names preference. It's only give you tools to manage the documents and connections between them with least possible effort automating routine operations such as formatting, keeping link titles up to date and many other.

This is all possible because of IWE's unique [Architecture](architecture.md). IWE loads notes into an in-memory graph structure, which understands the hierarchy of headers and lists. This allows it to go through the graph, reorganize, and transform the content as needed using graph iterators.

[Unique Features](comparison.md)
