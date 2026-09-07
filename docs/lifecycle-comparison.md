# Lifecycle comparison

Both this plugin and [capt-marbles/herdr-jcode-integration](https://github.com/capt-marbles/herdr-jcode-integration) report Jcode lifecycle state to Herdr. This document compares the two implementations at the time of writing (2026-09-07).

## How they differ

| | Theirs | Ours |
| --- | --- | --- |
| Jcode dependency | Requires `capt-marbles/jcode` fork (`feat/herdr-integration`) | Stock Jcode v0.81.5+ |
| Lifecycle hooks | Four hooks via shell script | Four hooks via Rust binary |
| State reporting | `pane report-agent` with `custom:jcode` | `pane report-agent` with `custom:leonardoacosta.herdr-jcode` |
| Release | `pane release-agent` on session_end | Same |
| Producer ordering | Fork adds `JCODE_HOOK_SEQUENCE` | Independent sequence addition in companion Jcode branch |
| Config safety | Direct file writes, reconstructs arrays | Locked atomic writes, preserves comments/order, rejects malformed input |
| Subprocess safety | Shell commands, no explicit timeout | Argument arrays, 2s timeout, 64 KiB cap, env minimization |
| Plugin guard | No enabled-state check in reporter | Checks plugin is registered and enabled before API calls |
| Diagnostics | Shell script status check | Structured JSON doctor with native detector, override, and ordering report |
| Install | Requires fork + plugin | `herdr plugin install leonardoacosta/herdr-jcode` |

## Where they agree

Neither implementation has complete lifecycle authority. Herdr's `full_lifecycle_hook_authority` does not include custom sources. Both report custom state (`idle`/`working`) rather than claiming native agent recognition.

Both lack the Jcode-side emitters needed for `blocked`, approval result, interrupt, and crash transitions. Neither can promise reliable restore without Herdr-side Jcode session recognition.