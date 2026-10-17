# Changelog

All notable changes to Fitifact are documented here.

## [Unreleased] - after `0.1.0-rc.6`

### [fixed]

- Bundled tool lookup accepts only `<tool>` or `<tool>-<target triple>`
  executables, not any file that starts with the tool name.
- Native tool diagnostics no longer mistake the app's launch directory for an
  output destination and disable otherwise healthy bundled media tools.
- Native file selection uses keyboard-accessible buttons. Damaged-file and
  Explorer failure messages describe recovery without promising a missing file exists.
- Captured media subprocesses no longer flash console windows on Windows.
- Cancelling a desktop media job on Windows also ends FFmpeg when it runs
  behind a launcher shim, so Cancel no longer waits for the encode to finish.
- A damaged image is reported as unreadable instead of as a requirements
  problem, and engine refusals read as plain sentences with the code under
  Technical details.
- Video re-encoded from a source with no color tags (common for screen
  recordings and generated clips) is now tagged BT.709, so it passes validation
  instead of failing every Discord fit.

### [added]

- One design system for the web app, the desktop app, and the docs: shared
  tokens and `ft-` components in light and dark, following the system theme by
  default. `DESIGN.md` documents the tokens, components, and every state.
- **Adapt and download** runs the visible plan, validates the output, and
  downloads it only if validation passes: a file that needs no consent takes
  three interactions (drop, destination, Adapt and download).
- Results compare each requirement's measured value before and after, show the
  original and the output side by side, and offer Copy and Share where the
  browser can hand over the exact validated file.
- Designed inspecting, consent, adapting, warning, refusal, and error states.
  Refusals offer new requirements instead of Retry.
- A strict Content Security Policy on the web build (its own origin only), and
  end-to-end checks for keyboard-only journeys, axe on every state in both
  themes, and zero cross-origin requests.
- A shared graphite/chalk interface, local dark/light/system appearance, real
  sample scenarios, and a compact workspace with inspect → plan → adapt → validate states.
- A static Fumadocs site with local search, task guides, examples, and accurate
  browser, desktop, and CLI download guidance, included in the application build.
- Named saved targets with rename and undo, report/constraint exports, and
  recovery actions that retain valid source files after rejected replacements.
- Native operation identifiers, cancellable subprocess execution, an atomic
  cancellation/publication boundary, and recoverable Show in folder actions.

- Desktop and CLI video: WebM/MKV sources, AAC audio encode, scale-to-fit, and
  VP8/VP9/AV1 transcode to H.264 when the destination needs it.
- Desktop and CLI: audio-only inspect/adapt/verify. Size-only destinations
  (Gmail, Discord file upload, Slack, WhatsApp, X) no-op when the file already
  fits; oversized or AAC-required sound encodes to AAC in MP4. Video destinations
  refuse audio-only files.
- Shipped size-only profiles: `discord/file-upload` (and Nitro caps),
  `slack/file-upload`, `whatsapp/audio`.
- `registry.json` at the site root lists every shipped profile with per-rule
  source type, URL, and check date. The UI shows when a profile's rules were
  checked, each rule's source and age, and marks profiles older than 30 days
  as stale. Profile sources gain the `fitifact-rule` type; trust is derived.
- `?for=<destination>` links preselect a destination.
- Byte windows: `file.bytes` accepts `gte`, the requirements parser reads
  floors and ranges such as "20-50 KB", and JPEG fitting lands inside the
  window or refuses. Files below a floor are refused, never padded.
- Visible intelligence in the UI: what was wrong, what Fitifact changes, what
  stays, and whether the result is verified.
- Desktop app ships `ffmpeg`/`ffprobe` as sidecars so video and audio work
  without a PATH install. CLI still uses FFmpeg from PATH. ffmpeg.wasm stays
  forbidden. PDF, ZIP, Office, and SVG are refused in one sentence.
- Windows desktop CI packages an unsigned NSIS installer (`desktop-windows` artifact). MSI is skipped because `0.1.0-rc.6` is not a valid MSI prerelease.
- Desktop results offer **Copy file**: the validated output, or the untouched
  original when it already fits, goes on the Windows clipboard as a file, so it
  pastes into Discord, an email, or an Explorer folder as a copy.
- A code signing policy in the README that meets SignPath Foundation's
  conditions. Windows releases stay unsigned until the project is accepted.

### [changed]

- Results put measured properties and output actions first. Compatible files keep
  their original bytes and filename, and mobile results collapse the target picker.
- Crop and first-frame removal require explicit approval for each file. Plans
  disclose provider metadata removal instead of attributing it to a destination.

- Destination-first copy and chips (Discord, Gmail, GitHub, WhatsApp, Slack, X,
  web image, web video). Public Pages stays still-image only.
