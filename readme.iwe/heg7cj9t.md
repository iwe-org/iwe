# Header levels normalization

IWE interprets nested structure created by the headers. It understands the relationships between the header. For example:

``` markdown
# First header

## Second header
```

`Second header` is sub-header of the first one. Markdown allows any headers structure. Including the cases where nesting cannot be interpreted. Like:

``` markdown
## First header

# Second header
```

IWE atomically fixes header levels for enforce correct nesting.

------------------------------------------------------------------------

IWE can also normalize the headers structure dropping unnecessary hedaer-levels, For example:

``` markdown
# First header

### Second header
```

Will be normalized into dropping unnecessary levels.

``` markdown
# First header

## Second header
```

------------------------------------------------------------------------

First header of the document/section determines zero-level. In this case it is set to level 3 so all subsequent headers are going to be adjusted to follow the starting point.

``` markdown
### First header

## Second header

### Third header
```

Will result in:

``` markdown
# First header

# Second header

# Third header
```
