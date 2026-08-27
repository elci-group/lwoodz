# Contributing to Lwoodz

Thank you for your interest in Lwoodz! This document explains how to
contribute effectively.

## Code of Conduct

All contributors are expected to follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## How to contribute

- **Bug reports:** Open an issue with a minimal reproduction and the version of
  Lwoodz you are using.
- **Feature requests:** Open an issue describing the use case, and which
  ecosystem(s) it affects (Cargo, npm, Go, Python).
- **Pull requests:** We welcome fixes, documentation improvements, and small,
  focused features. For large changes — a new ecosystem scanner, a new
  inference provider — please open an issue first to discuss the design.

## Development workflow

1. Fork the repository and create a feature branch.
2. Make your changes, including tests where appropriate.
3. Run the full Rust checks:

   ```bash
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   cargo audit
   cargo deny check advisories sources
   ```

4. Update `CHANGELOG.md` with a short entry describing your change.
5. Commit with a clear message and open a pull request.

## Coding conventions

- Follow the existing Rust style (`cargo fmt`).
- Keep public APIs documented.
- Prefer small, focused commits and pull requests.
- Add tests for new functionality and bug fixes — especially for ecosystem
  scanners (`src/manifest/*.rs`), where a false "license resolved" is worse
  than an honest "unknown."
- New source files should carry the standard header; run `lwoodz-cli headers`
  before committing rather than hand-writing it.

## Licensing

By contributing to Lwoodz, you agree that your contributions will be licensed
under the MIT license, matching the rest of the project.
