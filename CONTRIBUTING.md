# Contributing to Venturi

We love your input! We want to make contributing to Venturi as easy and transparent as possible.

## Development Setup

1. Clone the repository
2. `cargo test` to ensure all tests pass
3. Check `AGENTS.md` for architecture and codebase specifics

## Pull Requests

1. Fork the repo and create your branch from `master`.
2. If you've added code that should be tested, add tests.
3. If you've changed APIs, update the documentation and `man/venturi.1`.
4. Ensure the test suite passes (`cargo test`).
5. Run `cargo clippy` and `cargo fmt`.
6. Issue that pull request!

## Code Style

- Follow standard Rust formatting (`cargo fmt`).
- Use the standard `crate::error::Result` for fallible returns.
- Keep the DAG topology enforcement strict.

## Report bugs using GitHub's issues

We use GitHub issues to track public bugs. Report a bug by opening a new issue.
