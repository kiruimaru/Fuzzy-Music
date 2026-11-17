# Windows network integration over PR #17

## Base and ownership

Local branch `feat/windows-network-integration`, separate worktree, base PR #17
head `639f1ff`; fetched main remains `1931a58`. PR still open at inspection.
Existing review worktree and its uncommitted notes were preserved. The user has
authorized commit, push and a separate PR. PR #17 remains open; this follow-up
targets its actual head branch `feat/issue-14-tauri-shell`, not main, to avoid
duplicating its shell diff. Merge #17 first, then retarget this follow-up to main
and review its remaining diff. All new commits require both author and committer
`Maroi9p <134836326+Maroi9p@users.noreply.github.com>`.

## Implemented

Immutable startup network snapshot shared between source and mpv, Windows native
user proxy query, env precedence, HTTP/HTTPS manual mapping, simple bypass,
fixed safe diagnostics. See decision 0004 for limitations and restart strategy.
No UI additions or machine-wide changes. Legacy source API remains compatible.

## Actual checks (2026-11-17, Windows)

- `cargo test --workspace --all-targets --locked`: 23 passed, 2 ignored.
  Includes three policy regressions plus timeout, cancellation/descendant cleanup,
  Stop/late resolve and safe application errors.
- Strict all-target Clippy, cargo fmt check, git diff whitespace: passed.
- `npm ci`: passed, zero audit vulnerabilities; npm noted esbuild script policy.
- `npm test`: five passed; `npm run build`: passed.
- Browser fixture tests: initial default Linux executable missing; rerun with
  process-local CHROMIUM_PATH to existing Chrome: two passed.
- `npm run tauri -- build --debug --no-bundle`: passed, native EXE built.
- Initial Cargo compile exceeded 120s tool bound; resumed successfully.
- Initial strict Clippy found an unused compatibility wrapper; removed, rerun passed.

## Native runtime evidence

Started only the new worktree EXE using ProcessStartInfo, with ALL four proxy
environment variables removed (Windows case-insensitive). Only verified tool
paths and process-local accessibility were supplied. Did not inject a proxy.
Native API diagnostic: `Windows manual HTTP proxy policy`.
Actual UI search returned ten tracks. Selected the public HephestosMusic Mario
remix; fresh resolve reached Playing. Owned mpv PID 16308 had Established TCP to
loopback port 3067. Stop removed the owned mpv (count zero after one second).
Closed the test window; own app exited. Pre-existing user window was not touched.
This proves native system autodetection through search/resolve/media, without a
shell wrapper or proxy env injection. No secrets or temporary stream URLs logged.
WebView2 emitted an unregister-class warning on exit, but app cleanup succeeded.
The user subsequently confirmed the application works and searches for music.
Audio was previously confirmed by the user on the earlier build; this run only
claims Playing and network evidence, not a new listening confirmation.

## Handoff

Review decision 0004 and local diff. PAC/WPAD, advanced bypass, proxy UI and tool
packaging remain deferred. Restart is required after system proxy changes.
Publication is authorized; use Maroi9p identity exclusively. GitHub API verified
login Maroi9p, id 134836326 before publication. Non-Windows compilation uses no
Windows API; Linux runtime was not rerun for this follow-up. Frontend is unchanged;
the frontend/browser/native build results above are the prior checks of this diff.

## Publication

- PR: https://github.com/kiruimaru/Fuzzy-Music/pull/18; review requested from kiruimaru.
- Base: `feat/issue-14-tauri-shell` (#17); head: `feat/windows-network-integration`.
- Implementation commit: `c6bfb1a`; GitHub confirmed both author and committer
  login `Maroi9p`. Local author/committer use the required noreply identity above.
- Pre-publication Rust tests (23 passed, 2 ignored), fmt, strict Clippy and
  whitespace checks passed again. No merges performed; #14 remains open.
