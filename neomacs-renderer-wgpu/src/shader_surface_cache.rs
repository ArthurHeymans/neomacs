//! GPU cache for shader surfaces (`doc/display-engine/SHADER_SURFACES.md`).
//!
//! Mirrors `VideoCache`'s shape — a `HashMap<u32, CachedShaderSurface>` whose
//! entries own a texture plus the bind group the inline-media composite phase
//! samples — but where video *uploads* frames, this cache *renders* them: each
//! animated (or dirtied) surface gets one offscreen fullscreen-triangle pass
//! per frame with the user's compiled WGSL pipeline.
//!
//! Battery policy: a surface only re-renders while it was actually composited
//! recently (`mark_drawn` from the draw phase stamps `active_until`). Scrolled
//! offscreen, its demand lapses and `iTime` freezes; scrolling it back into
//! view resumes the clock. This is deliberately stricter than video's
//! process-wide demand.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::shader_surface::{
    SURFACE_UNIFORM_BYTES, SURFACE_USER_UNIFORM_SLOTS, SurfaceUniformInit, compose_surface_wgsl,
    uniform_accessor_name,
};

/// Largest allowed surface edge in physical pixels (matches
/// `ImageCache::MAX_TEXTURE_SIZE`).
pub const MAX_SURFACE_SIZE: u32 = 4096;

/// How long after its last composite a surface still counts as visible for
/// animation demand.
const ACTIVE_GRACE: Duration = Duration::from_millis(500);

pub struct CachedShaderSurface {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    /// Bind group for the inline-media composite phase (image pipeline
    /// layout: texture + sampler).
    pub composite_bind_group: wgpu::BindGroup,
    /// User render pipeline; `None` for pixel-upload surfaces.
    pipeline: Option<wgpu::RenderPipeline>,
    uniform_buffer: Option<wgpu::Buffer>,
    uniform_bind_group: Option<wgpu::BindGroup>,
    /// name -> (slot, components) for `set_uniform` by Lisp name.
    uniform_slots: HashMap<String, (usize, u8)>,
    custom: [[f32; 4]; SURFACE_USER_UNIFORM_SLOTS],
    elapsed: f32,
    frame_index: u32,
    animate: bool,
    /// Needs one render even if not animating (created / uniform changed).
    dirty: bool,
    /// Last time the composite phase drew this surface (plus grace).
    active_until: Option<Instant>,
    width_px: u32,
    height_px: u32,
    scale: f32,
}

pub struct ShaderSurfaceCache {
    surfaces: HashMap<u32, CachedShaderSurface>,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    last_tick: Option<Instant>,
}

