use crate::sfx::copy_missing_files;
use tauri::Manager;

#[tauri::command]
#[specta::specta]
pub async fn seed_overlays(app: tauri::AppHandle) -> Result<u32, String> {
    let src = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("assets")
        .join("overlays");
    let dest = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?
        .join("overlays");
    let copied = copy_missing_files(&src, &dest).map_err(|e| e.to_string())?;
    Ok(copied.len() as u32)
}
