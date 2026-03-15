# Custom Text Commands

IWE includes **text transformation** capabilities that can be accessed right from your text editor. You can effortlessly **rewrite** text, **expand** on ideas, **highlight** important words, or apply any custom transformation using CLI tools.

Transform actions work by piping content through external commands. You can use AI tools like `claude -p` (Claude CLI in pipe mode), custom scripts, or any command-line tool that reads from stdin and writes to stdout.

## How It Works

1. Select or place cursor on a text block in your editor
2. Open the code actions menu (`Ctrl+.` or `Cmd+.`)
3. Select a transform action (e.g., "Rewrite", "Expand")
4. The text is processed through your configured command
5. The result replaces the original content

## Configuration

Transform actions require two parts:

1. **Command definition** - the CLI tool to run
2. **Action definition** - how to use the command

``` toml
[commands.claude]
run = "claude -p"
timeout_seconds = 120

[actions.rewrite]
type = "transform"
title = "Rewrite"
command = "claude"
input_template = """
Rewrite this text to improve clarity and readability.
Keep links intact.

{{context}}
"""
```

Check [Configuration](configuration.md) section for detailed instructions on how to set up custom commands.

## Example Commands

**Using Claude CLI:**
``` toml
[commands.claude]
run = "claude -p"
timeout_seconds = 120
```

**Using a Python script:**
``` toml
[commands.summarize]
run = "python ~/scripts/summarize.py"
timeout_seconds = 30
```

**Using standard Unix tools:**
``` toml
[commands.uppercase]
run = "tr '[:lower:]' '[:upper:]'"
timeout_seconds = 5
```
