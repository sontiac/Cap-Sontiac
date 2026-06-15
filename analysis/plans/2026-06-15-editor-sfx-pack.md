# Editor SFX Pack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bundle ~31 named video-editing sound effects into the desktop app so they self-install into the editor's existing "Sound effects" panel on first open, on any machine and build.

**Architecture:** Commit normalized `.mp3` files to the repo, ship them as Tauri bundle resources, add an idempotent Rust `seed_sound_effects` command that copies any missing bundled file into `app_local_data_dir/sound_effects` (where `SfxTab` already reads), and call that command on panel mount plus from a new "Restore default sounds" button.

**Tech Stack:** Tauri v2 (Rust commands + `tauri-specta` TS bindings), SolidJS (`SfxTab.tsx`), ffmpeg for asset normalization.

**Spec:** `analysis/plans/sfx-pack-design.md`

**Branch:** `sol/editor-sfx-pack` (already checked out; design spec already committed here).

---

## File Structure

- `apps/desktop/src-tauri/assets/sound_effects/*.mp3` — **new**, committed normalized audio (31 files).
- `apps/desktop/src-tauri/tauri.conf.json` — **modify**, add the resource glob.
- `apps/desktop/src-tauri/src/sfx.rs` — **new**, `copy_missing_files` helper + `seed_sound_effects` command + unit tests.
- `apps/desktop/src-tauri/src/lib.rs` — **modify**, `mod sfx;` (after `mod screenshot_editor;` block, alphabetical-ish) + register `sfx::seed_sound_effects` in `collect_commands!`.
- `apps/desktop/src/utils/tauri.ts` — **regenerated** (not hand-edited) by the specta build; gains `commands.seedSoundEffects`.
- `apps/desktop/src/routes/editor/SfxTab.tsx` — **modify**, seed on mount + "Restore default sounds" button.

---

## Task 1: Source and normalize the 31 SFX files

**Files:**
- Create: `apps/desktop/src-tauri/assets/sound_effects/*.mp3`
- Staging (not committed): a scratch `~/sfx-raw/` for downloads

The 31 target filenames (kebab-case `.mp3`):

```
faaah  vine-boom  bruh  emotional-damage  record-scratch  airhorn
magic-reveal  sparkle  ta-da  shimmer-reveal
woosh  woosh-reverse  swoosh-transition  swish  glitch
boom-impact  cinematic-hit  bass-drop  riser  suspense-riser
pop  click  typewriter  ding  notification  error-buzzer
crowd-cheer  crowd-laugh  applause  cha-ching  camera-shutter
```

Sources: viral meme clips (faaah, vine-boom, bruh, emotional-damage, record-scratch, airhorn, ta-da, cha-ching) from soundboards (e.g. myinstants); transitions/impacts/UI/foley from royalty-free libraries (Pixabay, Mixkit). This is a private single-user fork (see spec licensing note) — do not push to a public mirror.

- [ ] **Step 1: Create staging + target dirs**

```bash
mkdir -p ~/sfx-raw
mkdir -p apps/desktop/src-tauri/assets/sound_effects
```

- [ ] **Step 2: Download each raw clip into `~/sfx-raw/<name>.<ext>`**

Acquire one source file per target name into `~/sfx-raw/` (any of mp3/wav/m4a/ogg). Audition each before normalizing — confirm it is the intended sound and is not silent/corrupt. If a source for a specific name is dead or unusable, pick the closest acceptable alternative and **record the substitution in the final report** rather than shipping a wrong/silent clip.

- [ ] **Step 3: Normalize every raw file into the committed target**

Run for each `<name>` (trims leading/trailing silence, normalizes loudness to -16 LUFS, encodes consistent 44.1kHz stereo mp3, hard-caps at 3s):

