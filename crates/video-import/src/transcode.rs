use std::path::Path;

use cap_enc_ffmpeg::{
    AudioEncoder,
    h264::{H264EncoderBuilder, H264Preset},
    opus::OpusEncoder,
    remux::get_media_duration,
};
use cap_media_info::{AudioInfo, FFRational, Pixel, VideoInfo, ensure_even};
use ffmpeg::{
    ChannelLayout,
    codec::{self as avcodec},
    format::{self as avformat},
};
use tracing::info;

use crate::ImportError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VideoRotation {
    None,
    Cw90,
    Half,
    Ccw90,
}

fn rotation_from_clockwise_degrees(deg: i32) -> VideoRotation {
    match deg.rem_euclid(360) {
        90 => VideoRotation::Cw90,
        180 => VideoRotation::Half,
        270 => VideoRotation::Ccw90,
        _ => VideoRotation::None,
    }
}

fn read_video_rotation(input: &avformat::context::Input) -> VideoRotation {
    let Some(stream) = input.streams().best(ffmpeg::media::Type::Video) else {
        return VideoRotation::None;
    };

    if let Some(rotate_str) = stream.metadata().get("rotate")
        && let Ok(deg) = rotate_str.parse::<i32>()
    {
        let rotation = rotation_from_clockwise_degrees(deg);
        if rotation != VideoRotation::None {
            return rotation;
        }
    }

    for side_data in stream.side_data() {
        if side_data.kind() != ffmpeg::codec::packet::side_data::Type::DisplayMatrix {
            continue;
        }
        let bytes = side_data.data();
        if bytes.len() < std::mem::size_of::<[i32; 9]>() {
            continue;
        }
        let mut matrix = [0i32; 9];
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                matrix.as_mut_ptr() as *mut u8,
                std::mem::size_of::<[i32; 9]>(),
            );
        }
        let ccw_angle = unsafe { ffmpeg::ffi::av_display_rotation_get(matrix.as_ptr()) };
        if ccw_angle.is_finite() {
            return rotation_from_clockwise_degrees(-ccw_angle.round() as i32);
        }
    }

    VideoRotation::None
}

fn get_video_stream_info(
    input: &avformat::context::Input,
) -> Result<(usize, VideoInfo), ImportError> {
    let stream = input
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or(ImportError::NoVideoStream)?;

    let stream_index = stream.index();
    let decoder_ctx = avcodec::Context::from_parameters(stream.parameters())
        .map_err(|e| ImportError::DecoderFailed(e.to_string()))?;
    let decoder = decoder_ctx
        .decoder()
        .video()
        .map_err(|e| ImportError::DecoderFailed(e.to_string()))?;

    let rate = stream.avg_frame_rate();
    let time_base = stream.time_base();

    let pixel_format = match decoder.format() {
        ffmpeg::format::Pixel::YUV420P => Pixel::YUV420P,
        ffmpeg::format::Pixel::NV12 => Pixel::NV12,
        ffmpeg::format::Pixel::BGRA => Pixel::BGRA,
        ffmpeg::format::Pixel::RGBA => Pixel::RGBA,
        ffmpeg::format::Pixel::RGB24 => Pixel::RGB24,
        ffmpeg::format::Pixel::BGR24 => Pixel::BGR24,
        _ => Pixel::YUV420P,
    };

    Ok((
        stream_index,
        VideoInfo {
            pixel_format,
            width: decoder.width(),
            height: decoder.height(),
            time_base: FFRational(time_base.numerator(), time_base.denominator()),
            frame_rate: FFRational(rate.numerator(), rate.denominator()),
        },
    ))
}

fn get_audio_stream_info(input: &avformat::context::Input) -> Option<(usize, AudioInfo)> {
    let stream = input.streams().best(ffmpeg::media::Type::Audio)?;
    let stream_index = stream.index();

    let decoder_ctx = avcodec::Context::from_parameters(stream.parameters()).ok()?;
    let decoder = decoder_ctx.decoder().audio().ok()?;

    let audio_info = AudioInfo::from_decoder(&decoder).ok()?;

    Some((stream_index, audio_info))
}

#[derive(Debug, Clone, Copy)]
struct PlaneDims {
    stride: usize,
    width: usize,
    height: usize,
}

