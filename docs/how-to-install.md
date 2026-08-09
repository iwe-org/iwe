# How to Install

Installation instructions are below. Editor integration is covered in the [quick start](https://iwe.md/quick-start) section.

## Using Homebrew (macOS/Linux)

The easiest way to install IWE on macOS or Linux is using Homebrew:

``` sh
brew tap iwe-org/iwe
brew install iwe
```

This installs the CLI (`iwe`), the LSP server (`iwes`), and the MCP server (`iwec`).

## Using npm (macOS/Linux/Windows)

Prebuilt binaries are published to npm, including Windows — no toolchain needed:

``` sh
npm install -g @iwe-org/iwe
```

This installs the same three binaries (`iwe`, `iwes`, `iwec`). You can also run the CLI without installing:

``` sh
npx -y @iwe-org/iwe --help
```

For AI agents, [@iwe-org/mcp](https://www.npmjs.com/package/@iwe-org/mcp) starts the MCP server directly via `npx -y @iwe-org/mcp` — see the [agentic docs](https://iwe.md/docs/agentic/).

## From Crates.IO

- Rust and Cargo must be installed on your system. You can get them from [rustup.rs](https://rustup.rs).

IWE is available at [crates.io](https://crates.io/crates/iwe). You can install IWE using cargo (and [iwes](https://crates.io/crates/iwes) for LSP server)

``` sh
cargo install iwe
cargo install iwes
cargo install iwec
```

The binaries will be installed to `$HOME/.cargo/bin`. You may need to add it to your `$PATH`.

## From Source

Clone the repository, navigate into the project directory, and build the project:

``` sh
git clone git@github.com:iwe-org/iwe.git
cd iwe
cargo build --release
```

This will create executables located in the `target/release` directory.