- Desktop accepts WebM and MKV. The web app still refuses video and audio with a
  pointer to desktop.
- FFmpeg missing: per-OS copy buttons instead of a command wall.
- Work surface is file, destinations, then the verdict and download. Paste override sits below so compatible/no-op is visible without scrolling past the form.
- Desktop no longer asks consumers to install FFmpeg. The CLI still does.
- A file that meets every hard constraint is never re-encoded. Picture format,
  bit depth, and HDR are explicit profile rules (`media.video.pixel_format`,
  `bit_depth`, `hdr`), and an encode cites the rule that failed. HDR is
  tone-mapped to SDR with zscale when FFmpeg has it and refused otherwise.
- Profiles re-verified on 2026-10-17: Discord Nitro uploads up to 1 GB;
  Discord images stay at 20 MB on every tier because larger images arrive as
  downloads; WhatsApp video is 64 MB in MKV, MOV, or MP4 with no codec rule;
  WhatsApp audio uses the 2 GB documents limit; X and GitHub images are GIF,
  JPEG, or PNG; GitHub video accepts MOV, MP4, or WebM; Slack images keep
  edges under 25,000 px. `x/audio` is removed because X takes no audio-only
  uploads. `whatsapp/photo` is shown as stale because its source page moved.
- `matroska,webm` probes classify as WebM only when the codecs are WebM-legal
  (VP8/VP9/AV1 plus Opus or Vorbis). H.264 in that probe is MKV and can remux.

## [0.1.0-rc.6] - 0.2 session Pages UI

Docs call this the 0.2 session. Do not invent a `v0.2.0` tag. Existing
`v0.1.0-rc.1` through `v0.1.0-rc.5` tags stay where they are.

### [added]

- Destination chips ("Where does it need to work?") with shipped YAML profiles compiled in WASM.
- First-paint drop skeleton via `boot.css`, plus brand mark, 32px favicon, and a 1200x630 `og.png`.

### [changed]

- Public Pages stays still-image only: Generic video is not a Pages chip. Setup is desktop first-run, not a Pages tour.
- Idle job is "Make your image pass the upload." Paste is an override when a chip is wrong.

### [fixed]

- Keep an unsupported drop instead of wiping the session. Map oversized stills onto inspection/resource limits.

## [0.1.0-rc.5] - 0.2 usable session

Docs call this the 0.2 session. Do not invent a `v0.2.0` tag. Existing
`v0.1.0-rc.1` through `v0.1.0-rc.4` tags stay where they are.

### [added]

- File-first web session: inspect without a target, auto-parse rejection text,
  hide the schema behind Looks right / Edit, and adapt with **Fix image**.
- Still WebP as an adapt source (JPEG/PNG out; animated WebP refused).
- Local named saved targets in `localStorage` only.
- D-028 public/default lazy HEIC decoder with notices and a decoder-free CI job.

### [changed]

- Public Pages and default web builds include the pinned lazy `libheif-js`
  decoder; `FITIFACT_HEIC_APPROVED=false` remains the decoder-free proof.
- README, GitHub metadata, and consumer docs describe drop-then-paste instead
  of a CLI-first product.
- Compact the landing so the drop zone is visible on first paint; the paste
  field stays empty so people paste their own rejection text.

### [fixed]

- Allow the owned still-WebP fixture in the public-readiness binary allowlist.
- Make crop consent a 44px keyboard-operable control so WebKit mobile can
  approve it before **Fix image**.

## [0.1.0-rc.4] - consumer image upload candidate

These changes must not be tagged as `v0.1.0-rc.1`. The freeze SHA remains
`b033552cb2729e96ca97c649a7bb4a223f2ad900`.

### [added]

- Consumer image upload workflow with local-only browser adaptation, owned
  synthetic image fixtures (including HEIC), pinned web CI, and a moderated
  ten-user viability protocol.

### [changed]

- Align the canonical product, specification, legal, competitor, threat, and
  roadmap surfaces to the confirmed-requirements consumer image boundary.
- Tighten the web workflow hierarchy, density, state clarity, keyboard focus,
  and mobile layout while preserving the existing product system.

### [fixed]

- Preserve short byte limits adjacent to dimension limits during natural-
  language requirement parsing, and allow opaque decoded HEIC pixels to enter
  the JPEG adaptation path without weakening transparency refusal.

## [0.1.0-rc.3] - buildable candidate

### [fixed]

- Collect only `crates/fitifact-cli` CycloneDX XML so the global release job
  does not fail when cargo-cyclonedx also emits a wasm SBOM.

## [0.1.0-rc.2] - buildable candidate

### [added]

