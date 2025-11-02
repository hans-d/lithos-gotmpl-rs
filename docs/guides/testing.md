# Testing guidelines

This repository treats the shared behavioural fixtures as the primary contract between the Rust
and Go implementations. Use this document as the reference for how new features should be
validated and how existing tests are organised.

## Testing layers

The project uses three complementary layers of tests:

1. **Behavioural fixtures** – JSON files and directory scenarios under `test-cases/` that feed both
the Rust integration tests and the Go oracle. Each fixture defines a focused template, the input
payload, and the expected rendering or error.
2. **Compatibility harness** – Rust integration tests (for example `crates/lithos-sprig/tests/compat.rs`)
that execute the shared fixtures through the Go implementation to guarantee parity before the Rust
assertions run.
3. **Supporting Rust unit tests** – module-level tests in the Rust crates that validate interpreter
internals such as scoping, whitespace semantics, or detailed error reporting that are impractical to
express as black-box fixtures.

## Working with behavioural fixtures

- Treat the fixtures in `test-cases/` as the source of truth for observable behaviour. Update or add
  them whenever you change template syntax, helper behaviour, or error messages that users see.
- Keep fixtures concise: prefer one assertion per entry, reuse JSON `data` payloads, and move
  multi-step scenarios into their own directories so both the Go and Rust runners can reuse assets.
- When you introduce new behaviours, update the relevant coverage documents (for example
  `../reference/function-coverage.md` or `../reference/template-syntax-coverage.md`) so contributors can see which
  areas are implemented or still missing tests.

## Go oracle expectations

- Use `just go-sanity` to execute the Go reference implementation locally when iterating on new or
  modified fixtures. This keeps the fixtures grounded in the upstream behaviour before Rust tests
  are involved.
- The CI recipe `just ci-behavior` must stay green; it shells out to the Go oracle and cross-checks
  that the shared fixtures still match Go. Update it if you add new fixture collections that should
  be part of the compatibility gate.

## Rust unit and integration tests

- Reserve Rust unit tests for invariants that cannot be represented cleanly in the behavioural
  fixtures: edge-case error contexts, internal state transitions, or interactions with helper
  metadata.
- Avoid duplicating simple input/output examples already covered by the shared fixtures. If a unit
  test reads like a fixture, consider moving it into `test-cases/` instead so both languages share
  it.
- When adding new helpers or syntax, ensure the integration tests load the new fixtures and assert
  the behaviour holistically. Use unit tests only to document additional reasoning that the fixtures
  cannot capture.

## Workflow checklist for contributors

1. Modify or add fixtures in `test-cases/` to describe the desired behaviour.
2. Run `just go-sanity` to confirm the Go implementation agrees with the fixtures.
3. Update the Rust integration or unit tests if extra internal coverage is required.
4. Run `just test` or the narrower recipes relevant to your changes before opening a pull request.
5. Update the coverage documentation when functionality coverage changes.

Following these guidelines keeps the Rust and Go implementations aligned and makes the behavioural
contract easy to understand for contributors and reviewers alike.