fn transpose_plane(
    src: &[u8],
    src_dims: PlaneDims,
    dst: &mut [u8],
    dst_dims: PlaneDims,
    rotation: VideoRotation,
) {
    match rotation {
        VideoRotation::Cw90 => {
            for row in 0..dst_dims.height {
                for col in 0..dst_dims.width {
                    let src_r = src_dims.height - 1 - col;
                    let src_c = row;
                    dst[row * dst_dims.stride + col] = src[src_r * src_dims.stride + src_c];
                }
            }
        }
        VideoRotation::Ccw90 => {
            for row in 0..dst_dims.height {
                for col in 0..dst_dims.width {
                    let src_r = col;
                    let src_c = src_dims.width - 1 - row;
                    dst[row * dst_dims.stride + col] = src[src_r * src_dims.stride + src_c];
                }
            }
        }
        VideoRotation::Half => {
            for row in 0..dst_dims.height {
                for col in 0..dst_dims.width {
                    let src_r = src_dims.height - 1 - row;
                    let src_c = src_dims.width - 1 - col;
                    dst[row * dst_dims.stride + col] = src[src_r * src_dims.stride + src_c];
                }
            }
        }
        VideoRotation::None => {}
    }
}

fn prepare_rotated_yuv420p(
    src: &ffmpeg::frame::Video,
    rotation: VideoRotation,
    format_converter: &mut Option<ffmpeg::software::scaling::Context>,
) -> Result<Option<ffmpeg::frame::Video>, ImportError> {
    if matches!(rotation, VideoRotation::None) {
        return Ok(None);
    }

    if src.format() == ffmpeg::format::Pixel::YUV420P {
        return Ok(Some(transpose_yuv420p_frame(src, rotation)?));
    }

    if format_converter.is_none() {
        *format_converter = Some(
            ffmpeg::software::scaling::Context::get(
                src.format(),
                src.width(),
                src.height(),
                ffmpeg::format::Pixel::YUV420P,
                src.width(),
                src.height(),
                ffmpeg::software::scaling::Flags::BILINEAR,
            )
            .map_err(|e| {
                ImportError::TranscodeFailed(format!(
                    "Failed to create rotation format converter: {e}"
                ))
            })?,
        );
    }

    let converter = format_converter
        .as_mut()
        .ok_or_else(|| ImportError::TranscodeFailed("rotation converter missing".to_string()))?;

    let mut yuv = ffmpeg::frame::Video::empty();
    yuv.set_format(ffmpeg::format::Pixel::YUV420P);
    yuv.set_width(src.width());
    yuv.set_height(src.height());
    let ret = unsafe { ffmpeg::ffi::av_frame_get_buffer(yuv.as_mut_ptr(), 0) };
    if ret < 0 {
        return Err(ImportError::TranscodeFailed(format!(
            "av_frame_get_buffer failed for rotation YUV target (ret={ret})"
        )));
    }
    converter.run(src, &mut yuv)?;
    yuv.set_pts(src.pts());

    Ok(Some(transpose_yuv420p_frame(&yuv, rotation)?))
}

