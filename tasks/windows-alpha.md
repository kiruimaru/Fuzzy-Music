# Windows portable alpha

Base origin/main faa334a; #17/#18 verified merged. Separate worktree/branch
feat/windows-alpha; previous GUI/review worktrees preserved. GitHub active user
verified Maroi9p. User subsequently authorized commit/push/PR; no release/tag.

Implemented executable-relative runtime discovery with dev overrides, pinned
verified downloads, deterministic ZIP packaging, bilingual runtime guide and
explicit third-party provenance/remaining obligations. Decision 0005.

Actual local Windows checks: 24 Rust tests passed, 2 ignored; five frontend
tests passed; frontend and native optimized Tauri no-bundle builds passed.
Strict Clippy initially found collapsible-if; corrected and rerun passed.
Initial test tool timeout during first compile; complete rerun passed.
Portable GUI smoke from path with spaces, cwd Windows, PATH Windows-only,
FUZZY/proxy variables removed: ten live results, fresh resolve -> Playing,
one owned mpv connected to system proxy loopback 3067, Stop cleaned up and
own window exited. No new human listening claim. WebView2 exit class warning
did not prevent process cleanup. scripts/smoke-windows-portable.ps1 reproduces it.

Linux task: https://github.com/kiruimaru/Fuzzy-Music/issues/19 assigned kiruimaru.
No duplicate broad CI issue; links #8 and planned Windows branch.

Remaining before public-ready alpha: full license notices/corresponding source
inventory for exact bundled mpv/transitive FFmpeg build and yt-dlp executable;
clean Windows/runtime DLL/WebView2 verification; missing-dependency UI text
improvement/verification. Candidate is functional locally, not yet release-ready.
No clean Windows/Sandbox or Linux runtime claim. Await approval for commit/PR
and eventual pre-release only after remaining acceptance work is complete.

Final candidate after shell/Tauri version alignment rebuilt and live smoke
repeated successfully. ZIP listing inspected: app, docs/LICENSE, packaged mpv
archive content and yt-dlp only; no developer caches/logs. Size 76,568,308 bytes.
SHA256: 77ef93ecaad2dddb3c0b549d4a4007a2abbc158df1c9036a30f0d28ed33f525c.
Path: dist/release/Fuzzy Music 0.1.0-alpha.1 Windows x64.zip.
Final fmt and git diff whitespace checks passed. Branch contains intentional
uncommitted source/docs changes; generated tools/dist/target remain ignored.

## yt-dlp console fix (2026-11-17)

`crates/soundcloud/src/process.rs` now uses the command-group builder and its
Windows-only CREATE_NO_WINDOW flag, matching playback. Existing job cleanup,
timeout, cancellation and pipe handling remain intact. Portable smoke now samples
EnumWindows during live search/resolve and verifies an owned active extractor.
Rust workspace all-target locked tests: 24 passed, 2 ignored; fmt check and strict
Clippy passed. Optimized Tauri build passed; existing packaging script succeeded
after an initial download timeout. Vite build cleared the previous ignored dist
output; current EXE/ZIP were regenerated. No licensing gate blocked local packaging;
the public-readiness limitations above remain.

Updated ZIP: `dist/release/Fuzzy Music 0.1.0-alpha.1 Windows x64.zip`, 76,568,263 bytes.
SHA256: `0db422966513fbfdd0cd3fd1be487238a215bf2c31686503d21d711f113cffcc`.
Actual portable GUI: ten results, Playing, packaged tools/mpv.exe, system proxy
loopback 3067 with proxy/FUZZY env removed and Windows-only PATH. EnumWindows:
641 samples, 204 with an owned active yt-dlp, zero new visible console windows.
Stop cleaned owned mpv; own app exited; existing user windows untouched. Sampling
does not exclude flashes between observations; no human listening or clean-machine
claim. WebView2 repeated its exit class warning without preventing cleanup.
No commit/push/release performed at that earlier checkpoint.

## Distribution completion (2026-11-17)

Upstream mpv release/build artifacts inspected: no complete corresponding-source
bundle; recipes use many moving Git dependencies. Chosen bootstrap distribution
excludes external executables and downloads pinned/hash-verified upstream assets
on the user's machine. One extra documented setup step; complete mpv archive and
yt-dlp upstream license texts retained after acquisition, provenance JSON emitted.
Application source, locked vendored Rust dependency sources/licenses, frontend
runtime package sources and npm notices accompany the executable. No incomplete
source offer is claimed. Decision 0005 and THIRD-PARTY.md explain the change.

Added actionable safe dependency spawn errors and missing/corrupt regressions.
Windows PR/manual workflow builds ZIP+SHA as a 14-day Actions artifact; no release.
Packaging output moved outside Vite's cleared dist to artifacts/windows-alpha.
An earlier dist candidate was cleared by Vite build; source/fix remain preserved.
First packaging vendoring exceeded tool timeout; retry with adequate timeout passed.
Strict Clippy found test-module ordering, corrected; rerun passed. Native release
build and frontend tests/build passed. Fresh bootstrap downloaded/extracted exact
tools successfully; GUI on developer Windows host: ten results, Playing, system
proxy connection, Stop/exit passed; 626 console samples, 337 active-extractor
samples, zero new consoles. WebView2 exit warning 1412 did not prevent cleanup.
No human listening or clean-machine claim. External Windows/WebView2 smoke remains.

Local bootstrap ZIP (before local runtime acquisition into staging):
`artifacts/windows-alpha-final/Fuzzy Music 0.1.0-alpha.1 Windows x64.zip`.
SHA256 `dbd49d99e19e1ddfb23001a7b527b5229005b5f1a11aa306ee90541918dbd663`.
Staging now also has locally acquired tools; ZIP itself excludes them.

## Publication

Implementation commit `57efb2491c841aae1bf9b62f127e861109c3c340` pushed normally
to `origin/feat/windows-alpha`; both local identities and GitHub API author/committer
logins verified as Maroi9p (noreply id 134836326). PR:
https://github.com/kiruimaru/Fuzzy-Music/pull/20 ; review requested from kiruimaru.
Windows CI run https://github.com/kiruimaru/Fuzzy-Music/actions/runs/44081260899
passed fmt, 27 Rust tests/2 ignored, strict Clippy, five frontend tests, native
release build and packaging. Downloadable artifact (14-day retention):
https://github.com/kiruimaru/Fuzzy-Music/actions/runs/44081260899/artifacts/14327467326 .
Artifact container size 77,507,976 bytes; includes CI-built ZIP and SHA256 sidecar.
Local ZIP size 81,459,498 bytes (different compiler build/source snapshot from CI).
No release/tag/merge. Clean-machine GUI validation remains external; hosted CI
build success does not establish that validation.
