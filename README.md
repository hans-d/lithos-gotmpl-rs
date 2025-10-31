# lithos-gotmpl-rs

A Rust implementation for go template and related libraries. The core surface is split into:

- `lithos-gotmpl-engine`: hand-rolled lexer/parser/runtime that evaluates Go
  text/template syntax. Apps inject function registries when building templates.
- `lithos-gotmpl-core`: default helper pack that mirrors the stock Go
  text/template built-ins (e.g. `eq`, `printf`, `index`, `default`).
- `lithos-sprig`: a Rust-friendly slice of the [`sprig`](https://github.com/Masterminds/sprig)
  function library so [`gitmpl`](https://github.com/hans-d/gittmpl) and similar engines can
  avoid cgo while staying behaviourally aligned with upstream Sprig.

## Project layout

- `crates/lithos-gotmpl-engine/src/` – core parser/evaluator. `Template` instances accept a
  `FunctionRegistry` and execute pipelines by delegating to registered helpers. Action nodes
  expose parsed `Pipeline`/`Command` structures, so constructs like `{{ .name | default "foo" | upper }}`
  surface as three distinct commands in the AST (field lookup, `default`, then `upper`). Branching
  forms such as `if`/`range`/`with` materialise as dedicated AST nodes with nested blocks, matching
  the shape of Go's `parse` package. The engine honours Go-style whitespace trim markers (`{{-` / `-}}`)
  and pipeline variable bindings like `{{$val := .item}}` with lexical scoping across control structures.
- `crates/lithos-gotmpl-core/src/` – base helper bundle that registers the canonical
  Go text/template built-ins atop the engine.
- `crates/lithos-sprig/src/` – sprig-style helpers layered on top of the engine/core crates.
- `test-cases/lithos-sprig.json` – shared test vectors describing calls that mirror
  Go sprig behaviour. Each entry captures both function arguments and a template snippet with
  its expected rendering.
- `test-cases/lithos-gotmpl-core.json` – vectors harvested from Go's
  `text/template` tests to validate our built-in helper semantics.
- `go-sanity/` – shared Go runner used to sanity-check test cases against the real Go implementation.
- `crates/lithos-sprig/tests/compat.rs` – integration test that drives the go-sanity runner and asserts the Rust
  implementations stay in lock-step.
- `justfile` – light CLI automation for common tasks.

## Development

You can work either inside the provided dev container or with a native toolchain:

- **Dev container** – use the [Dev Containers specification](https://containers.dev/). For example,
  with the `devcontainer` CLI:
  ```bash
  devcontainer up --workspace-folder .
  devcontainer exec --workspace-folder . just ci-test
  ```
  Editors such as VS Code or GitHub Codespaces can also consume the same definition. The container
  installs Rust (stable + nightly), Go 1.25.1, Python 3.12, and all required CLI tools via
  `.devcontainer/postCreate.sh`.
- **Native setup** – install the prerequisites manually and run `just install-ci-tools` to pull in
  the cargo utilities and linters we use in CI.

```
just go-sanity  # runs the shared Go sanity harness against sprig test cases
just test       # runs the Rust unit + integration tests
just verify     # executes both
just ci-test    # matches the GitHub Actions test job locally
just ci-behavior  # runs compat + optional property/contract suites
just ci-quality # fmt + clippy gate
just ci-security # cargo audit / cargo deny (requires tools installed)
just ci-mutation # cargo-mutants sweep (requires `cargo install cargo-mutants`)
just mutation   # alias for ci-mutation
just ci-fuzz    # smoke-tests the fuzz harnesses (requires nightly + cargo-fuzz)
just fuzz       # alias for ci-fuzz
just ci-release  # dry-run release-plz workflow
just release    # alias for ci-release
just install-ci-tools # installs cargo-audit, cargo-deny, cargo-tarpaulin, cargo-mutants, cargo-fuzz, actionlint, yamllint, release-plz
just install-cargo-audit|deny|tarpaulin|cargo-mutants|cargo-fuzz|actionlint|yamllint|release-plz # install individual extras
```

The integration test will skip itself gracefully if Go is missing. Keep the
cases in `test-cases/lithos-sprig.json` aligned with the functions implemented in Rust
for deterministic comparisons. When scenarios get more complex, add subdirectories under
`test-cases/` containing `input.tmpl`, `input.json`, and `expected.*` assets that both the
Rust harness and Go oracle can consume (see `test-cases/sprig/nested-default/` for an
example covering nested pipelines).

**Rust toolchain:** the workspace targets the 2021 edition. The minimum supported
Rust version (MSRV) is **1.70.0**; CI should exercise the latest stable release as
well as MSRV once it is wired up.

## Automation

- **GitHub Actions CI** (`.github/workflows/ci.yml`) fan-outs into dedicated jobs: the core unit
  test suite, behavioural validation (including the Go-backed compatibility harness), code quality
  (fmt, Clippy, actionlint, yamllint, optional Sonar scan), and security scans (`cargo audit` +
  `cargo deny`).
- **Deep Testing** (`.github/workflows/deep-testing.yml`) is scheduled weekly to run the fuzz
  harnesses (requires the Rust nightly toolchain) and, on the first of each month, mutation tests.
- **SonarQube / SonarCloud** (`.github/workflows/sonar.yml`) is ready to analyse the workspace.
  Provide a `SONAR_TOKEN` repository secret and, if needed, `SONAR_HOST_URL` before enabling the
  workflow; adjust `sonar-project.properties` with your actual organisation/project keys.
- **Renovate Bot** (`renovate.json`) is configured to group weekly Rust and Go dependency updates.
  Install the Renovate GitHub App (or hook through GitHub Actions) to activate automated PRs.
- **CodeRabbit** (`.coderabbit.yaml`) supplies default review hints for pull requests. Once the
  CodeRabbit GitHub App is installed, it will auto-review changes that touch the Rust crates or
  the Go oracle code.

## Using the helper crates

A typical integration wires the engine, default Go helpers, and Sprig helpers
together before parsing templates:

```rust
use lithos_gotmpl_core::{install_text_template_functions, Template};
use lithos_sprig::install_sprig_functions;

let mut builder = lithos_gotmpl_core::FunctionRegistryBuilder::new();
install_text_template_functions(&mut builder);      // Go defaults (`eq`, `printf`, ...)
install_sprig_functions(&mut builder);              // Sprig helpers (flow, strings, lists)

let registry = builder.build();
let tmpl = Template::parse_with_functions(
    "example",
    "{{ coalesce .name \"friend\" | title }}",
    registry,
)?;

let rendered = tmpl.render(&serde_json::json!({"name": null}))?;
assert_eq!(rendered, "Friend");
```

Run the ready-made example to see a fuller template in action:

```
cargo run --example flow_and_lists --package lithos-sprig
```

When you need a read-only view of helper usage, call `Template::analyze()` and
inspect the returned `TemplateAnalysis`. It reports function invocations,
variable paths, template calls, and control structures, allowing downstream
systems to reason about dynamic helper requirements without executing the
template.

```bash
cargo run --example analyze --package lithos-gotmpl-core
```

The Sprig layer is organised the same way as the upstream documentation:

- **Flow control:** `default`, `coalesce`, `ternary`, `empty`, `fail`, and JSON
  helpers (`fromJson`, `toJson`, plus the `must*` variants).
- **Strings:** case conversion, trim/affix helpers, `contains`, `replace`,
  `substr`, `trunc`, `wrap`, `indent`/`nindent`, `nospace`, and `repeat`.
- **String slices:** `splitList`, `split`, `splitn`, `join`, and `sortAlpha`.
- **Lists:** `list`, `first`/`last`/`rest`/`initial`, `append`/`prepend`/`concat`,
  `reverse`, `compact`, `uniq`, `without`, and `has`.
- **Maps:** `dict`, `merge`, `set`, `unset`, `hasKey`, `keys`, and `values`. Our
  implementation sorts the outputs of `keys` and `values` to guarantee
  deterministic results, deviating from Go’s unspecified map iteration order.

We intentionally omit Sprig helpers that rely on randomness, regular
expressions, or pluralisation until downstream demand materialises.

See `docs/function-coverage.md` for the full helper matrix, including the Go
defaults handled by `lithos-gotmpl-core`.

## Compatibility & Limitations

- Behaviour is intentionally focused on the subset of Go templates that our
  downstream tooling depends on. We do **not** support every construct that
  `text/template` provides (for example `else if`, `define`/`template`, or
  keyword helpers such as `break`). See `docs/template-syntax-coverage.md` for
  the current status.
- The project is not affiliated with or endorsed by the Go project or by the
  Masterminds/sprig maintainers. Compatibility tests rely on their public
  implementations for reference only.
- The Go-based `go-sanity` runner is an optional development aid. It requires a
  local Go toolchain and honours the upstream sprig licence. CI and local
  workflows should treat it as a verification step rather than a runtime
  dependency of the crates.

## Roadmap

- Flesh out additional helpers based on the needs of downstream consumers. Each
  new helper should add test cases and assertions to keep the Go sanity checks in sync.
- Explore publishing the helper registry as a trait or adapter
- Eventually wire in property tests for string/collection helpers once coverage
  expands.

## Attribution & Notices

This workspace includes test materials derived from the Go programming language
(`text/template`) and uses Masterminds' sprig library inside the `go-sanity`
tool. Full attribution details are recorded in [`NOTICE`](NOTICE).

## License

All crates in this workspace are dual-licensed under either the Apache License
2.0 or the MIT License. You may choose either licence to use the code. Copies of
both licences are provided in [`LICENSE-APACHE`](LICENSE-APACHE) and
[`LICENSE-MIT`](LICENSE-MIT). Unless otherwise stated, contributions are
accepted under the same dual-licence terms.
