# Linking and note templates

The "attach" code action allows you to link content under your cursor to another document. This feature is perfect for creating daily notes, collecting ideas in an inbox, or organizing thoughts into topic-specific documents.

Note: Enable [Inlay hints](feature-inlay-hints.md) to see the action results immediately. (the list of backlinks will be updated automatically)

## How It Works

When you place your cursor on a block reference and trigger a code action, IWE can:

1.  **Create or open** a target file based on your configuration
2.  **Append** block reference under cursor to the target file

The target file is determined by templates you configure, allowing for dynamic file creation based on dates or static collection files.

## Common Use Cases

### Daily Note Collection

Automatically link interesting thoughts, tasks, or notes to today's daily note:

``` toml
[actions.daily]
type = "attach"
title = "Add to Daily Note"
key_template = "daily/{{today}}"
document_template = "# Daily Note - {{today}}\n\n{{content}}\n\n"
```

**Workflow:**

1.  While reading or writing, place cursor on any content
2.  Trigger code action and select "Add to Daily Note"
3.  Content gets appended to `daily/2024-01-15.md` (or created if doesn't exist)
4.  Continue your work knowing the important bits are captured

### Inbox System

Create an inbox for collecting random thoughts and ideas:

``` toml
[actions.inbox]
type = "attach"
title = "Send to Inbox"
key_template = "inbox"
document_template = "# Inbox\n\n{{content}}\n\n"
```

**Workflow:**

1.  Come across something interesting while working
2.  Place cursor on the content and select "Send to Inbox"
3.  All collected items accumulate in a single `inbox.md` file
4.  Review and organize your inbox regularly

### Topic Collections

Organize content by themes or projects:

``` toml
[actions.research]
type = "attach"
title = "Add to Research Notes"
key_template = "research/general"
document_template = "# Research Notes\n\n{{content}}\n\n"

[actions.meeting_notes]
type = "attach"
title = "Add to Meeting Notes"
key_template = "meetings/{{today}}"
document_template = "# Meeting Notes - {{today}}\n\n{{content}}\n\n"
```

## File Creation Behavior

- **New files**: If the target file doesn't exist, it's created with the `document_template` content
- **Existing files**: Content is appended to the end of existing files
- **Duplicate prevention**: If the exact same block-reference already exists in the target file, the code action will not be suggested

## Template Variables

Attach actions support two template variables:

- `{{today}}`: Current date formatted using your configured date format
  - In `key_template`: Uses `library.date_format` (default: `"%Y-%m-%d"`)
  - In `document_template`: Uses `markdown.date_format` (default: `"%b %d, %Y"`)
- `{{content}}`: The actual content being attached

## Editor Integration

The attach code action appears in your editor's code action menu (usually triggered by Ctrl+. or Cmd+.) when:

1.  Your cursor is positioned on a block reference
2.  You have attach actions configured in your `.iwe/config.toml`

## Best Practices

### 1. Start Simple

Begin with a basic daily note or inbox system:

``` toml
[actions.capture]
type = "attach"
title = "Quick Capture"
key_template = "capture"
document_template = "# Capture\n\n{{content}}\n"
```

### 2. Use Descriptive Titles

Make it clear what each action does:

``` toml
title = "Add to Today's Notes"    # Good
title = "Attach"                  # Less clear
```

### 3. Consider File Organization

Use path separators in key templates to organize files:

``` toml
key_template = "daily/{{today}}"      # Creates files in daily/ folder
key_template = "projects/{{today}}"   # Creates files in projects/ folder
```

## Integration with PKM Workflows

The attach action works well with common Personal Knowledge Management approaches:

- **GTD (Getting Things Done)**: Use inbox actions for quick capture
- **Zettelkasten**: Create topic-specific collections and daily notes
- **PARA Method**: Organize attachments into Projects, Areas, Resources, Archives folders
- **Daily Notes**: Build a consistent journaling practice with date-based collection
