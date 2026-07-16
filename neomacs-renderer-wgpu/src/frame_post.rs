//! Full-frame post pass: a user WGSL shader over the composited frame
//! (`doc/display-engine/SHADER_SURFACES.md`). The Ghostty / Windows Terminal
//! `custom-shader` feature: CRT, bloom, color grading over the whole editor.
//!
//! The pass receives the rendered scene as `iChannel0` and draws into the
//! swapchain view with the same composed-WGSL contract as shader surfaces
//! (`neo_vs_main`/`neo_fs_main`, `NeoUniforms` at group(0) binding 0, channel
//! texture+sampler at bindings 1-2). The host validates and composes the
//! source on the Lisp thread; this module only compiles and runs it.
//!
//! v1 scope: fragCoord keeps the surface contract (y-up), while the captured
//! frame texture is top-left origin — sampling the pixel under fragCoord is
//! `vec2(fragCoord.x, u.iResolution.y - fragCoord.y) / u.iResolution.xy`.
//! Overlays drawn after the blit stage (transitions, menus, cursor) are not
//! post-processed. No custom uniforms yet.

use std::time::Instant;

use crate::shader_surface::SURFACE_UNIFORM_BYTES;

pub struct FramePost {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    start: Instant,
    last: Option<Instant>,
    frame_index: u32,
}

impl FramePost {
    /// Compile a composed shader-surface WGSL module into a post pipeline
    /// targeting `format`. Wrapped in a validation error scope: the host
    /// already naga-validated the same source, so a failure here is
    /// device-specific and reported, not fatal.
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        composed_source: &str,
    ) -> Result<Self, String> {
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Frame Post Uniforms"),
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

        let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Frame Post Module"),
            source: wgpu::ShaderSource::Wgsl(composed_source.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Frame Post Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Frame Post Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("neo_vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("neo_fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(format!("frame shader: pipeline rejected: {error}"));
        }

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frame Post Uniform Buffer"),
            size: SURFACE_UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Frame Post Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Ok(Self {
            pipeline,
            uniform_buffer,
            bind_group_layout,
            sampler,
            start: Instant::now(),
            last: None,
            frame_index: 0,
        })
    }

    /// Run the post pass: sample `src_view` (the rendered frame), write the
    /// shaded result into `dst_view`. `width_px`/`height_px` are physical
    /// pixels; `mouse` is the pointer in physical px, y-up (zeros when
    /// absent).
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        src_view: &wgpu::TextureView,
        dst_view: &wgpu::TextureView,
        width_px: u32,
        height_px: u32,
        scale: f32,
        mouse: (f32, f32),
    ) {
        let now = Instant::now();
        let dt = self
            .last
            .map(|t| now.duration_since(t).as_secs_f32().clamp(0.0, 0.1))
            .unwrap_or(0.0);
        self.last = Some(now);
        self.frame_index = self.frame_index.wrapping_add(1);

        let mut uniforms = [0.0f32; (SURFACE_UNIFORM_BYTES / 4) as usize];
        uniforms[0] = width_px as f32;
        uniforms[1] = height_px as f32;
        uniforms[2] = scale;
        uniforms[4] = mouse.0;
        uniforms[5] = mouse.1;
        uniforms[8] = now.duration_since(self.start).as_secs_f32();
        uniforms[9] = dt;
        uniforms[10] = self.frame_index as f32;
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Frame Post Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Frame Post Pass"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Frame Post"),
                multiview_mask: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dst_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit(std::iter::once(encoder.finish()));
    }
}
