# Sub-directories

IWE supports organizing your markdown files in subdirectories while maintaining full functionality across all features. This allows you to structure your knowledge base hierarchically without losing the ability to link, search, and process files across directory boundaries.

## How It Works

### Recursive Directory Scanning

IWE recursively scans the configured library path and all its subdirectories:

- **Includes**: All `.md` files in any subdirectory level
- **Excludes**: Hidden files and directories (starting with `.`)
- **File Keys**: Include the relative path from library root

### File Path Resolution

Files in subdirectories get keys that include their relative path:

```
Project Structure:
your-project/
├── .iwe/config.toml
├── docs/
│   ├── guide.md          → Key: "docs/guide"
│   ├── api/
│   │   └── reference.md  → Key: "docs/api/reference"
│   └── examples/
│       └── basic.md      → Key: "docs/examples/basic"
└── README.md             → Key: "README" (if library.path = "")
```

### Cross-Directory Linking

Links use relative paths based on each file's location in the directory structure:

``` markdown
<!-- In index.md (root level) -->
See the [guide](docs/guide.md) for details.

<!-- In docs/guide.md -->
Back to [index](../index.md) or see [API reference](api/reference.md).

<!-- In docs/api/reference.md -->
Check out the [basic example](../examples/basic.md) or [guide](../guide.md).
```

**Path Resolution Rules:**

- From root to subdirectory: `subdirectory/file.md`
- From subdirectory to root: `../file.md`
- Between subdirectories at same level: `../other-directory/file.md`
- Within same directory: `file.md`
