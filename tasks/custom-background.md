# Task: Custom player background

## Goal

Let the user personalise the player with their own background image and tune
its visibility live, without leaving the Settings view.

## Scope

### In scope

- Choose an image file of any common format; the app keeps its own copy.
- Live opacity slider (0–100%) applied while dragging and persisted.
- Restore the chosen image and opacity on the next launch.
- Remove the background and return to the plain theme.
- Keep the image behind all panels and above the theme background.

### Out of scope

- Image cropping, scaling controls, colour filters, or slideshows.
- Transcoding unsupported formats.
- Cloud sync of the selected image.

## Context

- Relevant files: `src/App.tsx`, `src/style.css`, `src/api.ts`,
  `src/settings.ts`, `src-tauri/src/library.rs`, `src-tauri/src/lib.rs`,
  `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`.
- Related decision: `docs/decisions/0015-custom-background.md`.
- Settings schema version 4: `background_image`, `background_opacity`.
- Asset protocol scope: `$APPDATA/**`; the copy lives in `background/`.
- Default opacity: 60%.

## Verification

- Frontend 38 tests passed, including choose / live opacity / remove.
- Rust shell tests 35 passed, 3 ignored, including settings round-trip and
  validation; workspace tests run before publishing.
- TypeScript/Vite build and Tauri debug no-bundle build pass.
- Manual check: pick a JPEG and a PNG, drag opacity, restart the app, then
  remove the background.
