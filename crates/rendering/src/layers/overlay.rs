use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use wgpu::{include_wgsl, util::DeviceExt};

#[derive(Debug, Clone)]
pub struct PreparedOverlay {
    pub file_path: String,
    pub center: cap_project::XY<f64>,
    pub size: cap_project::XY<f64>,
    pub opacity: f32,
}

pub fn prepare_overlays(
    frame_time: f64,
    segments: &[cap_project::OverlaySegment],
) -> Vec<PreparedOverlay> {
    segments
        .iter()
        .filter_map(|seg| {
            if frame_time < seg.start || frame_time > seg.end {
                return None;
            }
            let fade = seg.fade_duration.max(0.0);
            let opacity = if fade > 0.0 {
                let since_start = (frame_time - seg.start).max(0.0);
                let until_end = (seg.end - frame_time).max(0.0);
                let fade_in = (since_start / fade).min(1.0);
                let fade_out = (until_end / fade).min(1.0);
                (fade_in * fade_out) as f32 * seg.opacity
            } else {
                seg.opacity
            };
            Some(PreparedOverlay {
                file_path: seg.file_path.clone(),
                center: seg.center,
                size: seg.size,
                opacity,
            })
        })
        .collect()
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct OverlayUniforms {
    rect: [f32; 4],
    opacity: f32,
    _padding: [f32; 3],
}

struct OverlayDraw {
    bind_group: wgpu::BindGroup,
}

pub struct OverlayLayer {
    pipeline: OverlayPipeline,
    textures: HashMap<String, wgpu::Texture>,
    draws: Vec<OverlayDraw>,
}

impl OverlayLayer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            pipeline: OverlayPipeline::new(device),
            textures: HashMap::new(),
            draws: Vec::new(),
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_size: (u32, u32),
        segments: &[cap_project::OverlaySegment],
        frame_time: f64,
    ) {
        self.draws.clear();

        let (out_w, out_h) = (output_size.0 as f32, output_size.1 as f32);
        if out_w <= 0.0 || out_h <= 0.0 {
            return;
        }

        for overlay in prepare_overlays(frame_time, segments) {
            if overlay.opacity <= 0.0 {
                continue;
            }
            if !self.ensure_texture(device, queue, &overlay.file_path) {
                continue;
            }

            let Some(texture) = self.textures.get(&overlay.file_path) else {
                continue;
            };
            let (tex_w, tex_h) = (texture.width() as f32, texture.height() as f32);
            if tex_w <= 0.0 || tex_h <= 0.0 {
                continue;
            }
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let box_w = overlay.size.x as f32 * out_w;
            let box_h = overlay.size.y as f32 * out_h;
            if box_w <= 0.0 || box_h <= 0.0 {
                continue;
            }

            let image_ar = tex_w / tex_h;
            let box_ar = box_w / box_h;
            let (draw_w, draw_h) = if image_ar > box_ar {
                (box_w, box_w / image_ar)
            } else {
                (box_h * image_ar, box_h)
            };

            let cx = overlay.center.x as f32 * out_w;
            let cy = overlay.center.y as f32 * out_h;
            let left = cx - draw_w / 2.0;
            let right = cx + draw_w / 2.0;
            let top = cy - draw_h / 2.0;
            let bottom = cy + draw_h / 2.0;

            let rect = [
                left / out_w * 2.0 - 1.0,
                1.0 - bottom / out_h * 2.0,
                right / out_w * 2.0 - 1.0,
                1.0 - top / out_h * 2.0,
            ];

            let uniforms = OverlayUniforms {
                rect,
                opacity: overlay.opacity.clamp(0.0, 1.0),
                _padding: [0.0; 3],
            };
            let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Overlay Uniform Buffer"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            let bind_group = self.pipeline.bind_group(device, &uniform_buffer, &view);
            self.draws.push(OverlayDraw { bind_group });
        }
    }

    fn ensure_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str) -> bool {
        if self.textures.contains_key(path) {
            return true;
        }

        let Ok(image) = image::open(path) else {
            return false;
        };
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Overlay Image Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.textures.insert(path.to_string(), texture);
        true
    }

    pub fn has_content(&self) -> bool {
        !self.draws.is_empty()
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.draws.is_empty() {
            return;
        }

        pass.set_pipeline(&self.pipeline.render_pipeline);
        for draw in &self.draws {
            pass.set_bind_group(0, &draw.bind_group, &[]);
            pass.draw(0..4, 0..1);
        }
    }
}

struct OverlayPipeline {
    bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
}

impl OverlayPipeline {
    fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("OverlayBindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(include_wgsl!("../shaders/overlay.wgsl"));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("OverlayPipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("OverlayPipelineLayout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            bind_group_layout,
            render_pipeline,
        }
    }

    fn bind_group(
        &self,
        device: &wgpu::Device,
        uniforms: &wgpu::Buffer,
        texture: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("OverlayBindGroup"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cap_project::{OverlaySegment, XY};

    fn seg() -> OverlaySegment {
        OverlaySegment {
            id: "o".into(),
            start: 1.0,
            end: 3.0,
            file_path: "/x.png".into(),
            center: XY::new(0.5, 0.5),
            size: XY::new(0.5, 0.3),
            opacity: 1.0,
            fade_duration: 0.5,
        }
    }

    #[test]
    fn inactive_before_start() {
        assert!(prepare_overlays(0.5, &[seg()]).is_empty());
    }

    #[test]
    fn full_opacity_mid_segment() {
        let p = prepare_overlays(2.0, &[seg()]);
        assert_eq!(p.len(), 1);
        assert!((p[0].opacity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fades_in_at_edge() {
        let p = prepare_overlays(1.25, &[seg()]);
        assert!(p[0].opacity > 0.0 && p[0].opacity < 1.0);
    }
}
