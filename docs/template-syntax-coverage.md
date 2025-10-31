# Template Syntax Coverage

Status legend: ✅ implemented and covered by test cases; ⚠️ implemented but missing explicit
coverage; ❌ not implemented yet.

## Actions & Pipelines

| Feature | Status | Tests / Fixtures | Notes |
| --- | --- | --- | --- |
| Basic pipeline chaining (`{{ .x \| printf }}`) | ✅ | `parser::tests::parses_pipeline_into_individual_commands`, `tests::renders_with_custom_registry` | Commands preserved in AST and executed in order. |
| Leading variable declaration (`{{ $v := .x }}`) | ✅ | `tests::variable_binding_inside_if` | Stored in `PipelineDeclarations` and bound via runtime. |
| Assignment (`$v = ...`) to existing vars | ✅ | `tests::assignment_updates_existing_variable` | Parser accepts reassignment and runtime updates the binding. |
| Multiple declaration (`{{ range $i, $v := ... }}`) | ✅ | `tests::range_assigns_iteration_variables` | Covers key/value binding during range iteration. |
| Parenthesised pipeline expressions (`(.x | ... )`) | ✅ | `tests::pipeline_expression_inside_if`, `test-cases/lithos-sprig.json` (`default-with-nested-pipeline`) | Nested pipelines inside expressions evaluate correctly. |
| Else-if (`{{else if .cond}}`) | ❌ | — | Parser currently errors with "else-if is not yet supported". |

## Control Structures

| Feature | Status | Tests / Fixtures | Notes |
| --- | --- | --- | --- |
| `if` blocks with bindings | ✅ | `tests::renders_if_else_branches`, `tests::variable_binding_inside_if` | Bindings respected within then/else scopes. |
| `range` over arrays/maps | ✅ | `tests::renders_range_over_arrays`, `tests::range_assigns_iteration_variables` | Also records key/index variables. |
| `with` scopes | ✅ | `tests::renders_with_changes_context` | Pushes/pops scope correctly. |
| Template/block/define nodes | ❌ | — | Not parsed yet; analyzer only records potential template calls via identifiers. |

## Whitespace & Comments

| Feature | Status | Tests / Fixtures | Notes |
| --- | --- | --- | --- |
| Trim markers (`{{- ... }}`, `{{ ... -}}`) | ✅ | `tests::trims_whitespace_around_actions` | Removes surrounding whitespace on both sides. |
| Comments (`{{/* ... */}}`) pass-through | ✅ | `tests::comment_trimming_matches_go` | Trim markers around comments mirror Go's behaviour. |
| Standalone comment as whitespace | ✅ | `tests::comment_only_renders_empty_string` | Comment-only templates render as empty output. |

## Variables & Scoping

| Feature | Status | Tests / Fixtures | Notes |
| --- | --- | --- | --- |
| Root variable `$` auto-binding | ✅ | `tests::root_variable_resolves_to_input` | `$` resolves to the original root data. |
| Nested scope shadowing | ✅ | `tests::nested_scope_shadowing_preserves_outer` | Inner scopes rebind variables without mutating the outer binding. |
| Assignment error when variable unknown | ✅ | `tests::assignment_to_unknown_variable_fails` | Runtime raises when assigning to undeclared variable. |

## Keywords & Function Checks

| Feature | Status | Tests / Fixtures | Notes |
| --- | --- | --- | --- |
| Keyword/function collision (`break` example) | ❌ | — | Requires keyword-aware registry; currently not implemented. |
| Skip function existence validation | ❌ | — | Parser always validates via registry. Consider opt-in flag akin to Go's `SkipFuncCheck`. |

## Diagnostics

| Feature | Status | Tests / Fixtures | Notes |
| --- | --- | --- | --- |
| Unclosed action reporting | ✅ | `tests::parse_error_on_unclosed_action` | Mirrors Go's error message structure. |
| Comment parsing errors | ✅ | `parser::tests::parse_error_on_unclosed_comment` | Unterminated comments emit explicit parse errors. |
| Precise span tracking | ✅ | `parser::tests::spans_cover_action_body` | Action spans fully cover trimmed bodies. |

## Documentation & Future Work

- Once TODOs above are covered, mark the corresponding entries as ✅ along with the test name.
- Additional Go references worth porting are catalogued in `docs/go-parser-test-candidates.md`.
- We can leverage `cargo doc` (with module-level `//!` docs) to surface these tables automatically on
  docs.rs or internal pages. Consider embedding this table inside a module doc so `cargo doc -p
  lithos-gotmpl-engine` renders the coverage report.
