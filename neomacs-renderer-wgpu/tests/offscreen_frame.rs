//! Offscreen frame render harness for the frame scheduling plan's retained
//! geometry / shader cursor stages.
//!
//! Renders a real `FrameGlyphBuffer` through `WgpuRenderer::render_frame_glyphs`
//! into an offscreen texture and reads pixels back, with no window-system
//! surface. Used to assert the plan's core retained-scene invariant: a
//! cursor-only frame changes only cursor pixels, leaving the static scene
//! bit-identical.
//!
//! Skips (passes) cleanly where no GPU adapter is available.

use neomacs_display_protocol::frame_chrome::PresentationId;
use neomacs_display_protocol::frame_glyphs::{
    CursorStyle, DisplaySlotId, FrameGlyphBuffer, GlyphRowRole, PhysCursor,
};
use neomacs_display_protocol::types::{Color, DisplayWindowId, FaceId};
use neomacs_display_protocol::{
    BoxType, FaceAttributes, FrameRect, PointerAppearanceId, PointerAppearancePhase,
    PointerAppearanceSelection, PointerDrawMode, PointerImageRelief, PointerReliefCornerErase,
    PointerReliefEdges, PointerReliefMargins, PresentedPaintSpan, PresentedPointerAppearance,
    PresentedPointerRegion, PresentedPrimitiveKind,
};
use neomacs_renderer_wgpu::types::SubpixelRequest;
use neomacs_renderer_wgpu::{WgpuGlyphAtlas, WgpuRenderer};

const W: u32 = 96;
const H: u32 = 64;

struct Harness {
    renderer: WgpuRenderer,
    atlas: WgpuGlyphAtlas,
    target: wgpu::Texture,
    view: wgpu::TextureView,
}

fn try_harness() -> Option<Harness> {
    let renderer = WgpuRenderer::new(None, W, H).ok()?;
    let atlas = WgpuGlyphAtlas::new(renderer.device());
    let target = renderer.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen-frame-target"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Match the renderer's surfaceless pipeline target format so the
        // render pipelines are compatible with this pass. Bytes read back are
        // therefore BGRA, sRGB-encoded (see px()).
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    Some(Harness {
        renderer,
        atlas,
        target,
        view,
    })
}

fn frame_with_cursor(cursor_color: Color) -> FrameGlyphBuffer {
    let mut frame = FrameGlyphBuffer::with_size(W as f32, H as f32);
    frame.background = Color::rgb(0.10, 0.12, 0.16);
    // A bar cursor near the left edge: a clean top-layer cursor (no inverse
    // video), so its pixels are localized and its color is the only variable.
    frame.set_phys_cursor(PhysCursor {
        window_id: DisplayWindowId::new(1),
        charpos: 0,
        row: 0,
        col: 0,
        slot_id: DisplaySlotId {
            window_id: DisplayWindowId::new(1),
            row: 0,
            col: 0,
        },
        x: 8.0,
        y: 8.0,
        width: 4.0,
        height: 24.0,
        ascent: 20.0,
        style: CursorStyle::Bar(4.0),
        color: cursor_color,
        cursor_fg: Color::BLACK,
    });
    frame
}

fn read_back(h: &Harness) -> Vec<u8> {
    // bytes_per_row must be 256-aligned.
    let unpadded = W * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;
    let buf = h.renderer.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = h
        .renderer
        .device()
        .create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &h.target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    h.renderer.queue().submit(std::iter::once(enc.finish()));
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    h.renderer
        .device()
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(3)),
        })
        .expect("poll");
    let data = slice.get_mapped_range();
    // Un-pad into tight W*H*4.
    let mut out = vec![0u8; (unpadded * H) as usize];
    for row in 0..H {
        let src = (row * padded) as usize;
        let dst = (row * unpadded) as usize;
        out[dst..dst + unpadded as usize].copy_from_slice(&data[src..src + unpadded as usize]);
    }
    out
}

