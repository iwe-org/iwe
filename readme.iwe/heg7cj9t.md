# Header levels normalization

IWE reads and understands nested structures based on headers. It identifies how sub-headers relationships. Markdown allows header structure where the nesting isn't clear, like:

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

IWE can normalize the headers structure dropping unnecessary header-levels, For example:

``` markdown
# First header

### Second header
```

Will be normalized into dropping unnecessary levels and will look like:

``` markdown
# First header

## Second header
```
