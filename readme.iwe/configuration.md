# Configuration

IWE have limited configuration options at the moment, but it does come with sensible defaults.

For instance:

- Whenever a document is generated, it automatically gets a random file name made up of 8 alphanumeric characters.
- Links are generated without file extensions, with the default being `.md`.

If you'd like to tweak anything, feel free to open a pull request or an issue.

The only configuration options available lets you change the default extension for local links and the path where you want to keep the files (relative to current directory). For example:

``` json
{
  "markdown":{"refs_extension":".md"}
  "library":{"path":"readme.iwe"}
}
```

By default, IWE will add/keep `.md` extension to the local links.
