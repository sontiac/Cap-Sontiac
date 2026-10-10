use std::{path::Path, process::Command};

use cap_project::{ProjectConfiguration, RecordingMeta, RecordingMetaInner, StudioRecordingMeta};
use cap_video_import::{AppendSources, ImportError, append_video_to_project, prepare_video_import};

fn generate_clip(output_path: &Path, with_audio: bool) {
    let mut args: Vec<String> = vec![
        "-y".into(),
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        "testsrc=duration=2:size=180x320:rate=30".into(),
    ];
    if with_audio {
        args.extend([
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            "sine=frequency=440:duration=2".into(),
            "-c:a".into(),
            "aac".into(),
        ]);
    }
    args.extend([
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "ultrafast".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-shortest".into(),
        output_path.to_str().unwrap().into(),
    ]);
    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .expect("ffmpeg must be installed to run video-import tests");
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn import_project(dir: &Path, source: &Path) -> std::path::PathBuf {
    let project_path = dir.join("project.cap");
    prepare_video_import(source, &project_path, "project")
        .unwrap()
        .run(|_, _, _| {})
        .unwrap();
    project_path
}

fn segment_count(project_path: &Path) -> usize {
    let meta = RecordingMeta::load_for_project(project_path).unwrap();
    let RecordingMetaInner::Studio(studio) = meta.inner else {
        panic!("imported project is not a studio recording");
    };
    let StudioRecordingMeta::MultipleSegments { inner } = *studio else {
        panic!("imported project is not multi-segment");
    };
    inner.segments.len()
}

#[test]
fn appends_silent_clip_as_new_segment_and_timeline_entry() {
    let dir = tempfile::tempdir().unwrap();
    let talking = dir.path().join("talking.mp4");
    let silent = dir.path().join("silent.mp4");
    generate_clip(&talking, true);
    generate_clip(&silent, false);
    let project_path = import_project(dir.path(), &talking);

    let mut stages = Vec::new();
    let appended = append_video_to_project(
        &project_path,
        &AppendSources {
            display: &silent,
            camera: None,
        },
        |stage, _, _| stages.push(stage),
    )
    .unwrap();

    assert_eq!(appended.index, 1);
    assert!(!appended.has_audio);
    assert!((appended.duration_secs - 2.0).abs() < 0.2);
    assert_eq!(
        stages.last(),
        Some(&cap_video_import::ImportStage::Complete)
    );

    let meta = RecordingMeta::load_for_project(&project_path).unwrap();
    let RecordingMetaInner::Studio(studio) = meta.inner else {
        panic!("expected a studio recording");
    };
    let StudioRecordingMeta::MultipleSegments { inner } = *studio else {
        panic!("expected multiple segments");
    };
    assert_eq!(inner.segments.len(), 2);
    assert!(inner.segments[0].system_audio.is_some());
    assert!(inner.segments[1].system_audio.is_none());
    assert!(
        inner.segments[1]
            .display
            .path
            .to_path(&project_path)
            .is_file()
    );

    let config = ProjectConfiguration::load(&project_path).unwrap();
    let timeline = config.timeline.expect("append writes a timeline");
    let clips: Vec<u32> = timeline.segments.iter().map(|s| s.recording_clip).collect();
    assert_eq!(clips, vec![0, 1]);
    assert!(config.clips.iter().any(|clip| clip.index == 1));
}

#[test]
fn undecodable_source_leaves_project_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let talking = dir.path().join("talking.mp4");
    generate_clip(&talking, true);
    let project_path = import_project(dir.path(), &talking);
    let broken = dir.path().join("broken.mp4");
    std::fs::write(&broken, b"not a video").unwrap();

    let result = append_video_to_project(
        &project_path,
        &AppendSources {
            display: &broken,
            camera: None,
        },
        |_, _, _| {},
    );

    assert!(matches!(result, Err(ImportError::OpenFailed(_))));
    assert_eq!(segment_count(&project_path), 1);
    assert!(!project_path.join("content/segments/segment-1").exists());
    assert!(!project_path.join("project-config.json").exists());
}

#[test]
fn camera_source_becomes_camera_and_mic_tracks_of_the_new_segment() {
    let dir = tempfile::tempdir().unwrap();
    let talking = dir.path().join("talking.mp4");
    let background = dir.path().join("background.mp4");
    let selfie = dir.path().join("selfie.mp4");
    generate_clip(&talking, true);
    generate_clip(&background, false);
    generate_clip(&selfie, true);
    let project_path = import_project(dir.path(), &talking);

    let appended = append_video_to_project(
        &project_path,
        &AppendSources {
            display: &background,
            camera: Some(&selfie),
        },
        |_, _, _| {},
    )
    .unwrap();

    assert!(appended.has_camera);
    assert!(appended.has_audio);
    let meta = RecordingMeta::load_for_project(&project_path).unwrap();
    let RecordingMetaInner::Studio(studio) = meta.inner else {
        panic!("expected a studio recording");
    };
    let StudioRecordingMeta::MultipleSegments { inner } = *studio else {
        panic!("expected multiple segments");
    };
    let segment = &inner.segments[1];
    assert!(segment.system_audio.is_none());
    let camera = segment.camera.as_ref().expect("camera track");
    assert!(camera.path.to_path(&project_path).is_file());
    let mic = segment
        .mic
        .as_ref()
        .expect("mic track from the camera source");
    assert!(mic.path.to_path(&project_path).is_file());
}

#[test]
fn undecodable_camera_source_leaves_project_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let talking = dir.path().join("talking.mp4");
    generate_clip(&talking, true);
    let project_path = import_project(dir.path(), &talking);
    let broken = dir.path().join("broken.mp4");
    std::fs::write(&broken, b"not a video").unwrap();

    let result = append_video_to_project(
        &project_path,
        &AppendSources {
            display: &talking,
            camera: Some(&broken),
        },
        |_, _, _| {},
    );

    assert!(matches!(result, Err(ImportError::OpenFailed(_))));
    assert_eq!(segment_count(&project_path), 1);
}
