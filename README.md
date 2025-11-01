# lithos-gotmpl-rs

Rust-native helpers for working with Go-style templates without cgo. The workspace ships three crates:

- `lithos-gotmpl-engine`: lexer, parser, and evaluator for Go `text/template` syntax.
- `lithos-gotmpl-core`: default helper pack that mirrors the Go built-ins (`eq`, `printf`, `index`, …).
- `lithos-sprig`: a curated slice of Masterminds’ Sprig helpers that layers on top of the core registry.

## Highlights

- Go-compatible pipelines, whitespace trims, control structures, and raw strings.
- Pluggable helper registry with deterministic rendering and template analysis tooling.
- Sprig parity backed by shared fixtures and a Go-driven sanity harness.
- Dual MIT/Apache licensing with compliance docs and CI guardrails out of the box.

## Quick Start

Add the helper crates you need to your `Cargo.toml`:

```toml
[dependencies]
lithos-gotmpl-core = "0.1"
lithos-sprig = "0.1"
serde_json = "1"          # or any other serde-compatible data source
```

`lithos-gotmpl-engine` is re-exported through the core crate—you normally do not depend on it
directly unless you are extending the parser/runtime.

```rust
use lithos_gotmpl_core::{install_text_template_functions, FunctionRegistryBuilder, Template};
use lithos_sprig::install_sprig_functions;

let mut builder = FunctionRegistryBuilder::new();
install_text_template_functions(&mut builder);
install_sprig_functions(&mut builder);

let registry = builder.build();
let tmpl = Template::parse_with_functions(
    "welcome",
    "{{ coalesce .name \"friend\" | title }}",
    registry,
)?;
let rendered = tmpl.render(&serde_json::json!({ "name": null }))?;
assert_eq!(rendered, "Friend");
```

- Run the full example with `cargo run --package lithos-sprig --example flow_and_lists`.
- Explore template analysis via `cargo run --package lithos-gotmpl-core --example analyze`.

## Documentation
When you need a read-only view of helper usage, call `Template::analyze()` and
inspect the returned `TemplateAnalysis`. It reports function invocations,
variable paths, template calls, control structures, and also highlights helper
calls that the current registry does not recognise. Downstream systems can
reason about dynamic dependencies without executing the template, and surface
actionable warnings for missing registrations.

Extended guides live under [`docs/`](docs/index.md). Highlights include:

- [Testing strategy](docs/guides/testing.md) – behavioural fixtures, the Go oracle, and when to add Rust-only assertions.
- [Function coverage](docs/reference/function-coverage.md) – helper matrix covering Go built-ins and Sprig additions.
- [Template syntax coverage](docs/reference/template-syntax-coverage.md) – grammar/introspection status exercised by the suites.
- [Contributor workflow](docs/development/README.md) – environment setup, CI commands, and contributor expectations.
- [Releasing](docs/operations/releasing.md) – release-plz flow and crates.io publishing checklist.
- `docs/legal/` – licence compliance notes and generated reports.

## Compatibility & Caveats
- Behaviour targets the constructs required by downstream Hydros/Lithos tooling; some `text/template` features (`else if`, `define`, dynamic template inclusion) remain unimplemented. Track progress in [`docs/reference/template-syntax-coverage.md`](docs/reference/template-syntax-coverage.md).
- Deterministic map helpers (`keys`, `values`) intentionally diverge from Go’s random iteration order.
- The Go-based `go-sanity` runner is a development aid that mirrors upstream Sprig; install Go 1.25.1+ to enable the compat test suite.

- **Flow control:** `default`, `coalesce`, `ternary`, `empty`, `fail`, and JSON helpers (`fromJson`, `toJson`, plus the `must*` variants).
- **Strings:** case conversion helpers (`snakecase`, `camelcase`, `kebabcase`, `swapcase`), concatenation and quoting (`cat`, `quote`, `squote`), trim/affix helpers, `contains`, `replace`, `substr`, `trunc`, `wrap`, `indent`/`nindent`, `nospace`, and `repeat`.
- **String slices:** `splitList`, `split`, `splitn`, `join`, and `sortAlpha`.
- **Lists:** `list`, `first`/`last`/`rest`/`initial`, `append`/`prepend`/`concat`, `reverse`, `compact`, `uniq`, `without`, `has`, `max`, and `min`.
- **Maps:** `dict`, `merge`, `set`, `unset`, `hasKey`, `keys`, and `values`. Our implementation sorts the outputs of `keys` and `values` to guarantee deterministic results, deviating from Go’s unspecified map iteration order.

## Getting Help

Consult [`SUPPORT.md`](SUPPORT.md) for issue-reporting and triage guidance.
Development conventions live in [`docs/development/README.md`](docs/development/README.md), and release steps are documented under [`docs/operations/releasing.md`](docs/operations/releasing.md).

## Licence
Dual-licensed under Apache-2.0 or MIT. Refer to [`LICENSE-APACHE`](LICENSE-APACHE), [`LICENSE-MIT`](LICENSE-MIT), and [`NOTICE`](NOTICE) for details.

The project is not affiliated with or endorsed by the Go project or by the Masterminds/sprig maintainers. Compatibility tests rely on their public implementations for reference only, and the `go-sanity` helper should be treated as a verification aid rather than a runtime dependency.

## Roadmap

- Flesh out additional helpers based on the needs of downstream consumers. Each new helper should add test cases and assertions to keep the Go sanity checks in sync.
- Explore publishing the helper registry as a trait or adapter.
- Eventually wire in property tests for string/collection helpers once coverage expands.

## Attribution & Notices

This workspace includes test materials derived from the Go programming language (`text/template`) and uses Masterminds' sprig library inside the `go-sanity` tool. Full attribution details are recorded in [`NOTICE`](NOTICE).