impl ShaderSurfaceCache {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shader Surface Uniforms"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        Self {
            surfaces: HashMap::new(),
            uniform_bind_group_layout,
            last_tick: None,
        }
    }

    pub fn get(&self, id: u32) -> Option<&CachedShaderSurface> {
        self.surfaces.get(&id)
    }

    fn clamp_size(width: u32, height: u32, scale: f32) -> (u32, u32) {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let px = |v: u32| ((v as f32 * scale).round() as u32).clamp(1, MAX_SURFACE_SIZE);
        (px(width), px(height))
    }

    fn make_texture(
        device: &wgpu::Device,
        width_px: u32,
        height_px: u32,
        format: wgpu::TextureFormat,
        render_target: bool,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let mut usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        if render_target {
            usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shader Surface"),
            size: wgpu::Extent3d {
                width: width_px,
                height: height_px,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn composite_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shader Surface Composite"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// Create a surface driven by a user WGSL shader. The source is composed
    /// with the generated prelude and compiled inside a validation error
    /// scope; the Lisp thread already naga-validated the same composition, so
    /// a failure here (device-specific rejection) is reported, not fatal.
    #[allow(clippy::too_many_arguments)]
    pub fn create_shader(
        &mut self,
        device: &wgpu::Device,
        composite_layout: &wgpu::BindGroupLayout,
        composite_sampler: &wgpu::Sampler,
        target_format: wgpu::TextureFormat,
        id: u32,
        user_source: &str,
        uniforms: &[SurfaceUniformInit],
        width: u32,
        height: u32,
        scale: f32,
        animate: bool,
    ) -> Result<(), String> {
        let (width_px, height_px) = Self::clamp_size(width, height, scale);
        let names: Vec<(String, u8)> = uniforms
            .iter()
            .map(|u| (u.name.clone(), u.components))
            .collect();
        let source = compose_surface_wgsl(user_source, &names);

        let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader Surface Module"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shader Surface Pipeline Layout"),
            bind_group_layouts: &[Some(&self.uniform_bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shader Surface Pipeline"),
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
                    format: target_format,
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
            return Err(format!("shader surface {id}: pipeline rejected: {error}"));
        }

        let (texture, view) = Self::make_texture(device, width_px, height_px, target_format, true);
        let composite_bind_group =
            Self::composite_bind_group(device, composite_layout, composite_sampler, &view);

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shader Surface Uniform Buffer"),
            size: SURFACE_UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shader Surface Uniform Bind Group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let mut uniform_slots = HashMap::new();
        let mut custom = [[0.0f32; 4]; SURFACE_USER_UNIFORM_SLOTS];
        for (slot, init) in uniforms.iter().enumerate().take(SURFACE_USER_UNIFORM_SLOTS) {
            uniform_slots.insert(init.name.clone(), (slot, init.components));
            custom[slot] = init.value;
        }

        self.surfaces.insert(
            id,
            CachedShaderSurface {
                texture,
                view,
                composite_bind_group,
                pipeline: Some(pipeline),
                uniform_buffer: Some(uniform_buffer),
                uniform_bind_group: Some(uniform_bind_group),
                uniform_slots,
                custom,
                elapsed: 0.0,
                frame_index: 0,
                animate,
                dirty: true,
                active_until: None,
                width_px,
                height_px,
                scale: if scale.is_finite() && scale > 0.0 {
                    scale
                } else {
                    1.0
                },
            },
        );
        tracing::info!(
            "shader surface {id} created: {width_px}x{height_px}px animate={animate}"
        );
        Ok(())
    }

    /// Create a static surface from raw RGBA8 pixels (stage 1: GPU texture
    /// from Lisp data, no shader).
    #[allow(clippy::too_many_arguments)]
    pub fn create_pixels(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        composite_layout: &wgpu::BindGroupLayout,
        composite_sampler: &wgpu::Sampler,
        id: u32,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let width = width.clamp(1, MAX_SURFACE_SIZE);
        let height = height.clamp(1, MAX_SURFACE_SIZE);
        let expected = width as usize * height as usize * 4;
        if data.len() < expected {
            return Err(format!(
                "surface {id}: pixel data too short: {} bytes, need {expected}",
                data.len()
            ));
        }
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let (texture, view) = Self::make_texture(device, width, height, format, false);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data[..expected],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let composite_bind_group =
            Self::composite_bind_group(device, composite_layout, composite_sampler, &view);
        self.surfaces.insert(
            id,
            CachedShaderSurface {
                texture,
                view,
                composite_bind_group,
                pipeline: None,
                uniform_buffer: None,
                uniform_bind_group: None,
                uniform_slots: HashMap::new(),
                custom: [[0.0; 4]; SURFACE_USER_UNIFORM_SLOTS],
                elapsed: 0.0,
                frame_index: 0,
                animate: false,
                dirty: false,
                active_until: None,
                width_px: width,
                height_px: height,
                scale: 1.0,
            },
        );
        tracing::info!("pixel surface {id} created: {width}x{height}px");
        Ok(())
    }

    /// Update one named uniform; unknown names are ignored with a warning
    /// (the accessor set is fixed at create time).
    pub fn set_uniform(&mut self, id: u32, name: &str, value: [f32; 4]) {
        let Some(surface) = self.surfaces.get_mut(&id) else {
            tracing::warn!("set_uniform: no shader surface {id}");
            return;
        };
        match surface.uniform_slots.get(name) {
            Some((slot, _)) => {
                surface.custom[*slot] = value;
                surface.dirty = true;
            }
            None => tracing::warn!(
                "set_uniform: surface {id} has no uniform {name:?} (accessor {})",
                uniform_accessor_name(name)
            ),
        }
    }

    pub fn free(&mut self, id: u32) {
        if self.surfaces.remove(&id).is_some() {
            tracing::info!("shader surface {id} freed");
        }
    }

    /// Stamp a surface as composited this frame; animation demand and the
    /// iTime clock stay live for `ACTIVE_GRACE` past the last composite.
    pub fn mark_drawn(&mut self, id: u32) {
        if let Some(surface) = self.surfaces.get_mut(&id) {
            surface.active_until = Some(Instant::now() + ACTIVE_GRACE);
        }
    }

    /// Whether any animated surface was composited recently — the
    /// `DemandReason::ShaderSurface` signal.
    pub fn has_active_surfaces(&self) -> bool {
        let now = Instant::now();
        self.surfaces
            .values()
            .any(|s| s.pipeline.is_some() && (s.dirty || (s.animate && s.is_active(now))))
    }

    /// Render every surface that needs a new frame (dirty, or animated and
    /// recently composited). One encoder for all passes, submitted before the
    /// main frame pass samples the textures. Returns how many passes ran.
    pub fn render_pending(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> usize {
        let now = Instant::now();
        let dt = self
            .last_tick
            .map(|t| now.duration_since(t).as_secs_f32().clamp(0.0, 0.1))
            .unwrap_or(0.0);
        self.last_tick = Some(now);

        let mut rendered = 0usize;
        let mut encoder: Option<wgpu::CommandEncoder> = None;
        for (id, surface) in &mut self.surfaces {
            let (Some(pipeline), Some(buffer), Some(bind_group)) = (
                surface.pipeline.as_ref(),
                surface.uniform_buffer.as_ref(),
                surface.uniform_bind_group.as_ref(),
            ) else {
                continue;
            };
            let animating = surface.animate && surface.is_active(now);
            if !surface.dirty && !animating {
                continue;
            }
            if animating {
                surface.elapsed += dt;
            }
            surface.frame_index = surface.frame_index.wrapping_add(1);
            surface.dirty = false;

            let mut uniforms = [0.0f32; (SURFACE_UNIFORM_BYTES / 4) as usize];
            uniforms[0] = surface.width_px as f32;
            uniforms[1] = surface.height_px as f32;
            uniforms[2] = surface.scale;
            // uniforms[3] reserved; [4..8] = iMouse (zeros, stage 3).
            uniforms[8] = surface.elapsed;
            uniforms[9] = dt;
            uniforms[10] = surface.frame_index as f32;
            for (slot, value) in surface.custom.iter().enumerate() {
                uniforms[12 + slot * 4..12 + slot * 4 + 4].copy_from_slice(value);
            }
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&uniforms));

            let encoder = encoder.get_or_insert_with(|| {
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Shader Surface Passes"),
                })
            });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Shader Surface Pass"),
                    multiview_mask: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &surface.view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
            rendered += 1;
            tracing::trace!("shader surface {id} rendered (t={:.3})", surface.elapsed);
        }
        if let Some(encoder) = encoder {
            queue.submit(std::iter::once(encoder.finish()));
        }
        rendered
    }
}

impl CachedShaderSurface {
    fn is_active(&self, now: Instant) -> bool {
        self.active_until.is_some_and(|until| now < until)
    }
}
