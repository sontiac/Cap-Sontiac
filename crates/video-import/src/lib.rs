use serde::Serialize;
use specta::Type;

mod append;
mod hdr;
mod import;
mod thumbnail;
mod transcode;

pub use append::{
    AppendSources, AppendedSegment, add_clip_configs, append_video_to_project,
    ensure_multiple_segments, ensure_project_timeline, full_timeline_for_segments,
    single_segment_to_multiple, unique_segment_dir, video_duration_secs,
};
pub use import::{
    ImportedVideo, PreparedImport, check_project_exists, generate_project_name,
    prepare_video_import, sanitize_filename, unique_project_path,
};
pub use thumbnail::write_video_thumbnail;
pub use transcode::{TranscodeOutput, transcode_video};

#[derive(Serialize, Type, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportStage {
    Probing,
    Converting,
    Finalizing,
    Complete,
    Failed,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("Failed to open video file: {0}")]
    OpenFailed(String),
    #[error("No video stream found in file")]
    NoVideoStream,
    #[error("Failed to create decoder: {0}")]
    DecoderFailed(String),
    #[error("Failed to create encoder: {0}")]
    EncoderFailed(String),
    #[error("Failed to create project directory: {0}")]
    DirectoryFailed(std::io::Error),
    #[error("FFmpeg error: {0}")]
    Ffmpeg(#[from] ffmpeg::Error),
    #[error("Transcoding failed: {0}")]
    TranscodeFailed(String),
    #[error("Failed to save project metadata: {0}")]
    MetaSaveFailed(String),
    #[error("Import cancelled")]
    Cancelled,
    #[error("Failed to load project {0}")]
    ProjectLoadFailed(String),
    #[error("Instant mode recordings cannot be edited")]
    InstantRecording,
    #[error("Could not determine video duration: {0}")]
    UnknownDuration(String),
    #[error("Failed to save project config: {0}")]
    ConfigSaveFailed(String),
}
