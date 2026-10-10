use std::{path::Path, process::Command};

use cap_video_import::prepare_video_import;

const SOURCE_COLOR: &str = "0x8C8C8C";

fn generate_tagged_clip(output_path: &Path, vui: &str) {
    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={SOURCE_COLOR}:s=64x64:d=1:r=30"),
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p10le",
            "-x264-params",
            vui,
            output_path.to_str().unwrap(),
        ])
        .output()
        .expect("ffmpeg must be installed to run video-import tests");
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

struct ImportedColor {
    source: [i32; 3],
    imported: [i32; 3],
}

fn import_center_rgb(dir: &Path, name: &str, vui: &str) -> ImportedColor {
    let source = dir.join(format!("{name}.mp4"));
    generate_tagged_clip(&source, vui);
    let project_path = dir.join(format!("{name}.cap"));
    prepare_video_import(&source, &project_path, name)
        .unwrap()
        .run(|_, _, _| {})
        .unwrap();

    ImportedColor {
        source: center_rgb(&source),
        imported: center_rgb(&project_path.join("content/segments/segment-0/display.mp4")),
    }
}

fn center_rgb(video: &Path) -> [i32; 3] {
    let output = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-i",
            video.to_str().unwrap(),
            "-frames:v",
            "1",
            "-vf",
            "crop=2:2:31:31,scale=1:1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    [
        i32::from(output.stdout[0]),
        i32::from(output.stdout[1]),
        i32::from(output.stdout[2]),
    ]
}

fn channel_spread(rgb: [i32; 3]) -> i32 {
    rgb.iter().max().unwrap() - rgb.iter().min().unwrap()
}

#[test]
fn hlg_grey_is_tone_mapped_and_stays_neutral_while_sdr_is_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let sdr = import_center_rgb(
        dir.path(),
        "sdr",
        "colorprim=bt709:transfer=bt709:colormatrix=bt709",
    );
    let hlg = import_center_rgb(
        dir.path(),
        "hlg",
        "colorprim=bt2020:transfer=arib-std-b67:colormatrix=bt2020nc",
    );

    for (source, imported) in sdr.source.iter().zip(sdr.imported) {
        assert!(
            (source - imported).abs() <= 3,
            "SDR import changed colour: {:?} -> {:?}",
            sdr.source,
            sdr.imported
        );
    }

    assert!(
        (hlg.imported[1] - hlg.source[1]).abs() >= 15,
        "HLG source was not tone-mapped: {:?} -> {:?}",
        hlg.source,
        hlg.imported
    );
    assert!(
        channel_spread(hlg.imported) <= 3,
        "tone mapping tinted a neutral grey: {:?}",
        hlg.imported
    );
}
