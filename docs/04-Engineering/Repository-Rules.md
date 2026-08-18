---
title: "Repository Rules"
type: engineering
status: active
implementation: prepared
updated: 2026-08-18
canonical: true
tags:
  - github
  - governance
  - security
---

# Repository rules

These settings are the intended GitHub configuration for public
`mattbaconz/fitifact`. Owner directed repository creation on 2026-08-18.
Apply them on the live repo; they are not a claim that every control is
already enforced.

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

Create a protected tag ruleset for `v*` that blocks tag updates and deletions
and restricts creation to the release-maintainer role. Do not grant a broad
bypass; documented break-glass recovery must still preserve an audit record.

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

Set the repository workflow default to read-only. The release `plan` and build
jobs keep `contents: read`. Only the protected `host` publication job receives
`contents: write`, `id-token: write`, and `attestations: write`; do not grant
package, issue, pull-request, deployment, or other write permissions.

Before any release tag is pushed, create a GitHub Environment named
`public-release`. Configure required reviewers from the release-owner group,
prevent self-review where available, and allow deployment only from protected
`v*` tags. Separately create the repository variable
`FITIFACT_PUBLICATION_APPROVED` with value `false`. Owner/legal Fitifact
sign-off is required before a maintainer may temporarily set it to `true`.

The release workflow deliberately permits a matching tag to plan and build
artifacts while approval is false, but its `host` job is conditional on the
repository variable and protected by the Environment review. Thus neither a
tag alone nor the variable alone authorizes publication. Reset the variable to
`false` after the approved publication window.

Enable Dependabot for Cargo and GitHub Actions. Action updates must remain pinned
to full 40-character commits with a trailing version comment and must pass
`scripts/check-workflows.ps1`.
