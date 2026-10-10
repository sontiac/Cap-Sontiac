use ffmpeg::{color::TransferCharacteristic, filter, frame::Video};
use tracing::{info, warn};

use crate::ImportError;

const TONEMAP_FILTERS: &str = "zscale=t=linear:npl=203,format=gbrpf32le,zscale=p=bt709,\
     tonemap=tonemap=hable:desat=0,zscale=t=bt709:m=bt709:r=tv,format=yuv420p";
const REQUIRED_FILTERS: [&str; 2] = ["zscale", "tonemap"];

pub(crate) fn is_hdr_transfer(transfer: TransferCharacteristic) -> bool {
    matches!(
        transfer,
        TransferCharacteristic::ARIB_STD_B67 | TransferCharacteristic::SMPTE2084
    )
}

pub(crate) struct HdrToSdr {
    graph: filter::Graph,
}

impl HdrToSdr {
    fn new(first_frame: &Video) -> Result<Self, ImportError> {
        let pixel_format: ffmpeg::ffi::AVPixelFormat = first_frame.format().into();
        let color_space: ffmpeg::ffi::AVColorSpace = first_frame.color_space().into();
        let color_range: ffmpeg::ffi::AVColorRange = first_frame.color_range().into();
        let source_args = format!(
            "video_size={}x{}:pix_fmt={}:time_base=1/1:pixel_aspect=1/1:colorspace={}:range={}",
            first_frame.width(),
            first_frame.height(),
            pixel_format as i32,
            color_space as i32,
            color_range as i32,
        );

        let filter_error =
            |e: ffmpeg::Error| ImportError::TranscodeFailed(format!("HDR tone-map filter: {e}"));
        let mut graph = filter::Graph::new();
        let buffer = filter::find("buffer").ok_or_else(|| {
            ImportError::TranscodeFailed("ffmpeg build has no buffer filter".to_string())
        })?;
        let buffersink = filter::find("buffersink").ok_or_else(|| {
            ImportError::TranscodeFailed("ffmpeg build has no buffersink filter".to_string())
        })?;
        graph
            .add(&buffer, "in", &source_args)
            .map_err(filter_error)?;
        graph.add(&buffersink, "out", "").map_err(filter_error)?;
        graph
            .output("in", 0)
            .and_then(|parser| parser.input("out", 0))
            .and_then(|parser| parser.parse(TONEMAP_FILTERS))
            .map_err(filter_error)?;
        graph.validate().map_err(filter_error)?;

        Ok(Self { graph })
    }

    fn convert(&mut self, frame: &Video) -> Result<Video, ImportError> {
        let filter_error =
            |e: ffmpeg::Error| ImportError::TranscodeFailed(format!("HDR tone-map filter: {e}"));
        self.graph
            .get("in")
            .ok_or_else(|| ImportError::TranscodeFailed("HDR filter source missing".to_string()))?
            .source()
            .add(frame)
            .map_err(filter_error)?;

        let mut converted = Video::empty();
        self.graph
            .get("out")
            .ok_or_else(|| ImportError::TranscodeFailed("HDR filter sink missing".to_string()))?
            .sink()
            .frame(&mut converted)
            .map_err(filter_error)?;
        converted.set_pts(frame.pts());
        Ok(converted)
    }
}

pub(crate) enum HdrConversion {
    Pending,
    Active(HdrToSdr),
    Unavailable,
}

impl HdrConversion {
    pub(crate) fn convert_to_sdr(&mut self, frame: &Video) -> Result<Option<Video>, ImportError> {
        let transfer = frame.color_transfer_characteristic();
        if !is_hdr_transfer(transfer) {
            return Ok(None);
        }

        if matches!(self, Self::Pending) {
            let missing: Vec<&str> = REQUIRED_FILTERS
                .into_iter()
                .filter(|name| filter::find(name).is_none())
                .collect();
            *self = if missing.is_empty() {
                info!("Tone-mapping {transfer:?} HDR video to SDR BT.709");
                Self::Active(HdrToSdr::new(frame)?)
            } else {
                warn!(
                    "HDR ({transfer:?}) source imported without tone mapping: this ffmpeg build lacks {}; colours will look washed out",
                    missing.join(", ")
                );
                Self::Unavailable
            };
        }

        match self {
            Self::Active(converter) => converter.convert(frame).map(Some),
            Self::Pending | Self::Unavailable => Ok(None),
        }
    }
}