/// One pixel as (r, g, b, a). The target is BGRA, so swizzle on read.
fn px(buf: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * W + x) * 4) as usize;
    [buf[i + 2], buf[i + 1], buf[i], buf[i + 3]]
}

#[test]
fn offscreen_frame_renders_background_and_cursor() {
    let Some(mut h) = try_harness() else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };
    let frame = frame_with_cursor(Color::rgb(1.0, 0.0, 0.0));
    h.renderer.render_frame_glyphs(
        &h.view,
        &frame,
        &mut h.atlas,
        W,
        H,
        true,
        None,
        (0.0, 0.0),
        None,
        None,
        None,
    );
    let buf = read_back(&h);
    // A corner pixel is background (dark blue-ish), definitely not the red cursor.
    let corner = px(&buf, W - 2, H - 2);
    assert!(
        corner[2] > corner[0],
        "corner should be background (blue>red), got {corner:?}"
    );
    // The red bar cursor occupies its slot (x≈8..12, y≈8..31): red-dominant
    // pixels clearly distinct from the background.
    let mut found_red = false;
    for y in 8..31 {
        for x in 8..12 {
            let p = px(&buf, x, y);
            if p[0] > 180 && p[0] > p[1] + 60 && p[0] > p[2] + 60 {
                found_red = true;
            }
        }
    }
    assert!(found_red, "expected the red bar cursor to be drawn");
}

#[test]
fn ime_preedit_cjk_background_covers_the_shaped_run() {
    let Some(mut h) = try_harness() else {
        return;
    };
    let mut frame = FrameGlyphBuffer::with_size(W as f32, H as f32);
    frame.background = Color::rgb(0.10, 0.12, 0.16);
    h.renderer.render_frame_glyphs(
        &h.view,
        &frame,
        &mut h.atlas,
        W,
        H,
        false,
        None,
        (0.0, 0.0),
        None,
        None,
        None,
    );
    let base = read_back(&h);

    let preedit = "你好";
    let shaped = h
        .atlas
        .get_or_create_composed_atlas(
            h.renderer.device(),
            h.renderer.queue(),
            preedit,
            FaceId::new(0),
            0.0_f32.to_bits(),
            None,
            cosmic_text::SubpixelBin::Zero,
            cosmic_text::SubpixelBin::Zero,
            SubpixelRequest::Disabled,
        )
        .expect("the GUI test environment must provide a CJK fallback font");
    let shaped_width = shaped.advance_width;
    let fixed_cell_width = preedit.chars().count() as f32 * h.atlas.default_char_width();
    assert!(
        shaped_width > fixed_cell_width + 2.0,
        "the regression needs a wide CJK run: shaped={shaped_width}, fixed={fixed_cell_width}"
    );

    let cursor_x = 8.0;
    let cursor_y = 8.0;
    h.renderer.render_ime_preedit(
        &h.view,
        preedit,
        cursor_x,
        cursor_y,
        24.0,
        &mut h.atlas,
        W,
        H,
    );
    let rendered = read_back(&h);

    // Sample inside the final shaped advance.  The preedit background must
    // cover the complete run; using `chars().count() * default_char_width`
    // leaves this pixel untouched and also places adjacent CJK glyphs on top
    // of one another.
    let tail_x = (cursor_x + 2.0 + shaped_width - 1.0).floor() as u32;
    let sample_y = (cursor_y + 2.0) as u32;
    assert_ne!(
        px(&rendered, tail_x, sample_y),
        px(&base, tail_x, sample_y),
        "the preedit visual must cover its shaped run through x={tail_x}"
    );
}

