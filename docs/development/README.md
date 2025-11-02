# Development Guide

## Tooling Options

- **Dev Container** – build the container defined in `.devcontainer/` and execute commands with `devcontainer up` / `devcontainer exec`.
- **Native** – install Rust (stable + nightly if you fuzz), Go 1.25.1+, Python 3.12, and run `just install-ci-tools` to fetch cargo extras (`cargo-audit`, `cargo-deny`, `cargo-fuzz`, etc.).

## Everyday Commands

```
just test          # Rust unit + integration tests
just go-sanity     # Run Go-backed compatibility suite
just verify        # test + go-sanity together
just ci-test       # mirrors the CI test matrix
just ci-behavior   # compat + property/contract suites
just ci-quality    # fmt, clippy, yamllint, actionlint
just ci-security   # cargo-audit + cargo-deny
just ci-fuzz       # short fuzz runs (nightly toolchain)
just ci-mutation   # cargo-mutants sweep
just ci-legal      # lightweight licence checks
just ci-legal-full # full licence report/archive generation
just ci-release    # release-plz dry run
```

Individual installers exist (`just install-cargo-audit`, `just install-actionlint`, etc.) when you need a single tool.

## Workflow Notes

- Compatibility tests cache Go builds under `target/go-cache`; override `GOCACHE` if your environment requires a different path.
- Renovate, Sonar, CodeQL, scorecard, dependency-review, and security scanners run in GitHub Actions—see `.github/workflows/` for specifics.
- Release management is tracked in [`docs/operations/releasing.md`](../operations/releasing.md); follow that guide before running `just release`.

## Adding a Sprig Helper (example flow)

1. **Pick the function name and behaviour**  
   Use the upstream Sprig docs as the canonical reference. Decide whether the helper belongs in an existing module under `crates/lithos-sprig/src/functions/` or needs a new module.

2. **Implement the helper**  
   - Add the Rust implementation in the appropriate module (or create a new one and wire it into `functions/mod.rs`).  
   - If the helper needs shared utilities, prefer extending the existing helper traits instead of reintroducing bespoke plumbing.

3. **Register the helper**  
   Update the relevant `install_*` function so the helper is inserted into the registry when `install_sprig_functions` is called. The new helper should be available via the default builder path used by the examples.

4. **Add test coverage**  
   - **Unit/integration tests:** extend `crates/lithos-sprig/src/functions/...` tests to cover success and failure modes.  
   - **Go oracle fixture:** add an entry to `test-cases/lithos-sprig.json` or create a dedicated folder under `test-cases/sprig/` with `input.tmpl`, `input.json`, and `expected.txt`.  
   - Run `just go-sanity` (or `just ci-behavior`) to confirm the Go parity harness agrees with the new helper.

5. **Update documentation**  
   - Append the helper to [`docs/reference/function-coverage.md`](../reference/function-coverage.md).  
   - If template syntax support changes (e.g., new pipeline forms), update [`docs/reference/template-syntax-coverage.md`](../reference/template-syntax-coverage.md).

6. **Run checks**  
   Execute `just ci-test`, `just ci-behavior`, and `just ci-quality` before opening the PR. Depending on the helper, `just ci-security` may also be appropriate.

### Missing coverage detection

The `crates/lithos-sprig/tests/coverage.rs` suite enforces that every helper registered by `install_sprig_functions` appears in at least one fixture under `test-cases/`. If you add a helper without a corresponding fixture, this test fails with a list of missing function names—add a JSON or directory fixture before sending the PR.

Questions on process or environment setup? Open an issue or consult `AGENT.md`.
