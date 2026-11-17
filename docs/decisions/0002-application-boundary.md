# 0002: Runtime-independent application boundary

Date: 2026-11-17. Status: reviewed and merged in PR #15; shell integration pending.

Keep the existing synchronous SoundCloud adapter unchanged and introduce
`fuzzy-application` for source-agnostic serializable DTOs and safe errors.
Do not add Tauri, Tokio, playback, source registries, or global request state.

Retain nullable Track.streamUrl but leave it null; resolve returns a separate
temporary URL. Add sourceUrl so the UI can pass the adapter permalink back
without constructing a URL or requiring an in-memory track registry. Define
author fallback, safe millisecond conversion, no-pagination semantics and
error policies in `../architecture/contracts.md` alongside the implementation.

These are new implementation decisions, not an assertion of earlier agreement.
Shell owners must use blocking workers and local generation guards, retain
per-operation cancellation, and review these contracts before wiring IPC.
