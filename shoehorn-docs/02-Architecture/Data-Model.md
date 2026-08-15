---
title: "Data Model"
type: architecture
status: active
updated: 2026-08-15
canonical: true
tags:
  - data-model
---

# Data model

## Core entities

### Artifact
```text
id
content_hash
original_name
byte_length
family
inspection
source
```

### Inspection
```text
schema_version
provider
provider_version
facts
warnings
completeness
```

### ConstraintSet
```text
hard[]
preferences[]
unresolved[]
conflicts[]
provenance[]
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
id
planner_version
input_hash
constraints_hash
steps[]
expected_state
cost
reasons[]
warnings[]
```

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
