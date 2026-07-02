use std::collections::HashMap;
use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::frame_glyphs::FrameGlyphBuffer;
use neomacs_renderer_wgpu::{WgpuGlyphAtlas, WgpuRenderer};

fn create_wgpu_device() -> (Arc<wgpu::Device>, Arc<wgpu::Queue>) {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("benchmark requires a usable wgpu adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("Neomacs Glyph Vertex Bench Device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: Default::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        trace: wgpu::Trace::Off,
    }))
    .expect("benchmark requires a usable wgpu device");

    (Arc::new(device), Arc::new(queue))
}

fn target_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Neomacs Glyph Vertex Bench Target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn glyph_frame(cols: usize, rows: usize) -> (FrameGlyphBuffer, HashMap<u32, Face>) {
    let char_w = 8.0;
    let char_h = 16.0;
    let width = cols as f32 * char_w;
    let height = rows as f32 * char_h;
    let mut frame = FrameGlyphBuffer::with_size(width, height);
    frame.char_width = char_w;
    frame.char_height = char_h;
    frame.font_pixel_size = 14.0;
    frame.faces.insert(0, Face::new(0));

    for row in 0..rows {
        for col in 0..cols {
            let ch = (b'a' + (col % 26) as u8) as char;
            frame.add_char(
                ch,
                col as f32 * char_w,
                row as f32 * char_h,
                char_w,
                char_h,
                12.0,
                false,
            );
        }
    }

    let faces = frame.faces.clone();
    (frame, faces)
}

fn bench_glyph_vertex_build(c: &mut Criterion) {
    c.bench_function("glyph_vertex_build_10k_glyphs", |b| {
        let (device, queue) = create_wgpu_device();
        let width = 120 * 8;
        let height = 84 * 16;
        let mut renderer = WgpuRenderer::with_device(
            Arc::clone(&device),
            Arc::clone(&queue),
            width,
            height,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            1.0,
        );
        let mut atlas = WgpuGlyphAtlas::new(&device);
        let view = target_view(&device, width, height);
        let (frame, faces) = glyph_frame(120, 84);

        b.iter(|| {
            renderer.render_frame_glyphs(
                &view,
                black_box(&frame),
                &mut atlas,
                &faces,
                width,
                height,
                false,
                None,
                (0.0, 0.0),
                None,
            );
        });
    });
}

criterion_group!(benches, bench_glyph_vertex_build);
criterion_main!(benches);
