# Model display exploration

Inspected 2026-09-16. Request: show the model publisher logo and/or model short name in the Herdr agent row, supplied by Jcode, potentially through this plugin. No implementation is included in this exploration.

## Evidence

- Installed `herdr --version` returns 0.9.0, not the 0.8.2 previously assumed.
- Installed `herdr pane report-metadata --help` exposes `--display-agent`, `--title`, `--token NAME=VALUE`, `--clear-token`, `--seq`, `--ttl-ms`, and source scoping. It describes these as display-only metadata.
- Bundled Jcode `docs/HOOKS.md` documents `JCODE_HOOK_MODEL` on `turn_end`. It does not establish publisher or short-name fields, or immediate model-change notifications. Local emitter implementation was not verified.
- Bundled Jcode `docs/HERDR.md` describes a built-in lifecycle reporter. A display bridge should not introduce a competing lifecycle writer.
- Existing `/home/nyaptor/dev/herdr-sidebar-config/sidebar-layout.toml` renders custom `$hs_*` tokens. `sidebar.py:desired_rows` selects `hs_logo` from the agent identifier, not the model publisher.
- That sidebar plugin builds font glyphs from SVG files using `tools/build_font.py` and terminal font configuration. This is not inline SVG rendering in a sidebar row.

## Recommendation

Use a separate display-only metadata source in herdr-jcode, with namespaced tokens for model short name, publisher name, and optional publisher glyph. Add these tokens to the existing agent-row layout. Keep `agent=jcode` and retain existing lifecycle reporting. Do not overwrite the sidebar plugin's `hs_logo` token.

Text needs no new Herdr rendering API. Logos can follow the existing SVG-to-font-glyph pipeline, with a text fallback when fonts are unavailable. Direct SVG embedding is not exposed by the inspected metadata API.

## Constraints and open questions

- The authentication route is not necessarily the model publisher. Proxy/custom model aliases need explicit mapping or an unknown fallback.
- A turn-end hook can update after completion, but does not guarantee correct initial or immediately switched model display. Verify Jcode emitters before selecting the final transport.
- Per-session model data must follow client pane ownership, especially with swarm sessions and multiple clients.
- Bound and sanitize display strings, clear stale values on ownership changes, and order updates. Never forward prompts, credentials, or the full hook payload.
- Agent sidebar rows and horizontal tab titles are different consumers. This recommendation targets the existing agent sidebar row. Horizontal tab titles need a separate rendering check.
- No Context7 result was consulted. The installed CLI, local sidebar implementation, and bundled Jcode version-matched docs are the evidence used here.

## Verification needed for implementation

Use an isolated Herdr session to verify visible short names, font and text fallback, model switching, no collision with existing sidebar tokens, multi-client targeting, and metadata cleanup. CLI help and source inspection establish feasibility, not end-to-end rendering acceptance.
