# Build the Cap desktop **overlay editor UI** (Phase 2 of the game-UI overlay system)

You are working in the Cap-Sontiac fork at `/Users/sontiac/Documents/Content/content-creation-workspace/Cap-Sontiac` (a Turborepo: Tauri v2 + SolidStart desktop app in `apps/desktop`, Rust crates in `crates/*`). **Read `AGENTS.md` first — its invariants are enforced.**

## Where this fits (what already exists — do NOT rebuild)
The fork already has a fully working **image-overlay render pipeline**, merged to `main`:
- A PNG overlay can be placed on the timeline with a time range, position, size, opacity, fade, and slide/pop animation.
- It renders correctly in **both `cap export` and the editor live preview** (shared `cap-rendering` pipeline — confirm the preview shows overlays early; it should, since preview and export use the same `RendererLayers`).
- Render layer: `crates/rendering/src/layers/overlay.rs` (don't change unless you find a real bug).

**What's missing — and your job:** there is **no editor UI** for overlays. Overlays can only be added programmatically (config JSON). A human needs to open a project in the Cap desktop editor and **add / select / move / resize / retime / animate overlays by hand** — because an automated pass places them ~85% right but they sometimes cover the speaker's face or are too big. Build that UI.

## The data model (already defined — do NOT redefine; just read/write it)
`OverlaySegment` in `crates/project/src/configuration.rs`, stored in `TimelineConfiguration.overlay_segments` (JSON key `overlaySegments`). Fields (camelCase in JSON / TS):
- `id: string`
- `start`, `end`: number — **output-timeline seconds**
- `filePath: string` — absolute path to a transparent PNG
- `center: { x, y }` — normalized [0,1] of the output frame (0.5,0.5 = center)
- `size: { x, y }` — normalized [0,1] bounding box; the image is aspect-preserved *contain-fit* inside it
- `opacity: number` [0,1]
- `fadeDuration: number` (default 0.2) — opacity fade in/out, seconds
- `animIn`, `animOut`: optional enum — `"slideLeft" | "slideRight" | "slideUp" | "slideDown" | "pop"`
- `animDuration: number` (default 0.35) — entrance/exit motion seconds
`OverlaySegment` derives `specta::Type`, so it already flows into `apps/desktop/src/utils/tauri.ts` when bindings regenerate.

## Scope — build in this order, each phase independently usable & testable
**Phase A — "Overlays" tab (list + asset picker + numeric edit).** Add an Overlays tab to the editor side panel, mirroring `apps/desktop/src/routes/editor/SfxTab.tsx` (and `CaptionsTab.tsx` / `KeyboardTab.tsx`). It lists the project's overlay segments and lets the user: add one (pick a PNG), delete, and edit every field via inputs (start/end, center x/y, size x/y, opacity, fade, animIn/animOut/animDuration). Persist through the exact same `setProjectConfig` / `serializeProjectConfiguration` path SfxTab uses. The preview re-renders overlays from the config, so numeric edits should update it live. **Asset source:** the transparent overlay PNGs live in `../content-creation/assets/game-ui/transparent/` (sibling repo, 15 assets). Let the user browse/pick from a folder — or seed them into the project like SfxTab seeds its SFX pack (`seed_sound_effects` in `apps/desktop/src-tauri/src/sfx.rs`); investigate and mirror whichever fits.

**Phase B — on-canvas drag + resize (the core of "move stuff around").** When an overlay is selected, draw a bounding box on the editor preview canvas with **drag-to-move** (updates `center`) and **corner handles to resize** (updates `size`). **Investigate how the editor already manipulates positioned elements on the canvas** — text segments (`text_segments`, also normalized center/size), camera position, zoom focus — and mirror that exact interaction pattern rather than inventing one.

**Phase C — timeline clips.** Render each overlay as a clip on the editor timeline (like `zoomSegments`/`sfxSegments`) with draggable start/end. Investigate how those existing segment types render + drag on the timeline and mirror.

## Hard constraints (AGENTS.md — binding)
- **ZERO narrating comments.** Rust must be clippy-clean the first time; TS must be Biome-clean: **tab indent, double-quoted strings**, no `any`, `organizeImports` on.
- **Never edit generated files** (`**/tauri.ts`, `**/queries.ts`, `apps/desktop/src-tauri/gen/**`). Regenerate TS bindings via the project's normal mechanism (running the desktop app in dev regenerates them) — never hand-edit `tauri.ts`.
- **Never start dev servers yourself** (`pnpm dev*`). The human runs `pnpm dev:desktop` to test — hand off each phase for them to test before proceeding.
- After edits: run the narrowest applicable check (`pnpm exec biome check --write <files>`; `cargo check -p <crate>` if you touch Rust). Conventional commits (`feat(desktop): ...`).
- Follow existing editor patterns; don't restructure unrelated code.

## How to verify (you cannot run the app)
The human runs `pnpm dev:desktop`, imports a video (or opens an existing `.cap` project), then per phase: adds an overlay, sees it in preview, drags/resizes it, retimes it, exports. Get their confirmation after each phase.

## Full background (read for context)
- Design spec: `../content-creation/docs/superpowers/specs/2026-06-26-game-ui-overlay-system-design.md`
- Render-layer implementation plan: `../content-creation/docs/superpowers/plans/2026-06-28-cap-image-overlay-layer.md`
- Config type: `crates/project/src/configuration.rs` (search `OverlaySegment`, `OverlayAnim`).
- How overlays are added programmatically today (for parity of behavior): `../video-edit-lab/timeline.py` → `apply_overlay()`, driven via `cap project config get|set`.

**Start by:** reading AGENTS.md, then `SfxTab.tsx` and how it plumbs a segment array to the config, then confirming overlays already show in the preview, then propose a Phase-A plan before writing code.
