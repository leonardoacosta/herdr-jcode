# Verification evidence

Date: 2026-09-07. Linux x86_64. Installed Jcode v0.81.5 (`f46f9c354`), installed Herdr 0.8.2, Rust 1.98.0. No upstream branch or external fork code was imported. The standalone package does not change either host binary.

Jcode received an independent producer-sequence metadata addition (`JCODE_HOOK_SEQUENCE`) in the local working tree. This is a generic hook feature, not copied from any external fork. Without it, ordering is best-effort; with it, Herdr can reject stale events.

## Real public-interface acceptance

`tests/live_herdr.py` passed against actual installed Herdr and Jcode (run `herdr-plugin-live-nmvyxzue`, repeated `herdr-plugin-live-xatchjuy`). It created fresh HOME/XDG directories, dedicated sockets, and a disposable Herdr TUI. Herdr itself supplied each managed pane's identity. Two real Jcode clients used the same isolated Jcode socket. No model message was submitted and no user profile or live session was modified.

The observer witness records the real hook environment. A CLI trace wrapper forwards every call unchanged to the real Herdr binary and records only arguments/exit status. Neither component invents producer events or receiver responses. The disabled-plugin probe separately supplies synthetic hook fields to the real plugin and real Herdr API.

| Requirement / public output | Concrete check | Observed result |
| --- | --- | --- |
| Valid standalone marketplace manifest | Real `herdr plugin link`, `plugin list`, and `plugin action list` | Manifest accepted, exact plugin ID enabled, three actions registered |
| Explicit setup installs all four lifecycle hooks | Real `plugin action invoke setup` | All four hooks added while existing observer and comment remained |
| Idempotent setup | Invoke setup again and compare config bytes | No change |
| Use real Jcode hooks | Two actual Jcode clients started inside real managed panes | Two `session_start` callbacks, two distinct opaque session IDs, correct distinct pane IDs |
| Shared daemon attribution | Both clients used the same explicit Jcode socket | Reporter issued `pane get w1:p1` and `pane get w1:p2`, not the daemon's original pane twice |
| Existing user hooks keep working | Pre-existing observer ran alongside installed plugin | Both sessions reached the observer after setup |
| Honest unsupported-host behavior | Doctor via direct binary and Herdr action, real detector CLI from both hooks | `unsupported_host`, doctor exit 2, no native session mutation sent |
| Enabled-state guard | Real `plugin disable`, then invoke report against that real registry | `skipped`, reason `disabled_or_unlinked` |
| Removal restores hooks | Real remove action followed by config comparison | Original hooks restored; scalar-to-array conversion from install is preserved |
| Unlink | Real `plugin unlink`, then filtered list | Empty plugin list |
| Isolation cleanup | Both clients terminated, isolated daemon stopped, Herdr TUI terminated, process argv scan | No processes remained for the disposable root |

Raw witness/CLI/action responses remain in the session scratch directory, not in this repository. They are temporary test artifacts, not a production installation.

## Final delivery checks

The following complete pipeline passed with exit 0 on 2026-09-07:

```sh
cargo fmt --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline
cargo package --allow-dirty --locked --offline
cargo build --release --locked --offline
```

All 42 tests passed: 18 library tests, 7 public CLI tests, 7 config safety tests, and 10 synthetic reporter tests. Packaging included 15+ files and successfully compiled the extracted crate. Formatting and Clippy passed without warnings.

## Lifecycle reporting verification

Synthetic reporter tests cover all four lifecycle events with a fixture Herdr receiver:

- `turn_start` reports `working` state, `turn_end` and `session_start` report `idle`
- `session_end` calls `release-agent`
- Attach/resume on an already-working pane preserves working state
- Native session identity is reported alongside lifecycle state on `session_start`
- Disabled plugin guard, another-agent guard, and unknown-source guard all reject without mutation
- Herdr-discarded state reports return `not_confirmed`
- Missing producer sequence is explicitly `best_effort`; invalid sequence fails open
- Native identity detection remains separate on stock Herdr (`unsupported_host`)

Full lifecycle acceptance with real Jcode turns producing actual working/idle transitions in Herdr requires a model API key not available in the test environment.

## Jcode producer-sequence metadata

Independent addition in the local Jcode working tree (`crates/jcode-base/src/hooks.rs`): `JCODE_HOOK_SEQUENCE` and JSON `sequence` field, per-event process-scoped monotonic counter. See companion `docs/HOOKS.md` documentation. Without this, ordering is best-effort. This is a generic hook enhancement — not a fork, not a copy of external code.

## Regression and failure-path coverage

`cargo test --locked` covers these named behaviors:

- Config composition: scalar-to-array, array order/comments, duplicate setup, exact removal, no-op removal, mixed-array rejection, inline tables, malformed TOML without extra files, all-four-event validation before writes.
- Lifecycle: four-event setup/removal, public EVENTS constant, scalar-to-array for all events, removal preserves original content but not scalar form.
- Filesystem: absent file/directory, symlink file/parent rejection, permission preservation, existing-lock refusal, lock cleanup after success, path quoting with spaces/apostrophes/backslashes/quotes.
- CLI: help, setup/removal workflow, missing-host JSON diagnostics, environment override refusal for all lifecycle hooks without writes, report fail-open outside Herdr without payload disclosure.
- Runtime: actual Herdr native session `kind/value` schema, unknown-agent detector rejection, exact enabled registration, 64 KiB output cap, two-second timeout including an inherited output pipe.
- Synthetic lifecycle protocol tests: working/idle state, session-specific release, attach/resume preservation, native identity alongside state, unknown event/source rejection, disabled/foreign-agent guards, discarded reports, best-effort ordering, invalid sequence rejection.
- Synthetic native protocol tests: create/attach/resume source mapping, exact opaque-ID argv, positive native read-back, successful-but-discarded report, different identity refusal, disabled guard before pane access, unknown source without API calls.

## Remaining acceptance limits

1. **Native reporting/restore is blocked on host support.** Installed Herdr 0.8.2 has no registered Jcode detector/native session support. The plugin correctly diagnoses this. No native restore success is claimed.
2. **Same-pane session replacement remains conservative and incomplete.** A visible different identity is refused. Without a producer ordinal, detached initial reports can race. The Jcode branch's `JCODE_HOOK_SEQUENCE` addresses this once built.
3. **Jcode turn lifecycle has not been exercised with a real model.** The synthetic reporter tests verify the Herdr API interaction for all four events. Real-model end-to-end lifecycle acceptance requires a provider API key.
4. **No GitHub repository was published, remote installation performed, or marketplace listing accepted.** Local linking exercises the real plugin loader/actions, but does not replace GitHub install/index acceptance.
5. **macOS and Windows are untested** and absent from the manifest's platform list.
6. Crash-interruption, non-cooperating config editors, and ownership/symlink races are not exhaustively proven safe. See the README's lock and atomic-write limits.