```bash
for f in ~/sfx-raw/*; do
  name="$(basename "${f%.*}")"
  ffmpeg -y -i "$f" \
    -af "silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0.05,areverse,silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0.05,areverse,loudnorm=I=-16:TP=-1.5:LRA=11" \
    -ar 44100 -ac 2 -c:a libmp3lame -q:a 2 -t 3 \
    "apps/desktop/src-tauri/assets/sound_effects/${name}.mp3"
done
```

- [ ] **Step 4: Verify the pack — exactly 31 non-empty files, each < 3.5s**

Run:
```bash
cd apps/desktop/src-tauri/assets/sound_effects
echo "count: $(ls *.mp3 | wc -l)"   # expect 31
for f in *.mp3; do
  dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$f")
  size=$(stat -f%z "$f")
  echo "$f  ${dur}s  ${size}b"
  [ "$size" -gt 0 ] || echo "  !! EMPTY: $f"
done
```
Expected: `count: 31`, every duration under ~3.5s, no `!! EMPTY` lines.

- [ ] **Step 5: Commit the pack**

```bash
cd /Users/kshortrede/Documents/Content/Cap-Sontiac
git add apps/desktop/src-tauri/assets/sound_effects/*.mp3
git commit -m "feat(editor): add bundled sound-effects pack assets"
```
(The `assets/` path is tracked normally — verified the existing `assets/backgrounds/*` files are committed and the path is not gitignored — so plain `git add` works, no `-f`.)

---

## Task 2: Rust seed command with unit tests (TDD)

**Files:**
- Create: `apps/desktop/src-tauri/src/sfx.rs`
- Test: inline `#[cfg(test)]` module in the same file (mirrors `captions.rs`/`import.rs`)

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/src-tauri/src/sfx.rs` with only the test module and a stub signature:

```rust
use std::path::Path;

/// Copy every regular file in `src` that does not already exist in `dest`.
/// Creates `dest` if needed. Idempotent; never overwrites existing files.
/// Returns the file names that were copied.
fn copy_missing_files(_src: &Path, _dest: &Path) -> std::io::Result<Vec<String>> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn copies_missing_skips_existing_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dest).unwrap();

        fs::write(src.join("faaah.mp3"), b"AAAA").unwrap();
        fs::write(src.join("woosh.mp3"), b"BBBB").unwrap();
        // dest already has a user-customized faaah.mp3 that must NOT be clobbered
        fs::write(dest.join("faaah.mp3"), b"USERDATA").unwrap();

        let mut copied = copy_missing_files(&src, &dest).unwrap();
        copied.sort();
        assert_eq!(copied, vec!["woosh.mp3".to_string()]);
        assert_eq!(fs::read(dest.join("faaah.mp3")).unwrap(), b"USERDATA");
        assert_eq!(fs::read(dest.join("woosh.mp3")).unwrap(), b"BBBB");

        // second run copies nothing (idempotent)
        let copied2 = copy_missing_files(&src, &dest).unwrap();
        assert!(copied2.is_empty());
    }

    #[test]
    fn missing_src_returns_empty_and_does_not_error() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("dest");
        let copied = copy_missing_files(&tmp.path().join("nope"), &dest).unwrap();
        assert!(copied.is_empty());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p cap-desktop --lib sfx:: 2>&1 | tail -20`
Expected: compile fails / panics with `not implemented` (`unimplemented!()`), i.e. the module isn't yet declared OR the test panics. If it errors with "module sfx not found", that's expected too — it gets declared in Task 3; for now run with the temporary `mod sfx;` you add in Step 3 below, OR accept the fail and proceed.

Note: declare the module temporarily now so the test compiles — add `mod sfx;` to `lib.rs` (this is also Task 3 Step 1; doing it here is fine).

- [ ] **Step 3: Implement `copy_missing_files`**

Replace the stub body in `sfx.rs`:

```rust
fn copy_missing_files(src: &Path, dest: &Path) -> std::io::Result<Vec<String>> {
    let mut copied = Vec::new();
    if !src.exists() {
        return Ok(copied);
    }
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let target = dest.join(&name);
        if target.exists() {
            continue;
        }
        std::fs::copy(entry.path(), &target)?;
        copied.push(name.to_string_lossy().into_owned());
    }
    Ok(copied)
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p cap-desktop --lib sfx:: 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: Add the Tauri command wrapper**

