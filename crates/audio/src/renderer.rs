use crate::AudioData;

pub enum StereoMode {
    Stereo,
    MonoL,
    MonoR,
}

pub struct AudioRendererTrack<'a> {
    pub data: &'a AudioData,
    pub gain: f32,
    pub stereo_mode: StereoMode,
    pub offset: isize,
}

pub fn render_audio(
    tracks: &[AudioRendererTrack],
    offset: usize,
    samples: usize,
    out_offset: usize,
    out: &mut [f32],
) -> usize {
    let samples = samples.min(
        tracks
            .iter()
            .filter_map(|t| {
                let track_samples = t.data.samples().len() / t.data.channels() as usize;
                let available = track_samples as i128 - offset as i128 - t.offset as i128;
                if available > 0 {
                    usize::try_from(available).ok()
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0),
    );

    for i in 0..samples {
        let mut left = 0.0;
        let mut right = 0.0;

        for track in tracks {
            let source_index = offset as i128 + i as i128 + track.offset as i128;
            if source_index < 0 {
                continue;
            }
            let Ok(source_index) = usize::try_from(source_index) else {
                continue;
            };

            let data = track.data;
            let gain = gain_for_db(track.gain);

            if gain == f32::NEG_INFINITY {
                continue;
            }

            if data.channels() == 1 {
                if let Some(sample) = data.samples().get(source_index) {
                    left += sample * 0.707 * gain;
                    right += sample * 0.707 * gain;
                }
            } else if data.channels() == 2 {
                let base_idx = source_index.saturating_mul(2);
                let Some(l_sample) = data.samples().get(base_idx) else {
                    continue;
                };
                let Some(r_sample) = data.samples().get(base_idx + 1) else {
                    continue;
                };

                match track.stereo_mode {
                    StereoMode::Stereo => {
                        left += l_sample * gain;
                        right += r_sample * gain;
                    }
                    StereoMode::MonoL => {
                        left += l_sample * gain;
                        right += l_sample * gain;
                    }
                    StereoMode::MonoR => {
                        left += r_sample * gain;
                        right += r_sample * gain;
                    }
                }
            }
        }

        let l = left.clamp(-1.0, 1.0);
        let r = right.clamp(-1.0, 1.0);
        out[out_offset + i * 2] = l;
        out[out_offset + i * 2 + 1] = r;
    }

    samples
}

pub struct SfxFrameTrack<'a> {
    pub data: &'a AudioData,
    pub start_sample: usize,
    pub end_sample: usize,
    pub volume: f32,
}

pub fn mix_sfx_frame(
    sfx: &[SfxFrameTrack],
    frame_start: usize,
    frame_samples: usize,
    out: &mut [f32],
) {
    let frame_end = frame_start + frame_samples;

    for track in sfx {
        let data = track.data;
        let channels = data.channels();
        if channels != 1 && channels != 2 {
            continue;
        }

        let effective_end = track
            .end_sample
            .min(track.start_sample.saturating_add(data.sample_count()));
        let range_start = track.start_sample.max(frame_start);
        let range_end = effective_end.min(frame_end);
        if range_start >= range_end {
            continue;
        }

        let samples = data.samples();
        for pos in range_start..range_end {
            let i = pos - frame_start;
            let src = pos - track.start_sample;

            if channels == 1 {
                let Some(sample) = samples.get(src) else {
                    continue;
                };
                let contribution = sample * 0.707 * track.volume;
                out[i * 2] += contribution;
                out[i * 2 + 1] += contribution;
            } else {
                let Some(l_sample) = samples.get(src * 2) else {
                    continue;
                };
                let Some(r_sample) = samples.get(src * 2 + 1) else {
                    continue;
                };
                out[i * 2] += l_sample * track.volume;
                out[i * 2 + 1] += r_sample * track.volume;
            }
        }
    }

    for value in out.iter_mut() {
        *value = value.clamp(-1.0, 1.0);
    }
}

fn gain_for_db(db: f32) -> f32 {
    match db {
        // Fully mute when at minimum
        v if v <= -30.0 => f32::NEG_INFINITY,
        v => db_to_linear(v),
    }
}
fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(data: &AudioData, offset: isize) -> AudioRendererTrack<'_> {
        AudioRendererTrack {
            data,
            gain: 0.0,
            stereo_mode: StereoMode::Stereo,
            offset,
        }
    }

    // The mix read index is `offset + i + track.offset`, so the cursor and the
    // per-track offset both move the source read position.
    #[test]
    fn reads_from_cursor_and_track_offset() {
        // Stereo ramp: frame k carries L = R = (k + 1) / 100.
        let mut samples = Vec::new();
        for k in 0..10 {
            let v = (k as f32 + 1.0) / 100.0;
            samples.push(v);
            samples.push(v);
        }
        let data = AudioData::from_raw_f32(samples, 2);

        let mut out = vec![0.0; 4 * 2];
        let rendered = render_audio(&[track(&data, 0)], 3, 4, 0, &mut out);
        assert_eq!(rendered, 4);
        // cursor 3 -> first output frame reads source frame 3 (value 0.04).
        assert!((out[0] - 0.04).abs() < 1e-6);
        assert!((out[2] - 0.05).abs() < 1e-6);

        let mut out = vec![0.0; 4 * 2];
        render_audio(&[track(&data, 2)], 3, 4, 0, &mut out);
        // cursor 3 + track offset 2 -> source frame 5 (value 0.06).
        assert!((out[0] - 0.06).abs() < 1e-6);
    }

    #[test]
    fn negative_offset_delays_track_with_leading_silence() {
        let mut samples = Vec::new();
        for k in 0..4 {
            let v = (k as f32 + 1.0) / 10.0;
            samples.push(v);
            samples.push(v);
        }
        let data = AudioData::from_raw_f32(samples, 2);

        let mut out = vec![0.0; 6 * 2];
        let rendered = render_audio(&[track(&data, -2)], 0, 6, 0, &mut out);

        assert_eq!(rendered, 6);
        assert_eq!(out[0], 0.0);
        assert_eq!(out[2], 0.0);
        assert!((out[4] - 0.1).abs() < 1e-6);
        assert!((out[10] - 0.4).abs() < 1e-6);
    }

    // Regression guard for commit 2a6dce7: render mixes up to the LONGEST track
    // and pads shorter tracks with silence (the `.max()` in render_audio). A
    // `.min()` here would truncate the mix to the shortest track.
    #[test]
    fn mixes_to_longest_track_padding_short_with_silence() {
        let long = AudioData::from_raw_f32(vec![0.5; 20], 2); // 10 stereo frames
        let short = AudioData::from_raw_f32(vec![0.25; 8], 2); // 4 stereo frames

        let mut out = vec![0.0; 10 * 2];
        let rendered = render_audio(&[track(&long, 0), track(&short, 0)], 0, 10, 0, &mut out);

        assert_eq!(
            rendered, 10,
            "must render up to the longest track, not the shortest"
        );
        // Frames 0..4 mix both tracks.
        assert!((out[0] - 0.75).abs() < 1e-6);
        assert!((out[3 * 2] - 0.75).abs() < 1e-6);
        // Frames 4..10: short track exhausted -> contributes silence, long track remains.
        assert!((out[4 * 2] - 0.5).abs() < 1e-6);
        assert!((out[9 * 2] - 0.5).abs() < 1e-6);
    }

    fn sfx(
        data: &AudioData,
        start_sample: usize,
        end_sample: usize,
        volume: f32,
    ) -> SfxFrameTrack<'_> {
        SfxFrameTrack {
            data,
            start_sample,
            end_sample,
            volume,
        }
    }

    #[test]
    fn mix_sfx_mono_inside_frame() {
        let data = AudioData::from_raw_f32(vec![0.1, 0.2, 0.3, 0.4], 1);
        let mut out = vec![0.0; 8 * 2];
        let volume = 0.5;
        mix_sfx_frame(&[sfx(&data, 2, 6, volume)], 0, 8, &mut out);

        for i in 0..8 {
            let l = out[i * 2];
            let r = out[i * 2 + 1];
            if (2..6).contains(&i) {
                let expected = data.samples()[i - 2] * 0.707 * volume;
                assert!((l - expected).abs() < 1e-6, "left {i}");
                assert!((r - expected).abs() < 1e-6, "right {i}");
            } else {
                assert_eq!(l, 0.0, "left silent {i}");
                assert_eq!(r, 0.0, "right silent {i}");
            }
        }
    }

    #[test]
    fn mix_sfx_stereo_channels_and_volume() {
        let data = AudioData::from_raw_f32(vec![0.1, -0.1, 0.2, -0.2, 0.3, -0.3], 2);
        let mut out = vec![0.0; 6 * 2];
        let volume = 0.5;
        mix_sfx_frame(&[sfx(&data, 1, 4, volume)], 0, 6, &mut out);

        for i in 0..6 {
            let l = out[i * 2];
            let r = out[i * 2 + 1];
            if (1..4).contains(&i) {
                let src = i - 1;
                let expected_l = data.samples()[src * 2] * volume;
                let expected_r = data.samples()[src * 2 + 1] * volume;
                assert!((l - expected_l).abs() < 1e-6, "left {i}");
                assert!((r - expected_r).abs() < 1e-6, "right {i}");
            } else {
                assert_eq!(l, 0.0, "left silent {i}");
                assert_eq!(r, 0.0, "right silent {i}");
            }
        }
    }

    #[test]
    fn mix_sfx_zero_volume_is_noop() {
        let data = AudioData::from_raw_f32(vec![0.5, 0.5, 0.5, 0.5], 1);
        let mut out = vec![0.3; 8 * 2];
        mix_sfx_frame(&[sfx(&data, 1, 5, 0.0)], 0, 8, &mut out);

        for v in &out {
            assert!((v - 0.3).abs() < 1e-6);
        }
    }

    #[test]
    fn mix_sfx_started_before_frame_reads_intra_file_offset() {
        let data = AudioData::from_raw_f32(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6], 1);
        let mut out = vec![0.0; 4 * 2];
        let volume = 1.0;
        mix_sfx_frame(&[sfx(&data, 2, 10, volume)], 4, 4, &mut out);

        for i in 0..4 {
            let pos = 4 + i;
            let src = pos - 2;
            let expected = data.samples()[src] * 0.707 * volume;
            assert!((out[i * 2] - expected).abs() < 1e-6, "left {i}");
            assert!((out[i * 2 + 1] - expected).abs() < 1e-6, "right {i}");
        }
    }

    #[test]
    fn mix_sfx_trim_by_end_sample() {
        let data = AudioData::from_raw_f32(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6], 1);
        let mut out = vec![0.0; 8 * 2];
        let volume = 1.0;
        mix_sfx_frame(&[sfx(&data, 1, 4, volume)], 0, 8, &mut out);

        for i in 0..8 {
            if (1..4).contains(&i) {
                let expected = data.samples()[i - 1] * 0.707 * volume;
                assert!((out[i * 2] - expected).abs() < 1e-6, "active {i}");
            } else {
                assert_eq!(out[i * 2], 0.0, "trimmed {i}");
            }
        }
    }

    #[test]
    fn mix_sfx_file_shorter_than_segment_is_silent_past_file() {
        let data = AudioData::from_raw_f32(vec![0.1, 0.2], 1);
        let mut out = vec![0.0; 8 * 2];
        let volume = 1.0;
        mix_sfx_frame(&[sfx(&data, 1, 7, volume)], 0, 8, &mut out);

        for i in 0..8 {
            if (1..3).contains(&i) {
                let expected = data.samples()[i - 1] * 0.707 * volume;
                assert!((out[i * 2] - expected).abs() < 1e-6, "active {i}");
            } else {
                assert_eq!(out[i * 2], 0.0, "past file {i}");
            }
        }
    }

    #[test]
    fn mix_sfx_clamps_to_unit_range() {
        let data = AudioData::from_raw_f32(vec![0.9, 0.9], 1);
        let mut out = vec![0.8; 2 * 2];
        mix_sfx_frame(&[sfx(&data, 0, 2, 1.0)], 0, 2, &mut out);

        for i in 0..2 {
            assert!(0.8 + data.samples()[i] * 0.707 > 1.0);
            assert!((out[i * 2] - 1.0).abs() < 1e-6, "left clamp {i}");
            assert!((out[i * 2 + 1] - 1.0).abs() < 1e-6, "right clamp {i}");
        }
    }

    #[test]
    fn mix_sfx_empty_slice_leaves_out_untouched() {
        let mut out = vec![0.42; 4 * 2];
        mix_sfx_frame(&[], 0, 4, &mut out);

        for v in &out {
            assert!((v - 0.42).abs() < 1e-6);
        }
    }

    #[test]
    fn mix_sfx_reverse_range_is_noop() {
        let data = AudioData::from_raw_f32(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6], 1);
        let mut out = vec![0.3; 8 * 2];
        mix_sfx_frame(&[sfx(&data, 6, 2, 1.0)], 0, 8, &mut out);

        for v in &out {
            assert!((v - 0.3).abs() < 1e-6);
        }
    }

    #[test]
    fn mix_sfx_entirely_outside_frame_is_noop() {
        let data = AudioData::from_raw_f32(vec![0.1, 0.2, 0.3, 0.4], 1);
        let mut out = vec![0.3; 8 * 2];
        mix_sfx_frame(&[sfx(&data, 100, 110, 1.0)], 0, 8, &mut out);

        for v in &out {
            assert!((v - 0.3).abs() < 1e-6);
        }
    }
}
