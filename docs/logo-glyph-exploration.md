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

## Approved WebFetch follow-up

The user approved WebFetch for the six exact SVG assets. On 2026-09-16, text extraction produced empty output, then `format=html` returned the complete SVG markup for all six. The empty text result was not evidence of an empty asset.

| Asset | Retrieved structure | Preparation and risk |
| --- | --- | --- |
| Claude Code mono | 382 bytes, 24×24 viewBox, one compound path, currentColor, evenodd fill | Remove title element for current builder. Preserve eye cutouts. Simple pixel-like silhouette, a promising small-cell candidate. |
| OpenAI mono | 1,642 bytes, 24×24, one compound path, currentColor, evenodd fill | Remove title. Preserve knot holes and narrow gaps. Check contour winding and small-cell legibility. |
| DeepSeek default | 2,126 bytes, 24×24, one compound path, solid #4D6BFE | Geometry is already single-color. Font drops fill color; Herdr can supply foreground color. Fine internal details may disappear at small sizes. |
| Kimi default | 957 bytes, 512×512, three paths: black rounded background, blue accent, white foreground | Do not concatenate all paths into a single-color glyph unchanged. Prefer removing the background and combining the foreground mark/accent, or explicitly subtract foreground shapes for a badge. Source color layering is significant. |
| Qwen default | 1,783 bytes, 24×24, one compound path, #ffff fill, evenodd fill | Remove title. Already single-color geometry. Preserve interior cutouts and test thin gaps. |
| GLM-V mono | 3,532 bytes, 24×24, five paths, currentColor, evenodd fill | Remove title. Dense multi-path artwork has the highest small-cell detail risk. Identity remains GLM-V, not a verified GLM-5 mark. |

No retrieved asset contains image, script, external href, gradient, mask, clipPath, or filter elements. The clip-rule attribute on Claude is not a clipPath dependency. Kimi declares stroke-linejoin and stroke-miterlimit but does not define a painted stroke. All actual artwork consists of path elements.

The existing `tools/build_font.py:svg_glyph` rejects any child element other than path, including harmless title nodes. Thus four assets (Claude Code, OpenAI, Qwen, GLM-V) need metadata stripping before that function accepts them. The builder feeds outlines into a TrueType pen without color compositing or explicit evenodd-to-nonzero conversion. Reversing all contours does not by itself fix incompatible hole winding. Outline normalization and rendered comparison are therefore acceptance checks, not optional cleanup.

Conclusion: all six are structurally convertible SVG artwork. Five have directly usable monochrome geometry after light preparation and winding validation. Kimi needs a deliberate monochrome adaptation. This is XML/source inspection, not proof that a built font renders correctly. No fonts were built or installed. Recommended first implementation set is OpenAI, Claude-family mark selected deliberately, DeepSeek, Qwen, and adapted Kimi, with text-only GLM-5 until its intended mark is agreed.

## User approval and mapping decision

On 2026-09-16 the user approved the six rendered glyph shapes in the scratch approval sheet and explicitly selected the GLM-V artwork to represent GLM-5 in this integration. Treat this as a project display mapping (`glm-5` -> `glm-v` artwork), not a claim that the two model families or their official branding are identical. Preserve the original asset name and provenance in notices; display the model label as GLM-5.

Approved prototype artifacts: `/home/nyaptor/.jcode/scratch/glyph-preview/approval.ttf`, `approval.png`, and `build.py`. Shape approval does not settle single-cell versus wider terminal rendering. Next scoped work should integrate namespaced model-display metadata with the existing sidebar font pipeline, verify model-switch updates and publisher-versus-route mapping, and test terminal widths in an isolated Herdr session before modifying live configuration. No integration or font installation performed by this approval-record step.
