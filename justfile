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

ci-quality: ensure-yamllint ensure-cargo-geiger ensure-golangci-lint
    export PATH="$(go env GOPATH)/bin:$HOME/.local/bin:$PATH"
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --all-features
    GEIGER_NO_COLOR=1 cargo geiger --manifest-path "$(pwd)/crates/lithos-gotmpl-engine/Cargo.toml" --lib --forbid-only --output-format GitHubMarkdown
    GEIGER_NO_COLOR=1 cargo geiger --manifest-path "$(pwd)/crates/lithos-gotmpl-core/Cargo.toml" --lib --forbid-only --output-format GitHubMarkdown
    GEIGER_NO_COLOR=1 cargo geiger --manifest-path "$(pwd)/crates/lithos-sprig/Cargo.toml" --lib --forbid-only --output-format GitHubMarkdown
    (cd {{go_sanity_dir}} && golangci-lint run ./...)
    if command -v actionlint >/dev/null 2>&1; then \
        actionlint; \
    elif [ -x "$(go env GOPATH)/bin/actionlint" ]; then \
        "$(go env GOPATH)/bin/actionlint"; \
    else \
        echo "actionlint not found. Run \`just install-actionlint\` or ensure it is on PATH." >&2; \
        exit 1; \
    fi
    yamllint -c .yamllint.yaml .

ci-security: ensure-cargo-audit
    cargo audit --deny warnings

ci-legal: ensure-cargo-deny ensure-go-licenses
    cargo deny check
    mkdir -p target/go-cache
    (cd go-sanity && GOCACHE="$(pwd)/../target/go-cache" go-licenses check ./... --allowed_licenses MIT,Apache-2.0,BSD-2-Clause,BSD-3-Clause,Unicode-DFS-2016,Unicode-3.0)

ci-legal-full: ensure-cargo-deny ensure-cargo-about ensure-go-licenses
    cargo deny check
    mkdir -p target/legal target/go-cache
    cargo about generate --workspace --format json --fail --output-file target/legal/cargo-about.json
    (cd go-sanity && GOCACHE="$(pwd)/../target/go-cache" go-licenses check ./... --allowed_licenses MIT,Apache-2.0,BSD-2-Clause,BSD-3-Clause,Unicode-DFS-2016,Unicode-3.0)
    (cd go-sanity && GOCACHE="$(pwd)/../target/go-cache" go-licenses report .) > target/legal/go-licenses.csv
    rm -rf target/legal/go-licenses
    mkdir -p target/legal/go-licenses
    (cd go-sanity && GOCACHE="$(pwd)/../target/go-cache" go-licenses save ./... --save_path ../target/legal/go-licenses --force)
    python3 scripts/check_licenses.py
    python3 scripts/check_notice.py

ci-osv: ensure-osv-scanner
    mkdir -p target
    osv-scanner --recursive --output=target/osv-report.json .
    @echo "OSV report written to target/osv-report.json"

ci-dependencies:
    just ci-security
    just ci-legal

ci-dependencies-full:
    just ci-legal-full

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

install-ci-tools: install-cargo-audit install-cargo-deny install-cargo-about install-go-licenses install-cargo-tarpaulin install-cargo-mutants install-cargo-fuzz install-actionlint install-yamllint install-release-plz
    @echo "Installed core CI tools"

ensure-syft:
    @if ! command -v syft >/dev/null 2>&1; then \
        echo "syft not installed. Install it from https://github.com/anchore/syft#installation." >&2; \
        exit 1; \
    fi

ensure-scancode:
    @if ! command -v scancode >/dev/null 2>&1; then \
        echo "scancode not installed. See https://scancode-toolkit.readthedocs.io/en/latest/getting-started/install.html" >&2; \
        exit 1; \
    fi

# Syft SBOM generation ------------------------------------------------------

sbom: ensure-syft
    mkdir -p target/sbom
    syft dir:. --output cyclonedx-json=target/sbom/sbom.json

# ScanCode reports ----------------------------------------------------------

scancode: ensure-scancode
    mkdir -p target/scancode
    scancode --strip-root --html target/scancode/report.html --summary-json target/scancode/summary.json --license-text target/scancode/licenses --processes 4 .

ensure-gh:
    if ! command -v gh >/dev/null 2>&1; then \
        echo "gh CLI not installed. Install from https://cli.github.com/ before running this recipe." >&2; \
        exit 1; \
    fi

