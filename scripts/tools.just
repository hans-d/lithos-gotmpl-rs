default:


ensure-mise:
    @if ! command -v mise >/dev/null 2>&1; then \
        echo "mise not installed. Run \`just tools install-mise\` or install it manually." >&2; \
        exit 1; \
    fi

recommend-mise:
    @if ! command -v mise >/dev/null 2>&1; then \
        echo "mise not installed. Run \`just tools install-mise\` or install it manually." >&2; \
    fi

install-mise:
    if ! command -v mise >/dev/null 2>&1; then \
        echo "install mise manually, see https://mise.jdx.dev/installing-mise.html"
    fi

# cargo-audit
ensure-cargo-audit:
    @if ! command -v cargo-audit >/dev/null 2>&1; then \
        echo "cargo-audit not installed. Run \`just tools install-cargo-audit\` or install it manually." >&2; \
        exit 1; \
    fi

install-cargo-audit:
    if ! command -v cargo-audit >/dev/null 2>&1; then \
        cargo install --locked cargo-audit; \
    fi

# cargo-deny
ensure-cargo-deny:
    @if ! command -v cargo-deny >/dev/null 2>&1; then \
        echo "cargo-deny not installed. Run \`just tools install-cargo-deny\` or install it manually." >&2; \
        exit 1; \
    fi

install-cargo-deny:
    if ! command -v cargo-deny >/dev/null 2>&1; then \
        cargo install --locked cargo-deny; \
    fi

# cargo-about
ensure-cargo-about:
    @if ! command -v cargo-about >/dev/null 2>&1; then \
        echo "cargo-about not installed. Run \`just tools install-cargo-about\` or install it manually." >&2; \
        exit 1; \
    fi

install-cargo-about:
    if ! command -v cargo-about >/dev/null 2>&1; then \
        cargo install --locked cargo-about; \
    fi

# go-licenses
ensure-go-licenses:
    @if ! command -v go-licenses >/dev/null 2>&1; then \
        echo "go-licenses not installed. Run \`just tools install-go-licenses\` or install it manually." >&2; \
        exit 1; \
    fi

install-go-licenses:
    if ! command -v go-licenses >/dev/null 2>&1; then \
        go install github.com/google/go-licenses/v2@latest; \
    fi

# cargo-tarpaulin
ensure-cargo-tarpaulin:
    @if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
        echo "cargo-tarpaulin not installed. Run \`just tools install-cargo-tarpaulin\` or install it manually." >&2; \
        exit 1; \
    fi

install-cargo-tarpaulin:
    if ! command -v cargo-tarpaulin >/dev/null 2>&1; then \
        cargo install --locked cargo-tarpaulin; \
    fi

# cargo-mutants
ensure-cargo-mutants:
    @if ! command -v cargo-mutants >/dev/null 2>&1; then \
        echo "cargo-mutants not installed. Run \`just tools install-cargo-mutants\` or install it manually." >&2; \
        exit 1; \
    fi

install-cargo-mutants:
    if ! command -v cargo-mutants >/dev/null 2>&1; then \
        cargo install --locked cargo-mutants; \
    fi

# cargo-fuzz
ensure-cargo-fuzz:
    @if ! command -v cargo-fuzz >/dev/null 2>&1; then \
        echo "cargo-fuzz not installed. Run \`just tools install-cargo-fuzz\` or install it manually." >&2; \
        exit 1; \
    fi

install-cargo-fuzz:
    if ! command -v cargo-fuzz >/dev/null 2>&1; then \
        cargo install cargo-fuzz; \
    fi

# yamllint
ensure-yamllint: recommend-mise
    @if ! command -v yamllint >/dev/null 2>&1; then \
        echo "yamllint not installed. Use mise (preferred) or install manually." >&2; \
        exit 1; \
    fi

# cargo-geiger
ensure-cargo-geiger:
    @if ! command -v cargo-geiger >/dev/null 2>&1; then \
        echo "cargo-geiger not installed. Run \`just tools install-cargo-geiger\` or install it manually." >&2; \
        exit 1; \
    fi

install-cargo-geiger:
    if ! command -v cargo-geiger >/dev/null 2>&1; then \
        cargo install --locked cargo-geiger; \
    fi

# golangci-lint
ensure-golangci-lint: recommend-mise
    @if ! command -v golangci-lint >/dev/null 2>&1; then \
        echo "golangci-lint not installed. Use mise (preferred) or install manually." >&2; \
        exit 1; \
    fi

# osv-scanner
ensure-osv-scanner: recommend-mise
    @if ! command -v osv-scanner >/dev/null 2>&1; then \
        echo "osv-scanner not installed. Use mise (preferred) or install manually." >&2; \
        exit 1; \
    fi

# actionlint
ensure-actionlint: recommend-mise
    @if ! command -v actionlint >/dev/null 2>&1; then \
        echo "actionlint not installed. Use mise (preferred) or install manually." >&2; \
        exit 1; \
    fi

# release-plz
ensure-release-plz:
    @if ! command -v release-plz >/dev/null 2>&1; then \
        echo "release-plz not installed. Run \`just tools install-release-plz\` or install it manually." >&2; \
        exit 1; \
    fi

install-release-plz:
    if ! command -v release-plz >/dev/null 2>&1; then \
        cargo install release-plz; \
    fi

# syft
ensure-syft: recommend-mise
    @if ! command -v syft >/dev/null 2>&1; then \
        echo "syft not installed. Use mise (preferred) or install manually." >&2; \
        exit 1; \
    fi

# scancode
ensure-scancode:
    @if ! command -v scancode >/dev/null 2>&1; then \
        echo "scancode not installed. See https://scancode-toolkit.readthedocs.io/en/latest/getting-started/install.html" >&2; \
        exit 1; \
    fi

# gh CLI
ensure-gh: recommend-mise
    @if ! command -v gh >/dev/null 2>&1; then \
        echo "gh CLI not installed. Use mise (preferred) or install manually." >&2; \
        exit 1; \
    fi
