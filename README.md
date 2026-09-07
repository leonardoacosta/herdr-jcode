# herdr-jcode

Standalone Herdr plugin that reports Jcode lifecycle state (working/idle) plus session identity. Four-hook observer: `session_start`, `turn_start`, `turn_end`, and `session_end`. Independently implemented; no fork dependency. No external Jcode or Herdr patches required.

**Custom lifecycle state works on stock Herdr 0.8.2.** Native Jcode session identity/restore still requires a Jcode-capable Herdr build. The plugin reports working/idle lifecycle state alongside best-effort session identity.

## Requirements

- Linux. Other platforms are not advertised or tested.
- Rust/Cargo to build. Tested with Rust 1.98.0.
- Jcode v0.81.5 or later with hook arrays and client-scoped terminal environment. Earlier versions are not certified.
- Jcode working tree with `JCODE_HOOK_SEQUENCE` metadata (local branch) for best-effort ordering to become producer-sequence ordering. Without it, reports are `best_effort`.
- Herdr 0.8.2 or later for plugin actions and custom state reporting. Its version alone does **not** establish native Jcode session support.
- Python 3 for tests only, not plugin runtime.

## Build and link locally

```sh
cargo build --release --locked
herdr plugin link "$PWD"
herdr plugin action invoke doctor --plugin leonardoacosta.herdr-jcode
herdr plugin action invoke setup --plugin leonardoacosta.herdr-jcode
```

Run these from a Herdr-managed terminal. Actions run asynchronously. Inspect completion and output with `herdr plugin log list --plugin leonardoacosta.herdr-jcode`.

Setup is explicit. Build, link, and startup do not modify Jcode config. Setup appends the absolute executable path plus `report` to `[hooks]` for `session_start`, `turn_start`, `turn_end`, and `session_end`. It resolves config from `JCODE_HOME/config.toml`, otherwise `~/.jcode/config.toml`. Start a new Jcode daemon or reload its configuration after setup/removal. Existing daemons may cache hooks.

For an explicit configuration path or direct diagnostics:

```sh
./target/release/herdr-jcode setup --config /absolute/path/config.toml
HERDR_BIN_PATH="$(command -v herdr)" ./target/release/herdr-jcode doctor
./target/release/herdr-jcode remove --config /absolute/path/config.toml
```

Keep the binary at its configured path. If moving to another checkout/install location, remove the old registration using the old binary before setup at the new location.

## What it does

- Adds `session_start`, `turn_start`, `turn_end`, and `session_end` observers to `[hooks]`. Start a new Jcode daemon after setup; existing daemons cache configuration.
- `turn_start` reports `working` state to Herdr via `pane report-agent`. `turn_end` reports `idle`. `session_end` calls `pane release-agent` so Herdr hides agent state on exit.
- `session_start` reports `idle` (or preserves `working`/`blocked` on attach/resume) and separately attempts native session identity via `pane report-agent-session`.
- Uses `custom:leonardoacosta.herdr-jcode` source for custom state reporting, `herdr:jcode` for native identity. These are distinct; native identity requires a Jcode-capable Herdr build.
- Forwards `JCODE_HOOK_SEQUENCE` when present (requires local Jcode branch with sequence metadata). Without it, reports are `best_effort`.
- Preserves unrelated settings and existing user hook commands, including comments. Converts a scalar hook to an array when composing. Repeated setup is byte-identical. Removal removes only the exact owned command from each event, but does not convert a remaining one-element array back to its original scalar form.
- Uses a configuration lock, same-directory temporary file, restricted initial permissions, content/inode rechecks, and atomic replacement. Refuses symlinked configuration paths/ancestors, multiple hard links, invalid TOML, mixed-type hook arrays, and ownership changes it cannot preserve.
- Refuses setup when any `JCODE_HOOK_{SESSION_START,TURN_START,TURN_END,SESSION_END}` environment variable overrides config.
- Reads only hook event/source/session ID/sequence and initiating Herdr pane context. Does not read stdin or forward `JCODE_HOOK_PAYLOAD`, tool inputs, prompts, or provider credentials to Herdr.
- Checks that this exact plugin is registered and enabled before reading or mutating the pane.
- Calls `HERDR_BIN_PATH` using argument arrays, never a shell. Each CLI call has a two-second timeout and a 64 KiB output limit. No retries or background service.
- Returns `reported` only after confirming the expected state or session identity via `pane get` read-back. Reports `not_confirmed` when the host ignored or discarded the report.

