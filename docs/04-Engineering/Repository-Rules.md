---
title: "Repository Rules"
type: engineering
status: active
implementation: prepared
updated: 2026-08-16
canonical: true
tags:
  - github
  - governance
  - security
---

# Repository rules

These settings are a publication checklist, not a claim about current remote
state. The public `mattbaconz/fitifact` repository does not exist yet, so no
ruleset or branch protection can be applied until owner/legal Fitifact approval.

## Default-branch rules

When the repository is created, protect `main` with a repository ruleset that:

- requires pull requests and at least one approving review;
- dismisses stale approvals when the reviewed commit changes;
- requires the branch to be current before merge;
- requires signed GitHub web/verified commits when operationally available;
- blocks force pushes and branch deletion for every actor, including bypass
  roles except documented break-glass recovery;
- requires conversation resolution and linear history;
- restricts direct pushes to the release-maintainer role;
- enables immutable releases before `v0.1.0` is published.

Require these check families from [CI](../../.github/workflows/ci.yml) and the
release-plan pull-request path:

- `CI / quality`;
- all four `CI / platform (<target>)` matrix checks;
- `CI / MSRV 1.85`;
- `CI / supply chain`;
- `Release / plan`.

Verify the exact check names from a real pull request before saving the ruleset;
GitHub can only require checks that have reported in the repository.

## Automation permissions

Set the repository workflow default to read-only. Allow write permissions only
where the checked-in release workflow declares them: GitHub release creation
needs `contents: write`; attestation additionally needs `id-token: write` and
`attestations: write`. Do not grant package, issue, pull-request, or deployment
write permissions to the release jobs.

Enable Dependabot for Cargo and GitHub Actions. Action updates must remain pinned
to full 40-character commits with a trailing version comment and must pass
`scripts/check-workflows.ps1`.