- D-025 image slice: JPEG already matching JPEG is a no-op; PNG targeting JPEG
  encodes in-process; WebP, HEIC/HEIF, TIFF, animation, and resize/byte-fitting
  are refused. The image provider never constructs or spawns FFmpeg.
- Tracked `fixtures/image` pair, `--image-format`, and a bench proof that image
  adapt spawns ffmpeg zero times.
- `fitifact-wasm` byte API and a static `web/` drop page for that image matrix
  only. Video files tell the user to use the CLI. No ffmpeg.wasm and no uploads.
- `ft.` visual monogram in `web/public/` for README and future icon use. It is
  not a trademark filing.

### [fixed]

- Define `[profile.dist]` so cargo-dist 0.32 artifact builds can run. The
  freeze tag `v0.1.0-rc.1` does not include this; do not move that tag.
- Pin `image` to 0.25.9 so MSRV 1.85 still checks.
- Resolve Markdown files for the doc-link scan from `git ls-files` so CI pwsh
  does not need ripgrep.

## [0.1.0-rc.1] - unpublished candidate

### [added]

- Release-ready public repository documentation and governance files.
- Versioned constraint, artifact, check, plan, adaptation, error, and doctor
  contracts with strict YAML parsing and normalized all-stream media facts.
- Stable JSON CLI/error/doctor envelopes, bounded transform timeouts, strict
  human-size flags, FFmpeg capability diagnostics, and structured validation
  provenance claims.
- Deterministic synthetic media fixtures, native four-target CI, dependency
  policy, and cargo-dist 0.32.0 release assets with checksums, CycloneDX SBOM,
  installers, and GitHub artifact attestations.
- `fitifact bench` demo/benchmark report (`fitifact.bench/v1`) for the three
  canonical fixtures, CLI inspect cold start, and lazy-provider / no-network
  proofs.

### [changed]

- Renamed the Shoehorn prototype to Fitifact across crates, CLI, schemas,
  examples, fixtures, and documentation.
- Defined v0.1 as the CLI/media slice distributed through GitHub only.
- Replaced deprecated `serde_yaml` with maintained `yaml_serde` and bounded the
  planner to provider-neutral remux and selective video-transcode operations.
- Hardened system FFmpeg execution to typed file-only/no-clobber argv, bounded
  process output/time, hidden sibling staging, atomic create-if-absent
  persistence, and defensive typed-plan validation.

### [fixed]

- Distinguished executable adaptation constraints from check-only constraints.
- Labelled later image, web, profile, natural-language, cloud, packaging, and
  operations work as deferred.
- Refused unsafe stream topology, unsupported mutation classes, HDR/bit-depth
  conversion, and uncertain post-transform size claims.
- Made overlapping target sets order-independent, added strict public JSON
  constraint compilation, and refused unproved pixel/color conversion.
- Prevented existing-output overwrite, partial-stage leakage, silent stream
  loss, raw provider diagnostic leakage, and false adaptation success by fresh
  topology/fact/duration checks plus SHA-256 copied-stream provenance.
- Bound staging and cleanup to atomically reserved workspaces and stable file
  identities, reject post-validation replacement, identity-confirm the
  published hard link, and preserve validated finals when cleanup cannot
  complete.
- Removed pathname rollback deletion: Windows cleanup is handle-bound, Unix
  workspaces are atomically mode-0700, ordinary provider partials are claimed
  and removed, ambiguous or replaced objects are retained, and
  publication/cleanup uncertainty returns structured warning paths.
- Refused ambiguous MOV-family demuxer labels without a recognized MP4 or
  QuickTime brand, rejected mixed constraint-file/flag targets, bounded
  constraint-file reads to 1 MiB, and locked GitHub publication to one
  attestation-ordered release creation command.
- Treated FFprobe's shared `matroska,webm` label as unknown, kept unknown
  container facts from being reinterpreted during checks, and aligned planning
  and execution to the exact MOV/H.264 remux and MP4/HEVC transcode sources.
- Restricted MP4/MOV brand interpretation to ISO-BMFF/QuickTime demuxer labels
  so a borrowed `isom`/`qt` tag cannot promote Matroska or WebM evidence.
- Refused transforms whose video width, height, or duration is unknown instead
  of emitting a plan the runtime would reject as forged.
- Quoted the public-readiness secret-scan pattern so Git does not treat PEM
  markers as options, and printed preservation claims as readable phrases
  instead of debug enum names.
- Stopped container `parse_loose` from substring-matching probe soup, so
  `matroska,webm` stays unknown instead of becoming WebM, and printed unknown
  containers as `unknown (...)` rather than `MATROSKA,WEBM`.
- Load the FFmpeg transform provider only when an adaptation plan needs
  execution; compatible and unsatisfiable paths no longer construct it.