## Important limits

**Lifecycle reporting is custom state, not full-lifecycle authority.** Stock Herdr's `full_lifecycle_hook_authority` allowlist does not include `custom:leonardoacosta.herdr-jcode`. The plugin never claims to gate tools, change permission policy, or suppress Herdr's screen-based detection. It is a compatible layer that shows working/idle status to Herdr without native Jcode recognition. Herdr may additionally detect Jcode through its own screen analysis or process detection.

**Native session identity/restore requires a Jcode-capable Herdr build.** Doctor's `ready` means preflight checks passed, not that native restoration was tested. `native_restore_verified` remains false. Stock Herdr returns `unsupported_host` and doctor exits 2. Hook reports always exit 0 so failures do not block Jcode.

**Ordering is best-effort on stock Jcode.** Existing hooks have no producer ordinal or generation. Without `JCODE_HOOK_SEQUENCE`, reports are unsequenced. A host that already received sequenced reports from another source may ignore unsequenced ones, exposed as `not_confirmed`. With `JCODE_HOOK_SEQUENCE` metadata (local Jcode branch), reports carry a producer sequence and stale events are rejected. Do not install alongside another reporter for the same source.

**Do not claim Jcode lifecycle authority from turn hooks alone.** Herdr's full-lifecycle authority requires explicit blocked, approval-result, interrupt, and exit transitions that Jcode does not yet expose through hooks. The plugin never reports `blocked` state or claims complete lifecycle authority.

Configuration locks coordinate this plugin's writers. A non-cooperating editor can still race between the final check and atomic rename. Finish config edits before setup/removal. If a process is killed while holding the lock, verify no setup/remove process remains before manually removing the adjacent `config.toml.lock`. The plugin never guesses that a lock is stale.

## Disable and uninstall

```sh
herdr plugin disable leonardoacosta.herdr-jcode
# Re-enable to invoke the remove action, or run the binary's remove command directly.
herdr plugin enable leonardoacosta.herdr-jcode
herdr plugin action invoke remove --plugin leonardoacosta.herdr-jcode
# Wait for removal to finish, then use the matching command:
herdr plugin unlink leonardoacosta.herdr-jcode       # local link
# herdr plugin uninstall leonardoacosta.herdr-jcode # managed GitHub installation
```

Disable/unlink makes future reports inert. Already-running reports can finish. Herdr has no manifest uninstall hook, so **remove the Jcode registration before uninstalling**. Otherwise Jcode may retain a command pointing to the deleted checkout. Removal does not erase stored Herdr session identity or user sessions.

## Verification

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
JCODE_SCRATCH_DIR=/absolute/scratch python3 tests/live_herdr.py /absolute/herdr /absolute/jcode
```

The live test launches a disposable real Herdr TUI and two actual Jcode clients sharing an isolated daemon. It invokes real plugin actions, verifies four-hook installation, preserves a user observer, records real pane/session attribution with `agent: jcode` and `status: idle`, forwards CLI traffic to the real Herdr binary, checks unsupported-host behavior, and verifies disable/removal/unlink. No model prompt is sent and user profiles are untouched.

See [verification evidence](docs/verification.md) for requirement-specific observations. Positive native reporting and lifecycle protocol branches also have explicitly synthetic tests. They do not replace real-model end-to-end lifecycle acceptance.

## Marketplace publication

The manifest is ready for a GitHub-hosted plugin repository. Publication has not been performed. After publishing a tested revision, users can install it with `herdr plugin install OWNER/REPO --ref COMMIT --yes`. Herdr's marketplace indexes public repositories with the `herdr-plugin` topic and a valid `herdr-plugin.toml`. Listing is separate from compatibility approval. Do not publish as providing native Jcode support on stock Herdr.

License: [MIT](LICENSE).