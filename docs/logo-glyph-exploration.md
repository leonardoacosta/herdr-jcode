# Model logo glyph feasibility

Research date: 2026-09-16. Scope: inspect six user-selected theSVG entries using Firecrawl, without installing or implementing fonts.

## Evidence and method

Firecrawl CLI 1.23.3 was authenticated. Help and status were checked. Six public HTTPS pages were scraped successfully using markdown, rawHtml, and links, with concurrency capped at three. Public DNS addresses were validated before requests. Hosted redirects were trusted under the public-research policy, not independently inspected per hop. No private data was submitted.

Local research evidence was initially stored under `/home/nyaptor/.jcode/scratch/logo-research/.firecrawl/`. All six pages list MIT and their variants, but their code previews contain an empty SVG shell, not the logo paths. The other inline SVGs in rawHtml are site UI icons and cannot prove logo geometry.

| Requested family | Page | Discovered preferred asset | Assessment |
| --- | --- | --- | --- |
| Claude | https://thesvg.org/icon/claude-code | https://thesvg.org/icons/claude-code/mono.svg | Explicit mono candidate. This is Claude Code branding, not a publisher-specific Anthropic mark. |
| GPT | https://thesvg.org/icon/openai-chatgpt | https://thesvg.org/icons/openai-chatgpt/mono.svg | Explicit mono candidate. OpenAI/ChatGPT identity, not a particular GPT model. |
| DeepSeek | https://thesvg.org/icon/deepseek | https://thesvg.org/icons/deepseek/default.svg | Default icon candidate. Avoid wordmark for compact cells. Geometry unverified. |
| Kimi | https://thesvg.org/icon/kimi | https://thesvg.org/icons/kimi/default.svg | Default icon candidate. Other listed variants are wordmarks. Geometry unverified. |
| Qwen | https://thesvg.org/icon/qwen | https://thesvg.org/icons/qwen/default.svg | Default plus light/dark variants. Choose after checking actual paths and fill dependencies. |
| GLM-5 | https://thesvg.org/icon/glm-v | https://thesvg.org/icons/glm-v/mono.svg | Technically promising mono candidate, but identity mismatch: page links to GLM-4.1V-9B-Thinking, not GLM-5. Do not label this as a verified GLM-5 logo. |

## Blocking limitation

Firecrawl failed on all six direct SVG asset URLs. A bounded retry of Claude Code confirmed the reason: `image/svg+xml` is unsupported. The CLI returned exit 1 and explicitly said it cannot process this file type. Scrape help supports HTML/rawHtml but no raw binary asset download option.

No alternate network transport was used. Next approved fallback would be the built-in webfetch tool for only the six exact asset URLs above, to inspect XML. That requires explicit approval under the active Firecrawl policy. Until then, none of these six files has passed geometry parsing or font conversion.

## Internal prior art

The existing herdr-sidebar-config plugin uses SVGPath and fontTools pens in tools/build_font.py to produce monochrome icon-font glyphs. Its sidebar layout consumes text tokens such as hs_logo. This demonstrates an existing integration route, not successful conversion of the newly requested assets. No new library was selected. Context7 was not consulted for the image catalog research; version-specific converter documentation is a prerequisite if implementation proceeds.

## Recommendation and checks

Use a monochrome font glyph plus a readable model short name, with text fallback. Prefer an explicit mono asset when present. Use family branding intentionally rather than calling every mark a publisher logo. Model version should come from Jcode, not the logo name.

Before acceptance, inspect viewBox, paths, transforms, strokes, fill-rule, masks, clipping, gradients, external references, raster content, and scripts. Outline strokes and resolve compositing only where necessary. Preserve negative space when removing colors. Reject unsafe content rather than executing it. Check baseline, width, contrast, small-cell legibility, fallback fonts, existing private-use codepoint collisions, and terminal font reload behavior.

The catalog's MIT labels are evidence of its claim, not proof of underlying trademark rights or a complete redistribution license chain. Retain provenance and notices and check original brand terms before shipping modified artwork.

Alternatives: text-only model names need no font installation and are the safest first version; raster/SVG terminal graphics are a separate rendering path, not an inline sidebar glyph replacement.

No fonts, plugin runtime code, or user configuration changed during this research. Proposed follow-up: approve exact asset retrieval, audit XML, then test isolated glyph conversion and render at the actual terminal cell size before any installation.
