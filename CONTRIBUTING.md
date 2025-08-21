# Contributing to AgentState

Thanks for your interest! Please:
- Open an issue to discuss significant changes.
- Follow our code of conduct.
- Run formatting and linting before pushing.

## Dev setup

- Rust stable (cargo fmt/clippy)
- Node 20+ (for TS SDK)
- Python 3.11+ (for Py SDK)

## Pre-commit

Install hooks:

```
pipx install pre-commit
pre-commit install
```

## Tests

Use `make verify` for integration checks. Keep changes focused and documented.

