#!/usr/bin/env bash
set -euo pipefail

# Ensure PATH includes cargo/go/python user bins for this session and future shells.
GOBIN=$(go env GOPATH)
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:${GOBIN}/bin:$PATH"
if ! grep -q "lithos devcontainer tooling" "$HOME/.bashrc" 2>/dev/null; then
  cat <<'PATHBLOCK' >> "$HOME/.bashrc"
# lithos devcontainer tooling
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$(go env GOPATH)/bin:$PATH"
PATHBLOCK
fi

rustup toolchain install nightly
rustup component add clippy rustfmt --toolchain nightly

python3 -m pip install --user --upgrade pip

cargo install just

just install-ci-tools