#[test]
fn negative_box_line_width_paints_an_inset_border() {
    let Some(mut h) = try_harness() else {
        return;
    };
    let mut frame = FrameGlyphBuffer::with_size(W as f32, H as f32);
    frame.background = Color::BLACK;
    let face_id = FaceId::new(20);
    frame.set_face(
        face_id,
        Color::WHITE,
        Some(Color::BLACK),
        700,
        false,
        0,
        None,
        0,
        None,
        0,
        None,
    );
    let face = frame.faces.get_mut(&face_id).unwrap();
    face.attributes |= FaceAttributes::BOX;
    face.box_type = BoxType::Line;
    face.box_color = Some(Color::GREEN);
    face.box_line_width = (-2).into();
    frame.set_draw_context(DisplayWindowId::new(1), GlyphRowRole::TabBar, None);
    frame.add_stretch(20.0, 20.0, 40.0, 20.0, Color::BLACK, face_id, false);

    h.renderer.render_frame_glyphs(
        &h.view,
        &frame,
        &mut h.atlas,
        W,
        H,
        false,
        None,
        (0.0, 0.0),
        None,
        None,
        None,
    );
    let buf = read_back(&h);
    let top = px(&buf, 40, 20);
    let interior = px(&buf, 40, 25);
    assert!(
        top[1] > 180 && top[1] > top[0] + 80 && top[1] > top[2] + 80,
        "negative line-width must still paint its top box edge, got {top:?}"
    );
    assert!(
        interior[0] < 30 && interior[1] < 30 && interior[2] < 30,
        "inset border must not fill the box interior, got {interior:?}"
    );
}

fn boxed_one_cell_frame() -> FrameGlyphBuffer {
    let mut frame = FrameGlyphBuffer::with_size(W as f32, H as f32);
    frame.background = Color::BLACK;
    let face_id = FaceId::new(21);
    frame.set_face(
        face_id,
        Color::WHITE,
        Some(Color::rgb(0.2, 0.2, 0.2)),
        400,
        false,
        0,
        None,
        0,
        None,
        0,
        None,
    );
    let face = frame.faces.get_mut(&face_id).unwrap();
    face.attributes |= FaceAttributes::BOX;
    face.box_type = BoxType::Line;
    face.box_color = Some(Color::BLACK);
    face.box_line_width = (-4).into();
    frame.set_draw_context(DisplayWindowId::new(1), GlyphRowRole::Text, None);
    frame.add_char('p', 20.0, 20.0, 8.0, 18.0, 14.0, false);
    frame
}

fn boxed_p_is_visible(buf: &[u8]) -> bool {
    (20..28).any(|x| {
        (20..38).any(|y| {
            let pixel = px(buf, x, y);
            pixel[0] > 120 && pixel[1] > 120 && pixel[2] > 120
        })
    })
}

#[test]
fn negative_box_border_does_not_cover_a_one_cell_glyph() {
    let Some(mut h) = try_harness() else {
        return;
    };
    let frame = boxed_one_cell_frame();
    h.renderer.render_frame_glyphs(
        &h.view,
        &frame,
        &mut h.atlas,
        W,
        H,
        false,
        None,
        (0.0, 0.0),
        None,
        None,
        None,
    );
    let buf = read_back(&h);
    assert!(
        boxed_p_is_visible(&buf),
        "GNU draws character glyphs over their box; the boxed `p` must remain visible"
    );
}

#[test]
fn child_frame_negative_box_border_does_not_cover_a_one_cell_glyph() {
    let Some(mut h) = try_harness() else {
        return;
    };
    let frame = boxed_one_cell_frame();
    h.renderer.render_frame_content(
        &h.view,
        &frame,
        &mut h.atlas,
        W,
        H,
        0.0,
        0.0,
        false,
        None,
        0.0,
        None,
        None,
    );
    let buf = read_back(&h);
    assert!(
        boxed_p_is_visible(&buf),
        "child-frame glyphs must be drawn over their boxes like GNU Emacs"
    );
}

