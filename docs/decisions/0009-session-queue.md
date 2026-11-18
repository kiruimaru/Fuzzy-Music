# 0009: Editable session queue and volume settings port

Date: 2026-11-19. Status: implemented; contributor review pending.
Related issues: #22 (playback and queue), #21 (settings persistence).

## Queue ownership and transitions

React owns an in-memory sequence of normalized Tracks with distinct entry keys.
Duplicate tracks are allowed. Search and source-playlist discovery cannot replace
this sequence. Adding does not start playback; Play on a track search result
appends an entry and selects it. Queue entry selection, Previous, Next and replay
preserve the sequence. There is no wraparound, persistence, shuffle or repeat.

Removing the current entry stops playback and cancels resolving through a newer
playback generation. It leaves the remaining list without a current selection;
the user explicitly selects the next entry. Removing another entry keeps the
current track and adjusts its index. Stop retains the selected entry and list.
At natural EOF the next entry starts, or the player stays stopped at the end.
Resolve/playback errors disable automatic advancement until manual selection.

Session entries use `play_track` with `fresh: true`. Rust kills the previous
player before fresh resolution, ignores cached/prepared stream links and cached
HLS audio, and does not launch audio warmers. Every selection and EOF transition
resolves at launch; pause/resume stays on the same player. Session transitions
are sequential and do not promise gapless playback. Search, playback and source
playlist request generations stay separate.

Public source playlist playback remains a separate existing mode with prepared
mpv transitions and lookahead. Its track list is not the editable session queue.
Adding to the session queue while a source playlist plays does not change the
source playlist's active sequence. Selecting a session entry replaces that mode
through a newer playback generation. Preparation failures are reported once and
wait for manual selection instead of repeatedly retrying.

## Volume integration

`src/settings.ts` defines `AppSettings` and the `SettingsStore` interface.
The #22 UI reads it before enabling Play, passes preferred volume with every
play command, and saves successful changes through serialized writes. mpv gets
initial volume in its startup arguments, avoiding a full-volume startup burst.
Snapshots still reflect observed mpv properties. Volume is finite, 0..100;
default is 100, and Stop preserves it.

Until #21 provides its API, the production adapter is explicitly in-memory.
It does not restore volume after application exit. Tests inject this same
interface with loaded values, delayed writes and storage failures. No SQLite,
localStorage or competing persistence layer is introduced.

Proposed #21 commands are documented in the module contracts. Contributor
agreement, durable storage, human listening and Windows acceptance remain
separate work; this decision does not claim those have happened.
