# Lifecycle scope correction and existing-plugin comparison

Date: 2026-09-07. This is a source comparison, not certification of another author's fork.

## Scope correction

The user's goal includes Herdr awareness of Jcode lifecycle state. Our initial v0.1.0 implementation only installs `session_start` and attempts native identity reporting. It deliberately skips state mutation on stock Herdr 0.8.2. That implementation does not satisfy the complete lifecycle goal. Native restore support and custom working/idle reporting are separate capabilities. The native-support blocker should not have been treated as a blocker for all lifecycle reporting.

## Current external source

- Plugin HEAD verified through GitHub CLI: [`fc20b04afc4909c903c6e3e34f68b0f40d1950a8`](https://github.com/capt-marbles/herdr-jcode-integration/tree/fc20b04afc4909c903c6e3e34f68b0f40d1950a8).
- Its only implementation script, `scripts/manage.sh`, delegates install/status/uninstall to `jcode integration ... herdr`. It requires `capt-marbles/jcode`, branch `feat/herdr-integration`.
- Fork HEAD reviewed read-only: [`cf3dba91bcaae7cf8401df126d2de45142f9ddd6`](https://github.com/capt-marbles/jcode/tree/cf3dba91bcaae7cf8401df126d2de45142f9ddd6). No source imported or executed.
- `src/cli/herdr.rs` installs four hooks. `session_start` and `turn_end` report idle, `turn_start` reports working, and `session_end` releases the report. Reports use `custom:jcode`, `pane report-agent`, and optional session ID and sequence fields.
- `crates/jcode-base/src/hooks.rs` adds `JCODE_HOOK_SEQUENCE` at event creation. An atomic counter takes the maximum of wall-clock nanoseconds and the previous value plus one. This orders event creation within one process even if detached reporters arrive late. It does not establish a restart/clock-rollback or multi-process ownership guarantee.
- Fresh installed Jcode v0.81.5 invocation of `integration status herdr --json` rejects the `integration` subcommand. The competing plugin cannot install against this stock binary.

Firecrawl 1.19.27 search succeeded and again found the public directory entry describing the package's lifecycle behavior. Scrape help still exposes no per-redirect control required by the loaded skill, so no scrape was performed. Exact code/revisions came from separate read-only GitHub CLI source review. Temporary public-source evidence is in the session scratch `herdr-lifecycle-comparison` directory, mode 0700, with files mode 0600.

## Comparison

| Capability | Existing package plus required fork | Our committed v0.1.0 |
| --- | --- | --- |
| Working/idle lifecycle | Four-hook custom state reporter | Missing; only session_start is installed |
| Release | session_end calls release-agent | Missing |
| Producer ordering | Process-local monotonic event sequence | No producer sequence and no complete ordering guarantee |
| Stock Jcode support | Requires integration-enabled fork | Uses existing hooks on v0.81.5 |
| Native restore | Custom state payload is not proof of native restore | Explicit capability check and exact native identity readback, blocked on stock Herdr |
| Config preservation | Preserves command strings, reconstructs arrays, filters non-string elements, directly writes config | Rejects invalid arrays, preserves comments/order, atomic replacement, lock, permission and symlink checks |
| Reporter safeguards | Shell commands, no explicit timeout or plugin-enabled check in reporter | Bounded subprocesses/output, environment minimization, enabled-state guard and structured diagnostics |
| Validation available here | Source inspection and stock-command rejection, no fork runtime executed | 35 tests, packaged build, two real two-client runs covering our narrower scope |

Both implementations lack demonstrated complete permission/question blocking, cancellation, crash cleanup and ownership-transfer behavior. The existing package's README says authoritative lifecycle, but inspected Herdr classifies full-lifecycle sources with an allowlist that does not include `custom:jcode`. Do not equate a custom report with the official native/full-lifecycle contract.

## Corrected recommendation and acceptance

Keep the independent packaging and config safety, but add a separate custom-state path that does not require a native Jcode detector. Implement session/turn state and release as a limited lifecycle feature, not complete blocked-state authority. Reconcile attach/resume with in-flight work rather than blindly reporting idle. Resolve producer ordering and ownership before claiming reliable transitions; an adapter timestamp cannot reconstruct producer order. If generic Jcode event metadata must change, implement it independently on our own branch, not by copying the external fork.

Full lifecycle acceptance must exercise actual Jcode turns through real Herdr, observe working then idle, and cover cancellation, blocking/resolution, normal exit, abrupt disconnect, delayed old events, replacement, and two clients sharing a daemon. Stock-Herdr custom state/release support was already observed via its real public API in the Jcode repository's `docs/plans/2026-09-07-herdr-live-api-evidence.json`; that probe used synthetic event inputs and is not the missing end-to-end lifecycle test.

Our current advantages are compatibility, defensive installation and diagnostics, not lifecycle feature completeness. Implementation of the corrected lifecycle scope remains open. Native restore and marketplace publication remain separate open items.
