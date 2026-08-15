---
title: "Contribution Guide"
type: engineering
status: active
updated: 2026-08-15
canonical: true
tags:
  - contributing
---

# Contribution guide

## Contribution types
- profile;
- transform provider;
- inspector;
- planner;
- fixture;
- UI;
- docs;
- security;
- benchmarks.

## Profile PR
Include:
- ID/scope;
- constraints;
- source URLs;
- verification date;
- trust evidence;
- tests.

## Provider PR
Include:
- capability metadata;
- license notes;
- platform support;
- safe typed args;
- failure mapping;
- resource-limit behavior;
- fixtures;
- security notes.

## Core PR
Must:
- preserve product invariants;
- include tests;
- avoid destination-specific planner hacks;
- update schema/docs if needed.

## Labels
`compat-profile`, `provider`, `planner`, `security`, `performance`, `ux`, `cloud`, `good-first-profile`.

## Security
Use coordinated private disclosure for exploitable vulnerabilities.

## Quality bar
“Tool X can convert Y” is not enough; it must integrate into compatibility semantics and validation.
