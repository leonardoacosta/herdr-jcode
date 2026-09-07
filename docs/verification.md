# Verification evidence

Date: 2026-09-07. Linux x86_64. Installed Jcode v0.81.5 (`f46f9c354`), installed Herdr 0.8.2, Rust 1.98.0. No upstream branch or external fork code was imported. The standalone package does not change either host binary.

Jcode received an independent producer-sequence metadata addition (`JCODE_HOOK_SEQUENCE`) in the local working tree. This is a generic hook feature, not copied from any external fork. Without it, ordering is best-effort; with it, Herdr can reject stale events.

## Real public-interface acceptance (lifecycle replacement)

**Run:** `herdr-plugin-live-yvtt4yhq`, 2026-09-07. Jcode `v0.81.5-dev` with `JCODE_HOOK_SEQUENCE` from local branch `f565c14c6`. Plugin release binary `7e27acc`. Real installed Herdr 0.8.2, two real Jcode clients, isolated daemon.

**No model message sent. No user profile modified.**

### Requirement-to-evidence map

| Requirement | Concrete public check | Observed result |
| --- | --- | --- |
| Four lifecycle hooks configured | Inline TOML check after real setup | `session_start`, `turn_start`, `turn_end`, `session_end` all present with plugin command |
| Lifecycle state reported | Real `pane report-agent w1:p2` and `w1:p1` via Herdr CLI trace | Two calls: `--state idle --source custom:leonardoacosta.herdr-jcode --agent jcode` |
| Producer sequence forwarded | `--seq` argument in each report-agent call | Consecutive values `1788789131840676478`, `1788789131840676479` |
| Jcode recognized as agent | Real `herdr pane get w1:p1` via public Herdr API | `"agent": "jcode"` |
| Idle status visible | Same pane get response | `"agent_status": "idle"` |
| Native identity NOT claimed on stock Herdr | Traffic analysis: no `report-agent-session` calls | Zero native identity calls; correctly gated behind `unsupported_host` detector |
| Two distinct clients | Witness records two `pane` IDs and two `session` IDs | `session_tulip` (w1:p2), `session_rose` (w1:p1) |
| Existing observer preserved | Pre-existing witness observer ran alongside plugin | Both sessions reached witness |
| Idempotent setup | Second setup invocation, config bytes compared | No change |
| Enabled-state guard | Real disable + synthetic report | `skipped`, `reason: disabled_or_unlinked` |
| Doctor unsupported host | Direct binary + Herdr action | Exit 2, `status: unsupported_host`, `native_detector: false` |
| Safe removal | Remove action, config compared to original | Original restored |
| Safe unlink | Real unlink, filtered list | Empty plugin list |
| Isolation cleanup | Both clients terminated, daemon stopped, argv scan | No remaining processes |

### Raw Herdr API calls (from traffic trace)

```
pane get w1:p2
pane get w1:p1
pane report-agent w1:p2 --source custom:leonardoacosta.herdr-jcode --agent jcode --state idle --agent-session-id session_tulip_... --seq 1788789131840676478
pane report-agent w1:p1 --source custom:leonardoacosta.herdr-jcode --agent jcode --state idle --agent-session-id session_rose_... --seq 1788789131840676479
```

### Real Herdr pane get response (public API)

```json
{"agent": "jcode", "agent_status": "idle", "pane_id": "w1:p1", "terminal_title": "🌹 jcode Rose"}
```

These are real, unmodified Herdr API responses from the disposable isolated TUI. No fixture, stub, or mock server was used for acceptance.

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