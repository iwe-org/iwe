# How to install

Installation instructions are below. Editor integration is covered in the [quick start](https://iwe.md/quick-start) section.

## From Crates.IO

- Rust and Cargo must be installed on your system. You can get them from [rustup.rs](https://rustup.rs).

IWE is available at [crates.io](https://crates.io/crates/iwe). You can install IWE using cargo (and [iwes](https://crates.io/crates/iwes) for LSP server)

``` sh
cargo install iwe
cargo install iwes
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
