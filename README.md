# herdr-jcode

Standalone Herdr plugin that adds a Jcode `session_start` observer without replacing your existing hooks. Independently implemented, with no dependency on upstream PR #2248 merging.

**Status: experimental. Stock Herdr 0.8.2 can install/link this package and run setup, diagnostics, and removal, but cannot store native Jcode identity or restore Jcode sessions.** The reporter detects that limitation and does not send a misleading native report. A Jcode-capable Herdr build is required for reporting. This repository does not patch Herdr or Jcode.

## Requirements

- Linux. Other platforms are not advertised or tested.
- Rust/Cargo to build. Tested with Rust 1.98.0.
- Jcode v0.81.5 or later with hook arrays and client-scoped terminal environment. Earlier versions are not certified.
- Herdr 0.8.2 or later for plugin actions. Its version alone does **not** establish native Jcode support.
- Python 3 for tests only, not plugin runtime.

## Build and link locally

```sh
cargo build --release --locked
herdr plugin link "$PWD"
herdr plugin action invoke doctor --plugin leonardoacosta.herdr-jcode
herdr plugin action invoke setup --plugin leonardoacosta.herdr-jcode
```

Run these from a Herdr-managed terminal. Actions run asynchronously. Inspect completion and output with `herdr plugin log list --plugin leonardoacosta.herdr-jcode`.

Setup is explicit. Build, link, and startup do not modify Jcode config. Setup appends the absolute executable path plus `report` to `hooks.session_start`. It resolves config from `JCODE_HOME/config.toml`, otherwise `~/.jcode/config.toml`. Start a new Jcode daemon or reload its configuration after setup/removal. Existing daemons may cache hooks.

For an explicit configuration path or direct diagnostics:

```sh
./target/release/herdr-jcode setup --config /absolute/path/config.toml
HERDR_BIN_PATH="$(command -v herdr)" ./target/release/herdr-jcode doctor
./target/release/herdr-jcode remove --config /absolute/path/config.toml
```

Keep the binary at its configured path. If moving to another checkout/install location, remove the old registration using the old binary before setup at the new location.

## What it does

- Preserves unrelated settings and user hook commands, including comments. Converts a scalar hook to an array when composing. Repeated setup is byte-identical. Removal removes only the exact owned command, but does not convert a remaining one-element array back to its original scalar form.
- Uses a configuration lock, same-directory temporary file, restricted initial permissions, content/inode rechecks, and atomic replacement. Refuses symlinked configuration paths/ancestors, multiple hard links, invalid TOML, mixed-type hook arrays, and ownership changes it cannot preserve. Use a canonical config path if your configuration directory is reached through a symlink.
- Refuses setup when `JCODE_HOOK_SESSION_START` overrides config. Check the environment used to launch Jcode, not only the setup action.
- Reads only session event/source/ID and initiating Herdr pane context. It does not read stdin or forward `JCODE_HOOK_PAYLOAD`, tool inputs, prompts, or provider credentials to Herdr.
- Checks that this exact plugin is registered and enabled before reading or mutating the pane.
- Calls `HERDR_BIN_PATH` using argument arrays, never a shell. Each CLI call has a two-second timeout and a 64 KiB output limit. No retries or background service.
- Reports session identity only. It never reports working/idle/blocked state, gates tools, changes permission policy, or releases another reporter's authority.
- Returns `reported` only after exact native session read-back. `create`/`attach` map to `startup`, and `resume` maps to `resume`.

## Important limits

Native reports require **both** a recognized Jcode detector and native session support in the host. Doctor's `ready` means its preflight checks passed, not that restoration was tested. `native_restore_verified` remains false. Unsupported stock Herdr returns `unsupported_host` and doctor exits 2. Hook reports always exit 0 so failures do not block Jcode.

Session ordering is deliberately not presented as solved. Existing hooks have no producer ordinal or generation. This plugin sends unsequenced reports, refuses an already-visible different identity, and does not invent ordering from wall-clock timestamps. Two initially empty-pane reports can still race. A host that already received sequenced reports may ignore ours, which read-back exposes as `not_confirmed`. Do not install alongside another `herdr:jcode` reporter. Automatic same-pane session replacement and reliable restore require further producer/host work. Prefer a new pane when changing sessions.

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

The live test launches a disposable real Herdr TUI and two actual Jcode clients sharing an isolated daemon. It invokes real plugin actions, preserves a user observer, records real pane/session attribution, forwards CLI traffic to the real Herdr binary, checks unsupported-host behavior, and verifies disable/removal/unlink. No model prompt is sent and user profiles are untouched. It targets stock Herdr's unsupported-native behavior, not successful native restore.

See [verification evidence](docs/verification.md) for requirement-specific observations and the remaining native-host acceptance blocker. Positive native reporting and conflict branches also have explicitly synthetic protocol tests. They do not replace native restore acceptance.

## Marketplace publication

The manifest is ready for a GitHub-hosted plugin repository. Publication has not been performed. After publishing a tested revision, users can install it with `herdr plugin install OWNER/REPO --ref COMMIT --yes`. Herdr's marketplace indexes public repositories with the `herdr-plugin` topic and a valid `herdr-plugin.toml`. Listing is separate from compatibility approval. Do not publish this experimental version as providing native support on stock Herdr.

License: [MIT](LICENSE).