fn transpose_yuv420p_frame(
    src: &ffmpeg::frame::Video,
    rotation: VideoRotation,
) -> Result<ffmpeg::frame::Video, ImportError> {
    if src.format() != ffmpeg::format::Pixel::YUV420P {
        return Err(ImportError::TranscodeFailed(format!(
            "Cannot transpose non-YUV420P frame (got {:?})",
            src.format()
        )));
    }

    let src_w = src.width() as usize;
    let src_h = src.height() as usize;

    let (dst_w, dst_h) = match rotation {
        VideoRotation::Cw90 | VideoRotation::Ccw90 => (src_h, src_w),
        VideoRotation::Half => (src_w, src_h),
        VideoRotation::None => (src_w, src_h),
    };

    let mut dst = ffmpeg::frame::Video::empty();
    dst.set_format(ffmpeg::format::Pixel::YUV420P);
    dst.set_width(dst_w as u32);
    dst.set_height(dst_h as u32);
    let ret = unsafe { ffmpeg::ffi::av_frame_get_buffer(dst.as_mut_ptr(), 0) };
    if ret < 0 {
        return Err(ImportError::TranscodeFailed(format!(
            "av_frame_get_buffer failed for rotated frame (ret={ret})"
        )));
    }

    let src_y_stride = src.stride(0);
    let src_u_stride = src.stride(1);
    let src_v_stride = src.stride(2);

    let src_y = src.data(0).to_vec();
    let src_u = src.data(1).to_vec();
    let src_v = src.data(2).to_vec();

    let chroma_src_w = src_w / 2;
    let chroma_src_h = src_h / 2;
    let chroma_dst_w = dst_w / 2;
    let chroma_dst_h = dst_h / 2;

    {
        let dst_y_stride = dst.stride(0);
        let dst_y = dst.data_mut(0);
        transpose_plane(
            &src_y,
            PlaneDims {
                stride: src_y_stride,
                width: src_w,
                height: src_h,
            },
            dst_y,
            PlaneDims {
                stride: dst_y_stride,
                width: dst_w,
                height: dst_h,
            },
            rotation,
        );
    }
    {
        let dst_u_stride = dst.stride(1);
        let dst_u = dst.data_mut(1);
        transpose_plane(
            &src_u,
            PlaneDims {
                stride: src_u_stride,
                width: chroma_src_w,
                height: chroma_src_h,
            },
            dst_u,
            PlaneDims {
                stride: dst_u_stride,
                width: chroma_dst_w,
                height: chroma_dst_h,
            },
            rotation,
        );
    }
    {
        let dst_v_stride = dst.stride(2);
        let dst_v = dst.data_mut(2);
        transpose_plane(
            &src_v,
            PlaneDims {
                stride: src_v_stride,
                width: chroma_src_w,
                height: chroma_src_h,
            },
            dst_v,
            PlaneDims {
                stride: dst_v_stride,
                width: chroma_dst_w,
                height: chroma_dst_h,
            },
            rotation,
        );
    }

    dst.set_pts(src.pts());

    Ok(dst)
}

