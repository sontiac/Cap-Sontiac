use std::path::{Path, PathBuf};

use cap_enc_ffmpeg::remux::{get_media_duration, probe_video_can_decode};
use cap_project::{
    AudioMeta, ClipConfiguration, Cursors, MultipleSegment, MultipleSegments, ProjectConfiguration,
    RecordingMeta, RecordingMetaInner, SingleSegment, StudioRecordingMeta, StudioRecordingStatus,
    TimelineConfiguration, TimelineSegment, VideoMeta,
};
use relative_path::RelativePathBuf;
use tracing::{info, warn};

use crate::{ImportError, ImportStage, transcode::transcode_video};

const MIN_VALID_AUDIO_SIZE: u64 = 1000;

pub struct AppendedSegment {
    pub index: u32,
    pub fps: u32,
    pub has_audio: bool,
    pub has_camera: bool,
    pub duration_secs: f64,
}

pub(crate) fn imported_audio(
    sample_rate: Option<u32>,
    audio_path: &Path,
    relative_path: RelativePathBuf,
) -> Option<AudioMeta> {
    let audio_file_size = std::fs::metadata(audio_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if sample_rate.is_some() && audio_file_size > MIN_VALID_AUDIO_SIZE {
        Some(AudioMeta {
            path: relative_path,
            start_time: Some(0.0),
            device_id: None,
            gap_summary: None,
        })
    } else {
        None
    }
}

pub fn single_segment_to_multiple(segment: &SingleSegment) -> MultipleSegment {
    MultipleSegment {
        display: segment.display.clone(),
        camera: segment.camera.clone(),
        mic: segment.audio.clone(),
        system_audio: None,
        cursor: segment.cursor.clone(),
        keyboard: None,
    }
}

pub fn ensure_multiple_segments(
    meta: &mut RecordingMeta,
) -> Result<&mut MultipleSegments, ImportError> {
    let RecordingMetaInner::Studio(studio_meta) = &mut meta.inner else {
        return Err(ImportError::InstantRecording);
    };

    if let StudioRecordingMeta::SingleSegment { segment } = studio_meta.as_ref() {
        let segment = single_segment_to_multiple(segment);
        **studio_meta = StudioRecordingMeta::MultipleSegments {
            inner: MultipleSegments {
                segments: vec![segment],
                cursors: Cursors::default(),
                status: Some(StudioRecordingStatus::Complete),
            },
        };
    }

    let StudioRecordingMeta::MultipleSegments { inner } = studio_meta.as_mut() else {
        unreachable!("single-segment metadata was converted to multiple segments above");
    };
    Ok(inner)
}

pub fn video_duration_secs(path: &Path) -> Result<f64, ImportError> {
    get_media_duration(path)
        .map(|duration| duration.as_secs_f64())
        .ok_or_else(|| ImportError::UnknownDuration(path.display().to_string()))
}

pub fn full_timeline_for_segments(
    project_path: &Path,
    segments: &[MultipleSegment],
) -> Result<Vec<TimelineSegment>, ImportError> {
    segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            let duration = video_duration_secs(&segment.display.path.to_path(project_path))?;
            Ok(TimelineSegment {
                recording_clip: index as u32,
                timescale: 1.0,
                start: 0.0,
                end: duration,
            })
        })
        .collect()
}

pub fn ensure_project_timeline<'a>(
    config: &'a mut ProjectConfiguration,
    project_path: &Path,
    segments: &[MultipleSegment],
) -> Result<&'a mut TimelineConfiguration, ImportError> {
    let initial_segments = if config.timeline.is_none() {
        full_timeline_for_segments(project_path, segments)?
    } else {
        Vec::new()
    };

    Ok(config
        .timeline
        .get_or_insert_with(|| TimelineConfiguration {
            segments: initial_segments,
            zoom_segments: Vec::new(),
            scene_segments: Vec::new(),
            mask_segments: Vec::new(),
            text_segments: Vec::new(),
            caption_segments: Vec::new(),
            keyboard_segments: Vec::new(),
            sfx_segments: Vec::new(),
            overlay_segments: Vec::new(),
        }))
}

pub fn add_clip_configs(
    config: &mut ProjectConfiguration,
    base_index: u32,
    segments: &[MultipleSegment],
) {
    for (offset, segment) in segments.iter().enumerate() {
        let index = base_index + offset as u32;
        let offsets = segment.calculate_audio_offsets();

        if let Some(existing) = config.clips.iter_mut().find(|clip| clip.index == index) {
            existing.offsets = offsets;
        } else {
            config.clips.push(ClipConfiguration { index, offsets });
        }
    }
}

pub fn unique_segment_dir(
    project_path: &Path,
    index: u32,
) -> Result<(PathBuf, String), std::io::Error> {
    let segments_root = project_path.join("content").join("segments");
    std::fs::create_dir_all(&segments_root)?;

    let mut counter = 0;
    loop {
        let name = if counter == 0 {
            format!("segment-{index}")
        } else {
            format!("segment-{index}-import-{counter}")
        };
        let path = segments_root.join(&name);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
            return Ok((path, format!("content/segments/{name}")));
        }
        counter += 1;
    }
}

