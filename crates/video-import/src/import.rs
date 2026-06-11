use std::path::{Path, PathBuf};

use cap_enc_ffmpeg::remux::probe_video_can_decode;
use cap_project::{
    AudioMeta, Cursors, MultipleSegment, MultipleSegments, Platform, RecordingMeta,
    RecordingMetaInner, StudioRecordingMeta, StudioRecordingStatus, VideoMeta,
};
use relative_path::RelativePathBuf;
use tracing::error;

use crate::{
    ImportError, ImportStage, thumbnail::write_video_thumbnail, transcode::transcode_video,
};

pub struct ImportedVideo {
    pub fps: u32,
    pub has_audio: bool,
}

fn check_project_exists(project_path: &Path) -> bool {
    project_path.exists() && project_path.join("recording-meta.json").exists()
}

pub fn generate_project_name(source_path: &Path) -> String {
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported Video");

    let now = chrono::Local::now();
    let date_str = now.format("%Y-%m-%d at %H.%M.%S").to_string();

    format!("{stem} {date_str}")
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

pub fn unique_project_path(library_dir: &Path, source_path: &Path) -> (PathBuf, String) {
    let project_name = generate_project_name(source_path);
    let sanitized_name = sanitize_filename(&project_name);
    let mut project_path = library_dir.join(format!("{sanitized_name}.cap"));
    let mut counter = 1;
    while project_path.exists() {
        project_path = library_dir.join(format!("{sanitized_name} ({counter}).cap"));
        counter += 1;
    }
    (project_path, project_name)
}

pub fn import_video(
    source_path: &Path,
    project_path: &Path,
    pretty_name: &str,
    mut on_progress: impl FnMut(ImportStage, f64, &str),
) -> Result<ImportedVideo, ImportError> {
    on_progress(ImportStage::Probing, 0.0, "Analyzing video file...");

    let can_decode = probe_video_can_decode(source_path)
        .map_err(|e| ImportError::OpenFailed(format!("Cannot decode video: {e}")))?;
    if !can_decode {
        return Err(ImportError::OpenFailed(
            "Video format not supported or file is corrupted".to_string(),
        ));
    }

    std::fs::create_dir_all(project_path).map_err(ImportError::DirectoryFailed)?;
    let segment_dir = project_path
        .join("content")
        .join("segments")
        .join("segment-0");
    std::fs::create_dir_all(&segment_dir).map_err(ImportError::DirectoryFailed)?;

    let output_video_path = segment_dir.join("display.mp4");
    let output_audio_path = segment_dir.join("audio.ogg");

    let initial_meta = import_meta(
        project_path,
        pretty_name,
        30,
        None,
        StudioRecordingStatus::InProgress,
    );
    initial_meta.save_for_project().map_err(|e| {
        ImportError::TranscodeFailed(format!("Failed to save initial metadata: {e:?}"))
    })?;

    on_progress(ImportStage::Converting, 0.0, "Starting conversion...");

    let transcoded = transcode_video(
        source_path,
        &output_video_path,
        Some(&output_audio_path),
        |progress| {
            on_progress(
                ImportStage::Converting,
                progress,
                &format!("Converting video... {}%", (progress * 100.0) as u32),
            )
        },
        || !check_project_exists(project_path),
    )?;

    on_progress(
        ImportStage::Finalizing,
        0.95,
        "Creating project metadata...",
    );

    const MIN_VALID_AUDIO_SIZE: u64 = 1000;
    let audio_file_size = std::fs::metadata(&output_audio_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let system_audio = if transcoded.sample_rate.is_some() && audio_file_size > MIN_VALID_AUDIO_SIZE
    {
        Some(AudioMeta {
            path: RelativePathBuf::from("content/segments/segment-0/audio.ogg"),
            start_time: Some(0.0),
            device_id: None,
            gap_summary: None,
        })
    } else {
        None
    };
    let has_audio = system_audio.is_some();

    let meta = import_meta(
        project_path,
        pretty_name,
        transcoded.fps,
        system_audio,
        StudioRecordingStatus::Complete,
    );
    meta.save_for_project()
        .map_err(|e| ImportError::TranscodeFailed(format!("Failed to save metadata: {e:?}")))?;

    let screenshots_dir = project_path.join("screenshots");
    match std::fs::create_dir_all(&screenshots_dir) {
        Ok(()) => {
            if let Err(e) = write_video_thumbnail(
                &output_video_path,
                &screenshots_dir.join("display.jpg"),
                None,
            ) {
                error!("Failed to create thumbnail for imported video: {e}");
            }
        }
        Err(e) => error!("Failed to create screenshots directory: {e:?}"),
    }

    on_progress(ImportStage::Complete, 1.0, "Import complete!");
    Ok(ImportedVideo {
        fps: transcoded.fps,
        has_audio,
    })
}

fn import_meta(
    project_path: &Path,
    pretty_name: &str,
    fps: u32,
    system_audio: Option<AudioMeta>,
    status: StudioRecordingStatus,
) -> RecordingMeta {
    RecordingMeta {
        platform: Some(Platform::default()),
        project_path: project_path.to_path_buf(),
        pretty_name: pretty_name.to_string(),
        sharing: None,
        inner: RecordingMetaInner::Studio(Box::new(StudioRecordingMeta::MultipleSegments {
            inner: MultipleSegments {
                segments: vec![MultipleSegment {
                    display: VideoMeta {
                        path: RelativePathBuf::from("content/segments/segment-0/display.mp4"),
                        fps,
                        start_time: Some(0.0),
                        device_id: None,
                    },
                    camera: None,
                    mic: None,
                    system_audio,
                    cursor: None,
                    keyboard: None,
                }],
                cursors: Cursors::default(),
                status: Some(status),
            },
        })),
        upload: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_name_derives_from_file_stem() {
        let name = generate_project_name(Path::new("/tmp/My Video.mp4"));
        assert!(name.contains("My Video"), "got: {name}");
    }

    #[test]
    fn sanitize_strips_path_separators() {
        let sanitized = sanitize_filename("a/b\\c:d");
        assert!(!sanitized.contains('/') && !sanitized.contains('\\') && !sanitized.contains(':'));
    }

    #[test]
    fn unique_project_path_appends_counter_when_taken() {
        let dir = tempfile::tempdir().unwrap();
        let (first, _) = unique_project_path(dir.path(), Path::new("clip.mp4"));
        std::fs::create_dir_all(&first).unwrap();
        let (second, _) = unique_project_path(dir.path(), Path::new("clip.mp4"));
        assert_ne!(first, second);
        assert!(second.to_string_lossy().ends_with("(1).cap"));
    }
}
