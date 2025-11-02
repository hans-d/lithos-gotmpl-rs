# Contributing Guide

Thanks for your interest in improving this project! This document outlines how to set up a
development environment, run the test suite, and prepare changes for review.

## Getting Started

### Option A: Dev Container

1. Install the [Dev Containers CLI](https://github.com/devcontainers/cli) (or use an editor that
   supports the same specification, such as VS Code or Codespaces).
2. From the repository root run:
   ```bash
   devcontainer up --workspace-folder .
   devcontainer exec --workspace-folder . just ci-test
   ```
   The container installs Rust (stable + nightly), Go, Python, and all required command line tooling
   automatically via `.devcontainer/postCreate.sh`.

### Option B: Native Setup

1. Install Rust (MSRV 1.70.0+) and Go 1.25.1.
2. Install the helpers in one step:
   ```bash
   just install-ci-tools
   ```
   This pulls in `cargo-audit`, `cargo-deny`, `cargo-tarpaulin`, `cargo-mutants`, `cargo-fuzz`,
   `actionlint`, and `yamllint`.
3. (Optional) Install nightly Rust (`rustup toolchain install nightly`) for fuzzing.

## Project Layout

See the top-level `README.md` for a tour of the workspace. In brief:

- `crates/lithos-gotmpl-engine` – parser / evaluator core
- `crates/lithos-gotmpl-core` – default Go helper registry
- `crates/lithos-sprig` – Sprig-compatible helpers layered on the core
- `test-cases/` – shared fixtures driven by both Rust tests and the Go oracle
- `go-sanity/` – Go implementation used for behavioural parity checks

## Running Checks

| Command | Purpose |
| --- | --- |
| `just ci-test` | Runs the full Rust test suite (workspace, doc tests, compat harness) |
| `just ci-behavior` | Invokes the Go oracle compatibility tests |
| `just ci-quality` | Runs `cargo fmt`, `cargo clippy`, `actionlint`, and `yamllint` |
| `just ci-security` | Executes `cargo audit` and `cargo deny` |
| `just ci-mutation` | Runs `cargo-mutants` (requires installation) |
| `just ci-fuzz` | Smoke-tests both fuzz harnesses (requires nightly + cargo-fuzz) |

Before submitting a pull request:

1. `just ci-test`
2. `just ci-quality`
3. Other checks as relevant to your change (e.g. `ci-behavior` when adding Sprig helpers).

## Coding Standards

- Rust code is formatted with `cargo fmt` and linted with `cargo clippy` (warnings are treated as
  issues in CI).
- Public APIs should carry concise rustdoc comments.
- If you add a new helper or template feature, mirror it in `test-cases/` and extend the Go oracle
  fixtures when possible.
- Keep `docs/reference/function-coverage.md` and `docs/reference/template-syntax-coverage.md` in sync with new
  capabilities.

## Fuzzing & Mutation Testing

- Fuzz harnesses live under `fuzz/`. Run `just ci-fuzz` for a quick sanity sweep. For longer runs:
  ```bash
  cargo +nightly fuzz run template_parse
  cargo +nightly fuzz run template_render
  ```
- Mutation tests use `cargo-mutants`: `just ci-mutation`.

## Submitting Changes

1. Fork the repository and create a feature branch.
2. Make your changes, update documentation/tests as needed.
3. Ensure the relevant `just ci-*` commands pass.
4. Open a pull request describing the motivation, testing performed, and any follow-up work.

We appreciate your contributions!
