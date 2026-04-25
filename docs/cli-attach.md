# IWE Attach

Attach a document as a block reference under one or more configured **attach actions**.

## Usage

``` bash
iwe attach --list
iwe attach --to <NAME> [--to <NAME> ...] -k <KEY>
```

## Options

| Flag                   | Description                                                          | Default |
| ---------------------- | -------------------------------------------------------------------- | ------- |
| `--to <NAME>`          | Configured attach action to attach to. Repeatable for multiple targets. | (required when not in list mode) |
| `-k, --key <KEY>`      | Source document key to attach                                        | (required when not in list mode) |
| `--list`               | List configured attach actions and the resolved target keys, then exit | false |
| `--dry-run`            | Preview without writing                                              | false   |
| `--quiet`              | Suppress progress output                                             | false   |

## How it works

For each `--to <NAME>`:

1. Look up `<NAME>` in `[actions]` of `.iwe/config.toml`. The action must be of type `attach`.
2. Render the action's `key_template` to compute the target document key (e.g. `daily/{{today}}` becomes `daily/2026-04-25`).
3. If the target document doesn't exist yet, it is created with the action's `title` as the H1 and the new block reference as the body.
4. If the target exists, the new block reference is appended.
5. If the source is **already attached** in the target, that target is silently skipped — no error, no warning, no duplicate write.

The reference text on the new block is the source document's title.

## Configuration

Define an attach action in `.iwe/config.toml`:

``` toml
[actions.today]
type = "attach"
title = "{{today}}"
key_template = "daily/{{today}}"
```

## Examples

``` bash
# Discover available attach actions
iwe attach --list

# Attach a document under one configured target
iwe attach --to today -k meetings/standup

# Attach the same document under multiple targets at once
iwe attach --to today --to weekly -k meetings/standup

# Preview without writing
iwe attach --to today --to weekly -k meetings/standup --dry-run
```

## Relationship to MCP

This command mirrors the `iwe_attach` MCP tool. The MCP tool's `to` field accepts an array of action names with the same semantics.
