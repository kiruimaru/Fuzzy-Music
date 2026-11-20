# Task: Player motion

## Goal

Make the player feel responsive: smooth hover, press, drag and panel states with
short, calm transitions that never delay playback or block clicks.

## Scope

### In scope

- Motion tokens for duration and easing shared by player surfaces.
- Transport buttons: hover lift, press feedback, animated Play/Pause glyph swap.
- Progress and volume sliders: track and thumb feedback on hover and while dragging.
- Volume popover: animated enter and exit.
- Queue panel: animated entrance from the queue button.
- Track changes: now-playing artwork and text fade in; active-row highlight moves smoothly.
- Now-playing title: gentle marquee only when the text does not fit.
- Reduced-motion support.

### Out of scope

- Layout changes, new panels or new dependencies.
- Animated queue exit (the panel unmounts immediately to keep rapid toggles instant).
- Playback engine or backend changes.

## Context

- Relevant files: `src/style.css`, `src/App.tsx`.
- Related decision: `docs/decisions/0013-player-motion.md`.
- Durations: 140 ms fast, 180 ms base, 240 ms slow; easing
  `cubic-bezier(.2, .7, .3, 1)`; no bounce.
- Animations are compositor-friendly (opacity, transform, colours) and do not
  intercept pointer events.

## Verification

- Frontend 37 tests passed; TypeScript/Vite build passed (CSS 18.71 kB, JS 270.06 kB).
- Integration validation on 2026-11-20: 60 Rust tests passed, four ignored;
  strict Clippy, rustfmt, context smoke and Bash syntax checks passed.
- Eight Chromium fixture scenarios passed at 1000px and 320px after supplying
  the missing `playlist_entries` response in each fixture. Long-title overflow,
  the hidden duplicate and reduced-motion animation suppression are asserted.
- Native Tauri debug/no-bundle build passed on Linux.
- Manual check: quick skipping and rapid queue toggles stay immediate; sliders
  respond while dragging; volume popover fades out without blocking the player;
  long titles glide only when clipped, short titles stay still.

## Integration handoff (2026-11-20)

- Added fixture/regression coverage in `5fcfe36`; PR #28 merged as `224ecbd`.
  PRs #24–#27 were already merged; no open PRs remained at the final check.
- Rebuilt `main` with `npm run tauri -- build --debug --no-bundle`.
- Launched the standalone binary as `fuzzy-music-test.service`; service is
  active and the native Fuzzy Music window is mapped, not hidden.
- Reused the existing local proxy with process-only environment overrides;
  SoundCloud returned HTTP 200. No proxy endpoint or personal path is stored here.
- Next: user listening, rapid skips, queue/volume interaction, real downloads
  and restart persistence. Windows and accepted lifecycle findings remain separate.
