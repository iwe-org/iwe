# Search

Search is one of the key features. IWE, creates all possible document paths by considering the block-references structure. This means it can come up with lists like:

```
Readme - Features
Readme - Features - Navigation
Readme - Features - Search
```

And provide this list to your text editor as Workspace Symbols.

This allows for context-aware fuzzy searching, making it easier for you to find what you need.

The search results are ordered by page-rank which is based on the number of references to the target note.
