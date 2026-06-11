use std::path::{Path, PathBuf};

use cap_video_import::{ImportStage, prepare_video_import, unique_project_path};
use clap::Args;
use serde::Serialize;

use crate::{recordings, write_json_line};

#[derive(Args)]
pub struct ImportArgs {
    /// Path to the source video file
    pub source: PathBuf,
    /// Destination library directory (defaults to the Cap Desktop recordings library)
    #[arg(long)]
    pub dir: Option<PathBuf>,
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ImportEvent<'a> {
    Progress {
        stage: &'a str,
        progress: f64,
    },
    Completed {
        project_path: &'a Path,
        fps: u32,
        has_audio: bool,
    },
    Error {
        error: &'a str,
    },
}

fn stage_name(stage: ImportStage) -> &'static str {
    match stage {
        ImportStage::Probing => "probing",
        ImportStage::Converting => "converting",
        ImportStage::Finalizing => "finalizing",
        ImportStage::Complete => "complete",
        ImportStage::Failed => "failed",
    }
}

impl ImportArgs {
    pub fn run(self, json: bool) -> Result<(), String> {
        let result = self.run_inner(json);
        if json && let Err(message) = &result {
            let _ = write_json_line(&ImportEvent::Error { error: message });
        }
        result
    }

    fn run_inner(&self, json: bool) -> Result<(), String> {
        if !self.source.is_file() {
            return Err(format!("Source video not found: {}", self.source.display()));
        }
        let library_dir = match &self.dir {
            Some(dir) => dir.clone(),
            None => recordings::default_library_dir()?,
        };
        std::fs::create_dir_all(&library_dir)
            .map_err(|e| format!("Failed to create library directory: {e}"))?;

        let (project_path, pretty_name) = unique_project_path(&library_dir, &self.source);

        if json {
            let _ = write_json_line(&ImportEvent::Progress {
                stage: "probing",
                progress: 0.0,
            });
        } else {
            eprintln!("Analyzing video file...");
        }
        let prepared = prepare_video_import(&self.source, &project_path, &pretty_name)
            .map_err(|e| e.to_string())?;

        let mut last_stage = None;
        let imported = prepared
            .run(|stage, progress, message| {
                if json {
                    let _ = write_json_line(&ImportEvent::Progress {
                        stage: stage_name(stage),
                        progress,
                    });
                } else if last_stage != Some(stage) {
                    last_stage = Some(stage);
                    eprintln!("{message}");
                }
            })
            .map_err(|e| e.to_string())?;

        if json {
            write_json_line(&ImportEvent::Completed {
                project_path: &project_path,
                fps: imported.fps,
                has_audio: imported.has_audio,
            })
        } else {
            println!("{}", project_path.display());
            Ok(())
        }
    }
}
