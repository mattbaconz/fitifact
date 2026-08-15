---
title: "API Examples"
type: examples
status: active
updated: 2026-08-15
canonical: true
tags:
  - examples
  - api
---

# API examples

## Check

```ts
const artifact = await shoehorn.inspect(file);

const constraints = {
  maxBytes: 25_000_000,
  media: {
    container: ["mp4"],
    videoCodec: ["h264"],
    audioCodec: ["aac"]
  }
};

const report = shoehorn.check(artifact, constraints);

if (report.compatible) {
  useOriginal(file);
}
```

## Plan

```ts
const plans = await shoehorn.plan(artifact, constraints, {
  preserve: {
    resolution: "high",
    frameRate: "high",
    audio: "high"
  }
});
```

## Execute local

```ts
const result = await shoehorn.execute(file, plans.recommended, {
  mode: "local"
});

if (result.validation.status !== "pass") {
  throw new Error("Adaptation failed validation");
}
```

## Uploader pseudocode

```ts
async function onFileSelected(file) {
  const check = await shoehorn.checkFile(file, policy);

  if (check.compatible) return upload(file);

  const consent = await showRepairPlan(check.plan);
  if (!consent) return;

  const adapted = await shoehorn.execute(file, check.plan);
  return upload(adapted.output);
}
```

## Cloud

```text
POST /v1/uploads
PUT <presigned-url>
POST /v1/jobs
```

Signed webhook returns completed validation result.
