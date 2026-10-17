# Fitifact design system

One system serves the web app, the desktop app (which loads the same `web/dist`
build), and the documentation site.

| Layer | File | Used by |
| --- | --- | --- |
| Tokens | `web/public/brand/tokens.css` | web, desktop, docs |
| Components (`ft-*`) | `web/public/brand/components.css` | web, desktop, docs |
| Theme bootstrap | `web/public/brand/theme.js` | web, desktop, docs |
| App layout and app-only parts | `web/src/styles.css` | web, desktop |
| Docs bridge to Fumadocs | `apps/docs/app/styles.css` | docs |

The docs build copies `web/public/brand` into `apps/docs/public/brand`
(`apps/docs/scripts/sync-brand.mjs`), so all three surfaces render from the
same files. Change a token once, in `tokens.css`.

## Themes

`<html data-theme>` is `system`, `light`, or `dark`. The default is `system`,
which follows `prefers-color-scheme`. The choice is stored locally under
`fitifact.theme` and shared by the app and the docs. Dark is the base palette;
light overrides it for `[data-theme="light"]` and for `system` when the OS
prefers light.

Every text token meets 4.5:1 against `--bg`, `--panel`, and `--raised` in both
themes. `--line-strong` meets 3:1 for control boundaries.

## Tokens

| Group | Tokens |
| --- | --- |
| Type | `--font-sans` (Geist), `--font-mono` (Geist Mono), `--text-xs` 12 → `--text-2xl` 30, `--text-display`, `--leading`, `--leading-tight`, `--weight-regular/medium/bold`, `--tracking-tight/display/caps` |
| Space | `--space-1` 4 … `--space-16` 64 |
| Radius | `--radius-sm` 4, `--radius` 8, `--radius-lg` 12, `--radius-pill` |
| Layout and motion | `--tap` 44px minimum target, `--page-max`, `--ease`, `--duration-fast`, `--duration` |
| Layers | `--z-overlay`, `--z-dialog`, `--z-skip` |
| Surfaces | `--bg`, `--panel`, `--raised`, `--scrim`, `--shadow-overlay` |
| Text and lines | `--ink`, `--muted`, `--subtle`, `--line`, `--line-strong`, `--focus` |
| Action | `--accent`, `--accent-ink` |
| Tones | `--success`, `--warning`, `--danger`, `--info`, each with a `-surface` tint |

## Components

| Class | Use |
| --- | --- |
| `ft-btn` | Primary action. One per state. |
| `ft-btn ft-btn-secondary` | Other actions (Copy, Share, Retry, Cancel). |
| `ft-btn ft-btn-text` | Quiet actions and recovery links. |
| `ft-btn-block` | Full-width modifier for the state's main action. |
| `ft-card` | A surface: the status card, the file card, the inspecting card. |
| `ft-notice` | A callout. Add a tone class for meaning. |
| `ft-tone-success`, `-warning`, `-danger`, `-info` | Set `--tone` and `--tone-surface` for notices, badges, and icons. |
| `ft-badge` | A short status label. |
| `ft-field` | Text inputs, selects, and text areas. |
| `ft-eyebrow` | The small label above a title. |
| `ft-numeric` | Tabular numbers for sizes, dimensions, and counts. |

Status never relies on color alone: every tone pairs an icon with words, and
the measured table marks each value as Pass, Fail, or Not measured.

## States

Each state is one designed screen. `web/e2e/states.spec.ts` drives every state,
runs axe on it in both themes, and captures it at 1280×900 and 390×844.

| State | When | Title | Tone | Main action |
| --- | --- | --- | --- | --- |
| Empty | Nothing chosen yet | Make your file work there. | neutral | Choose a file, or drop one |
| Inspecting | Reading the file locally | Inspecting {file} | neutral | Cancel processing |
| Consent | A crop or first-frame change needs approval | Crop approval required | info | Approve, then Adapt and download |
| Adapting | Executing the plan | Working locally, with progress | neutral | Cancel processing |
| Already fits | The file meets every requirement | Already compatible. Nothing changed. | success | Use original |
| Adapted | The output passed validation | Adapted and verified | success | Download |
| Warning | A result with rules Fitifact ignored, or destination rules not re-checked in 30 days | (result title) plus "Check before you rely on it" | warning | Download, after reading the caveat |
| Refusal | Unsupported input, or requirements no output can meet | Fitifact can't adapt this file / Cannot satisfy these requirements | refusal | Change requirements |
| Error | Processing failed | Could not process this file | danger | Retry or Change requirements |

## The main path

A file that needs no consent takes three interactions: drop it, pick a
destination, and choose **Adapt and download**. The plan (what fails, the
minimum change, what is kept) is on screen before that button, so every plan is
inspectable before execution. The download starts only after the output passes
validation; a failed validation shows the error state and saves nothing. A file
that already fits takes the same three: drop, destination, **Use original**.

Results show:

- each requirement with the measured value before and after;
- the original and the output side by side;
- what changed, why, and what was kept, in plain words;
- Download everywhere, plus Copy and Share where the browser can hand over the
  exact validated file.

## Accessibility and motion

- Every interactive target is at least `--tap` (44px).
- Focus is always visible (`--focus` outline).
- Every primary journey completes with the keyboard alone
  (`web/e2e/keyboard.spec.ts`).
- `prefers-reduced-motion` turns off transitions and the inspecting pulse.

## Privacy

The web build's Content Security Policy allows connections only to its own
origin (`connect-src 'self'`). The desktop build adds Tauri IPC. The Vite
plugin in `web/vite.config.ts` writes the final policy. `web/e2e/privacy.spec.ts`
asserts that a full journey makes no cross-origin request and that the policy
refuses one.
