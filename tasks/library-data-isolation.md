# Task: Library data isolation and newer-schema errors

## Goal

Prevent development builds from breaking the installed release's music
library, and make a database written by a newer version produce a concrete,
actionable error instead of a generic storage failure.

## Scope

### In scope

- Keep debug-build library data, copied background images and the mpv session
  log in a `dev` subdirectory of the application data directory.
- Keep release builds on the existing application data location.
- Return the dedicated non-retryable `library_too_new` error when the database
  schema is newer than the build supports.
- Preserve the open error in the managed library state so every library command
  reports the same reason.
- Show the newer-version message in the library banner and the settings footer;
  keep the existing wording for unrelated failures.
- Tests for the version guard, the directory split and the renderer message.

### Out of scope

- Merging or migrating a database written by a newer version.
- Automatic movement of an existing development database into `dev`.
- Changing the release identifier or the asset-protocol scope.
- Recovery of data after an actual downgrade without a backup.

## Context

- Relevant files: `crates/application/src/lib.rs`, `src-tauri/src/library.rs`,
  `src-tauri/src/lib.rs`, `src-tauri/src/playback.rs`, `src/api.ts`,
  `src/App.tsx`, `src/app.test.tsx`.
- Related decision: `docs/decisions/0018-library-data-isolation.md`.
- The failure was reproduced with a real schema-v4 database and the published
  0.1.0 executable; a fresh database at version 3 opened normally.

## Verification

- Frontend 42 tests passed, including the newer-library message.
- Rust workspace tests passed: 65 passed, 4 ignored.
- rustfmt and strict clippy clean.
- Tauri debug build passed.
- Manual check: create a database with `PRAGMA user_version=5`, launch the app
  and confirm the actionable message appears; confirm a normal database still
  opens and the folder picker works.
