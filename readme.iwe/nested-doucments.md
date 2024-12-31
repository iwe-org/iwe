# Nested documents

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
