# Block-reference

Block-reference is a markdown **paragraph containing single reference**.

- In markdown, it's a paragraph that contains one link to a note. Like this:

  [A block references example](document-id.md)

IWE reads this type of markdown structure as a nested document, allowing it to create "Paths" in the graph for search purposes. It can also squash references into the parent document, adding extra content to it. ([transclusion](https://en.wikipedia.org/wiki/Transclusion))
