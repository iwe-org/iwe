# Header Levels Normalization

IWE reads and understands nested structures based on headers. It identifies sub-header relationships. Markdown allows header structures where the nesting isn't clear, like:

``` markdown
## First Header

# Second Header
```

IWE automatically fixes the header levels to ensure they're nested correctly. So the example above corrects to:

``` markdown
# First Header

# Second Header
```

## Removing unnecessary levels

IWE can normalize the headers structure by dropping unnecessary header levels, for example:

``` markdown
# First header

### Second header
```

Will be normalized by dropping unnecessary levels and will look like:

``` markdown
# First header

## Second header
```
