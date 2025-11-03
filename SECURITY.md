# Security Policy

## Supported Versions

We aim to keep the main branch secure. Security fixes are delivered through regular development rather than backporting to historical tags.

## Roadmap for a multi-maintainer security process

When additional maintainers join, we plan to:

- Enable branch protection with required reviews, status checks, and last-push approval for `main`.
- Publish signed release artifacts and document verification steps.
- Establish a private vulnerability intake channel (e.g., security@ alias) with a documented response SLA (≤14 days initial response).
- Produce human-readable release notes summarising changes and any CVEs addressed per release.
- Maintain the security/threat model (see `docs/security/threat-model.md`) and extend it with a full architecture overview.
- Expand CONTRIBUTING with explicit coding standards, test requirements, and release guidelines.

## Reporting a Vulnerability

FUTURE: dedicated reporting channel to be documented when additional maintainers join; the project currently has a single maintainer.
