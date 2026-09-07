# Verification

2026-09-07. Linux x86_64. Jcode v0.81.5, Herdr 0.8.2, Rust 1.98.0.

## Real Herdr acceptance

Run `herdr-plugin-live-yvtt4yhq`. Real Herdr 0.8.2, two Jcode clients, isolated daemon. No model messages, no user profile changes.

| Requirement | Check | Result |
| --- | --- | --- |
| Four hooks configured | Config inspection after setup | `session_start`, `turn_start`, `turn_end`, `session_end` present |
| Lifecycle state reported | `pane report-agent` via Herdr CLI trace | Two calls: `--state idle --source custom:leonardoacosta.herdr-jcode --agent jcode` |
| Producer sequence | `--seq` arg | Consecutive `1788789131840676478`, `1788789131840676479` |
| Agent recognized | `herdr pane get w1:p1` | `"agent": "jcode"` |
| Status visible | Same response | `"agent_status": "idle"` |
| No native identity claims | Traffic: zero `report-agent-session` | Correctly gated behind `unsupported_host` |
| Two distinct clients | Witness: two pane IDs, two session IDs | `session_tulip` (w1:p2), `session_rose` (w1:p1) |
| Existing observer preserved | Witness ran alongside plugin | Both sessions reached witness |
| Idempotent setup | Second setup, config bytes | No change |
| Disabled guard | Disable + synthetic report | `skipped`, `reason: disabled_or_unlinked` |
| Doctor on stock Herdr | Direct binary + Herdr action | Exit 2, `status: unsupported_host` |
| Removal | Remove action, config comparison | Original restored |
| Unlink | Real unlink, filtered list | Empty |
| Cleanup | Clients terminated, daemon stopped | No remaining processes |

Raw Herdr traffic:

```
pane get w1:p2
pane get w1:p1
pane report-agent w1:p2 --source custom:leonardoacosta.herdr-jcode --agent jcode --state idle --agent-session-id session_tulip_... --seq 1788789131840676478
pane report-agent w1:p1 --source custom:leonardoacosta.herdr-jcode --agent jcode --state idle --agent-session-id session_rose_... --seq 1788789131840676479
```

Real `pane get` response:

```json
{"agent": "jcode", "agent_status": "idle", "pane_id": "w1:p1"}
```

No fixture, stub, or mock. Real Herdr binary, real Jcode binary, isolated disposable TUI.

## Pipeline

```sh
cargo fmt --check        # pass
cargo clippy --locked --offline --all-targets -- -D warnings  # pass
cargo test --locked --offline    # 42 tests pass
cargo package --allow-dirty --locked --offline  # 18 files, 83.9 KiB
cargo build --release --locked --offline  # pass
```

## Test breakdown

**18 library tests:** config composition, scalar-to-array, array order/comments, duplicate setup, exact removal, mixed-array rejection, inline tables, malformed TOML, four-event validation. Filesystem: absent file/directory, symlink rejection, permission preservation, lock refusal/cleanup, path quoting. Runtime: native schema, detector rejection, registration guard, output cap, timeout.

**7 CLI tests:** help, setup/removal round-trip, manifest validation, env override refusal (all four hooks), report fail-open, doctor machine-readable output.

**7 config safety tests:** scalar composition, reinstall position/comments, mixed-array rejection, malformed config cleanup, inline table preservation, argv escaping, missing directory creation.

**10 synthetic protocol tests:** all four lifecycle events with fixture receiver, attach/resume preservation, native identity alongside state, disabled/foreign-agent guards, unknown event/source rejection, discarded reports, best-effort ordering, invalid sequence fail-open.

## Jcode producer sequence

`crates/jcode-base/src/hooks.rs` on branch `dev` at `f565c14c6`. Per-event process-scoped monotonic counter. Exported as `JCODE_HOOK_SEQUENCE` env and JSON `sequence` field. Generic hook enhancement, not copied from any fork.

## Remaining limits

- **Native restore** needs Herdr-side Jcode session recognition. Stock Herdr 0.8.2 lacks it.
- **Full lifecycle authority** needs Jcode-side `blocked`, approval, interrupt, crash emitters.
- **Real-model turns** need a provider API key.
- macOS and Windows are untested.
- Crash-interruption, editor races, and symlink/ownership races are not exhaustively proven.