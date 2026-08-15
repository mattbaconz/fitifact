---
title: "Agent Workflow"
type: engineering
status: active
updated: 2026-08-15
canonical: true
tags:
  - agents
  - engineering
---

# Agent workflow

## Before coding

Answer:
1. What incompatibility does this solve?
2. Which constraint is missing?
3. Which inspector fact is required?
4. Which transform capability is needed?
5. How is output validated?
6. What is the minimum fixture?

## Task template

```text
Goal:
User story:
Input fixture:
Target constraints:
Expected violations:
Expected plan:
Expected output facts:
Failure cases:
Security risks:
Docs touched:
```

## Planner-first

Model edge and planner tests before provider wiring.

## Stop conditions

Stop for human decision if:
- decision log conflict;
- semantic destruction;
- uncertain license;
- weak profile source;
- cloud-by-default requirement;
- local-first weakened;
- destination special case added to generic planner.

## Research workflow

Use current primary sources, capture date, separate fact from inference, update Source Ledger.

## Review questions

- Can this mutate less?
- Could input already be valid?
- Are constraints sourced?
- Is validation independent?
- Can hostile input exploit this?
- Did UX become converter-first?
