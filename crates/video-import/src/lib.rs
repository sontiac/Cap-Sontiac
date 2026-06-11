use serde::Serialize;
use specta::Type;

mod import;
mod thumbnail;
mod transcode;

pub use thumbnail::write_video_thumbnail;
pub use transcode::transcode_video;

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
    #[error("Import cancelled")]
    Cancelled,
}
