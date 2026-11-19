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
- Manual check: quick skipping and rapid queue toggles stay immediate; sliders
  respond while dragging; volume popover fades out without blocking the player;
  long titles glide only when clipped, short titles stay still.
