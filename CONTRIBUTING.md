# Contributing to miga

Thank you for your interest in contributing! All contributions are welcome —
bug reports, feature requests, documentation improvements and code patches.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Reporting bugs](#reporting-bugs)
- [Requesting features](#requesting-features)
- [Development setup](#development-setup)
- [Code style](#code-style)
- [Submitting a pull request](#submitting-a-pull-request)
- [Commit messages](#commit-messages)

---

## Code of Conduct

This project follows our [Code of Conduct](CODE_OF_CONDUCT.md).
By participating you agree to abide by its terms.

---

## Reporting bugs

1. Search [existing issues](https://github.com/HormigaDev/miga/issues) first.
2. If none match, open a new issue with:
   - miga version (`miga --version`)
   - Operating system and version
   - Minimal steps to reproduce
   - Expected vs actual behaviour
   - Any relevant error output

---

## Requesting features

Open a [GitHub issue](https://github.com/HormigaDev/miga/issues) with the
`enhancement` label. Describe:

- The problem you are trying to solve.
- Your proposed solution (if you have one).
- Any alternatives you considered.

---

## Development setup

### Prerequisites

- Rust 1.78 or later (install via [rustup](https://rustup.rs/))
- OpenSSL development headers (needed by `reqwest` / `native-tls`)

  ```bash
  # Debian / Ubuntu
  sudo apt install libssl-dev pkg-config

  # Fedora
  sudo dnf install openssl-devel
  ```

### Building

```bash
git clone https://github.com/HormigaDev/miga.git
cd miga
cargo build
```

### Running tests

```bash
cargo test
```

### Running the dev binary

```bash
cargo run -- <subcommand> [args]
```

---

## Code style

- **Language**: all code and comments must be in **English**.
- **Formatting**: run `cargo fmt` before committing.
- **Lints**: the project must compile without warnings (`cargo clippy -- -D warnings`).
- **Error handling**: use `anyhow::Result` for all fallible functions.
  Prefer `context()` / `with_context()` over bare `.unwrap()`.
- **Shared logic**: avoid duplicating helpers across command modules.
  Place reusable project I/O in `utils/project.rs` and shared utilities
  in the appropriate `utils/` module.
- **Path handling**: prefer `&Path` / `AsRef<Path>` over `&str` or
  `.to_str().unwrap()` when working with file-system paths.

---

## Submitting a pull request

1. Fork the repository and create a branch from `main`:

   ```bash
   git checkout -b fix/my-bug-fix
   ```

2. Make your changes. Ensure all tests pass and there are no new warnings:

   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

3. Push your branch and open a PR against `main`.
4. Fill in the PR template (description, motivation, testing notes).
5. A maintainer will review and merge or request changes.

---

## Commit messages

Use the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <short summary>
```

| Type | When to use |
|------|-------------|
| `feat` | A new feature |
| `fix` | A bug fix |
| `docs` | Documentation only |
| `refactor` | Code change with no behaviour change |
| `test` | Adding or updating tests |
| `chore` | Build system, dependencies, CI changes |

**Example**

```
feat(fetch): resolve transitive dependencies automatically
fix(build): correct mcaddon archive path on Windows
docs: add CONTRIBUTING.md
```