#[test]
fn cursor_visible_false_suppresses_cursor() {
    let Some(mut h) = try_harness() else {
        return;
    };
    let frame = frame_with_cursor(Color::rgb(1.0, 0.0, 0.0));
    h.renderer.render_frame_glyphs(
        &h.view,
        &frame,
        &mut h.atlas,
        W,
        H,
        false,
        None,
        (0.0, 0.0),
        None,
        None,
        None,
    );
    let buf = read_back(&h);
    let mut red = 0;
    for y in 0..H {
        for x in 0..W {
            let p = px(&buf, x, y);
            if p[0] > 180 && p[0] > p[1] + 60 && p[0] > p[2] + 60 {
                red += 1;
            }
        }
    }
    eprintln!("cursor_visible=false red pixels: {}", red);
    assert_eq!(red, 0, "cursor_visible=false must draw no cursor");
}

fn presented_pointer_integration_relief(pressed: bool, background: Color) -> PointerImageRelief {
    let light = Color::rgb(0.95, 0.95, 0.95);
    let dark = Color::rgb(0.08, 0.08, 0.08);
    let (top_left, bottom_right) = if pressed {
        (dark, light)
    } else {
        (light, dark)
    };
    PointerImageRelief::new(
        top_left,
        bottom_right,
        2.0,
        PointerReliefMargins::new(0.0, 0.0, 0.0, 0.0),
        PointerReliefEdges::new(true, true, true, true),
        PointerReliefCornerErase::new(background, 4.0, 1.0),
    )
}

fn presented_pointer_integration_image_frame() -> FrameGlyphBuffer {
    let background = Color::rgb(0.05, 0.06, 0.07);
    let mut frame = FrameGlyphBuffer::with_size(W as f32, H as f32);
    frame.presentation_id = PresentationId::new(502);
    frame.background = background;
    frame.add_image(
        neomacs_display_protocol::ImageId::new(77),
        24.0,
        20.0,
        24.0,
        24.0,
    );
    frame
        .install_presented_pointer(
            vec![PresentedPointerRegion::new(
                FrameRect::new(20.0, 16.0, 32.0, 32.0).unwrap(),
                Some(neomacs_display_protocol::InteractionId::new(3)),
                Some(PointerAppearanceId::try_from(0usize).unwrap()),
            )],
            vec![PresentedPointerAppearance::new(
                vec![PresentedPaintSpan::new(
                    PresentedPrimitiveKind::Image,
                    0,
                    1,
                    FrameRect::new(24.0, 20.0, 24.0, 24.0).unwrap(),
                )],
                PointerDrawMode::ImageRelief(presented_pointer_integration_relief(
                    false, background,
                )),
                PointerDrawMode::ImageRelief(presented_pointer_integration_relief(
                    true, background,
                )),
            )],
        )
        .unwrap();
    frame
}

fn pixel_luma(pixel: [u8; 4]) -> u16 {
    u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2])
}

#[test]
fn presented_pointer_integration_image_relief_flips_edge_polarity_without_moving_content() {
    let Some(mut h) = try_harness() else {
        eprintln!("SKIP: no GPU adapter");
        return;
    };
    let pixels = [255_u8, 40, 180, 80].repeat(16 * 16);
    h.renderer
        .load_image_argb32_with_id(77, &pixels, 16, 16, 16 * 4);
    let decode_deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < decode_deadline {
        h.renderer.process_pending_images();
        if h.renderer.is_image_ready(77) {
            break;
        }
        std::thread::yield_now();
    }
    assert!(h.renderer.is_image_ready(77), "test image must decode");
    let frame = presented_pointer_integration_image_frame();
    let render = |h: &mut Harness, selection| {
        h.renderer.render_frame_glyphs(
            &h.view,
            &frame,
            &mut h.atlas,
            W,
            H,
            false,
            None,
            (36.0, 32.0),
            None,
            selection,
            None,
        );
        read_back(h)
    };
    let selection = |phase| {
        Some(PointerAppearanceSelection::new(
            PointerAppearanceId::try_from(0usize).unwrap(),
            phase,
        ))
    };

    let base = render(&mut h, None);
    let raised = render(&mut h, selection(PointerAppearancePhase::Hover));
    let sunken = render(&mut h, selection(PointerAppearancePhase::Pressed));
    let restored = render(&mut h, None);

    // GNU's thick-edge correction paints the outermost top/left pixel with
    // the opposite shade, so sample the inner pixel of each 2px edge.
    let raised_top = pixel_luma(px(&raised, 36, 21));
    let raised_bottom = pixel_luma(px(&raised, 36, 42));
    let sunken_top = pixel_luma(px(&sunken, 36, 21));
    let sunken_bottom = pixel_luma(px(&sunken, 36, 42));
    assert!(
        raised_top > raised_bottom,
        "raised top edge must be lighter than bottom: {raised_top} <= {raised_bottom}"
    );
    assert!(
        sunken_top < sunken_bottom,
        "sunken top edge must be darker than bottom: {sunken_top} >= {sunken_bottom}"
    );
    assert_eq!(
        px(&raised, 36, 32),
        px(&base, 36, 32),
        "relief must not move or recolor interior image content"
    );
    assert_eq!(px(&sunken, 36, 32), px(&base, 36, 32));
    assert_eq!(
        restored, base,
        "leaving restores byte-identical base pixels"
    );
}