fn load_project_config(project_path: &Path) -> Result<ProjectConfiguration, ImportError> {
    match ProjectConfiguration::load(project_path) {
        Ok(config) => Ok(config),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ProjectConfiguration::default()),
        Err(e) => Err(ImportError::ProjectLoadFailed(format!(
            "project-config.json: {e}"
        ))),
    }
}

struct TranscodedTrack {
    video: VideoMeta,
    audio: Option<AudioMeta>,
    duration_secs: f64,
}

struct TrackFiles<'a> {
    video_name: &'a str,
    audio_name: &'a str,
}

const DISPLAY_FILES: TrackFiles<'static> = TrackFiles {
    video_name: "display.mp4",
    audio_name: "audio.ogg",
};
const CAMERA_FILES: TrackFiles<'static> = TrackFiles {
    video_name: "camera.mp4",
    audio_name: "mic.ogg",
};

fn probe_decodable(source_path: &Path) -> Result<(), ImportError> {
    let can_decode = probe_video_can_decode(source_path)
        .map_err(|e| ImportError::OpenFailed(format!("Cannot decode video: {e}")))?;
    if can_decode {
        Ok(())
    } else {
        Err(ImportError::OpenFailed(format!(
            "Video format not supported or file is corrupted: {}",
            source_path.display()
        )))
    }
}

fn transcode_track(
    project_path: &Path,
    source_path: &Path,
    relative_dir: &str,
    files: &TrackFiles,
    progress_range: (f64, f64),
    on_progress: &mut impl FnMut(ImportStage, f64, &str),
) -> Result<TranscodedTrack, ImportError> {
    let video_relative_path = RelativePathBuf::from(format!("{relative_dir}/{}", files.video_name));
    let audio_relative_path = RelativePathBuf::from(format!("{relative_dir}/{}", files.audio_name));
    let video_path = video_relative_path.to_path(project_path);
    let audio_path = audio_relative_path.to_path(project_path);

    let (range_start, range_end) = progress_range;
    on_progress(
        ImportStage::Converting,
        range_start,
        "Starting conversion...",
    );
    let transcoded = transcode_video(
        source_path,
        &video_path,
        Some(&audio_path),
        |progress| {
            let overall = range_start + progress * (range_end - range_start);
            on_progress(
                ImportStage::Converting,
                overall,
                &format!("Converting video... {}%", (overall * 100.0) as u32),
            )
        },
        || !crate::check_project_exists(project_path),
    )?;

    Ok(TranscodedTrack {
        video: VideoMeta {
            path: video_relative_path,
            fps: transcoded.fps,
            start_time: Some(0.0),
            device_id: None,
        },
        audio: imported_audio(transcoded.sample_rate, &audio_path, audio_relative_path),
        duration_secs: video_duration_secs(&video_path)?,
    })
}

struct TranscodedSegment {
    segment: MultipleSegment,
    duration_secs: f64,
}

fn transcode_segment(
    project_path: &Path,
    sources: &AppendSources,
    relative_dir: &str,
    on_progress: &mut impl FnMut(ImportStage, f64, &str),
) -> Result<TranscodedSegment, ImportError> {
    let display_range = if sources.camera.is_some() {
        (0.0, 0.47)
    } else {
        (0.0, 0.94)
    };
    let display = transcode_track(
        project_path,
        sources.display,
        relative_dir,
        &DISPLAY_FILES,
        display_range,
        on_progress,
    )?;
    let camera = sources
        .camera
        .map(|camera_source| {
            transcode_track(
                project_path,
                camera_source,
                relative_dir,
                &CAMERA_FILES,
                (0.47, 0.94),
                on_progress,
            )
        })
        .transpose()?;

    let duration_secs = camera.as_ref().map_or(display.duration_secs, |camera| {
        display.duration_secs.min(camera.duration_secs)
    });
    let (camera_video, mic) = match camera {
        Some(camera) => (Some(camera.video), camera.audio),
        None => (None, None),
    };

    Ok(TranscodedSegment {
        segment: MultipleSegment {
            display: display.video,
            camera: camera_video,
            mic,
            system_audio: display.audio,
            cursor: None,
            keyboard: None,
        },
        duration_secs,
    })
}

pub struct AppendSources<'a> {
    pub display: &'a Path,
    pub camera: Option<&'a Path>,
}

