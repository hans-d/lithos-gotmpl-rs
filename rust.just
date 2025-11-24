fuzz_runs := "1000"
fuzz_time := ""
fuzz_constraint := if fuzz_time != "" { "-max_total_time=" + fuzz_time } else { "-runs=" + fuzz_runs }

default:
    just --list

test:
    cargo test --workspace

test-deep: test-mutations test-fuzz

test-mutations:
    cargo mutants --workspace --timeout 120 --output mutants.json

ensure-nightly:
    if ! rustup toolchain list | grep -q '^nightly'; then \
        echo "Rust nightly toolchain is required. Install via \`rustup toolchain install nightly\`." >&2; \
        exit 1; \
    fi

test-fuzz: test-fuzz-parse test-fuzz-render

test-fuzz-parse: ensure-nightly
    cargo +nightly fuzz run template_parse -- {{fuzz_constraint}}

test-fuzz-render: ensure-nightly
    cargo +nightly fuzz run template_render -- {{fuzz_constraint}}

deps:
    # vulnerabilities
    cargo audit --deny warnings
    # licenses and vulnerabilities
    cargo deny check

lint-unsafe:
    cargo geiger --manifest-path "./crates/lithos-gotmpl-engine/Cargo.toml" --lib --forbid-only --output-format GitHubMarkdown
    cargo geiger --manifest-path "./crates/lithos-gotmpl-core/Cargo.toml" --lib --forbid-only --output-format GitHubMarkdown
    cargo geiger --manifest-path "./crates/lithos-sprig/Cargo.toml" --lib --forbid-only --output-format GitHubMarkdown

lint-basic:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features

lint: lint-basic lint-unsafe

licenses:
    mkdir -p target/legal
    cargo about generate \
        --workspace \
        --format json \
        --fail \
        --output-file target/legal/cargo-about.json

test-behavior:
    just rust test-sprig-compat
    just rust test-behavior-properties
    just rust test-behavior-contracts

test-sprig-compat:
    cargo test --package lithos-sprig --test compat

test-behavior-properties:
    if find . -path "./target" -prune -o -name behavior_properties.rs -print -quit | grep -q .; then \
        PROPTEST_CASES="${PROPTEST_CASES:-10000}" cargo test --test behavior_properties; \
    else \
        echo "Skipping behavior_properties (no tests found)"; \
    fi

test-behavior-contracts:
    if find . -path "./target" -prune -o -name behavior_contracts.rs -print -quit | grep -q .; then \
        cargo test --test behavior_contracts; \
    else \
        echo "Skipping behavior_contracts (no tests found)"; \
    fi

ci-pr-run: lint test
    just rust test-behavior

ci-pr: ci-pr-run
ci-mutation: test-mutations
ci-fuzz-render: test-fuzz-render
ci-fuzz-parse: test-fuzz-parse
