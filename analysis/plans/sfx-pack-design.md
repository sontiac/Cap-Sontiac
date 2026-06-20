# SFX Pack — Bundled Sound Effects for the Editor

**Date:** 2026-06-15
**Status:** Approved design, ready for implementation plan
**Scope:** Personal fork (`Cap-Sontiac`) only — not for upstream/public mirror.

## Goal

Ship a curated set of ~31 named, short-form video-editing sound effects so they
appear automatically in the editor's existing **Sound effects** panel
(`SfxTab`), on any machine and any build, with no manual file copying.

## Context / Current State

- `apps/desktop/src/routes/editor/SfxTab.tsx` already implements the panel: it
  reads audio files (`mp3/wav/m4a/ogg/aac/flac`) from
  `appLocalDataDir()/sound_effects`, previews them, and inserts an `sfxSegment`
  into the timeline at the playhead via `commands.setProjectConfig`.
- The `sound_effects` folder is created on first panel open (`ensureSfxDir`) but
  ships **empty** — there is no bundled content.
- Resolved folder paths:
  - dev build: `~/Library/Application Support/so.cap.desktop.dev/sound_effects`
  - release build: `~/Library/Application Support/so.cap.desktop/sound_effects`
- `tauri.conf.json` already bundles static assets via `bundle.resources` (the
  backgrounds use this exact pattern). `apps/desktop/src-tauri/src/import.rs` is
  a clean model for a new single-purpose command file.
- The timeline `sfxSegment` model is complete and is **not** changed by this work.

## Licensing Note

The pack includes recognizable viral meme clips (Faaah, Vine boom, etc.). These
are not cleanly licensed for redistribution. Acceptable here because this is a
private, single-user fork for personal short-form content. **Must not** be
pushed to a public mirror or proposed upstream.

## Design

### 1. The pack (31 sounds)

Stored in the repo at `apps/desktop/src-tauri/assets/sound_effects/`. Every file
is normalized at authoring time:

- Converted to **mp3**.
- Loudness-normalized via ffmpeg `loudnorm` so insert volumes are consistent.
- Leading/trailing silence trimmed; length kept to ~1–2s so insert-at-playhead
  lands cleanly against `SFX_DEFAULT_DURATION = 1`.

Naming is kebab-case `.mp3`, chosen so the names are searchable/scannable in the
panel list.

| Category | Files |
|---|---|
| Reaction/comedic (6) | `faaah`, `vine-boom`, `bruh`, `emotional-damage`, `record-scratch`, `airhorn` |
| Reveals (4) | `magic-reveal`, `sparkle`, `ta-da`, `shimmer-reveal` |
| Transitions (5) | `woosh`, `woosh-reverse`, `swoosh-transition`, `swish`, `glitch` |
| Impact/build (5) | `boom-impact`, `cinematic-hit`, `bass-drop`, `riser`, `suspense-riser` |
| UI/text (6) | `pop`, `click`, `typewriter`, `ding`, `notification`, `error-buzzer` |
| Crowd/foley (5) | `crowd-cheer`, `crowd-laugh`, `applause`, `cha-ching`, `camera-shutter` |

### 2. Bundle the pack

Add to `tauri.conf.json` `bundle.resources`:

```
"assets/sound_effects/*": "assets/sound_effects/"
```

Same mechanism the backgrounds already use, so the files ship inside the app
bundle and are resolvable via the Tauri resource dir at runtime.

### 3. Seed command (Rust)

New file `apps/desktop/src-tauri/src/sfx.rs` exposing a Tauri command
`seed_sound_effects`:

- Resolve the bundled resource directory (`app.path().resource_dir()`) →
  `assets/sound_effects`.
- Resolve the live folder (`app_local_data_dir()` → `sound_effects`), creating it
  if missing (mirrors `ensureSfxDir`).
- Copy every bundled file that is **not already present** in the live folder.
  Files already present (user-added, or previously seeded) are left untouched, so
  the command is idempotent and never clobbers the user's own sounds.
- Return the count of files copied.

Register the command in `lib.rs` (`generate_handler!`) following the existing
pattern.

### 4. Wire into SfxTab

- On panel mount, call `commands.seedSoundEffects()` before `listSfxFiles`, then
  refetch — the pack appears the first time the tab is opened, on any machine.
- Add a **"Restore default sounds"** button alongside the existing Refresh /
  Open-folder buttons that calls `seedSoundEffects()` then refetches. Because the
  command only copies missing files, this restores any defaults the user deleted
  without duplicating or overwriting anything.

## What is intentionally NOT changed

- Timeline / `sfxSegment` insertion model (already works).
- Preview, insert, refresh, open-folder behaviors (already work).
- No metadata/categories/manifest in the panel — the flat folder + filenames are
  sufficient; categories above are an authoring aid only.

## Files Touched

- `apps/desktop/src-tauri/assets/sound_effects/*.mp3` (new — committed audio)
- `apps/desktop/src-tauri/tauri.conf.json` (add resource glob)
- `apps/desktop/src-tauri/src/sfx.rs` (new — seed command)
- `apps/desktop/src-tauri/src/lib.rs` (register command)
- `apps/desktop/src/routes/editor/SfxTab.tsx` (seed on mount + restore button)

## Risks / Honest Caveats

- **Source reliability:** a few obscure clips may have dead or unusable download
  sources at implementation time. If a source must be swapped, the substitution
  will be flagged explicitly rather than shipping a silent or wrong clip.
- **Resource-dir resolution** differs slightly across dev vs. bundled builds;
  the command must use the Tauri path API (not a hardcoded path) and be verified
  in both modes.
