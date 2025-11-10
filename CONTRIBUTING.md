# Contributing to IWE

Thank you for your interest in contributing to IWE!

## License

This project is licensed under the [Apache License 2.0](LICENSE-APACHE).

## Contribution Terms

By submitting a contribution to this project, you agree to the following terms:

1. **License Grant**: All contributions submitted to this project will be licensed under the Apache License 2.0, the same license as the project itself.

2. **Rights Transfer**: By contributing, you willingly grant all rights to your contribution to the project maintainers and agree that your contribution becomes part of the project under the Apache License 2.0.

3. **Original Work**: You certify that your contribution is your original work or that you have the right to submit it under the Apache License 2.0.

4. **No Additional Terms**: You understand that no additional terms or conditions will apply to your contributions beyond those specified in the Apache License 2.0.

## Contribution Guidelines

### What We Accept

- **Bug fixes**: Well-documented bug fixes with test cases
- **New features**: Must start with a discussion (see below)
- **Documentation improvements**: Substantial improvements to documentation
- **Performance improvements**: With benchmarks demonstrating the improvement

### What We Reject

We will reject pull requests that only contain:

- Formatting or style changes without functional improvements
- Linting fixes unrelated to actual issues
- Typo fixes in code comments or variable names
- Other trivial changes that don't add meaningful value

### Before Contributing a New Feature

**Important**: Before starting work on a new feature, please open a GitHub Discussion or Issue to discuss:

- The problem you're trying to solve
- Your proposed solution
- How it fits into the project's goals
- Implementation approach

This helps avoid wasted effort on features that may not align with the project's direction or may already be in development.

## How to Contribute

### Getting Started

1. Fork the repository
2. Create a new branch for your feature or bugfix
3. Make your changes
4. Test your changes thoroughly
5. Submit a pull request

### Development Setup

See the [CLAUDE.md](CLAUDE.md) file for detailed development instructions for different parts of the monorepo:

- **Rust workspace** (CLI and LSP): See "Rust Development" section
- **VS Code extension**: See "VS Code Extension" section
- **Neovim plugin**: See "Neovim Plugin Development" section
- **Documentation**: See "Documentation Site" and "Book Documentation" sections

### Code Standards

- Follow the existing code style and conventions
- Run tests and linting before submitting
- Write clear commit messages
- Update documentation as needed

### Testing

- **Rust**: Run `cargo test` to execute all tests
- **VS Code Extension**: Run `npm test`
- **Neovim Plugin**: Run `make test`

## Questions?

If you have questions about contributing, please open an issue in the GitHub repository.

---

**By submitting a pull request or otherwise contributing to this project, you acknowledge that you have read and agree to these contribution terms.**
