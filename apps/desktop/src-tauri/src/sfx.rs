use std::path::Path;
use tauri::Manager;

/// Copy every regular file in `src` that does not already exist in `dest`.
/// Creates `dest` if needed. Idempotent; never overwrites existing files.
/// Returns the file names that were copied.
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
