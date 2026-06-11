use std::path::{Path, PathBuf};

use cap_project::{RecordingMeta, RecordingMetaInner, StudioRecordingMeta};
use serde::Serialize;

use crate::{OutputFormat, write_json};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInspection {
    pub project_path: PathBuf,
    pub output_path: PathBuf,
    /// camelCase convenience fields so agents never reach into the snake_case `meta` passthrough.
    pub name: String,
    pub recording_type: &'static str,
    pub meta: RecordingMeta,
    pub config: cap_project::ProjectConfiguration,
}

pub fn config_get(project_path: PathBuf) -> Result<(), String> {
    let config = match cap_project::ProjectConfiguration::load(&project_path) {
        Ok(config) => config,
        // Instant and un-edited studio recordings have no project-config.json; return the
        // effective default the editor/exporter would use rather than erroring.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::from_str("{}")
            .map_err(|e| format!("Failed to build default project config: {e}"))?,
        Err(e) => return Err(format!("Failed to load project config: {e}")),
    };
    crate::write_json(&config)
}

fn numbers_match(a: &serde_json::Number, b: &serde_json::Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() <= 1e-6 * x.abs().max(y.abs()).max(1.0),
        _ => false,
    }
}

fn is_vacuous(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::Array(items) => items.is_empty(),
        serde_json::Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

fn collect_dropped_paths(
    input: &serde_json::Value,
    round_tripped: &serde_json::Value,
    path: &str,
    out: &mut Vec<String>,
) {
    match input {
        serde_json::Value::Object(map) => match round_tripped.as_object() {
            Some(rt) => {
                for (key, value) in map {
                    let child_path = format!("{path}.{key}");
                    match rt.get(key) {
                        Some(rt_value) => collect_dropped_paths(value, rt_value, &child_path, out),
                        None => out.push(child_path),
                    }
                }
            }
            None if is_vacuous(input) => {}
            None => out.push(path.to_string()),
        },
        serde_json::Value::Array(items) => match round_tripped.as_array() {
            Some(rt) if rt.len() == items.len() => {
                for (index, (value, rt_value)) in items.iter().zip(rt).enumerate() {
                    collect_dropped_paths(value, rt_value, &format!("{path}[{index}]"), out);
                }
            }
            _ if is_vacuous(input) => {}
            _ => out.push(path.to_string()),
        },
        serde_json::Value::Number(number) => {
            if !numbers_match(number, round_tripped) {
                out.push(path.to_string());
            }
        }
        serde_json::Value::Null => {}
        _ => {
            if input != round_tripped {
                out.push(path.to_string());
            }
        }
    }
}

pub fn config_set(
    project_path: PathBuf,
    settings_json: &str,
    format: OutputFormat,
) -> Result<(), String> {
    let submitted: serde_json::Value = serde_json::from_str(settings_json)
        .map_err(|e| format!("Invalid project config JSON: {e}"))?;
    let config: cap_project::ProjectConfiguration = serde_json::from_str(settings_json)
        .map_err(|e| format!("Invalid project config JSON: {e}"))?;
    let round_tripped = serde_json::to_value(&config)
        .map_err(|e| format!("Failed to re-serialize project config: {e}"))?;

    let mut dropped = Vec::new();
    collect_dropped_paths(&submitted, &round_tripped, "$", &mut dropped);
    if !dropped.is_empty() {
        return Err(format!(
            "Write aborted: these submitted fields would not survive a round-trip through this Cap version's project config and would be silently dropped or rewritten: {}. Run `cap project config get {}` for the valid shape.",
            dropped.join(", "),
            project_path.display()
        ));
    }

    // write() validates internally before its atomic temp-file-then-rename.
    config
        .write(&project_path)
        .map_err(|e| format!("Failed to write project config: {e}"))?;
    if let OutputFormat::Json = format {
        crate::write_json(&serde_json::json!({ "ok": true }))?;
    }
    Ok(())
}

pub fn inspect(project_path: PathBuf, format: OutputFormat) -> Result<(), String> {
    let meta = RecordingMeta::load_for_project(&project_path)
        .map_err(|e| format!("Failed to load recording meta: {e}"))?;
    let output_path = meta.output_path();
    let config = meta.project_config();

    match format {
        OutputFormat::Text => {
            println!("project: {}", project_path.display());
            println!("name: {}", meta.pretty_name);
            println!("type: {}", recording_type(&meta));
            println!("output: {}", output_path.display());
            Ok(())
        }
        OutputFormat::Json => write_json(&ProjectInspection {
            project_path,
            output_path,
            name: meta.pretty_name.clone(),
            recording_type: recording_type(&meta),
            meta,
            config,
        }),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileCheck {
    role: &'static str,
    path: PathBuf,
    exists: bool,
    required: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidationReport {
    project_path: PathBuf,
    valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    recording_type: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    checks: Vec<FileCheck>,
    missing: Vec<PathBuf>,
}

fn recording_type(meta: &RecordingMeta) -> &'static str {
    match meta.inner {
        RecordingMetaInner::Studio(_) => "studio",
        RecordingMetaInner::Instant(_) => "instant",
    }
}

fn required_check(role: &'static str, path: PathBuf) -> FileCheck {
    FileCheck {
        exists: path.exists(),
        role,
        path,
        required: true,
    }
}

fn optional_check(role: &'static str, path: PathBuf) -> FileCheck {
    FileCheck {
        exists: path.exists(),
        role,
        path,
        required: false,
    }
}

fn studio_checks(meta: &RecordingMeta, studio: &StudioRecordingMeta) -> Vec<FileCheck> {
    let mut checks = Vec::new();

    match studio {
        StudioRecordingMeta::SingleSegment { segment } => {
            checks.push(required_check(
                "displayVideo",
                meta.path(&segment.display.path),
            ));
            if let Some(camera) = &segment.camera {
                checks.push(required_check("camera", meta.path(&camera.path)));
            }
            if let Some(audio) = &segment.audio {
                checks.push(required_check("audio", meta.path(&audio.path)));
            }
            if let Some(cursor) = &segment.cursor {
                checks.push(optional_check("cursor", meta.path(cursor)));
            }
        }
        StudioRecordingMeta::MultipleSegments { inner } => {
            for segment in &inner.segments {
                checks.push(required_check(
                    "displayVideo",
                    meta.path(&segment.display.path),
                ));
                if let Some(camera) = &segment.camera {
                    checks.push(required_check("camera", meta.path(&camera.path)));
                }
                if let Some(mic) = &segment.mic {
                    checks.push(required_check("mic", meta.path(&mic.path)));
                }
                if let Some(system_audio) = &segment.system_audio {
                    checks.push(required_check("systemAudio", meta.path(&system_audio.path)));
                }
                if let Some(cursor) = &segment.cursor {
                    checks.push(optional_check("cursor", meta.path(cursor)));
                }
            }
        }
    }

    checks
}

fn build_report(project_path: &Path, meta: &RecordingMeta) -> ValidationReport {
    let mut checks = vec![required_check(
        "recordingMeta",
        project_path.join("recording-meta.json"),
    )];
    checks.push(optional_check(
        "projectConfig",
        project_path.join("project-config.json"),
    ));

    if let RecordingMetaInner::Studio(studio) = &meta.inner {
        checks.extend(studio_checks(meta, studio));
    }

    checks.push(optional_check("output", meta.output_path()));

    let missing: Vec<PathBuf> = checks
        .iter()
        .filter(|c| c.required && !c.exists)
        .map(|c| c.path.clone())
        .collect();

    let valid = missing.is_empty();
    // Every `--json` command signals failure with an `error` field (see AGENT_HELP / `cap guide`), so a
    // missing-media validation must carry one too, not just `valid:false`.
    let error = (!valid).then(|| format!("project is missing {} required file(s)", missing.len()));

    ValidationReport {
        project_path: project_path.to_path_buf(),
        valid,
        recording_type: Some(recording_type(meta)),
        error,
        checks,
        missing,
    }
}

pub fn validate(project_path: PathBuf, format: OutputFormat) -> Result<(), String> {
    let report = match RecordingMeta::load_for_project(&project_path) {
        Ok(meta) => build_report(&project_path, &meta),
        Err(e) => ValidationReport {
            checks: vec![required_check(
                "recordingMeta",
                project_path.join("recording-meta.json"),
            )],
            missing: vec![project_path.join("recording-meta.json")],
            project_path: project_path.clone(),
            valid: false,
            recording_type: None,
            error: Some(format!("Failed to load recording meta: {e}")),
        },
    };

    let valid = report.valid;

    match format {
        OutputFormat::Json => write_json(&report)?,
        OutputFormat::Text => {
            println!("project: {}", report.project_path.display());
            if let Some(error) = &report.error {
                println!("error: {error}");
            }
            for check in &report.checks {
                let status = if check.exists { "ok" } else { "missing" };
                let required = if check.required {
                    "required"
                } else {
                    "optional"
                };
                println!(
                    "  [{status}] {} ({required}): {}",
                    check.role,
                    check.path.display()
                );
            }
            println!("valid: {valid}");
        }
    }

    if valid {
        Ok(())
    } else {
        Err("project validation failed".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dropped(input: serde_json::Value) -> Vec<String> {
        let config: cap_project::ProjectConfiguration =
            serde_json::from_value(input.clone()).unwrap();
        let round_tripped = serde_json::to_value(&config).unwrap();
        let mut out = Vec::new();
        collect_dropped_paths(&input, &round_tripped, "$", &mut out);
        out
    }

    #[test]
    fn unknown_top_level_field_is_reported() {
        assert_eq!(dropped(json!({ "notAField": 1 })), vec!["$.notAField"]);
    }

    #[test]
    fn typo_inside_timeline_is_reported() {
        let paths = dropped(json!({
            "timeline": { "segments": [], "zoomSegments": [], "segemnts": [] }
        }));
        assert_eq!(paths, vec!["$.timeline.segemnts"]);
    }

    #[test]
    fn valid_timeline_passes() {
        let paths = dropped(json!({
            "timeline": {
                "segments": [
                    { "recordingSegment": 0, "timescale": 1.0, "start": 1.5, "end": 9.25 }
                ],
                "zoomSegments": []
            }
        }));
        assert!(paths.is_empty(), "unexpected: {paths:?}");
    }

    #[test]
    fn integer_for_float_field_passes() {
        let paths = dropped(json!({
            "timeline": {
                "segments": [{ "recordingSegment": 0, "timescale": 1, "start": 0, "end": 10 }],
                "zoomSegments": []
            }
        }));
        assert!(paths.is_empty(), "unexpected: {paths:?}");
    }

    #[test]
    fn null_and_empty_inputs_are_not_reported() {
        let paths = dropped(json!({ "timeline": null }));
        assert!(paths.is_empty(), "unexpected: {paths:?}");

        let paths = dropped(json!({ "annotations": [], "hotkeys": {} }));
        assert!(paths.is_empty(), "unexpected: {paths:?}");
    }
}
