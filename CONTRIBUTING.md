# Contributing Guide

Thanks for your interest in improving this project! This document outlines:

- Where to ask questions or report problems.
- How to set up a development environment and run the test suite.
- What we currently expect from contributions (including open items we still need to formalise).

For a security-focused status snapshot, see `docs/security/threat-model.md`.

## Getting help, reporting bugs, requesting features

- **Issues & feature requests:** use the [GitHub issue tracker](https://github.com/hans-d/lithos-gotmpl-rs/issues). Please search first to avoid duplicates. For general feedback, open an issue under the `feedback` label so we can track follow-up work.
- **Security reports:** see the guidance in `SECURITY.md`. FUTURE: define a private reporting channel once additional maintainers join.
- **Discussions:** we use issues/PRs for all proposals so that the history stays searchable without proprietary tooling.

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

## Contribution expectations & open decisions

The project is still evolving its formal policies. The table below captures the **current
expectations** as well as items we still need to define (marked FUTURE).

| Topic | Current expectation | FUTURE work |
| --- | --- | --- |
| **Code style** | Rust: `cargo fmt` + `cargo clippy` must pass. Go oracle: keep files `go fmt`-clean and ensure `go test ./...` succeeds. | Document explicit style guide references (e.g., Rust API docs conventions). |
| **Static analysis** | CI runs `cargo geiger` (Rust) and `golangci-lint` (Go). If you touch those areas, run the same commands locally before pushing. | Define how to suppress/triage findings once multiple maintainers share review duties. |
| **Tests** | Add or update fixtures under `test-cases/` and/or Rust unit tests for behaviour changes. Run `just ci-test` and `just ci-quality` before opening a PR. | Capture a written “tests must accompany major functionality changes” policy in this guide and in PR template. |
| **Documentation** | Update README/docs when user-visible behaviour changes. Keep coverage docs in sync. | Draft a contributor-focused checklist for doc updates, including security/architecture notes. |
| **Release notes** | Release automation is minimal today; mention notable changes in PR descriptions. | Define a release note process (e.g., changelog aggregation) before the next tagged release. |
| **Branch protection / reviews** | Single maintainer currently; Scorecard branch-protection check disabled (see workflows). | Reinstate branch protection and two-person reviews once a second maintainer joins. |
| **Security reporting** | Follow `SECURITY.md`; currently references FUTURE private channel. | Establish private intake channel and response SLA. |

Please add TODOs or notes in the relevant files when you touch areas marked FUTURE—the intent is to
make it clear what remains to be decided.

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
3. Ensure the relevant `just ci-*` commands pass (see the expectations table above).
4. Open a pull request describing the motivation, testing performed, and any follow-up work.

We appreciate your contributions!
