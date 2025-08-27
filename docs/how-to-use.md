# How to use with your text editor

## Purpose

This page intends to teach you how to trigger IWE's features from inside your text editor.

## Background

IWE's features are implemented as Language Server Protocol (**LSP**) capabilities. This makes IWE *editor-agnostic*; it's intended to work the same across all text editors that support the LSP standard.

What this means for you is that to interact with IWE from inside your editor, you need to use *LSP requests*. These may be accessed differently across editors, and it's up to each editor to implement them properly. **If you've ever used something like "Find References" or "Go To Definition" before, then you're already familiar with LSP requests.**

## Primary Features

IWE provides comprehensive features for markdown-based knowledge management:

### Core LSP Features

- 🤖 **AI-Powered Text Generation**: Generate, rewrite, or modify text using configurable AI commands
- 🔍 **Global Search**: Search through all notes using fuzzy matching on document paths and content
- 🧭 **Link Navigation**: Follow links between documents with Go To Definition
- 📥 **Extract/Inline Notes**: Split sections into separate files or merge them back
- 📝 **Auto-Format**: Normalize document structure, headers, lists, and link titles
- 🔄 **Rename Refactoring**: Rename files while automatically updating all references
- 🔗 **Backlinks Discovery**: Find all documents that reference the current document
- 💡 **Inlay Hints**: Display parent document references and link usage counts
- ✨ **Auto-Complete**: Smart completion for links as you type
- 🗂️ **Document Symbols**: Navigate document outline via table of contents
- 🔧 **Text Manipulation**: Transform lists to headers and vice-versa, change list types

## LSP Feature Reference

Here's a reference connecting each LSP request with IWE features:

|IWE Feature|LSP Request|Description|
|-----------|-----------|-----------|
|Extract/Inline Notes|Code Action|Split sections into files or merge them back|
|AI Text Generation|Code Action|Generate, rewrite, or modify text using AI|
|Text Transformation|Code Action|Convert lists to headers, change list types|
|Link Navigation|Go To Definition|Follow markdown links to target documents|
|Backlinks|Go To References|Find all documents referencing current document|
|Document Outline|Document Symbols|View table of contents for navigation|
|Global Search|Workspace Symbols|Search through all notes with fuzzy matching|
|Auto-Format|Document Formatting|Normalize structure, headers, and links|
|File Renaming|Rename Symbol|Rename files and update all references|
|Link Completion|Completion|Auto-complete links as you type|
|Visual Hints|Inlay Hints|Show parent references and link counts|

## Usage Example

> **Editor Compatibility**: Most editors have keybindings for LSP requests. Common patterns include:
>
> - VS Code: `Ctrl+Shift+P` (Command Palette) → search for LSP commands
> - Neovim: `<leader>ca` (code actions), `gd` (go to definition), `gr` (references)
> - Helix: `space+a` (code actions), `gd` (go to definition), `gr` (references)
> - Zed: `Cmd+.` (code actions), `F12` (go to definition), `Shift+F12` (references)

Suppose that you have the following in a Markdown file:

``` markdown
# My First Note

There's some content here.

## Another section

With a list inside it:

- list item
- another item
```

### Extracting a Section

1.  Move your cursor to the `## Another section` line
2.  Invoke the **Code Action** command (varies by editor)
3.  Select "Extract section" from the options
4.  Your file will now look like this:

``` markdown
# My First Note

There's some content here.

[Another section](2sbdlvhe)
```

The `2sbdlvhe` refers to the name of a new file IWE generated for you.

### Following the Link

1.  Move your cursor anywhere on the `[Another section](2sbdlvhe)` link
2.  Use **Go To Definition** command
3.  Your editor will open the new file containing:

``` markdown
# Another section

With a list inside it:

- list item
- another item
```

### Finding Backlinks

1.  In the extracted file, move your cursor to the `# Another section` line
2.  Use the **Go To References** command
3.  You'll see a list of all files that link to this document
4.  Select the original file to navigate back

## Advanced Features

### AI-Powered Actions

IWE supports configurable AI commands that can:

- Rewrite and improve text
- Generate new content based on prompts
- Expand on ideas and concepts
- Add formatting and structure

Configure AI actions in your `.iwe/config.toml`:

``` toml
[models.default]
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com"
name = "gpt-4o"

[actions.rewrite]
title = "Improve Text"
model = "default"
context = "Document"
prompt_template = "Improve this text: {{context}}"
```

[Configuration](Configuration.md)

### Text Transformations

Use **Code Actions** to transform document structure:

- Convert bullet lists to numbered lists
- Transform lists into header hierarchies
- Convert headers back to lists
- Change outline organization

### Auto-Formatting

The **Document Formatting** command will:

- Normalize header formatting and spacing
- Standardize list formatting
- Update link titles automatically
- Fix markdown syntax issues
- Ensure consistent document structure

### Global Search

Use **Workspace Symbols** to:

- Search across all documents
- Find content by fuzzy matching
- Navigate to specific sections
- Explore document relationships

Results show full paths like:

```
Journal, 2025 ⇒ Week 3 - Coffee week ⇒ Jan 26, 2025 - Cappuccino
My Coffee Journey ⇒ Week 3 - Coffee week ⇒ Jan 26, 2025 - Cappuccino
```

## Inlining Extracted Sections

To reverse section extraction:

1.  Move your cursor to a link like `[Another section](2sbdlvhe)`
2.  Invoke **Code Action**
3.  Select "Inline section"
4.  The content returns to the original document:

``` markdown
# My First Note

There's some content here.

## Another section

With a list inside it:

- list item
- another item
```

**Note**: Inlining automatically deletes the separate file after merging content back.

## Working with New Files

When IWE creates new files (via extraction):

- Files are initially created in memory/buffer
- Save them using your editor's save command
- In some editors, use "Save All" to ensure all new files are written to disk
- Files use unique identifiers as filenames for reliable linking

## Best Practices

1.  **Use Meaningful Headers**: Clear section titles improve navigation and search
2.  **Link Liberally**: Create connections between related concepts
3.  **Regular Formatting**: Use document formatting to maintain consistency
4.  **Organize with Extraction**: Break large documents into focused, linked sections
5.  **Leverage Search**: Use global search to discover connections and content
6.  **Configure AI**: Set up AI actions that match your writing workflow
7.  **Use Inlay Hints**: Enable hints to understand document relationships at a glance