Append to `sfx.rs` (above the `#[cfg(test)]` module), importing the path trait:

```rust
use tauri::Manager;

/// Seed the user's `sound_effects` folder from the bundled default pack.
/// Copies only files that are not already present. Returns the count copied.
#[tauri::command]
#[specta::specta]
pub async fn seed_sound_effects(app: tauri::AppHandle) -> Result<u32, String> {
    let src = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("assets")
        .join("sound_effects");
    let dest = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("sound_effects");
    let copied = copy_missing_files(&src, &dest).map_err(|e| e.to_string())?;
    Ok(copied.len() as u32)
}
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check -p cap-desktop 2>&1 | tail -20`
Expected: finishes with no errors (warnings about unused `seed_sound_effects` are fine until Task 3 registers it).

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src-tauri/src/sfx.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add seed_sound_effects command with copy-missing logic"
```

---

## Task 3: Register the module + command and regenerate TS bindings

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs` (module decl + `collect_commands!`)
- Regenerated: `apps/desktop/src/utils/tauri.ts`

- [ ] **Step 1: Declare the module (if not already done in Task 2)**

In `apps/desktop/src-tauri/src/lib.rs`, add alongside the other `mod` lines (after line 35 `mod screenshot_editor;`):

```rust
mod sfx;
```

- [ ] **Step 2: Register the command in `collect_commands!`**

In `lib.rs`, inside the `tauri_specta::collect_commands![ ... ]` list (around line 4145, next to the `import::*` entries), add:

```rust
            sfx::seed_sound_effects,
```

- [ ] **Step 3: Regenerate the TS bindings**

The `commands.seedSoundEffects` binding in `apps/desktop/src/utils/tauri.ts` is generated by the `tauri-specta` builder's `export_to` when the app builds. Trigger generation:

Run: `pnpm --filter @cap/desktop tauri dev` (let it reach the point where bindings are written, then stop it), **or** if a lighter typegen path exists, use it.
Expected: `git diff apps/desktop/src/utils/tauri.ts` shows a new `seedSoundEffects` entry under `commands`.

Verify:
```bash
grep -n "seedSoundEffects" apps/desktop/src/utils/tauri.ts
```
Expected: at least one match.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri/src/lib.rs apps/desktop/src/utils/tauri.ts
git commit -m "feat(desktop): register seed_sound_effects command + regen bindings"
```

---

## Task 4: Add the resource bundle entry

**Files:**
- Modify: `apps/desktop/src-tauri/tauri.conf.json`

- [ ] **Step 1: Add the sound_effects glob to `bundle.resources`**

In `tauri.conf.json`, the `bundle.resources` object currently maps several `assets/...` globs. Add this entry (keep valid JSON — add a comma after the new line if it is not last):

```json
"assets/sound_effects/*": "assets/sound_effects/"
```

- [ ] **Step 2: Verify it parses and the entry is present**

Run:
```bash
python3 -c "import json;d=json.load(open('apps/desktop/src-tauri/tauri.conf.json'));print(d['bundle']['resources'].get('assets/sound_effects/*'))"
```
Expected: prints `assets/sound_effects/`.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src-tauri/tauri.conf.json
git commit -m "feat(desktop): bundle sound_effects pack as app resource"
```

---

## Task 5: Wire SfxTab — seed on mount + Restore button

**Files:**
- Modify: `apps/desktop/src/routes/editor/SfxTab.tsx`

- [ ] **Step 1: Seed before listing on mount**

In `SfxTab.tsx`, change the `listSfxFiles` resource so it seeds first. Replace the `listSfxFiles` function (lines 37-43) and the resource fetcher (lines 52-54) so the resource calls a seeding wrapper:

