# Verification evidence

Date: 2026-09-07. Linux x86_64. Installed Jcode v0.81.5 (`f46f9c354`), installed Herdr 0.8.2, Rust 1.98.0. No upstream branch code was imported. The standalone package does not change either host binary.

## Real public-interface acceptance

`tests/live_herdr.py` passed against actual installed Herdr and Jcode. It created fresh HOME/XDG directories, dedicated sockets, and a disposable Herdr TUI. Herdr itself supplied each managed pane's identity. Two real Jcode clients used the same isolated Jcode socket. No model message was submitted and no user profile or live session was modified.

The observer witness records the real hook environment. A CLI trace wrapper forwards every call unchanged to the real Herdr binary and records only arguments/exit status. Neither component invents producer events or receiver responses. The disabled-plugin probe separately supplies synthetic hook fields to the real plugin and real Herdr API.

| Requirement / public output | Concrete check | Observed result |
| --- | --- | --- |
| Valid standalone marketplace manifest | Real `herdr plugin link`, `plugin list`, and `plugin action list` | Manifest accepted, exact plugin ID enabled, three actions registered |
| Explicit setup | Real `plugin action invoke setup` | Session hook added while existing observer and comment remained |
| Idempotent setup | Invoke setup again and compare config bytes | No change |
| Use real Jcode hooks | Two actual Jcode clients started inside real managed panes | Two `session_start` callbacks, two distinct opaque session IDs, correct distinct pane IDs |
| Shared daemon attribution | Both clients used the same explicit Jcode socket | Reporter issued `pane get w1:p1` and `pane get w1:p2`, not the daemon's original pane twice |
| Existing user hooks keep working | Pre-existing observer ran alongside installed plugin | Both sessions reached the observer after setup |
| Honest unsupported-host behavior | Doctor via direct binary and Herdr action, real detector CLI from both hooks | `unsupported_host`, doctor exit 2, no native session mutation sent |
| Enabled-state guard | Real `plugin disable`, then invoke report against that real registry | `skipped`, reason `disabled_or_unlinked` |
| Removal | Real remove action followed by exact config comparison | Original array-form user config restored byte-for-byte |
| Unlink | Real `plugin unlink`, then filtered list | Empty plugin list |
| Isolation cleanup | Both clients terminated, isolated daemon stopped, Herdr TUI terminated, process argv scan | No processes remained for the disposable root |

Initial passing run: `herdr-plugin-live-nmvyxzue`. The final release binary passed again in `herdr-plugin-live-xatchjuy`. Both runs exercised two real clients. A process argv scan after the final run found no remaining processes for its disposable root. Raw witness/CLI/action responses remain in the session scratch directory, not in this repository. They are temporary test artifacts, not a production installation.

## Final delivery checks

The following complete pipeline passed with exit 0 on 2026-09-07:

```sh
cargo fmt --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline
cargo package --allow-dirty --locked --offline
cargo build --release --locked --offline
python3 tests/live_herdr.py /home/nyaptor/.local/bin/herdr /home/nyaptor/.local/bin/jcode
```

All 35 tests passed: 16 library tests, 6 public CLI tests, 7 config safety tests, and 6 synthetic reporter tests. Packaging included 15 files and successfully compiled the extracted crate. Formatting and Clippy passed without warnings. The live run then exercised the release binary through the real host interfaces mapped above. These results do not remove the native-host and publication limits below.

## Regression and failure-path coverage

`cargo test --locked` covers these named behaviors:

- Config composition: scalar-to-array, array order/comments, duplicate setup, exact removal, no-op removal, mixed-array rejection, inline tables, malformed TOML without extra files.
- Filesystem: absent file/directory, symlink file/parent rejection, permission preservation, existing-lock refusal, lock cleanup after success, path quoting with spaces/apostrophes/backslashes/quotes.
- CLI: help, setup/removal workflow, missing-host JSON diagnostics, environment override refusal without writes, report fail-open outside Herdr without payload disclosure.
- Runtime: actual Herdr native session `kind/value` schema, unknown-agent detector rejection, exact enabled registration, 64 KiB output cap, two-second timeout including an inherited output pipe.
- Synthetic native protocol tests: create/attach/resume source mapping, exact opaque-ID argv, positive native read-back, successful-but-discarded report, different identity refusal, disabled guard before pane access, unknown source without API calls.

The positive native tests explicitly use a fixture receiver. They are **not** proof that a stock Herdr host supports Jcode identity, restoration, or replacement. Regression tests exposed and corrected ignored scalar hooks, reordered array entries, accepted mixed-type arrays, unescaped paths, missing-directory handling, and initially incorrect response parsing before delivery.

## Remaining acceptance limits

1. **Native reporting/restore is blocked on host support.** Installed Herdr 0.8.2 has no registered Jcode detector/native session support. The plugin correctly diagnoses this. No native restore success is claimed.
2. Same-pane session replacement remains conservative and incomplete. A visible different identity is refused. Detached initial reports can race because Jcode does not supply a producer ordinal/generation. No complete ordering guarantee is claimed.
3. No GitHub repository was published, remote installation performed, or marketplace listing accepted. Local linking exercises the real plugin loader/actions, but does not replace GitHub install/index acceptance.
4. macOS and Windows are untested and absent from the manifest's platform list.
5. Crash-interruption, non-cooperating config editors, and ownership/symlink races are not exhaustively proven safe. See the README's lock and atomic-write limits.

## Research provenance

The design follows Firecrawl research of both Jcode and Herdr, Context7 indexed primary documentation cross-checked against source, and read-only review of Herdr PR #2248 at `70a219f1cfb32f5a2716f2805e3fb6e9e2466d1f`. Exact-version Context7 snapshots were unavailable. The existing marketplace package requires a Jcode fork, so its implementation was not reused. Neither its code nor the upstream PR was copied into this package.
