# Configuration

IWE doesn't have much configuration options at the moment, but it does come with some sensible defaults.

For instance:

- Whenever a document is generated, it automatically gets a random file name made up of 8 alphanumeric characters.
- Links are generated without file extensions, with the default being `.md`.

If you'd like to tweak anything, feel free to open a pull request or an issue.

The only configuration option available lets you change the default extension for local links. For example:

``` json
{"markdown":{"refs_extension":".md"}}
```

By default, the extension is omitted.
