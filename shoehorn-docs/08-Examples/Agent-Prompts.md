---
title: "Agent Prompts"
type: examples
status: active
updated: 2026-08-15
canonical: true
tags:
  - agents
  - examples
---

# Agent prompts

## Add a destination profile

> Read AGENTS.md, Product-Principles.md, Compatibility-Registry.md and Profile-Spec.md. Use current official sources. Model exact scope. Do not invent undocumented constraints. Include provenance, last_verified, trust status and fixtures.

## Add transform provider

> Read AGENTS.md, Plugin-Transform-System.md, Execution-Runtime.md, Security-Privacy.md and Plan-Spec.md. Define provider-neutral capability first, add planner tests, then safe typed execution. No shell strings. Re-inspect and validate output.

## Review planner

> Find cases where a broader/lossier transform outranks no-op, remux or selective transforms. Check hard/soft separation, unknown handling, dominance and explanation reasons.

## Red-team feature scope

> Decide whether the feature deepens destination-first compatibility or merely adds converter functionality. Recommend reject/defer if it does not create a new compatibility primitive.

## Competitor research

> Use current primary sources. Date findings. Separate product fact from inference. Compare destination-first intent, minimum mutation, validation, local execution, registry, API and integrations. Never infer nonexistence from search absence.

## Security review

> Treat file, requirement text, profiles and model output as untrusted. Trace all data into provider execution. Identify command injection, parser, path, SSRF, decompression, tenant isolation and resource exhaustion risks.
