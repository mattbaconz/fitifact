---
title: "Data Model"
type: architecture
status: active
implementation: mixed
updated: 2026-08-16
canonical: true
tags:
  - data-model
---

# Data model

## Core entities

### Artifact
```text
schema: fitifact.artifact/v1
path?
byte_length
family
container?
streams[]
duration_ms?
inspection
```

### Inspection
```text
provider
provider_version
warnings
completeness
```

Each normalized stream retains its probe index when known and a tagged type.
Video streams carry codec, dimensions, rational frame rate, pixel format, bit
depth, color facts, and explicit HDR status. Audio and all non-A/V stream types
remain represented rather than being silently dropped.

### ConstraintSet
```text
schema: fitifact.constraints/v1
hard[]
preferences
```

### DestinationProfile
```text
id
scope
revision
constraints
sources
tests
trust
last_verified
```

### TransformCapability
```text
id
provider
preconditions
effects
side_effects
cost_model
execution_modes
```

### Plan
```text
schema: fitifact.plan/v1
planner_version: 0.1.0
steps[]
warnings[]
```

Steps carry typed provider-neutral operations and targets, reasons, expected
post-step facts, preservation claims, and warnings. Provider commands and argv
are execution details and never part of the plan contract.

### ExecutionResult
```text
job_id
plan_id
status
output_artifact
step_results
resource_usage
```

### ValidationReport
```text
status
checks[]
unknowns[]
```

### AdaptationResult
```text
status: compatible | adapted | cannot_satisfy | failed
original
output?
violations
plan?
validation?
explanation
```

## Reproducibility

Hash canonical serialized constraints and provider capability snapshot.

## Sensitive data

Do not put raw content, GPS, document author or filename into telemetry by default.

Cloud payload metadata and object storage should be separate.
