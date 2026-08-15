# Security policy

## Supported versions

Fitifact is pre-release software. Security fixes are applied to the latest code
on the default branch and to the latest GitHub release when one exists.

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
telemetry, network access, or implicit upload. See
[`docs/07-Specs/Security-Model.md`](docs/07-Specs/Security-Model.md) for the
broader design; deferred cloud sections are not part of v0.1.
