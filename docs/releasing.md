# Releasing

We use [release-plz](https://github.com/MarcoIeni/release-plz) to automate version bumps.

## Prerequisites

1. Install release-plz:
   ```bash
   cargo install release-plz
   ```
2. Ensure `release-plz.toml` reflects the desired release configuration.

## Dry-run

```
just ci-release
```

(Or run `release-plz release --dry-run` directly.)

This will show the version bumps and git operations release-plz would perform.

## Create a Release

```
just release
```

(`just release` simply calls `release-plz release`.) The command updates manifests, creates tags,
and pushes the release branch/PR to origin.

## Notes

- Publishing to crates.io is currently disabled (`publish = false` in release-plz configuration). Adjust if you plan to publish artifacts.
- Review the generated changelog entries and version bumps before pushing.
