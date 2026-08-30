# Security policy

## Supported versions

Fitifact is pre-release software. Security fixes are applied to the latest code
on the default branch and to the latest GitHub release when one exists.

There is currently no public repository or published release. Until owner/legal
approval creates the repository, GitHub private vulnerability reporting is not
available and the repository-owner fallback below is the only prepared route.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use GitHub's private
vulnerability reporting for
[`mattbaconz/fitifact`](https://github.com/mattbaconz/fitifact/security/advisories/new).
If that channel is not available, contact the repository owner privately
through their GitHub profile and request a secure reporting channel. Do not send
sensitive exploit details in a public message.

Include the affected version or commit, operating system, reproduction steps,
impact, and any suggested mitigation. You should receive acknowledgement within
seven days. Disclosure timing will be coordinated after triage and a fix.

## Security model

Fitifact treats media files and external tool output as untrusted. It invokes
system `ffmpeg` and `ffprobe` with argument arrays, preserves original files,
refuses existing output paths, and validates generated output. It performs no
telemetry, network access, or implicit upload. Deferred cloud sections are not
part of v0.1.

Release automation uses least-privilege job permissions, full-SHA GitHub Action
pins, SHA-256 manifests, CycloneDX SBOMs, and GitHub artifact attestations.
Dependencies are checked against RustSec advisories and the repository's
license/source policy. Windows and macOS binaries are intentionally unsigned in
v0.1, and system FFmpeg is external and must be assessed under its own build
configuration and licensing.