pub fn append_video_to_project(
    project_path: &Path,
    sources: &AppendSources,
    mut on_progress: impl FnMut(ImportStage, f64, &str),
) -> Result<AppendedSegment, ImportError> {
    let mut meta = RecordingMeta::load_for_project(project_path)
        .map_err(|e| ImportError::ProjectLoadFailed(format!("recording-meta.json: {e}")))?;
    let mut config = load_project_config(project_path)?;
    let existing_segments = {
        let inner = ensure_multiple_segments(&mut meta)?;
        inner.status = Some(StudioRecordingStatus::Complete);
        inner.segments.clone()
    };
    ensure_project_timeline(&mut config, project_path, &existing_segments)?;

    on_progress(ImportStage::Probing, 0.0, "Analyzing video file...");
    probe_decodable(sources.display)?;
    if let Some(camera_source) = sources.camera {
        probe_decodable(camera_source)?;
    }

    let index = existing_segments.len() as u32;
    let (segment_dir, relative_dir) =
        unique_segment_dir(project_path, index).map_err(ImportError::DirectoryFailed)?;

    let transcoded = match transcode_segment(project_path, sources, &relative_dir, &mut on_progress)
    {
        Ok(transcoded) => transcoded,
        Err(e) => {
            if let Err(cleanup_error) = std::fs::remove_dir_all(&segment_dir) {
                warn!(
                    "Failed to remove partial segment directory {}: {cleanup_error}",
                    segment_dir.display()
                );
            }
            return Err(e);
        }
    };

    on_progress(
        ImportStage::Finalizing,
        0.95,
        "Updating project metadata...",
    );

    let TranscodedSegment {
        segment,
        duration_secs,
    } = transcoded;
    let appended = AppendedSegment {
        index,
        fps: segment.display.fps,
        has_audio: segment.system_audio.is_some() || segment.mic.is_some(),
        has_camera: segment.camera.is_some(),
        duration_secs,
    };

    ensure_multiple_segments(&mut meta)?
        .segments
        .push(segment.clone());
    ensure_project_timeline(&mut config, project_path, &existing_segments)?
        .segments
        .push(TimelineSegment {
            recording_clip: index,
            timescale: 1.0,
            start: 0.0,
            end: duration_secs,
        });
    add_clip_configs(&mut config, index, std::slice::from_ref(&segment));

    meta.save_for_project()
        .map_err(|e| ImportError::MetaSaveFailed(format!("{e:?}")))?;
    config
        .write(project_path)
        .map_err(|e| ImportError::ConfigSaveFailed(e.to_string()))?;

    info!(
        "Appended {} to {} as segment {index}",
        sources.display.display(),
        project_path.display()
    );
    on_progress(ImportStage::Complete, 1.0, "Import complete!");
    Ok(appended)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cap_project::Platform;

    fn studio_meta(project_path: &Path, studio: StudioRecordingMeta) -> RecordingMeta {
        RecordingMeta {
            platform: Some(Platform::default()),
            project_path: project_path.to_path_buf(),
            pretty_name: "test".to_string(),
            sharing: None,
            inner: RecordingMetaInner::Studio(Box::new(studio)),
            upload: None,
        }
    }

    fn video_meta(path: &str) -> VideoMeta {
        VideoMeta {
            path: RelativePathBuf::from(path),
            fps: 30,
            start_time: Some(0.0),
            device_id: None,
        }
    }

    #[test]
    fn ensure_multiple_segments_converts_single_segment() {
        let mut meta = studio_meta(
            Path::new("/tmp/x.cap"),
            StudioRecordingMeta::SingleSegment {
                segment: SingleSegment {
                    display: video_meta("content/display.mp4"),
                    camera: None,
                    audio: None,
                    cursor: None,
                },
            },
        );

        let inner = ensure_multiple_segments(&mut meta).unwrap();

        assert_eq!(inner.segments.len(), 1);
        assert_eq!(
            inner.segments[0].display.path.as_str(),
            "content/display.mp4"
        );
        assert!(matches!(
            meta.inner,
            RecordingMetaInner::Studio(ref studio)
                if matches!(**studio, StudioRecordingMeta::MultipleSegments { .. })
        ));
    }

    #[test]
    fn add_clip_configs_updates_existing_and_adds_missing() {
        let mut config = ProjectConfiguration::default();
        config.clips.push(ClipConfiguration {
            index: 0,
            offsets: Default::default(),
        });
        let segment = MultipleSegment {
            display: video_meta("content/segments/segment-0/display.mp4"),
            camera: None,
            mic: None,
            system_audio: None,
            cursor: None,
            keyboard: None,
        };

        add_clip_configs(&mut config, 0, &[segment.clone(), segment]);

        let mut indices: Vec<u32> = config.clips.iter().map(|clip| clip.index).collect();
        indices.sort_unstable();
        assert_eq!(indices, vec![0, 1]);
    }

    #[test]
    fn unique_segment_dir_skips_taken_names() {
        let dir = tempfile::tempdir().unwrap();
        let (first, first_relative) = unique_segment_dir(dir.path(), 1).unwrap();
        let (second, second_relative) = unique_segment_dir(dir.path(), 1).unwrap();

        assert_eq!(first_relative, "content/segments/segment-1");
        assert_eq!(second_relative, "content/segments/segment-1-import-1");
        assert!(first.is_dir() && second.is_dir());
    }

    #[test]
    fn missing_project_config_loads_as_default() {
        let dir = tempfile::tempdir().unwrap();
        let config = load_project_config(dir.path()).unwrap();
        assert!(config.timeline.is_none());
    }

    #[test]
    fn corrupt_project_config_is_an_error_not_a_reset() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("project-config.json"), "{ not json").unwrap();
        assert!(matches!(
            load_project_config(dir.path()),
            Err(ImportError::ProjectLoadFailed(_))
        ));
    }
}
