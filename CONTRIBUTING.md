# Contributing to UPG

Thank you for your interest in contributing to the Universal Payment Gateway! This document provides guidelines and instructions for contributing.

---

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment. Be kind, constructive, and professional in all interactions.

---

## How to Contribute

### Reporting Bugs

1. Check [existing issues](https://github.com/revengerrr/upg/issues) to avoid duplicates.
2. Open a new issue with:
   - Clear title describing the bug
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details (OS, Solana CLI version, Anchor version)

### Suggesting Features

1. Open a GitHub issue with the `enhancement` label.
2. Describe the feature, its use case, and why it benefits UPG.

### Submitting Code

1. **Fork** the repository.
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** following the coding standards below.
4. **Write tests** for your changes.
5. **Run the test suite** to ensure nothing is broken:
   ```bash
   # On-chain programs
   anchor test

   # Off-chain services
   cargo test
   ```
6. **Commit** with a clear message:
   ```bash
   git commit -m "feat: add payment timeout handling"
   ```
7. **Push** and open a **Pull Request** against `main`.

---

## Coding Standards

### Rust (On-Chain Programs & Services)

- Follow standard Rust conventions (`rustfmt`, `clippy`)
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings
- Use meaningful variable and function names
- Document public functions with `///` doc comments

### TypeScript (SDK & Tests)

- Use ESLint + Prettier for formatting
- Use TypeScript strict mode
- Write JSDoc comments for exported functions

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new feature
fix: fix a bug
docs: update documentation
test: add or update tests
refactor: code refactoring (no feature change)
chore: maintenance tasks
```

---

## Development Setup

### Prerequisites

- Rust (latest stable)
- Solana CLI v1.18+
- Anchor Framework v0.30+
- Node.js 18+
- PostgreSQL 15+
- Redis 7+

### Local Development

```bash
# Clone the repo
git clone https://github.com/revengerrr/upg.git
cd upg

# Build on-chain programs
anchor build

# Run local Solana validator
solana-test-validator

# Run tests
anchor test
```

---

## Pull Request Process

1. Ensure all tests pass.
2. Update documentation if your change affects public APIs.
3. Request review from at least one maintainer.
4. Squash commits before merging (or maintainer will squash-merge).

---

## Questions?

Open a [Discussion](https://github.com/revengerrr/upg/discussions) on GitHub or reach out to the maintainers.
