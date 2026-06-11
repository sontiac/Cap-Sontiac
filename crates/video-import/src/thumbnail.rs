use std::path::Path;

pub fn write_video_thumbnail(
    input: &Path,
    output: &Path,
    size: Option<(u32, u32)>,
) -> Result<(), String> {
    let mut ictx = ffmpeg::format::input(&input).map_err(|e| e.to_string())?;
    let input_stream = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or("No video stream found")?;
    let video_stream_index = input_stream.index();

    let mut decoder = ffmpeg::codec::context::Context::from_parameters(input_stream.parameters())
        .map_err(|e| e.to_string())?
        .decoder()
        .video()
        .map_err(|e| e.to_string())?;

    let mut scaler = ffmpeg::software::scaling::context::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGB24,
        size.map_or(decoder.width(), |s| s.0),
        size.map_or(decoder.height(), |s| s.1),
        ffmpeg::software::scaling::flag::Flags::BILINEAR,
    )
    .map_err(|e| e.to_string())?;

    let mut frame = ffmpeg::frame::Video::empty();
    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet).map_err(|e| e.to_string())?;
            if decoder.receive_frame(&mut frame).is_ok() {
                let mut rgb_frame = ffmpeg::frame::Video::empty();
                scaler
                    .run(&frame, &mut rgb_frame)
                    .map_err(|e| e.to_string())?;

                let width = rgb_frame.width() as usize;
                let height = rgb_frame.height() as usize;
                let bytes_per_pixel = 3;
                let src_stride = rgb_frame.stride(0);
                let dst_stride = width * bytes_per_pixel;

                let mut img_buffer = vec![0u8; height * dst_stride];

                for y in 0..height {
                    let src_slice = &rgb_frame.data(0)[y * src_stride..y * src_stride + dst_stride];
                    let dst_slice = &mut img_buffer[y * dst_stride..(y + 1) * dst_stride];
                    dst_slice.copy_from_slice(src_slice);
                }

                let img = image::RgbImage::from_raw(width as u32, height as u32, img_buffer)
                    .ok_or("Failed to create image from frame data")?;

                img.save_with_format(&output, image::ImageFormat::Jpeg)
                    .map_err(|e| e.to_string())?;

                return Ok(());
            }
        }
    }

    Err("Failed to create screenshot".to_string())
}
