# Go text/template Parser Test Candidates

The upstream Go parser (`src/text/template/parse`) already exercises a broad matrix of syntactic
corner cases. Below is a curated short list of scenarios that map directly to the features we just
implemented (whitespace trimming, pipeline declarations, variable scoping, keyword collisions). These
should be prime candidates for porting into Rust unit/test-case suites.

## Whitespace & Comment Trimming

| Go Test Name | Source Location | Notes |
| --- | --- | --- |
| `TestParseWithComments/comment trim left` | `parse/parse_test.go:391` | Ensures `{{- /* ... */}}` removes preceding whitespace. |
| `TestParseWithComments/comment trim right` | `parse/parse_test.go:391` | Confirms `{{/* ... */ -}}` strips whitespace following the comment. |
| `TestParseWithComments/comment trim left and right` | `parse/parse_test.go:391` | Both leading and trailing trims; good regression for our `trim_trailing_whitespace`. |

## Pipeline Declarations & Variable Scope

| Scenario | Source | Why It Matters |
| --- | --- | --- |
| `{{$x := .Y}}` inside control flow | `parse/parse_test.go` table cases | Validates parsing + scoping for single-variable declarations. |
| `{{range $i, $v := .List}}` | `parse/parse_test.go` table cases | Covers multi-variable declarations within range loops. |
| Assignment (`$x = ...`) vs declare (`$x := ...`) errors | `parse/parse_test.go:errorTests` | Ensures we reject illegal assignments or missing operands the same way Go does. |

## Keyword / Function Collisions

| Scenario | Source | Why It Matters |
| --- | --- | --- |
| `TestKeywordsAndFuncs` | `parse/parse_test.go:410` | Confirms keywords like `break` are treated as functions when supplied. Useful once we expose keyword hooks. |
| `TestSkipFuncCheck` | `parse/parse_test.go:431` | Ensures parser can skip function existence checks—good to mimic optional validation flags. |

## Structural Helpers

| Scenario | Source | Why It Matters |
| --- | --- | --- |
| `TestIsEmpty` cases | `parse/parse_test.go:454` | Exercises detection of empty trees, comments-only templates, etc.—helps validate our analysis helpers. |
| Error line/column tracking (`errorTests`) | `parse/parse_test.go:482` | Data set of expected parse errors + messages to align error reporting.

### Next Steps

1. Port the comment trim cases into `lithos-gotmpl-engine` unit tests (they directly target our new
   whitespace logic).
2. Translate at least one pipeline declaration case (e.g. `{{if $x := .value}}`) into a fixture to
   confirm bindings survive analysis + rendering.
3. Evaluate whether we need a mode flag similar to Go’s `SkipFuncCheck` before porting those tests.

This document is intentionally short; we can expand it with more upstream references as we broaden
coverage.
