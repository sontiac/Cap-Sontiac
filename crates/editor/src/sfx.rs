use std::collections::HashMap;
use std::sync::Arc;

use cap_audio::AudioData;
use cap_project::SfxSegment;

pub struct SfxTrack {
    pub data: Arc<AudioData>,
    pub start_sample: usize,
    pub end_sample: usize,
    pub volume: f32,
}

#[derive(Default)]
pub struct SfxCache {
    decoded: HashMap<String, Option<Arc<AudioData>>>,
}

impl SfxCache {
    pub fn resolve(&mut self, segments: &[SfxSegment]) -> Vec<SfxTrack> {
        let mut tracks = Vec::new();

        for seg in segments {
            let entry = self
                .decoded
                .entry(seg.file_path.clone())
                .or_insert_with(|| match AudioData::from_file(&seg.file_path) {
                    Ok(data) => Some(Arc::new(data)),
                    Err(e) => {
                        tracing::warn!(path = %seg.file_path, error = %e, "failed to decode sfx; skipping");
                        None
                    }
                });

            let Some(arc) = entry else {
                continue;
            };

            let (start_sample, end_sample) = sample_range(seg.start, seg.end);

            tracks.push(SfxTrack {
                data: arc.clone(),
                start_sample,
                end_sample,
                volume: seg.volume,
            });
        }

        tracks
    }

    #[cfg(test)]
    pub(crate) fn insert_decoded(&mut self, path: String, data: Option<Arc<AudioData>>) {
        self.decoded.insert(path, data);
    }
}

fn sample_range(start: f64, end: f64) -> (usize, usize) {
    let sample_rate = AudioData::SAMPLE_RATE as f64;
    let start_sample = (start * sample_rate).max(0.0).round() as usize;
    let end_sample = (end * sample_rate).max(0.0).round() as usize;
    (start_sample, end_sample)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(id: &str, path: &str, start: f64, end: f64, volume: f32) -> SfxSegment {
        SfxSegment {
            id: id.to_string(),
            start,
            end,
            file_path: path.to_string(),
            volume,
        }
    }

    #[test]
    fn skips_segments_with_undecodable_assets() {
        let mut cache = SfxCache::default();
        cache.insert_decoded("missing.mp3".to_string(), None);

        let tracks = cache.resolve(&[segment("a", "missing.mp3", 1.0, 2.0, 1.0)]);

        assert!(tracks.is_empty());
    }

    #[test]
    fn sample_range_rounds_to_nearest_sample() {
        assert_eq!(sample_range(1.0, 2.0), (48_000, 96_000));
        assert_eq!(sample_range(0.5, 1.5), (24_000, 72_000));
    }

    #[test]
    fn sample_range_clamps_negative_start_to_zero() {
        assert_eq!(sample_range(-1.0, 1.0), (0, 48_000));
    }

    #[test]
    fn sample_range_rounds_half_to_nearest() {
        let (start, _) = sample_range(1.0 / 96_000.0, 0.0);
        assert_eq!(start, 1);
    }
}
