# Task: Linux presentation fixes for 0.2.1

## Goal

Fix the three issues found during the first project presentation on the Linux
0.2.1 build.

## Scope

### In scope

- Make the vertical volume slider respond to vertical dragging on WebKitGTK.
- Keep an animated GIF/WebP/AVIF background running when the user leaves the
  Settings view.
- Record the intended Windows/Linux dependency-setup parity in the related task
  (`tasks/windows-in-app-runtime.md`).

### Out of scope

- Redesigning the player, moving the volume control, or adding new audio
  analysis.
- Changing the background storage format or asset protocol.
- Platform-specific rendering work beyond the reported Linux defects.

## Context

- `writing-mode: vertical-lr` painted the range input vertically on WebKitGTK,
  but pointer coordinates remained horizontal. A horizontal input rotated by
  -90 degrees makes both the painting and the hit testing vertical.
- The custom background lived in `body::before` as a CSS background image. The
  animated image was also rendered in the Settings preview; when that preview
  unmounted during navigation, WebKitGTK stopped the shared animation. A
  persistent `<img class="app-background">` outside the view content keeps one
  image element alive for the whole session.
- Related decision: `docs/decisions/0015-custom-background.md`.

## Verification

- Frontend tests cover the persistent background element and its survival when
  the Settings preview unmounts.
- The volume input keeps its accessible label and playback wiring.
- Manual Linux check: drag the vertical volume slider, navigate away from
  Settings with an animated background and confirm it keeps moving.
