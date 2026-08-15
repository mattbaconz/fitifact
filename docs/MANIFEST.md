---
title: "Vault Manifest"
type: manifest
status: active
implementation: mixed
updated: 2026-08-15
canonical: true
tags:
  - manifest
---

# Vault manifest

Updated: **2026-08-15**

Markdown documents: **93**

## Latest architecture update

The Rust CLI/media core is current in v0.1. Web, browser, OS, mobile, API,
image, and cloud items in this section are deferred design constraints.

Fitifact now explicitly treats **extreme lightweightness** as a product and engineering requirement:

- one shared compatibility core;
- thin web/browser/OS/mobile/CLI/API integrations;
- lazy transform providers;
- no mandatory idle daemon;
- browser extension remains a small evidence/IPC client;
- heavy media runtime is not part of initial web/extension load;
- operating-system codec frameworks can act as providers;
- plan/check can occur without uploading payloads;
- cloud execution is chosen only when needed/explicit;
- performance budgets and packaging strategy are documented.

## Key new documents

- `02-Architecture/Lightweight-Architecture.md`
- `03-Surfaces/Integration-Strategy.md`
- `04-Engineering/Packaging-Distribution.md`
- `04-Engineering/Performance-Budgets.md`

## Files

- `00-Foundation/Decision-Log.md`
- `00-Foundation/Executive-Summary.md`
- `00-Foundation/Glossary.md`
- `00-Foundation/Open-Questions.md`
- `00-Foundation/Product-Principles.md`
- `00-Foundation/Vision.md`
- `00-Foundation/_INDEX.md`
- `01-Product/Compatibility-Flow.md`
- `01-Product/Naming-Brand.md`
- `01-Product/Non-Goals.md`
- `01-Product/Positioning-Messaging.md`
- `01-Product/Product-Definition.md`
- `01-Product/Product-Requirements.md`
- `01-Product/UX-Spec.md`
- `01-Product/Use-Cases.md`
- `01-Product/User-Journeys.md`
- `01-Product/_INDEX.md`
- `02-Architecture/Compatibility-Registry.md`
- `02-Architecture/Constraint-Compiler.md`
- `02-Architecture/Constraint-Model.md`
- `02-Architecture/Core-Engine.md`
- `02-Architecture/Data-Model.md`
- `02-Architecture/Execution-Runtime.md`
- `02-Architecture/File-Inspection.md`
- `02-Architecture/Lightweight-Architecture.md`
- `02-Architecture/Local-vs-Cloud.md`
- `02-Architecture/Planner-Scoring.md`
- `02-Architecture/Plugin-Transform-System.md`
- `02-Architecture/Quality-Loss-Model.md`
- `02-Architecture/System-Architecture.md`
- `02-Architecture/Transformation-Graph.md`
- `02-Architecture/Validation.md`
- `02-Architecture/_INDEX.md`
- `03-Surfaces/Browser-Extension.md`
- `03-Surfaces/CLI.md`
- `03-Surfaces/Desktop-OS.md`
- `03-Surfaces/Integration-Strategy.md`
- `03-Surfaces/Mobile.md`
- `03-Surfaces/SDK-API.md`
- `03-Surfaces/Web-App.md`
- `03-Surfaces/_INDEX.md`
- `04-Engineering/Agent-Workflow.md`
- `04-Engineering/Build-vs-Buy.md`
- `04-Engineering/Contribution-Guide.md`
- `04-Engineering/Legal-Licensing.md`
- `04-Engineering/MVP-Scope.md`
- `04-Engineering/Observability.md`
- `04-Engineering/Packaging-Distribution.md`
- `04-Engineering/Performance-Budgets.md`
- `04-Engineering/Performance.md`
- `04-Engineering/Release-Strategy.md`
- `04-Engineering/Roadmap.md`
- `04-Engineering/Security-Privacy.md`
- `04-Engineering/Testing-QA.md`
- `04-Engineering/_INDEX.md`
- `05-Business/Competitive-Response.md`
- `05-Business/Enterprise.md`
- `05-Business/FOSS-Strategy.md`
- `05-Business/GTM-Distribution.md`
- `05-Business/Metrics.md`
- `05-Business/Pricing.md`
- `05-Business/SaaS-Business-Model.md`
- `05-Business/Unit-Economics.md`
- `05-Business/YouTube-Launch.md`
- `05-Business/_INDEX.md`
- `06-Research/Alternatives.md`
- `06-Research/Competitors.md`
- `06-Research/Differentiation.md`
- `06-Research/Market-Map.md`
- `06-Research/Prior-Art.md`
- `06-Research/Research-Methodology.md`
- `06-Research/Source-Ledger.md`
- `06-Research/Threats.md`
- `06-Research/_INDEX.md`
- `07-Specs/API-Spec.md`
- `07-Specs/CLI-Spec.md`
- `07-Specs/Constraint-Schema.md`
- `07-Specs/Error-Model.md`
- `07-Specs/Plan-Spec.md`
- `07-Specs/Profile-Spec.md`
- `07-Specs/Security-Model.md`
- `07-Specs/_INDEX.md`
- `08-Examples/API-Examples.md`
- `08-Examples/Agent-Prompts.md`
- `08-Examples/Profile-Examples.md`
- `08-Examples/User-Scenarios.md`
- `08-Examples/_INDEX.md`
- `AGENTS.md`
- `CONTEXT.md`
- `INDEX.md`
- `NEXT.md`
- `README.md`
