# Threat Model Overview

_Last reviewed: November 3, 2025_

This project delivers a set of Rust crates for parsing and evaluating Go-style
templates. It does **not** make network calls, spawn child processes, or read
from the filesystem beyond what the embedding application exposes. In other
words, we are a library that executes within the host application's trust
boundaries.

## Assets and goals

- **Deterministic template evaluation** – inputs (templates plus data) should
  produce the documented outputs without panics or memory corruption.
- **Predictable helper implementations** – Go helper parity and Sprig helpers
  should match their documented behaviour.
- **Supply-chain integrity** – released crates should correspond to reviewed
  source and known dependency versions.

## Trust boundaries

| Boundary | Description | Notes |
| --- | --- | --- |
| Host application → library | The embedding application calls into the library with templates and data. | We assume the host validates/sanitises data appropriate to its threat model. The library treats inputs as untrusted and attempts to handle malformed templates/data without undefined behaviour. |
| Library → dependencies | The library depends on the Rust standard library and select crates; the Go sanity harness depends on Go modules (Sprig, x/crypto). | Dependencies are scanned via `cargo-audit`, `cargo-deny`, and `go-licenses`; version bumps are handled in CI. |
| Release pipeline | Crates published to crates.io / source tags in git. | FUTURE: add signed releases and documented verification per SECURITY.md roadmap. |

## Threats in scope

- **Malformed or hostile templates/data** causing panics, infinite loops, or
  excessive resource usage. Mitigations: extensive unit/integration tests,
  shared fixtures with the Go oracle, fuzzing and mutation testing (run in
  scheduled CI).
- **Logic inconsistencies** between Rust helpers and the Go reference causing
  unexpected behaviour. Mitigations: CI runs the Go oracle and compatibility
  suites on every change.
- **Dependency vulnerabilities** in Rust crates or Go modules. Mitigations:
  automated scanning in CI plus manual updates (e.g., tracking
  `golang.org/x/crypto`).

## Threats considered out of scope

- **Sandboxing / multi-tenant isolation.** The embedding application is
  responsible for constraining resource usage (timeouts, memory limits) and for
  ensuring that untrusted template authors cannot escalate privileges.
- **Network/file-system security.** The library never reaches out to external
  systems; any access is mediated by the host application.
- **Authentication/authorisation.** The library assumes callers have already
  validated who may execute templates.

## Process controls

- **Secure development practices:** code review (when a second maintainer
  joins), linting (`cargo clippy`, `go vet`), fmt enforcement, CI gates, and
  nightly scheduled fuzz/mutation jobs.
- **Issue response:** vulnerabilities and bugs track through GitHub issues; the
  SECURITY.md roadmap documents the plan for a private intake channel and SLAs.
- **Release hygiene:** release-plz automation keeps version bumps consistent;
  FUTURE work will add signed artifacts and curated release notes (see
  SECURITY.md).

Maintainers should revisit this document when new functionality introduces
different trust boundaries (e.g., executing plugins, embedding network helpers,
persisting to disk) or when the security roadmap in SECURITY.md is updated.
