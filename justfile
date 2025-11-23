set shell := ["bash", "-cu"]
go_sanity_dir := "go-sanity"

default:

mod go-sanity
mod go 'go-sanity'
mod tools 'scripts/tools.just'
mod rust 'rust.just'
mod github '.github/justfile'

lint-automation: github::lint

yaml-lint: tools::ensure-yamllint
    yamllint -c .yamllint.yaml .

deps: rust::deps go::deps
licenses: rust::licenses go::licenses

test: rust::test

# tbd

ci-osv: tools::ensure-osv-scanner
    mkdir -p target
    osv-scanner --recursive --output=target/osv-report.json .
    @echo "OSV report written to target/osv-report.json"

coverage: tools::ensure-cargo-tarpaulin
    cargo tarpaulin --workspace --all-features --engine llvm --out Html

sbom: tools::ensure-syft
    mkdir -p target/sbom
    syft dir:. --output cyclonedx-json=target/sbom/sbom.json

scancode: tools::ensure-scancode
    mkdir -p target/scancode
    scancode --strip-root --html target/scancode/report.html --summary-json target/scancode/summary.json --license-text target/scancode/licenses --processes 4 .

gh-repo-audit: tools::ensure-gh
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