// Stage 4 core invariant: compositing (static-scene-without-cursor) + blit +
// cursor-only produces the same pixels as a single full render. This proves
// the retained-scene fast path is correct by construction for a clean cursor.
fn make_tex(r: &WgpuRenderer, label: &str) -> (wgpu::Texture, wgpu::TextureView) {
    make_tex_sized(r, label, W, H)
}

fn make_tex_sized(
    r: &WgpuRenderer,
    label: &str,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let t = r.device().create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let v = t.create_view(&wgpu::TextureViewDescriptor::default());
    (t, v)
}
fn read_tex(r: &WgpuRenderer, t: &wgpu::Texture) -> Vec<u8> {
    let unpadded = W * 4;
    let padded =
        unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buf = r.device().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (padded * H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = r.device().create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: t,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    r.queue().submit(std::iter::once(enc.finish()));
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    r.device()
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(3)),
        })
        .expect("poll");
    let data = slice.get_mapped_range();
    let mut out = vec![0u8; (unpadded * H) as usize];
    for row in 0..H {
        let s = (row * padded) as usize;
        let d = (row * unpadded) as usize;
        out[d..d + unpadded as usize].copy_from_slice(&data[s..s + unpadded as usize]);
    }
    out
}
fn pxb(buf: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * W + x) * 4) as usize;
    [buf[i + 2], buf[i + 1], buf[i], buf[i + 3]]
}

#[test]
fn composite_matches_full_render() {
    let Some(mut h) = try_harness() else {
        return;
    };
    let frame = frame_with_cursor(Color::rgb(1.0, 0.0, 0.0));

    // A: full render (cursor inline).
    let (ta, va) = make_tex(&h.renderer, "full");
    h.renderer.render_frame_glyphs(
        &va,
        &frame,
        &mut h.atlas,
        W,
        H,
        true,
        None,
        (0.0, 0.0),
        None,
        None,
        None,
    );
    let full = read_tex(&h.renderer, &ta);

    // B: static (no cursor) -> retained tex; blit -> composite tex; cursor-only.
    let (_ts, vs) = make_tex(&h.renderer, "static");
    h.renderer.render_frame_glyphs(
        &vs,
        &frame,
        &mut h.atlas,
        W,
        H,
        false,
        None,
        (0.0, 0.0),
        None,
        None,
        None,
    );
    let (tc, vc) = make_tex(&h.renderer, "composite");
    let bg = h.renderer.create_texture_bind_group(&vs);
    h.renderer.blit_texture_to_view(&bg, &vc, W, H);
    h.renderer
        .render_cursor_only(&vc, &frame, W, H, true, None, (0.0, 0.0));
    let comp = read_tex(&h.renderer, &tc);

    // Compare: allow tiny per-channel tolerance for the sRGB blit round-trip.
    let mut max_diff = 0i32;
    let mut ndiff = 0;
    for y in 0..H {
        for x in 0..W {
            let a = pxb(&full, x, y);
            let b = pxb(&comp, x, y);
            for c in 0..4 {
                let d = (a[c] as i32 - b[c] as i32).abs();
                if d > max_diff {
                    max_diff = d;
                }
                if d > 2 {
                    ndiff += 1;
                }
            }
        }
    }
    eprintln!(
        "composite vs full: max_diff={} pixels_over_tol={}",
        max_diff, ndiff
    );
    assert!(
        max_diff <= 2,
        "composite must match full render within sRGB round-trip tolerance, max_diff={max_diff}"
    );
}

