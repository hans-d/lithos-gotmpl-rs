set shell := ["bash", "-cu"]
go_sanity_dir := "go-sanity"
cases_path := "test-cases/lithos-sprig.json"

# Core CI commands -----------------------------------------------------------

ci-test:
    cargo test --workspace

ci-behavior:
    cargo test --package lithos-sprig --test compat
    if find . -path "./target" -prune -o -name behavior_properties.rs -print -quit | grep -q .; then \
        PROPTEST_CASES="${PROPTEST_CASES:-10000}" cargo test --test behavior_properties; \
    else \
        echo "Skipping behavior_properties (no tests found)"; \
    fi
    if find . -path "./target" -prune -o -name behavior_contracts.rs -print -quit | grep -q .; then \
        cargo test --test behavior_contracts; \
    else \
        echo "Skipping behavior_contracts (no tests found)"; \
    fi

ci-quality: ensure-yamllint
    export PATH="$(go env GOPATH)/bin:$HOME/.local/bin:$PATH"
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features
    if command -v actionlint >/dev/null 2>&1; then \
        actionlint; \
    elif [ -x "$(go env GOPATH)/bin/actionlint" ]; then \
        "$(go env GOPATH)/bin/actionlint"; \
    else \
        echo "actionlint not found. Run \`just install-actionlint\` or ensure it is on PATH." >&2; \
        exit 1; \
    fi
    yamllint -c .yamllint.yaml .

ci-security: ensure-cargo-audit ensure-cargo-deny
    cargo audit --deny warnings
    cargo deny check

ci-mutation: ensure-cargo-mutants
    cargo mutants --workspace --timeout 120 --output mutants.json

ci-fuzz: ensure-cargo-fuzz
    if ! rustup toolchain list | grep -q '^nightly'; then \
        echo "Rust nightly toolchain is required. Install via \`rustup toolchain install nightly\`." >&2; \
        exit 1; \
    fi
    cargo +nightly fuzz run template_parse -- -runs=1000
    cargo +nightly fuzz run template_render -- -runs=1000

ci-release: ensure-release-plz
    release-plz release --dry-run

# Developer shortcuts --------------------------------------------------------

# Run the Rust unit and integration suite.
test: ci-test

# Execute the Go sanity runner to cross-check the sprig test cases.
go-sanity:
    cd {{go_sanity_dir}} && go run . -cases "$(git rev-parse --show-toplevel)/{{cases_path}}"

# Full validation: Go sanity runner + Rust tests.
verify: go-sanity ci-test

# Generate branch coverage using cargo-tarpaulin (requires `cargo install cargo-tarpaulin`).
coverage: ensure-cargo-tarpaulin
    cargo tarpaulin --workspace --all-features --engine llvm --out Html

mutation: ci-mutation
fuzz: ci-fuzz
release: ci-release

install-ci-tools: install-cargo-audit install-cargo-deny install-cargo-tarpaulin install-cargo-mutants install-cargo-fuzz install-actionlint install-yamllint install-release-plz

ensure-cargo-audit:
    if ! command -v cargo-audit >/dev/null 2>&1; then \
        echo "cargo-audit not installed. Run \`just install-cargo-audit\` or \`cargo install --locked cargo-audit\`." >&2; \
        exit 1; \
    fi

ensure-cargo-deny:
    if ! command -v cargo-deny >/dev/null 2>&1; then \
        echo "cargo-deny not installed. Run \`just install-cargo-deny\` or \`cargo install --locked cargo-deny\`." >&2; \
        exit 1; \
    fi

ensure-cargo-tarpaulin:
    if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
        echo "cargo-tarpaulin not installed. Run \`just install-cargo-tarpaulin\` or \`cargo install --locked cargo-tarpaulin\`." >&2; \
        exit 1; \
    fi

ensure-cargo-mutants:
    if ! command -v cargo-mutants >/dev/null 2>&1; then \
        echo "cargo-mutants not installed. Run \`just install-cargo-mutants\` or \`cargo install --locked cargo-mutants\`." >&2; \
        exit 1; \
    fi

ensure-cargo-fuzz:
    if ! command -v cargo-fuzz >/dev/null 2>&1; then \
        echo "cargo-fuzz not installed. Run \`just install-cargo-fuzz\` or \`cargo install cargo-fuzz\`." >&2; \
        exit 1; \
    fi

ensure-yamllint:
    if ! command -v yamllint >/dev/null 2>&1; then \
        echo "yamllint not installed. Run \`just install-yamllint\` or \`pip install --user yamllint\`." >&2; \
        exit 1; \
    fi

ensure-release-plz:
    if ! command -v release-plz >/dev/null 2>&1; then \
        echo "release-plz not installed. Run `just install-release-plz` or `cargo install release-plz`." >&2; \
        exit 1; \
    fi

install-cargo-audit:
    if ! command -v cargo-audit >/dev/null 2>&1; then \
        cargo install --locked cargo-audit; \
    else \
        echo "cargo-audit already installed"; \
    fi

install-cargo-deny:
    if ! command -v cargo-deny >/dev/null 2>&1; then \
        cargo install --locked cargo-deny; \
    else \
        echo "cargo-deny already installed"; \
    fi

install-cargo-tarpaulin:
    if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
        cargo install --locked cargo-tarpaulin; \
    else \
        echo "cargo-tarpaulin already installed"; \
    fi

install-cargo-mutants:
    if ! command -v cargo-mutants >/dev/null 2>&1; then \
        cargo install --locked cargo-mutants; \
    else \
        echo "cargo-mutants already installed"; \
    fi

install-cargo-fuzz:
    if ! command -v cargo-fuzz >/dev/null 2>&1; then \
        cargo install cargo-fuzz; \
    else \
        echo "cargo-fuzz already installed"; \
    fi

install-actionlint:
    if ! command -v actionlint >/dev/null 2>&1; then \
        go install github.com/rhysd/actionlint/cmd/actionlint@latest; \
    else \
        echo "actionlint already installed"; \
    fi

install-yamllint:
    if ! command -v yamllint >/dev/null 2>&1; then \
        pip install --user yamllint; \
    else \
        echo "yamllint already installed"; \
    fi

install-release-plz:
    if ! command -v release-plz >/dev/null 2>&1; then \
        cargo install release-plz; \
    else \
        echo "release-plz already installed"; \
    fi
