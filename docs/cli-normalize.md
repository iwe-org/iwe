# IWE Normalize

Performs comprehensive document normalization across all markdown files.

## Usage

``` bash
iwe normalize
```

## Operations performed

- Updates link titles to match target document headers
- Adjusts header levels for consistent hierarchy
- Renumbers ordered lists
- Fixes markdown formatting (newlines, indentation)
- Standardizes list formatting
- Normalizes document structure

## Examples

``` bash
# Basic normalization
iwe normalize

# With debug output (global verbose option)
iwe -v 2 normalize
```

**Important:** Always backup your files before running normalization, especially the first time.
