---
title: "Security and Privacy"
type: engineering
status: active
updated: 2026-08-15
canonical: true
tags:
  - security
  - privacy
---

# Security and privacy

## Threat premise

Shoehorn parses hostile user-controlled files with complex libraries. This is a serious security boundary.

## Threats and mitigations

### Parser vulnerabilities
Use patched providers, sandboxing, least privilege and isolated cloud jobs.

### Decompression bombs
Pixel/page/frame limits, archive expansion ratios, disk quotas.

### Command injection
Typed args, allowlisted binaries, no shell concatenation.

### Path traversal
Generated workspace paths and safe archive extraction.

### SSRF if URL inputs are later supported
Controlled fetcher, private-range blocking where appropriate, DNS rebinding defenses, byte/time caps.

### Malicious documents
Do not execute macros/scripts.

### Active browser content
Do not render arbitrary SVG/HTML in privileged origin.

### Cross-tenant cloud leakage
Per-job workspace and scoped credentials.

### Supply chain
Pin providers, verify artifacts, publish SBOM.

## Privacy

- local by default;
- explicit cloud;
- short retention;
- no file contents in logs;
- filenames redacted from analytics;
- metadata treated as user data;
- no training on user files by default.

## Telemetry

Okay by default:
- family;
- transform class;
- failure code;
- duration;
- provider version.

Avoid by default:
- filename;
- raw error text;
- target URL;
- document metadata;
- stable content hashes in analytics.

## Sandboxing

Cloud:
- isolated container/microVM;
- read-only base;
- ephemeral workspace;
- no network by default;
- cgroup/resource limits;
- secret isolation.

Native:
- OS sandbox where practical;
- never require elevation.

## Profile security

Profiles are declarative data only:
- no shell hooks;
- no scripts;
- no arbitrary provider flags.

## Model-assisted parser

Model output is untrusted candidate data. Schema/evidence validation is mandatory.
