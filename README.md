# herdr-jcode

Herdr plugin that reports Jcode lifecycle state so you can see when a coding agent is working, idle, or gone. Four hooks: `session_start`, `turn_start`, `turn_end`, and `session_end`. No fork or patch needed.

`turn_start` signals `working`. `turn_end` signals `idle`. `session_end` releases the pane so Herdr stops showing the agent. Runs on stock Herdr 0.8.2 and stock Jcode v0.81.5+.

## How it works

Jcode fires hook events at each lifecycle transition. This plugin receives them, talks to Herdr over the CLI, and reports the agent state on the matching pane.

```
+---------------- [ LIFECYCLE ] ------------------+
|                                                  |
| ●  session_start  Report idle (or working, if    |
| │                 resuming)                      |
| │                                                |
| ●  turn_start     Report working                 |
| │                                                |
| ●  turn_end       Report idle                    |
| │                                                |
| ○  session_end    Release agent from pane        |
|                                                  |
+--------------------------------------------------+
```

The plugin uses `pane report-agent` with `custom:leonardoacosta.herdr-jcode` as the source. Herdr treats custom sources as advisory rather than authoritative, same as other lifecycle plugins. The agent type `jcode` is set but Herdr does not assign full lifecycle authority to custom sources.

On session start the plugin also calls `pane report-agent-session` when the Herdr build has native Jcode detection. This is optional. State reporting works without it.

## Install

```sh
herdr plugin install leonardoacosta/herdr-jcode --yes
herdr plugin action invoke setup --plugin leonardoacosta.herdr-jcode
```

From source:

```sh
cargo build --release --locked
herdr plugin link "$PWD"
herdr plugin action invoke setup --plugin leonardoacosta.herdr-jcode
```

Run these inside a Herdr managed terminal. Actions are asynchronous. Check results with `herdr plugin log list --plugin leonardoacosta.herdr-jcode`.

Setup writes the plugin binary path to `[hooks]` in Jcode's config for all four events. It reads from `JCODE_HOME/config.toml` or `~/.jcode/config.toml`. Restart Jcode or reload config after setup.

Direct:

```sh
./target/release/herdr-jcode setup --config /absolute/path/config.toml
HERDR_BIN_PATH="$(command -v herdr)" ./target/release/herdr-jcode doctor
./target/release/herdr-jcode remove --config /absolute/path/config.toml
```

## Independent from the fork

The [fork based integration](https://github.com/capt-marbles/herdr-jcode-integration) requires a patched Jcode build. This plugin works on stock Jcode v0.81.5+.

```
+------------------ [ APPROACH ] -----------------+
|                                                  |
|              Fork    Here                        |
| Jcode        patched stock                       |
| Install      clone   plugin install              |
| Config       shell   Rust (locked, atomic)       |
| Safety       none    2s timeout, 64 KiB cap       |
| Sequencing   fork    companion branch            |
| Doctor       shell   structured JSON             |
|                                                  |
+--------------------------------------------------+
```

## Safety

- Preserves existing hooks, comments, and TOML structure. Scalars become arrays when composed. Second setup does nothing.
- Locked, atomic config writes. Refuses symlink paths, multiple hard links, invalid TOML, mixed type arrays, and ownership it cannot preserve.
- Refuses setup when any `JCODE_HOOK_{SESSION_START,TURN_START,TURN_END,SESSION_END}` env var overrides config.
- Reads only hook event, source, session ID, sequence, and pane identity. Does not read stdin or forward `JCODE_HOOK_PAYLOAD`, tool data, prompts, or credentials.
- Checks the plugin is registered and enabled before touching the pane.
- Two second timeout and 64 KiB cap on every Herdr CLI call. Argument arrays, no shell.
- Exits 0 on hook failures so Jcode is never blocked.

## Limits

Herdr sees `agent: jcode` with `status: idle` or `working` via `custom:leonardoacosta.herdr-jcode`. That is custom state, not full lifecycle authority. The plugin does not gate tools, change permissions, or suppress Herdr's screen detection.

Native session identity (`pane report-agent-session`) requires a Herdr build with Jcode recognition. Stock Herdr 0.8.2 does not have it. Doctor reports `unsupported_host` and exits 2. Hook reports always exit 0.

Without `JCODE_HOOK_SEQUENCE`, reports carry no ordering. With it (from the companion Jcode branch), reports include a producer sequence and stale events are rejected. Do not install alongside another reporter for the same source.

Config locks serialize this plugin's writes. An uncooperative editor can still race. Finish edits before setup or removal. If a process dies while holding the lock, remove `config.toml.lock` after verifying nothing holds it.

## Remove

```sh
herdr plugin disable leonardoacosta.herdr-jcode
herdr plugin enable leonardoacosta.herdr-jcode
herdr plugin action invoke remove --plugin leonardoacosta.herdr-jcode
herdr plugin unlink leonardoacosta.herdr-jcode
```

Remove the Jcode registration before uninstalling. Jcode otherwise keeps a command pointing to a deleted binary.

## Verify

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
python3 tests/live_herdr.py /path/to/herdr /path/to/jcode
```

42 tests. Full pipeline clean. The live test launches a disposable Herdr TUI and two Jcode clients on an isolated daemon. No model prompt is sent and no user profile is touched. See [verification evidence](docs/verification.md).

## Requirements

- Linux (only platform tested)
- Rust 1.98.0+
- Jcode v0.81.5+ with hook arrays and client scoped terminal env
- Herdr 0.8.2+
- Python 3 for tests only

License: [MIT](LICENSE).