```tsx
async function seedAndListSfxFiles(): Promise<SfxFile[]> {
	try {
		await commands.seedSoundEffects();
	} catch (err) {
		console.error("Failed to seed default SFX:", err);
	}
	return listSfxFiles();
}
```

And update the resource:

```tsx
	const [files, { refetch }] = createResource(seedAndListSfxFiles, {
		initialValue: [],
	});
```

Leave `listSfxFiles` itself unchanged (the `refresh` button keeps calling `refetch`, which now re-seeds too — harmless because seeding is idempotent).

- [ ] **Step 2: Add a "Restore default sounds" handler**

Add near `openSfxFolder` (after line 84):

```tsx
	const restoreDefaults = async () => {
		try {
			await commands.seedSoundEffects();
		} catch (err) {
			console.error("Failed to restore default SFX:", err);
		}
		refetch();
	};
```

- [ ] **Step 3: Add the Restore button to the header**

In the header button row (the `<div class="flex gap-1">` block, lines 90-107), add a third button before the existing Refresh button:

```tsx
						<button
							type="button"
							onClick={restoreDefaults}
							class="p-1 rounded hover:bg-gray-3 text-gray-11 hover:text-gray-12"
							aria-label="Restore default sounds"
							title="Restore default sounds"
						>
							<IconLucideRotateCcw class="size-4" />
						</button>
```

And add the icon import alongside the existing icon imports (after line 9):

```tsx
import IconLucideRotateCcw from "~icons/lucide/rotate-ccw";
```

- [ ] **Step 4: Verify type-check / lint passes**

Run: `pnpm --filter @cap/desktop exec tsc --noEmit 2>&1 | tail -20` (or the repo's configured check, e.g. `pnpm biome check apps/desktop/src/routes/editor/SfxTab.tsx`)
Expected: no errors referencing `SfxTab.tsx` or `seedSoundEffects`.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/routes/editor/SfxTab.tsx
git commit -m "feat(editor): seed default SFX on panel open + restore button"
```

---

## Task 6: End-to-end manual verification

**Files:** none (verification only)

- [ ] **Step 1: Remove any stale folder so first-run seeding is exercised**

```bash
rm -rf "$HOME/Library/Application Support/so.cap.desktop.dev/sound_effects"
```

- [ ] **Step 2: Run the app and open the editor SFX panel**

Run: `pnpm --filter @cap/desktop tauri dev`
Open a project in the editor, open the **Sound effects** panel.
Expected: the list is populated with the 31 named sounds without any manual file copying.

- [ ] **Step 3: Confirm seeding actually wrote the folder**

```bash
ls "$HOME/Library/Application Support/so.cap.desktop.dev/sound_effects" | wc -l
```
Expected: `31`.

- [ ] **Step 4: Exercise preview, insert, and restore**

- Click a sound name → it previews (audible).
- Click **Insert** at a playhead position → an `sfxSegment` is added (verify the timeline updates and config saves with no console error).
- Delete one file from the folder, click **Restore default sounds** → the deleted file reappears; others are untouched.

Expected: all three behaviors work; no errors in the dev console.

- [ ] **Step 5: Final report**

Write a short report covering: which sounds (if any) were substituted from the planned sources in Task 1, confirmation of the 31-file count, and that seed/preview/insert/restore all verified. Note any fragile points honestly.

---

## Notes for the implementer

- **Branch:** stay on `sol/editor-sfx-pack`. Do not push unless asked.
- **Path match is critical:** `SfxTab` reads `appLocalDataDir()/sound_effects`; the Rust command must use `app_local_data_dir()` (NOT `app_data_dir()`). They differ on macOS only subtly but must match exactly — `captions.rs:181` is the reference for the Rust call.
- **Idempotency is the whole safety story:** `copy_missing_files` never overwrites, so seeding on every mount and via Restore can never destroy a user's own added/edited sounds.
- **TS bindings are generated, never hand-edited.** If `seedSoundEffects` doesn't appear after a dev build, the command wasn't added to `collect_commands!` correctly.