pub fn transcode_video(
    source_path: &Path,
    output_path: &Path,
    audio_output_path: Option<&Path>,
    mut on_progress: impl FnMut(f64),
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<(u32, Option<u32>), ImportError> {
    use std::time::Duration as StdDuration;

    let mut input =
        avformat::input(source_path).map_err(|e| ImportError::OpenFailed(e.to_string()))?;

    let (video_stream_index, video_info) = get_video_stream_info(&input)?;
    let audio_stream_info = get_audio_stream_info(&input);

    let rotation = read_video_rotation(&input);
    let needs_rotation = !matches!(rotation, VideoRotation::None);
    if needs_rotation {
        info!("Applying rotation to imported video: {:?}", rotation);
    }
    let (effective_source_width, effective_source_height) = match rotation {
        VideoRotation::Cw90 | VideoRotation::Ccw90 => (video_info.height, video_info.width),
        _ => (video_info.width, video_info.height),
    };

    let output_width = ensure_even(effective_source_width);
    let output_height = ensure_even(effective_source_height);
    let fps = if video_info.frame_rate.1 > 0 {
        ((video_info.frame_rate.0 as f64 / video_info.frame_rate.1 as f64).round() as u32)
            .clamp(1, 120)
    } else {
        30
    };

    let duration = get_media_duration(source_path);
    let total_frames = duration
        .map(|d| (d.as_secs_f64() * fps as f64) as u64)
        .unwrap_or(1000);

    let video_decoder_ctx =
        avcodec::Context::from_parameters(input.stream(video_stream_index).unwrap().parameters())
            .map_err(|e| ImportError::DecoderFailed(e.to_string()))?;
    let mut video_decoder = video_decoder_ctx
        .decoder()
        .video()
        .map_err(|e| ImportError::DecoderFailed(e.to_string()))?;

    let video_time_base = input.stream(video_stream_index).unwrap().time_base();

    let mut audio_decoder = audio_stream_info.as_ref().and_then(|(idx, _)| {
        let stream = input.stream(*idx)?;
        let decoder_ctx = avcodec::Context::from_parameters(stream.parameters()).ok()?;
        let mut decoder = decoder_ctx.decoder().audio().ok()?;
        if decoder.channel_layout().is_empty() {
            decoder.set_channel_layout(ChannelLayout::default(decoder.channels() as i32));
        }
        decoder.set_packet_time_base(stream.time_base());
        Some((*idx, decoder, stream.time_base()))
    });

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(ImportError::DirectoryFailed)?;
    }

    let mut output =
        avformat::output(output_path).map_err(|e| ImportError::EncoderFailed(e.to_string()))?;

    let encoder_video_info = VideoInfo {
        pixel_format: Pixel::YUV420P,
        width: output_width,
        height: output_height,
        time_base: video_info.time_base,
        frame_rate: FFRational(fps as i32, 1),
    };

    let mut video_encoder = H264EncoderBuilder::new(encoder_video_info)
        .with_preset(H264Preset::Medium)
        .with_output_size(output_width, output_height)
        .map_err(|e| ImportError::EncoderFailed(e.to_string()))?
        .build(&mut output)
        .map_err(|e| ImportError::EncoderFailed(e.to_string()))?;

    let mut audio_output: Option<avformat::context::Output> = None;
    let mut audio_encoder: Option<Box<dyn AudioEncoder + Send>> = None;
    let sample_rate = if let Some((_, audio_info)) = &audio_stream_info {
        if let Some(audio_path) = audio_output_path {
            let mut audio_out = avformat::output(audio_path).map_err(|e| {
                ImportError::EncoderFailed(format!("Failed to create audio output: {e}"))
            })?;

            audio_encoder = Some(Box::new(
                OpusEncoder::init(*audio_info, &mut audio_out)
                    .map_err(|e| ImportError::EncoderFailed(e.to_string()))?,
            ));
            audio_out.write_header().map_err(|e| {
                ImportError::EncoderFailed(format!("Failed to write audio header: {e}"))
            })?;
            audio_output = Some(audio_out);
        }
        Some(audio_info.sample_rate)
    } else {
        None
    };

    output
        .write_header()
        .map_err(|e| ImportError::EncoderFailed(format!("Failed to write header: {e}")))?;

    let mut video_frame = ffmpeg::frame::Video::empty();
    let mut audio_frame = ffmpeg::frame::Audio::empty();
    let mut frames_processed = 0u64;
    let mut last_progress = 0.0;

    let mut scaler: Option<ffmpeg::software::scaling::Context> = None;
    let mut rotation_format_converter: Option<ffmpeg::software::scaling::Context> = None;

    for (stream, packet) in input.packets() {
        let stream_index = stream.index();

        if stream_index == video_stream_index {
            video_decoder.send_packet(&packet)?;

            while video_decoder.receive_frame(&mut video_frame).is_ok() {
                let timestamp = video_frame.pts().unwrap_or(0);
                let time_secs = timestamp as f64 * video_time_base.numerator() as f64
                    / video_time_base.denominator().max(1) as f64;
                let duration = StdDuration::from_secs_f64(time_secs.max(0.0));

                let rotated_owned = prepare_rotated_yuv420p(
                    &video_frame,
                    rotation,
                    &mut rotation_format_converter,
                )?;
                let frame_for_scaling: &ffmpeg::frame::Video =
                    rotated_owned.as_ref().unwrap_or(&video_frame);

                let frame_to_encode = if frame_for_scaling.format()
                    != ffmpeg::format::Pixel::YUV420P
                    || frame_for_scaling.width() != output_width
                    || frame_for_scaling.height() != output_height
                {
                    if scaler.is_none() {
                        scaler = Some(
                            ffmpeg::software::scaling::Context::get(
                                frame_for_scaling.format(),
                                frame_for_scaling.width(),
                                frame_for_scaling.height(),
                                ffmpeg::format::Pixel::YUV420P,
                                output_width,
                                output_height,
                                ffmpeg::software::scaling::Flags::BILINEAR,
                            )
                            .map_err(|e| {
                                ImportError::TranscodeFailed(format!(
                                    "Failed to create scaler: {e}"
                                ))
                            })?,
                        );
                    }
                    let scaler = scaler.as_mut().unwrap();

                    let mut scaled_frame = ffmpeg::frame::Video::empty();
                    scaled_frame.set_format(ffmpeg::format::Pixel::YUV420P);
                    scaled_frame.set_width(output_width);
                    scaled_frame.set_height(output_height);
                    let ret =
                        unsafe { ffmpeg::ffi::av_frame_get_buffer(scaled_frame.as_mut_ptr(), 0) };
                    if ret < 0 {
                        return Err(ImportError::TranscodeFailed(
                            "Failed to allocate frame buffer".to_string(),
                        ));
                    }

                    scaler.run(frame_for_scaling, &mut scaled_frame)?;
                    scaled_frame.set_pts(video_frame.pts());
                    scaled_frame
                } else {
                    frame_for_scaling.clone()
                };

                video_encoder
                    .queue_frame(frame_to_encode, duration, &mut output)
                    .map_err(|e| ImportError::TranscodeFailed(e.to_string()))?;

                frames_processed += 1;

                let progress = (frames_processed as f64 / total_frames as f64).min(0.99);
                if progress - last_progress >= 0.01 {
                    last_progress = progress;

                    if is_cancelled() {
                        info!("Import cancelled: project directory was deleted");
                        return Err(ImportError::Cancelled);
                    }

                    on_progress(progress);
                }
            }
        } else if let Some((audio_idx, decoder, _)) = audio_decoder.as_mut()
            && stream_index == *audio_idx
            && let (Some(encoder), Some(audio_out)) =
                (audio_encoder.as_mut(), audio_output.as_mut())
        {
            decoder.send_packet(&packet)?;

            while decoder.receive_frame(&mut audio_frame).is_ok() {
                encoder.send_frame(audio_frame.clone(), audio_out);
            }
        }
    }

    video_decoder.send_eof()?;
    while video_decoder.receive_frame(&mut video_frame).is_ok() {
        let timestamp = video_frame.pts().unwrap_or(0);
        let time_secs = timestamp as f64 * video_time_base.numerator() as f64
            / video_time_base.denominator().max(1) as f64;
        let duration = StdDuration::from_secs_f64(time_secs.max(0.0));

        let rotated_owned =
            prepare_rotated_yuv420p(&video_frame, rotation, &mut rotation_format_converter)?;
        let frame_for_scaling: &ffmpeg::frame::Video =
            rotated_owned.as_ref().unwrap_or(&video_frame);

        let frame_to_encode = if frame_for_scaling.format() != ffmpeg::format::Pixel::YUV420P
            || frame_for_scaling.width() != output_width
            || frame_for_scaling.height() != output_height
        {
            if let Some(scaler) = &mut scaler {
                let mut scaled_frame = ffmpeg::frame::Video::empty();
                scaled_frame.set_format(ffmpeg::format::Pixel::YUV420P);
                scaled_frame.set_width(output_width);
                scaled_frame.set_height(output_height);
                let ret = unsafe { ffmpeg::ffi::av_frame_get_buffer(scaled_frame.as_mut_ptr(), 0) };
                if ret < 0 {
                    return Err(ImportError::TranscodeFailed(
                        "Failed to allocate frame buffer".to_string(),
                    ));
                }
                scaler.run(frame_for_scaling, &mut scaled_frame)?;
                scaled_frame.set_pts(video_frame.pts());
                scaled_frame
            } else {
                frame_for_scaling.clone()
            }
        } else {
            frame_for_scaling.clone()
        };

        video_encoder
            .queue_frame(frame_to_encode, duration, &mut output)
            .map_err(|e| ImportError::TranscodeFailed(e.to_string()))?;
    }

    if let Some((_, decoder, _)) = audio_decoder.as_mut() {
        decoder.send_eof()?;
        while decoder.receive_frame(&mut audio_frame).is_ok() {
            if let (Some(encoder), Some(audio_out)) =
                (audio_encoder.as_mut(), audio_output.as_mut())
            {
                encoder.send_frame(audio_frame.clone(), audio_out);
            }
        }
    }

    video_encoder
        .flush(&mut output)
        .map_err(|e| ImportError::TranscodeFailed(format!("Failed to flush video: {e}")))?;

    if let (Some(encoder), Some(audio_out)) = (&mut audio_encoder, &mut audio_output) {
        encoder
            .flush(audio_out)
            .map_err(|e| ImportError::TranscodeFailed(format!("Failed to flush audio: {e}")))?;
        audio_out.write_trailer().map_err(|e| {
            ImportError::TranscodeFailed(format!("Failed to write audio trailer: {e}"))
        })?;
    }

    output
        .write_trailer()
        .map_err(|e| ImportError::TranscodeFailed(format!("Failed to write trailer: {e}")))?;

    drop(output);

    if let Ok(file) = std::fs::File::open(output_path) {
        let _ = file.sync_all();
    }
    if let Some(audio_path) = audio_output_path
        && let Ok(file) = std::fs::File::open(audio_path)
    {
        let _ = file.sync_all();
    }

    Ok((fps, sample_rate))
}