// Stage 4 reuse invariant: a retained static scene built once is reused
// across cursor color changes. The static region stays bit-identical while
// only the cursor color updates — the actual cursor-cycling win.
#[test]
fn retained_static_reused_across_cursor_colors() {
    let Some(mut h) = try_harness() else {
        return;
    };
    // The default config cycles cursor color from time; disable it so the
    // frame's explicit cursor colors drive the pixels for this test.
    h.renderer.effects.cursor_color_cycle.enabled = false;
    let frame_a = frame_with_cursor(Color::rgb(1.0, 0.0, 0.0)); // red cursor
    let frame_b = frame_with_cursor(Color::rgb(0.0, 0.4, 1.0)); // blue cursor

    // Build the retained (cursorless) static scene ONCE.
    let (_ts, vs) = make_tex(&h.renderer, "static");
    h.renderer.render_frame_glyphs(
        &vs,
        &frame_a,
        &mut h.atlas,
        W,
        H,
        false,
        None,
        (0.0, 0.0),
        None,
        None,
        None,
    );
    let bg = h.renderer.create_texture_bind_group(&vs);

    // Composite the SAME retained scene with the red then blue cursor.
    let (tr, vr) = make_tex(&h.renderer, "comp-red");
    h.renderer.blit_texture_to_view(&bg, &vr, W, H);
    h.renderer
        .render_cursor_only(&vr, &frame_a, W, H, true, None, (0.0, 0.0));
    let red = read_tex(&h.renderer, &tr);

    let (tb, vb) = make_tex(&h.renderer, "comp-blue");
    h.renderer.blit_texture_to_view(&bg, &vb, W, H);
    h.renderer
        .render_cursor_only(&vb, &frame_b, W, H, true, None, (0.0, 0.0));
    let blue = read_tex(&h.renderer, &tb);

    // Outside the cursor slot (x>=16), the two composites are bit-identical.
    let mut static_diffs = 0;
    for y in 0..H {
        for x in 16..W {
            if pxb(&red, x, y) != pxb(&blue, x, y) {
                static_diffs += 1;
            }
        }
    }
    assert_eq!(
        static_diffs, 0,
        "static scene must be identical across cursor colors"
    );

    // The cursor slot itself differs (red vs blue).
    let mut cursor_changed = false;
    for y in 8..31 {
        for x in 8..12 {
            let r = pxb(&red, x, y);
            let b = pxb(&blue, x, y);
            if r[0] > b[0] + 40 && b[2] > r[2] + 40 {
                cursor_changed = true;
            }
        }
    }
    assert!(
        cursor_changed,
        "cursor color must change between composites"
    );
}

// Filled-box cursor over a glyph: the composite (static cursorless scene +
// blit + scissored cell redraw with box + char in cursor_fg) must equal the
// full render. The glyph renders as an emergency fallback here, but both
// paths use it identically, so pixel-equality proves the composite logic.
fn filled_box_frame() -> FrameGlyphBuffer {
    use neomacs_display_protocol::types::FaceId;
    let mut frame = FrameGlyphBuffer::with_size(W as f32, H as f32);
    frame.background = Color::rgb(0.10, 0.12, 0.16);
    frame.set_face(
        FaceId::default(),
        Color::rgb(0.9, 0.9, 0.9),
        None,
        400,
        false,
        0,
        None,
        0,
        None,
        0,
        None,
    );
    frame.add_char('A', 20.0, 16.0, 10.0, 18.0, 14.0, false);
    frame.set_phys_cursor(PhysCursor {
        window_id: DisplayWindowId::new(1),
        charpos: 0,
        row: 0,
        col: 0,
        slot_id: DisplaySlotId {
            window_id: DisplayWindowId::new(1),
            row: 0,
            col: 0,
        },
        x: 20.0,
        y: 16.0,
        width: 10.0,
        height: 18.0,
        ascent: 14.0,
        style: CursorStyle::FilledBox,
        color: Color::rgb(1.0, 0.5, 0.0),
        cursor_fg: Color::rgb(0.05, 0.05, 0.05),
    });
    frame
}

