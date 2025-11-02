# Agent Reference

This file summarizes expectations and useful commands for automated assistants working in this
repository.

## Key Tooling

- **Just**: preferred command runner. The important recipes are documented in `README.md`. Common
  ones:
  - `just ci-test` – run the full Rust test suite
  - `just ci-quality` – fmt, clippy, actionlint, yamllint
  - `just ci-behavior` – Go oracle compatibility checks
  - `just ci-fuzz` – short fuzzing smoke test (requires nightly)
  - `just ci-release` – release-plz dry run (updates versions only)
- **Go oracle**: `just go-sanity` executes the Go reference implementation for Sprig fixtures.
- **Release automation**: release-plz configuration lives in `release-plz.toml`; the tool only
  proposes version bumps (no changelog management).
- **Fuzzing**: harnesses in `fuzz/` (excluded from the main workspace); needs nightly + cargo-fuzz.

## Environment Notes

- The repository provides a devcontainer definition under `.devcontainer/`. It installs Rust
  (stable + nightly), Go 1.25.1, Python 3.12, and required tools via `just install-ci-tools` during
  post-create.
- When running outside the container, initialize tooling with `just install-ci-tools`.
- Go binaries (e.g., `actionlint`) are expected under `$(go env GOPATH)/bin`; ensure that directory
  is on `PATH` when invoking just recipes.
- Do **not** modify `justfile` targets to work around sandboxed shells or missing permissions.
  Recipes should assume a normal developer environment; handle ad-hoc sandbox quirks manually
  instead of baking them into the shared automation.
- GitHub CLI (`gh`) is available; use it for repository setting audits, dependency review checks,
  or workflow status queries when needed.

## Coding Standards

- Rust edition 2021, MSRV 1.70.0.
- Lints: `unsafe_code` forbidden, `missing_docs` warned, Clippy `all` + `pedantic` as warnings.
- Public APIs should carry rustdoc comments.
- Sprig/helper additions should be mirrored in `test-cases/` and validated with the Go oracle.

## Documentation

- Architectural overview lives in `README.md`.
- `docs/README.md` indexes the extended documentation set.
- Testing strategy and expectations are documented in `docs/guides/testing.md`.
- Releasing instructions are in `docs/operations/releasing.md`.
- Contributor guidance is in `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md`.

## Miscellaneous

- `fuzz/` is excluded from the workspace to avoid nightly requirements in standard workflows.
- Workflow linting uses actionlint + yamllint; keep `.github/workflows/*.yml` compliant with the
  current configuration.