gh-repo-audit: ensure-gh
    export GH_PAGER=; \
    repo=$(gh repo view --json nameWithOwner --jq '.nameWithOwner') && \
    echo "# Repository security configuration" && \
    gh api repos/"$repo" --jq '{visibility: .visibility, private: .private, allow_forking: .allow_forking, security_and_analysis: .security_and_analysis, default_workflow_permissions: .default_workflow_permissions}' && \
    wf_perm=$(gh api repos/"$repo" --jq '.default_workflow_permissions // "unset"') && \
    if [ "$wf_perm" = "write" ]; then \
        echo "!! default_workflow_permissions is 'write' (should be 'read')." >&2; \
    elif [ "$wf_perm" = "unset" ]; then \
        echo "!! default_workflow_permissions not explicit (uses org default). Consider setting to 'read'." >&2; \
    fi && \
    echo "# Actions permissions" && \
    actions_json=$(gh api repos/"$repo"/actions/permissions) && \
    echo "$actions_json" | jq '{enabled: .enabled, allowed_actions: .allowed_actions, can_approve_pull_request_reviews: .can_approve_pull_request_reviews}' && \
    allowed=$(echo "$actions_json" | jq -r '.allowed_actions // "all"') && \
    if [ "$allowed" = "all" ]; then \
        echo "!! allowed_actions is 'all' (consider restricting to selected)." >&2; \
    elif [ "$allowed" = "selected" ]; then \
        echo "# Selected actions configuration" && \
        gh api repos/"$repo"/actions/permissions/selected-actions --jq '{github_owned_allowed: .github_owned_allowed, verified_allowed: .verified_allowed, actions: .actions, patterns: .patterns}'; \
    fi && \
    echo "# Rulesets" && \
    gh api repos/"$repo"/rulesets --jq '[.[] | {name: .name, target: .target, enforcement: .enforcement}]' || \
      echo "(rulesets API unavailable or no rulesets configured)" && \
    echo "# Branch protection (main)" && \
    gh api repos/"$repo"/branches/main/protection || \
      echo "(classic branch protection not configured; see rulesets above)" && \
    echo "# Merge settings" && \
    gh api repos/"$repo" --jq '{default_branch: .default_branch, allow_squash_merge: .allow_squash_merge, allow_merge_commit: .allow_merge_commit, allow_rebase_merge: .allow_rebase_merge}' && \
    echo "# Security advisories (Dependabot alerts require Dependabot; skipped for Renovate workflows)"

ensure-cargo-audit:
    if ! command -v cargo-audit >/dev/null 2>&1; then \
        echo "cargo-audit not installed. Run \`just install-cargo-audit\` or \`cargo install --locked cargo-audit\`." >&2; \
        exit 1; \
    fi

ensure-cargo-deny:
    @if ! command -v cargo-deny >/dev/null 2>&1; then \
        echo "cargo-deny not installed. Run \`just install-cargo-deny\` or \`cargo install --locked cargo-deny\`." >&2; \
        exit 1; \
    fi

ensure-cargo-about:
    @if ! command -v cargo-about >/dev/null 2>&1; then \
        echo "cargo-about not installed. Run \`just install-cargo-about\` or \`cargo install --locked cargo-about\`." >&2; \
        exit 1; \
    fi

ensure-go-licenses:
    @if ! command -v go-licenses >/dev/null 2>&1; then \
        echo "go-licenses not installed. Run \`just install-go-licenses\` or \`go install github.com/google/go-licenses/v2@latest\`." >&2; \
        exit 1; \
    fi

ensure-cargo-tarpaulin:
    @if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
        echo "cargo-tarpaulin not installed. Run \`just install-cargo-tarpaulin\` or \`cargo install --locked cargo-tarpaulin\`." >&2; \
        exit 1; \
    fi

ensure-cargo-mutants:
    @if ! command -v cargo-mutants >/dev/null 2>&1; then \
        echo "cargo-mutants not installed. Run \`just install-cargo-mutants\` or \`cargo install --locked cargo-mutants\`." >&2; \
        exit 1; \
    fi

ensure-cargo-fuzz:
    @if ! command -v cargo-fuzz >/dev/null 2>&1; then \
        echo "cargo-fuzz not installed. Run \`just install-cargo-fuzz\` or \`cargo install cargo-fuzz\`." >&2; \
        exit 1; \
    fi

ensure-yamllint:
    @if ! command -v yamllint >/dev/null 2>&1; then \
        echo "yamllint not installed. Run \`just install-yamllint\` or \`pip install --user yamllint\`." >&2; \
        exit 1; \
    fi

ensure-cargo-geiger:
    @if ! command -v cargo-geiger >/dev/null 2>&1; then \
        echo "cargo-geiger not installed. Run \`cargo install --locked cargo-geiger\`." >&2; \
        exit 1; \
    fi

ensure-golangci-lint:
    @if ! command -v golangci-lint >/dev/null 2>&1; then \
        echo "golangci-lint not installed. Install via https://golangci-lint.run/usage/install/ (e.g. `curl -sSfL https://raw.githubusercontent.com/golangci-lint/master/install.sh | sh -s -- -b "$(go env GOPATH)/bin" v1.59.1`)." >&2; \
        exit 1; \
    fi

ensure-osv-scanner:
    @if ! command -v osv-scanner >/dev/null 2>&1; then \
        echo "osv-scanner not installed. Install via `go install github.com/google/osv-scanner/cmd/osv-scanner@latest`." >&2; \
        exit 1; \
    fi



ensure-release-plz:
    @if ! command -v release-plz >/dev/null 2>&1; then \
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

install-cargo-about:
    if ! command -v cargo-about >/dev/null 2>&1; then \
        cargo install --locked cargo-about; \
    else \
        echo "cargo-about already installed"; \
    fi

install-go-licenses:
    if ! command -v go-licenses >/dev/null 2>&1; then \
        GOBIN="${HOME}/go/bin" go install github.com/google/go-licenses/v2@latest; \
    else \
        echo "go-licenses already installed"; \
    fi
