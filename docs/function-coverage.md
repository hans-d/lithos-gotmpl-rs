# Go Template & Sprig Coverage

## Reference Implementations

- Go text/template version: the `go-sanity` runner uses Go 1.25.1 (see `go-sanity/go.mod`).
- Documentation baseline: Go 1.25.2 `text/template` package reference for the predefined helper list.
- Sprig reference: github.com/Masterminds/sprig/v3 pinned at v3.3.0 for behaviour expectations.

## Go `text/template` Predefined Helpers

Current implementation status for the helpers described in the Go 1.25.2 documentation:

| Function | Purpose (summary) | Implemented? | Notes |
|----------|-------------------|--------------|-------|
| `and` | short-circuits to the first empty argument, else returns the last | ✅ | Part of `install_text_template_functions` |
| `call` | Invoke a function-valued argument with parameters | ✅ | Accepts the registry function name as string |
| `html` | Escape for HTML contexts | ✅ | Implemented via `escape_html` helper |
| `index` | Retrieve element by key or index from map/slice | ✅ | Returns `Null` for missing entries |
| `js` | Escape for JavaScript string literal | ✅ | JSON escaping with additional `<`, `>`, `&`, `'`, `="` patches |
| `len` | Length of map/slice/string | ✅ | Handles strings, arrays, objects (maps) |
| `not` | Boolean negation | ✅ | Delegates to `is_truthy` |
| `or` | Returns first truthy argument | ✅ | Part of core registry |
| `print` | Concatenate arguments | ✅ | Mirrors Go’s `fmt.Sprint` semantics |
| `printf` | Format according to a format string | ✅ | Already available |
| `println` | Concatenate with spaces and trailing newline | ✅ | Mirrors Go’s `fmt.Sprintln` semantics |
| `slice` | Construct subslice with optional indices | ✅ | Supports up to two indices for strings/arrays (indices must align to UTF-8 boundaries) |
| `urlquery` | URL-encode with query semantics | ✅ | Percent-encode with space-to-`+` conversion |
| `eq`/`ne`/`lt`/`le`/`gt`/`ge` | Comparison operators | ✅ | Numbers and strings supported |

## Sprig Helpers Implemented

Grouped following the upstream Sprig documentation.

### Flow Control

| Function | Purpose (summary) | Status |
|----------|-------------------|--------|
| `default` | Uses the first non-empty value from arguments | ✅ |
| `coalesce` | Returns the first non-empty argument | ✅ |
| `ternary` | Picks between two branches based on a condition | ✅ |
| `empty` | Tests whether a value is empty | ✅ |
| `fail` | Aborts template execution with an error | ✅ |
| `fromJson` / `mustFromJson` | Parse JSON strings into template values | ✅ |
| `toJson` / `mustToJson` | Serialise values to compact JSON strings | ✅ |
| `toPrettyJson` / `mustToPrettyJson` | Serialise values to pretty JSON | ✅ |
| `toRawJson` / `mustToRawJson` | Serialise values without additional escaping | ✅ |

### Strings

| Function | Purpose (summary) | Status |
|----------|-------------------|--------|
| `upper` / `lower` / `title` | Case conversions | ✅ |
| `trim` / `trimAll` / `trimPrefix` / `trimSuffix` | Strip whitespace or custom affixes | ✅ |
| `hasPrefix` / `hasSuffix` | Prefix/suffix checks | ✅ |
| `contains` | Tests if substring appears in a string | ✅ |
| `replace` | Replace substrings (optionally limited) | ✅ |
| `substr` | Extract substring by start/length | ✅ |
| `trunc` | Truncate strings with optional ellipsis | ✅ |
| `wrap` | Wrap text at a fixed width | ✅ |
| `indent` / `nindent` | Indent multi-line strings | ✅ |
| `nospace` | Remove all whitespace | ✅ |
| `repeat` | Repeats a string `count` times | ✅ |
| `cat` | Concatenate arguments with spaces | ✅ |
| `quote` / `squote` | Wrap values in double or single quotes | ✅ |
| `snakecase` / `camelcase` / `kebabcase` / `swapcase` | Convert between naming conventions | ✅ |

### String Slice Helpers

| Function | Purpose (summary) | Status |
|----------|-------------------|--------|
| `splitList` / `splitn` | Split strings into slices | ✅ |
| `split` | Split string into a map keyed by position | ✅ |
| `join` | Join slice elements with a separator | ✅ |
| `sortAlpha` | Sort slices of strings alphabetically | ✅ |

### Lists

| Function | Purpose (summary) | Status |
|----------|-------------------|--------|
| `list` | Construct a slice from arguments | ✅ |
| `first` / `last` / `rest` / `initial` | Pick leading/trailing elements | ✅ |
| `append` / `prepend` / `concat` | Add or merge slices | ✅ |
| `reverse` | Reverse slice order | ✅ |
| `compact` | Remove empty values | ✅ |
| `uniq` | Deduplicate while preserving order | ✅ |
| `without` | Remove specified values | ✅ |
| `has` | Test membership | ✅ |

### Maps

| Function | Purpose (summary) | Status |
|----------|-------------------|--------|
| `dict` / `merge` | Build or combine maps from key/value pairs | ✅ |
| `set` / `unset` | Add or remove keys from a map | ✅ |
| `hasKey` | Check whether a key exists | ✅ |
| `keys` / `values` | Return map keys or underlying values (sorted deterministically) | ✅ |

### Sprig Helpers Not Yet Ported

Refer to the [Sprig function index](https://masterminds.github.io/sprig/) for the full catalog; any helper not listed above is pending evaluation against the v3.3.0 behaviour.
