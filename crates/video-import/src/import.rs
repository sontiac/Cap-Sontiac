use std::path::{Path, PathBuf};

use cap_enc_ffmpeg::remux::probe_video_can_decode;
use cap_project::{
    AudioMeta, Cursors, MultipleSegment, MultipleSegments, Platform, RecordingMeta,
    RecordingMetaInner, StudioRecordingMeta, StudioRecordingStatus, VideoMeta,
};
use relative_path::RelativePathBuf;
use tracing::{error, info};

use crate::{
    ImportError, ImportStage, thumbnail::write_video_thumbnail, transcode::transcode_video,
};

pub struct ImportedVideo {
    pub fps: u32,
    pub has_audio: bool,
}

pub struct PreparedImport {
    source_path: PathBuf,
    project_path: PathBuf,
    pretty_name: String,
}

pub fn check_project_exists(project_path: &Path) -> bool {
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

pub fn prepare_video_import(
    source_path: &Path,
    project_path: &Path,
    pretty_name: &str,
) -> Result<PreparedImport, ImportError> {
    let can_decode = probe_video_can_decode(source_path)
        .map_err(|e| ImportError::OpenFailed(format!("Cannot decode video: {e}")))?;
    if !can_decode {
        return Err(ImportError::OpenFailed(
            "Video format not supported or file is corrupted".to_string(),
        ));
    }

    std::fs::create_dir_all(project_path).map_err(ImportError::DirectoryFailed)?;
    std::fs::create_dir_all(segment_dir(project_path)).map_err(ImportError::DirectoryFailed)?;

    let initial_meta = import_meta(
        project_path,
        pretty_name,
        30,
        None,
        StudioRecordingStatus::InProgress,
    );
    initial_meta
        .save_for_project()
        .map_err(|e| ImportError::MetaSaveFailed(format!("{e:?}")))?;

    Ok(PreparedImport {
        source_path: source_path.to_path_buf(),
        project_path: project_path.to_path_buf(),
        pretty_name: pretty_name.to_string(),
    })
}

impl PreparedImport {
    pub fn run(
        self,
        mut on_progress: impl FnMut(ImportStage, f64, &str),
    ) -> Result<ImportedVideo, ImportError> {
        let segment_dir = segment_dir(&self.project_path);
        let output_video_path = segment_dir.join("display.mp4");
        let output_audio_path = segment_dir.join("audio.ogg");

        on_progress(ImportStage::Converting, 0.0, "Starting conversion...");

        let transcoded = transcode_video(
            &self.source_path,
            &output_video_path,
            Some(&output_audio_path),
            |progress| {
                on_progress(
                    ImportStage::Converting,
                    progress,
                    &format!("Converting video... {}%", (progress * 100.0) as u32),
                )
            },
            || !check_project_exists(&self.project_path),
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
        let system_audio =
            if transcoded.sample_rate.is_some() && audio_file_size > MIN_VALID_AUDIO_SIZE {
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
            &self.project_path,
            &self.pretty_name,
            transcoded.fps,
            system_audio,
            StudioRecordingStatus::Complete,
        );
        meta.save_for_project()
            .map_err(|e| ImportError::MetaSaveFailed(format!("{e:?}")))?;

        let screenshots_dir = self.project_path.join("screenshots");
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

        info!("Video import complete: {:?}", self.project_path);
        on_progress(ImportStage::Complete, 1.0, "Import complete!");
        Ok(ImportedVideo {
            fps: transcoded.fps,
            has_audio,
        })
    }
}

fn segment_dir(project_path: &Path) -> PathBuf {
    project_path
        .join("content")
        .join("segments")
        .join("segment-0")
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

    #[test]
    fn import_meta_serializes_expected_on_disk_contract() {
        let meta = import_meta(
            Path::new("/tmp/x.cap"),
            "x",
            30,
            None,
            StudioRecordingStatus::InProgress,
        );
        let json = serde_json::to_value(&meta).unwrap();

        let segments = json["segments"].as_array().unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(
            segments[0]["display"]["path"],
            "content/segments/segment-0/display.mp4"
        );
        assert_eq!(segments[0]["display"]["fps"], 30);
        assert!(!json["status"].is_null());
    }
}