#[test]
fn filled_box_composite_matches_full_render() {
    let Some(mut h) = try_harness() else {
        return;
    };
    h.renderer.effects.cursor_color_cycle.enabled = false;
    let frame = filled_box_frame();
    h.atlas
        .set_current_frame_fonts(&frame.fonts, &frame.char_fonts, &frame.shaped_clusters);

    // A: full render with the filled-box cursor inline.
    let (ta, va) = make_tex(&h.renderer, "fb-full");
    h.renderer.render_frame_glyphs(
        &va,
        &frame,
        &mut h.atlas,
        W,
        H,
        true,
        None,
        (0.0, 0.0),
        None,
        None,
        None,
    );
    let full = read_tex(&h.renderer, &ta);

    // B: cursorless static -> blit -> scissored cell redraw (box + char).
    let (_ts, vs) = make_tex(&h.renderer, "fb-static");
    h.renderer.render_frame_glyphs(
        &vs,
        &frame,
        &mut h.atlas,
        W,
        H,
        false,
        None,
        (0.0, 0.0),
        None,
        None,
        None,
    );
    let (tc, vc) = make_tex(&h.renderer, "fb-composite");
    let bg = h.renderer.create_texture_bind_group(&vs);
    h.renderer.blit_texture_to_view(&bg, &vc, W, H);
    // Match the runtime sequence exactly: render_cursor_only draws the box
    // (cursor_bg) unscissored, then the scissored cell redraw adds box + char.
    h.renderer
        .render_cursor_only(&vc, &frame, W, H, true, None, (0.0, 0.0));
    // cursor cell = the glyph cell (20,16,10,18)
    h.renderer.render_frame_cell_loaded(
        &vc,
        &frame,
        &mut h.atlas,
        W,
        H,
        true,
        None,
        (0.0, 0.0),
        (20, 16, 10, 18),
    );
    let comp = read_tex(&h.renderer, &tc);

    let mut max_diff = 0i32;
    for y in 0..H {
        for x in 0..W {
            let a = pxb(&full, x, y);
            let b = pxb(&comp, x, y);
            for c in 0..4 {
                let d = (a[c] as i32 - b[c] as i32).abs();
                if d > max_diff {
                    max_diff = d;
                }
            }
        }
    }
    eprintln!("filled-box composite vs full: max_diff={}", max_diff);
    assert!(
        max_diff <= 2,
        "filled-box composite must match full render, max_diff={max_diff}"
    );
}

#[test]
fn filled_box_cell_redraw_ignores_stale_cell_below_resized_surface() {
    let Some(mut h) = try_harness() else {
        return;
    };
    let frame = filled_box_frame();
    h.atlas
        .set_current_frame_fonts(&frame.fonts, &frame.char_fonts, &frame.shaped_clusters);

    // Model a rapid shrink: the committed frame still places the cursor cell
    // at y=16..34, while the newly acquired surface is only eight pixels tall.
    // The stale cell has no intersection with the render target and must be
    // skipped rather than encoded as an out-of-bounds zero-height scissor.
    let (_target, view) = make_tex_sized(&h.renderer, "resized-below-cursor", W, 8);
    h.renderer.render_frame_cell_loaded(
        &view,
        &frame,
        &mut h.atlas,
        W,
        8,
        true,
        None,
        (0.0, 0.0),
        (20, 16, 10, 18),
    );
}
