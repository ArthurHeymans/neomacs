use crate::display_space::{DisplaySpaceKey, display_space_positive_number, is_display_space_spec};
use crate::engine::LayoutEngine;
use crate::neovm_bridge::FaceResolver;
use crate::neovm_bridge::ResolvedFace;
use crate::unicode::decode_utf8;
use neomacs_display_protocol::face::{BoxType, Face, FaceAttributes, UnderlineStyle};
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::types::Color;
use neovm_core::buffer::{BufferId, CharPos0, text_props::TextPropertyTable};
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::value::get_string_text_properties_table_for_value;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub(crate) struct LispStringSource {
    pub string: Value,
}

#[allow(dead_code)]
impl LispStringSource {
    pub(crate) fn text(self) -> Option<String> {
        self.string.as_runtime_string_owned()
    }

    pub(crate) fn build_row_spec(
        self,
        engine: &mut LayoutEngine,
        request: DisplayRowRequest,
        next_face_id: &mut u32,
        face_resolver: &FaceResolver,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Option<DisplayRowSpec> {
        engine.build_display_row_spec_from_lisp_source(
            self,
            request,
            next_face_id,
            face_resolver,
            symbol_values,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextSource {
    pub buffer_id: BufferId,
    pub window_id: i64,
    pub start: CharPos0,
    pub end: CharPos0,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) enum DisplayRowSourceKind {
    LispString(LispStringSource),
    BufferText(BufferTextSource),
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct DisplayRowRequest {
    pub role: GlyphRowRole,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub window_id: i64,
    pub matrix_row: Option<usize>,
    pub base_face: ResolvedFace,
    pub source: LispStringSource,
}

fn char_index_to_byte(text: &[u8], char_index: usize) -> usize {
    std::str::from_utf8(text)
        .ok()
        .and_then(|s| {
            if char_index == s.chars().count() {
                Some(s.len())
            } else {
                s.char_indices().nth(char_index).map(|(byte, _)| byte)
            }
        })
        .unwrap_or_else(|| char_index.min(text.len()))
}

fn underline_style_from_code(code: u8) -> UnderlineStyle {
    UnderlineStyle::from_gnu_code(code).unwrap_or_default()
}

/// Shared render-facing face spec for all status-line backends.
#[derive(Debug, Clone)]
pub(crate) struct StatusLineFace {
    pub(crate) face_id: u32,
    pub(crate) foreground: Color,
    pub(crate) background: Color,
    pub(crate) use_default_foreground: bool,
    pub(crate) use_default_background: bool,
    pub(crate) font_family: String,
    pub(crate) font_file_path: Option<String>,
    pub(crate) font_weight: u16,
    pub(crate) italic: bool,
    pub(crate) font_size: f32,
    pub(crate) underline_style: u8,
    pub(crate) underline_color: Option<Color>,
    pub(crate) strike_through: bool,
    pub(crate) strike_through_color: Option<Color>,
    pub(crate) overline: bool,
    pub(crate) overline_color: Option<Color>,
    pub(crate) box_type: BoxType,
    pub(crate) box_color: Option<Color>,
    pub(crate) box_line_width: i32,
    pub(crate) box_corner_radius: i32,
    pub(crate) box_border_style: u32,
    pub(crate) box_border_speed: f32,
    pub(crate) box_color2: Option<Color>,
    pub(crate) box_h_line_width: i32,
    pub(crate) terminal_inverse_video: bool,
    pub(crate) font_char_width: f32,
    pub(crate) font_ascent: f32,
    pub(crate) font_descent: i32,
    pub(crate) underline_position: i32,
    pub(crate) underline_thickness: i32,
}

impl StatusLineFace {
    pub(crate) fn from_resolved(face_id: u32, face: &ResolvedFace) -> Self {
        let font_descent = if face.font_line_height > 0.0 && face.font_ascent > 0.0 {
            (face.font_line_height - face.font_ascent).max(0.0).ceil() as i32
        } else {
            0
        };
        let box_type = BoxType::from_gnu_code(face.box_type).unwrap_or_default();
        Self {
            face_id,
            foreground: Color::from_pixel(face.fg),
            background: Color::from_pixel(face.bg),
            use_default_foreground: face.use_default_foreground,
            use_default_background: face.use_default_background,
            font_family: if face.font_family.is_empty() {
                "monospace".to_string()
            } else {
                face.font_family.clone()
            },
            font_file_path: None,
            font_weight: face.font_weight,
            italic: face.italic,
            font_size: face.font_size,
            underline_style: face.underline_style,
            underline_color: (face.underline_style > 0)
                .then(|| Color::from_pixel(face.underline_color)),
            strike_through: face.strike_through,
            strike_through_color: face
                .strike_through
                .then(|| Color::from_pixel(face.strike_through_color)),
            overline: face.overline,
            overline_color: face
                .overline
                .then(|| Color::from_pixel(face.overline_color)),
            box_type,
            box_color: (box_type != BoxType::None && face.box_color != 0)
                .then(|| Color::from_pixel(face.box_color)),
            box_line_width: face.box_line_width,
            box_corner_radius: 0,
            box_border_style: 0,
            box_border_speed: 1.0,
            box_color2: None,
            box_h_line_width: face.box_line_width,
            terminal_inverse_video: face.terminal_inverse_video,
            font_char_width: face.font_char_width,
            font_ascent: face.font_ascent,
            font_descent,
            underline_position: 1,
            underline_thickness: 1,
        }
    }

    pub(crate) fn with_color_override(
        &self,
        face_id: u32,
        fg: Option<Color>,
        bg: Option<Color>,
    ) -> Self {
        let mut face = self.clone();
        face.face_id = face_id;
        if let Some(color) = fg {
            face.foreground = color;
            face.use_default_foreground = false;
        }
        if let Some(color) = bg {
            face.background = color;
            face.use_default_background = false;
        }
        if fg.is_some() || bg.is_some() {
            face.terminal_inverse_video = false;
        }
        face
    }

    pub(crate) fn render_face(&self) -> Face {
        let underline_style = underline_style_from_code(self.underline_style);
        let mut attrs = FaceAttributes::empty();
        if self.font_weight >= 700 {
            attrs |= FaceAttributes::BOLD;
        }
        if self.italic {
            attrs |= FaceAttributes::ITALIC;
        }
        if underline_style != UnderlineStyle::None {
            attrs |= FaceAttributes::UNDERLINE;
        }
        if self.strike_through {
            attrs |= FaceAttributes::STRIKE_THROUGH;
        }
        if self.overline {
            attrs |= FaceAttributes::OVERLINE;
        }
        if !matches!(self.box_type, BoxType::None) {
            attrs |= FaceAttributes::BOX;
        }
        if self.terminal_inverse_video {
            attrs |= FaceAttributes::INVERSE;
        }
        Face {
            id: self.face_id,
            foreground: self.foreground,
            background: self.background,
            use_default_foreground: self.use_default_foreground,
            use_default_background: self.use_default_background,
            underline_color: self.underline_color,
            overline_color: self.overline_color,
            strike_through_color: self.strike_through_color,
            box_color: self.box_color,
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            font_weight: self.font_weight,
            attributes: attrs,
            underline_style,
            box_type: self.box_type,
            box_line_width: self.box_line_width,
            box_corner_radius: self.box_corner_radius,
            box_border_style: self.box_border_style,
            box_border_speed: self.box_border_speed,
            box_color2: self.box_color2,
            font_file_path: self.font_file_path.clone(),
            font_ascent: self.font_ascent as i32,
            font_descent: self.font_descent,
            underline_position: self.underline_position.max(1),
            underline_thickness: self.underline_thickness.max(1),
            background_gradient: None,
        }
    }
}

/// A face run within an overlay/display string: byte offset + fg/bg colors + face_id.
#[derive(Debug, Clone)]
pub(crate) struct OverlayFaceRun {
    pub byte_offset: u16,
    pub fg: u32,
    pub bg: u32,
    #[cfg(test)]
    /// Emacs face ID for full face attribute resolution via FFI
    pub extend: bool,
    /// Emacs face ID for full face attribute resolution via FFI
    pub face_id: u32,
}

/// Parse face runs appended after text in a buffer.
/// Runs are stored as 14-byte records: u16 byte_offset + u32 fg + u32 bg + u32 face_id.
#[cfg(test)]
pub(crate) fn parse_overlay_face_runs(
    buf: &[u8],
    text_len: usize,
    nruns: i32,
) -> Vec<OverlayFaceRun> {
    let mut runs = Vec::with_capacity(nruns as usize);
    let runs_start = text_len;
    for ri in 0..nruns as usize {
        let off = runs_start + ri * 14;
        if off + 14 <= buf.len() {
            let byte_offset = u16::from_ne_bytes([buf[off], buf[off + 1]]);
            let fg = u32::from_ne_bytes([buf[off + 2], buf[off + 3], buf[off + 4], buf[off + 5]]);
            let raw_bg =
                u32::from_ne_bytes([buf[off + 6], buf[off + 7], buf[off + 8], buf[off + 9]]);
            #[cfg(test)]
            let extend = (raw_bg & 0x80000000) != 0;
            let bg = raw_bg & 0x00FFFFFF;
            let face_id =
                u32::from_ne_bytes([buf[off + 10], buf[off + 11], buf[off + 12], buf[off + 13]]);
            runs.push(OverlayFaceRun {
                byte_offset,
                fg,
                bg,
                #[cfg(test)]
                extend,
                face_id,
            });
        }
    }
    runs
}

/// An align-to entry within an overlay string.
#[derive(Debug, Clone)]
pub(crate) struct OverlayAlignEntry {
    pub byte_offset: u16,
    pub covers_bytes: u16,
    pub align_to_px: f32,
}

/// Apply the face run covering the current byte index.
/// Returns the updated current_run index.
#[cfg(test)]
pub(crate) fn apply_overlay_face_run(
    runs: &[OverlayFaceRun],
    byte_idx: usize,
    current_run: usize,
) -> usize {
    let mut cr = current_run;
    // Advance to the correct run
    while cr + 1 < runs.len() && byte_idx >= runs[cr + 1].byte_offset as usize {
        cr += 1;
    }
    if byte_idx >= runs[cr].byte_offset as usize {
        // Pre-advance if next run starts at next byte
        if cr + 1 < runs.len() && byte_idx + 1 >= runs[cr + 1].byte_offset as usize {
            cr += 1;
        }
    }
    cr
}

/// A display property record extracted from a mode-line string.
/// Only width participates in the current backend walker.
#[derive(Debug, Clone)]
pub(crate) struct DisplayPropRecord {
    pub(crate) byte_offset: u16,
    pub(crate) covers_bytes: u16,
    pub(crate) width: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct DisplayRowSpec {
    pub(crate) role: GlyphRowRole,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) char_width: f32,
    pub(crate) ascent: f32,
    pub(crate) face: StatusLineFace,
    pub(crate) text: Vec<u8>,
    pub(crate) face_runs: Vec<OverlayFaceRun>,
    pub(crate) run_faces: HashMap<u32, StatusLineFace>,
    pub(crate) display_props: Vec<DisplayPropRecord>,
    pub(crate) align_entries: Vec<OverlayAlignEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StatusLineOutputProgress {
    pub end_x: f32,
    pub end_col: i64,
    pub y: f32,
    pub height: f32,
}

impl DisplayRowSpec {
    fn plain(
        role: GlyphRowRole,
        y: f32,
        width: f32,
        height: f32,
        char_width: f32,
        ascent: f32,
        face: StatusLineFace,
        text: String,
    ) -> Self {
        Self {
            role,
            y,
            width,
            height,
            char_width,
            ascent,
            face,
            text: text.into_bytes(),
            face_runs: Vec::new(),
            run_faces: HashMap::new(),
            display_props: Vec::new(),
            align_entries: Vec::new(),
        }
    }
}

fn same_resolved_face(lhs: &ResolvedFace, rhs: &ResolvedFace) -> bool {
    lhs.fg == rhs.fg
        && lhs.bg == rhs.bg
        && lhs.font_family == rhs.font_family
        && lhs.font_weight == rhs.font_weight
        && lhs.italic == rhs.italic
        && (lhs.font_size - rhs.font_size).abs() <= f32::EPSILON
        && lhs.underline_style == rhs.underline_style
        && lhs.underline_color == rhs.underline_color
        && lhs.strike_through == rhs.strike_through
        && lhs.strike_through_color == rhs.strike_through_color
        && lhs.overline == rhs.overline
        && lhs.overline_color == rhs.overline_color
        && lhs.box_type == rhs.box_type
        && lhs.box_color == rhs.box_color
        && lhs.box_line_width == rhs.box_line_width
        && lhs.extend == rhs.extend
        && lhs.terminal_inverse_video == rhs.terminal_inverse_video
}

impl LayoutEngine {
    #[allow(dead_code)]
    pub(crate) fn build_display_row_spec_from_lisp_source(
        &mut self,
        source: LispStringSource,
        request: DisplayRowRequest,
        next_face_id: &mut u32,
        face_resolver: &FaceResolver,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Option<DisplayRowSpec> {
        self.build_display_row_spec_from_request(
            DisplayRowRequest { source, ..request },
            next_face_id,
            face_resolver,
            symbol_values,
        )
    }

    pub(crate) fn build_display_row_spec_from_request(
        &mut self,
        request: DisplayRowRequest,
        next_face_id: &mut u32,
        face_resolver: &FaceResolver,
        symbol_values: std::collections::HashMap<String, Value>,
    ) -> Option<DisplayRowSpec> {
        let char_w = request.base_face.font_char_width.max(1.0);
        let ascent = request.base_face.font_ascent.max(request.height * 0.8);
        self.build_propertized_display_row_spec(
            request.x,
            request.y,
            request.width,
            request.height,
            request.window_id,
            char_w,
            ascent,
            next_face_id,
            &request.base_face,
            request.source.string,
            face_resolver,
            symbol_values,
            request.role,
        )
    }

    pub(crate) fn render_display_row_spec_to_glyph_row(
        &mut self,
        spec: &DisplayRowSpec,
        mut on_progress: Option<&mut dyn FnMut(StatusLineOutputProgress)>,
    ) -> Option<(
        neomacs_display_protocol::glyph_matrix::GlyphRow,
        StatusLineOutputProgress,
    )> {
        use crate::display_backend::{DisplayBackend, GlyphKind, TtyDisplayBackend};
        use neomacs_display_protocol::glyph_matrix::GlyphRow;

        if spec.text.is_empty() {
            let mut row = GlyphRow::new(spec.role);
            row.enabled = true;
            row.mode_line = true;
            row.height_px = spec.height.max(1.0);
            row.ascent_px = spec.face.font_ascent.max(0.0).min(row.height_px);
            return Some((
                row,
                StatusLineOutputProgress {
                    end_x: 0.0,
                    end_col: 0,
                    y: spec.y,
                    height: spec.height,
                },
            ));
        }

        // The backend collects all character / stretch glyphs for
        // the row. Face ids are set on each produced glyph via the
        // TtyDisplayBackend::produce_glyph path (which now honors
        // `face.id` after the Step 3.4 fix), so no per-glyph face id
        // bookkeeping is needed outside the backend.
        //
        // Note: the status-line walker does its own per-character
        // measurement via `spec.char_width` / `status_line_advance`
        // (which consult `self.font_metrics` on the GUI path), so
        // `backend.char_advance` is never consulted here. The
        // backend is used purely for glyph accumulation, and that
        // accumulation is identical between `TtyDisplayBackend` and
        // `GuiDisplayBackend`. So this site always constructs a
        // `TtyDisplayBackend`, which also avoids the borrow
        // conflict between `self.font_metrics` (held by a
        // hypothetical `GuiDisplayBackend`) and the walker's
        // `self.status_line_advance` call a few lines below.
        let mut backend = TtyDisplayBackend::new();
        // The backend Face we feed into produce_glyph — rebuilt on
        // each face change so that `face.id` on the produced glyph
        // matches the active run face.
        let mut current_render_face = spec.face.render_face();

        let mut sl_x_offset = 0.0f32;
        let mut byte_idx = 0usize;
        let mut current_run = 0usize;
        let mut dp_idx = 0usize;
        let mut align_idx = 0usize;
        let mut emit_progress = |end_x: f32| {
            if let Some(ref mut cb) = on_progress {
                cb(StatusLineOutputProgress {
                    end_x: end_x.min(spec.width).max(0.0),
                    end_col: (end_x / spec.char_width.max(1.0)).round().max(0.0) as i64,
                    y: spec.y,
                    height: spec.height,
                });
            }
        };

        // Text-geometry fields mirror the original; they are unused
        // for glyph emission (the backend produces cell glyphs with
        // no per-glyph y coordinate) but computed here so future
        // GUI backends can receive them through the same walker.
        let _ascent = if spec.face.font_ascent > 0.0 {
            spec.face.font_ascent
        } else {
            spec.ascent
        };
        let _inset = if spec.face.box_h_line_width > 0 {
            spec.face.box_h_line_width as f32
        } else {
            0.0
        };

        while byte_idx < spec.text.len() && sl_x_offset < spec.width {
            // --- align-to entries ---
            if align_idx < spec.align_entries.len()
                && byte_idx == spec.align_entries[align_idx].byte_offset as usize
            {
                let align = &spec.align_entries[align_idx];
                let target_x = align.align_to_px;
                if target_x > sl_x_offset {
                    let gap = target_x - sl_x_offset;
                    let cols = (gap / spec.char_width.max(1.0)).round() as usize;
                    // Emit `cols` individual space glyphs via the
                    // backend. The TtyDisplayBackend then materializes
                    // them as Char(' ') glyphs, matching the 3.3′
                    // workaround retained from 3.3' so align-to
                    // gaps remain materialized as explicit cells
                    // instead of a single aggregated stretch glyph.
                    for _ in 0..cols {
                        backend.produce_glyph_with_pixel_width(
                            GlyphKind::Char(' '),
                            &current_render_face,
                            0,
                            spec.char_width.max(1.0),
                        );
                        sl_x_offset += spec.char_width.max(1.0);
                        emit_progress(sl_x_offset);
                    }
                    sl_x_offset = target_x;
                    emit_progress(sl_x_offset);
                }
                align_idx += 1;
                byte_idx = (align.byte_offset + align.covers_bytes) as usize;
                continue;
            }

            // --- display-prop entries (stretch / image) ---
            if dp_idx < spec.display_props.len() {
                let dp = &spec.display_props[dp_idx];
                if byte_idx == dp.byte_offset as usize {
                    if dp.width > 0 {
                        sl_x_offset += dp.width as f32;
                        emit_progress(sl_x_offset);
                    }
                    byte_idx = (dp.byte_offset + dp.covers_bytes) as usize;
                    dp_idx += 1;
                    continue;
                }
            }

            // --- resolve face for current run ---
            if current_run < spec.face_runs.len() {
                while current_run + 1 < spec.face_runs.len()
                    && byte_idx >= spec.face_runs[current_run + 1].byte_offset as usize
                {
                    current_run += 1;
                }
                if byte_idx >= spec.face_runs[current_run].byte_offset as usize {
                    let run = &spec.face_runs[current_run];
                    if run.fg != 0 || run.bg != 0 {
                        if let Some(run_face) = spec.run_faces.get(&run.face_id) {
                            current_render_face = run_face.render_face();
                        } else if run.face_id != 0 {
                            let rf = spec.face.with_color_override(
                                run.face_id,
                                Some(Color::from_pixel(run.fg)),
                                Some(Color::from_pixel(run.bg)),
                            );
                            current_render_face = rf.render_face();
                        } else {
                            current_render_face = spec.face.render_face();
                        }
                    }
                }
            }

            // --- compute end of current text segment ---
            let mut end_byte = spec.text.len();
            if align_idx < spec.align_entries.len() {
                end_byte = end_byte.min(spec.align_entries[align_idx].byte_offset as usize);
            }
            if dp_idx < spec.display_props.len() {
                end_byte = end_byte.min(spec.display_props[dp_idx].byte_offset as usize);
            }
            if current_run < spec.face_runs.len() {
                let current_run_offset = spec.face_runs[current_run].byte_offset as usize;
                if byte_idx < current_run_offset {
                    end_byte = end_byte.min(current_run_offset);
                } else if current_run + 1 < spec.face_runs.len() {
                    end_byte = end_byte.min(spec.face_runs[current_run + 1].byte_offset as usize);
                }
            }
            end_byte = end_byte.max(byte_idx);

            // --- emit text run through the backend ---
            //
            // Walks the text slice char-by-char, measuring via the
            // backend's own char_advance and stopping when the
            // remaining width is exhausted. Mirrors the inner loop of
            // `render_text_run` for the backend path.
            let fallback_width = spec.char_width.max(1.0);
            let mut run_offset = 0usize;
            let mut run_advance = 0.0f32;
            while run_offset < (end_byte - byte_idx) && sl_x_offset + run_advance < spec.width {
                let (ch, ch_len) = decode_utf8(&spec.text[byte_idx + run_offset..end_byte]);
                run_offset += ch_len;
                if ch == '\n' || ch == '\r' {
                    continue;
                }
                let advance = fallback_width;
                backend.produce_glyph_with_pixel_width(
                    GlyphKind::Char(ch),
                    &current_render_face,
                    0,
                    advance,
                );
                run_advance += advance;
                emit_progress(sl_x_offset + run_advance);
            }
            sl_x_offset += run_advance;
            byte_idx = end_byte;
        }

        // Flush the row through the backend and install the
        // produced text-area glyphs into the caller's matrix
        // builder wholesale. The backend is the sole producer.
        let mut flush_row = GlyphRow::new(spec.role);
        flush_row.enabled = true;
        flush_row.mode_line = true;
        flush_row.pixel_y = spec.y;
        flush_row.height_px = spec.height.max(1.0);
        flush_row.ascent_px = spec.face.font_ascent.max(0.0).min(flush_row.height_px);
        backend.finish_row(flush_row);
        let mut rows = backend.take_rows();
        let mut row = rows.pop()?;
        crate::matrix_builder::GlyphMatrixBuilder::normalize_external_row(&mut row);
        let progress = StatusLineOutputProgress {
            end_x: sl_x_offset.min(spec.width).max(0.0),
            end_col: (sl_x_offset / spec.char_width.max(1.0)).round().max(0.0) as i64,
            y: spec.y,
            height: spec.height,
        };
        Some((row, progress))
    }

    pub(crate) fn render_display_row_spec_via_backend(
        &mut self,
        spec: &DisplayRowSpec,
        matrix_row: Option<usize>,
        mut builder: Option<&mut crate::matrix_builder::GlyphMatrixBuilder>,
        on_progress: Option<&mut dyn FnMut(StatusLineOutputProgress)>,
    ) -> Option<StatusLineOutputProgress> {
        if let Some(ref mut b) = builder {
            if let Some(row) = matrix_row {
                b.begin_row(row, spec.role);
                let row_ascent = if spec.face.font_ascent > 0.0 {
                    spec.face.font_ascent
                } else {
                    spec.ascent
                }
                .max(0.0)
                .min(spec.height.max(1.0));
                b.set_current_row_metrics(spec.y, spec.height, row_ascent);
            } else if b.begin_status_line_row(spec.role) {
                let row_ascent = if spec.face.font_ascent > 0.0 {
                    spec.face.font_ascent
                } else {
                    spec.ascent
                }
                .max(0.0)
                .min(spec.height.max(1.0));
                b.set_current_window_last_row_metrics(spec.y, spec.height, row_ascent);
            }

            b.insert_face(spec.face.face_id, spec.face.render_face());
            for run_face in spec.run_faces.values() {
                b.insert_face(run_face.face_id, run_face.render_face());
            }
        }

        let (mut row, progress) = self.render_display_row_spec_to_glyph_row(spec, on_progress)?;
        if let Some(ref mut b) = builder {
            let text_glyphs = std::mem::take(&mut row.glyphs[1]);
            if matrix_row.is_some() {
                b.install_current_row_glyphs(text_glyphs);
                b.end_row();
            } else {
                b.install_status_line_row_glyphs(text_glyphs);
            }
        }
        Some(progress)
    }

    /// Step 3.5 entry point: equivalent to `render_rust_status_line_value`
    /// but routes glyph emission through `TtyDisplayBackend` via
    /// `render_display_row_spec_via_backend`.
    pub(crate) fn render_rust_status_line_value_via_backend(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        matrix_row: usize,
        window_id: i64,
        char_w: f32,
        ascent: f32,
        next_face_id: &mut u32,
        face: &ResolvedFace,
        rendered: Value,
        face_resolver: &FaceResolver,
        symbol_values: std::collections::HashMap<String, Value>,
        role: GlyphRowRole,
        builder: Option<&mut crate::matrix_builder::GlyphMatrixBuilder>,
        on_progress: Option<&mut dyn FnMut(StatusLineOutputProgress)>,
    ) -> Option<StatusLineOutputProgress> {
        if let Some(spec) = self.build_propertized_display_row_spec(
            x,
            y,
            width,
            height,
            window_id,
            char_w,
            ascent,
            next_face_id,
            face,
            rendered,
            face_resolver,
            symbol_values,
            role,
        ) {
            return self.render_display_row_spec_via_backend(
                &spec,
                Some(matrix_row),
                builder,
                on_progress,
            );
        }
        None
    }

    fn resolved_display_row_face_at_char(
        face_resolver: &FaceResolver,
        base_face: &ResolvedFace,
        props: &TextPropertyTable,
        charpos: usize,
    ) -> ResolvedFace {
        let mut face = base_face.clone();
        let charpos = CharPos0::new(charpos);
        let face_prop = props.get_property_at_char_pos(charpos, Value::symbol("face"));
        let font_lock_face_prop =
            props.get_property_at_char_pos(charpos, Value::symbol("font-lock-face"));
        if let Some(value) = face_prop.or(font_lock_face_prop)
            && let Some(next) = face_resolver.resolve_face_value_over(&face, &value)
        {
            face = next;
        }
        face
    }

    pub(crate) fn build_propertized_display_row_spec(
        &mut self,
        _x: f32,
        y: f32,
        width: f32,
        height: f32,
        _window_id: i64,
        char_w: f32,
        ascent: f32,
        next_face_id: &mut u32,
        base_face: &ResolvedFace,
        rendered: Value,
        face_resolver: &FaceResolver,
        symbol_values: std::collections::HashMap<String, Value>,
        role: GlyphRowRole,
    ) -> Option<DisplayRowSpec> {
        let text = rendered.as_runtime_string_owned()?;
        // Use the resolved face's cache ID when already assigned
        // (basic faces get fixed IDs from BasicFaceId); otherwise
        // allocate from the per-frame counter.
        let base_face_id = if base_face.face_id != 0 {
            base_face.face_id
        } else {
            let id = *next_face_id;
            *next_face_id += 1;
            id
        };
        let face = self.realize_status_line_face(base_face_id, base_face, char_w, ascent, height);
        let char_width = self.status_line_char_width(&face, char_w);
        let mut spec =
            DisplayRowSpec::plain(role, y, width, height, char_width, ascent, face, text);

        if !rendered.is_string() {
            return Some(spec);
        }
        let Some(props) = get_string_text_properties_table_for_value(rendered) else {
            return Some(spec);
        };

        let mut boundaries = vec![0usize];
        for interval in props.intervals_snapshot() {
            if interval.properties.contains_key(&Value::symbol("face"))
                || interval
                    .properties
                    .contains_key(&Value::symbol("font-lock-face"))
            {
                boundaries.push(interval.start);
                boundaries.push(interval.end);
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();

        let mut current_face = base_face.clone();
        for boundary in boundaries {
            if boundary >= spec.text.len() {
                continue;
            }
            let resolved =
                Self::resolved_display_row_face_at_char(face_resolver, base_face, &props, boundary);
            if same_resolved_face(&resolved, &current_face) {
                continue;
            }

            let (face_id, run_face) = if same_resolved_face(&resolved, base_face) {
                (spec.face.face_id, None)
            } else {
                let face_id = *next_face_id;
                *next_face_id += 1;
                let run_face =
                    self.realize_status_line_face(face_id, &resolved, char_w, ascent, height);
                (face_id, Some(run_face))
            };

            spec.face_runs.push(OverlayFaceRun {
                byte_offset: char_index_to_byte(&spec.text, boundary) as u16,
                fg: resolved.fg,
                bg: resolved.bg,
                #[cfg(test)]
                extend: resolved.extend,
                face_id,
            });
            if let Some(run_face) = run_face {
                spec.run_faces.insert(face_id, run_face);
            }
            current_face = resolved;
        }

        // ----- Display property harvesting for (space :width N) and
        // (space :align-to E) -----
        //
        // Mirrors GNU's handle_display_prop → produce_stretch_glyph
        // chain (xdisp.c:5858 → 32510): when the walker encounters a
        // string byte whose `display` text property is a `(space …)`
        // spec, it emits a stretch glyph of the evaluated width/
        // position. For neomacs, we evaluate the spec here at harvest
        // time and feed the result into the existing align_entries /
        // display_props buffers that the render loop already knows
        // how to consume (status_line.rs:651+).
        //
        // The older harvester at lines 803-812 intentionally only
        // scanned for `face` and `font-lock-face` — `display` was
        // silently dropped, which caused doom-modeline's
        // `(space :align-to (- right rhs-width))` to collapse to zero
        // width and the mode-line to render without right-aligned
        // content. Step 2 of the unification plan landed the
        // calc_pixel_width_or_height port; this harvester extension
        // is what actually wires it into the mode-line path.
        //
        // Coordinate system: the align_entries/display_props fields
        // on DisplayRowSpec use status-line-local pixel offsets
        // (0 = left edge of the status line). We build a
        // PixelCalcContext where `text_area_*` reflect the status
        // line's own width, so a form like `(- right 200)` resolves
        // to `spec.width - 200` in the same coordinate system.
        use crate::display_pixel_calc::{PixelCalcContext, calc_pixel_width_or_height};
        let pctx = PixelCalcContext {
            frame_column_width: char_width as f64,
            frame_line_height: height as f64,
            frame_res_x: 96.0,
            frame_res_y: 96.0,
            face_font_height: height as f64,
            face_font_width: char_width as f64,
            text_area_left: 0.0,
            text_area_right: width as f64,
            text_area_width: width as f64,
            left_margin_left: 0.0,
            left_margin_width: 0.0,
            right_margin_left: width as f64,
            right_margin_width: 0.0,
            left_fringe_width: 0.0,
            right_fringe_width: 0.0,
            fringes_outside_margins: false,
            scroll_bar_width: 0.0,
            scroll_bar_on_left: false,
            line_number_pixel_width: 0.0,
            symbol_values,
        };

        for interval in props.intervals_snapshot() {
            let Some(disp_prop) = interval.properties.get(&Value::symbol("display")) else {
                continue;
            };
            // Only handle (space …) specs here. Other display values
            // (strings, images, margin specs) follow their own paths
            // that status_line.rs does not yet support; they remain
            // TODO for future commits.
            if !is_display_space_spec(disp_prop) {
                continue;
            }
            let byte_start = char_index_to_byte(&spec.text, interval.start);
            let byte_end = char_index_to_byte(&spec.text, interval.end);
            let covers_bytes = byte_end.saturating_sub(byte_start) as u16;
            let Some(items) = neovm_core::emacs_core::value::list_to_vec(disp_prop) else {
                continue;
            };
            // items[0] = 'space; walk the keyword-value plist.
            let mut i = 1usize;
            let mut done = false;
            while i + 1 < items.len() && !done {
                let key = items[i];
                let val = items[i + 1];
                match DisplaySpaceKey::from_lisp_value(key) {
                    Some(DisplaySpaceKey::Width) => {
                        if let Some(pixels) = calc_pixel_width_or_height(&pctx, &val, true, None) {
                            let byte_offset =
                                (byte_start as u16).min(spec.text.len().saturating_sub(1) as u16);
                            spec.display_props.push(DisplayPropRecord {
                                byte_offset,
                                covers_bytes,
                                width: (pixels as u16).max(0),
                            });
                            done = true;
                        }
                    }
                    Some(DisplaySpaceKey::RelativeWidth) => {
                        if let Some(factor) = display_space_positive_number(val) {
                            let byte_offset =
                                (byte_start as u16).min(spec.text.len().saturating_sub(1) as u16);
                            spec.display_props.push(DisplayPropRecord {
                                byte_offset,
                                covers_bytes,
                                width: (factor * char_width).round().max(0.0) as u16,
                            });
                            done = true;
                        }
                    }
                    Some(DisplaySpaceKey::AlignTo) => {
                        let mut align_to: i32 = -1;
                        if let Some(pixels) =
                            calc_pixel_width_or_height(&pctx, &val, true, Some(&mut align_to))
                        {
                            // See the buffer-text display-space geometry
                            // path for the same shape of post-processing: if the
                            // expression contained a window-box symbol
                            // (`right`, `text`, …) it resolved a base
                            // position and `pixels` is the offset from it;
                            // otherwise `pixels` is a column-relative
                            // offset and the caller adds content_x. For
                            // status-line-local coordinates, content_x is
                            // 0.
                            let target_x = if align_to >= 0 {
                                align_to as f32 + pixels as f32
                            } else {
                                pixels as f32
                            };
                            let byte_offset =
                                (byte_start as u16).min(spec.text.len().saturating_sub(1) as u16);
                            spec.align_entries.push(OverlayAlignEntry {
                                byte_offset,
                                covers_bytes,
                                align_to_px: target_x,
                            });
                            done = true;
                        }
                    }
                    _ => {}
                }
                i += 2;
            }
        }

        // The render loop expects align_entries and display_props
        // sorted by byte_offset so its single-pass walker can advance
        // through them in order.
        spec.align_entries.sort_by_key(|e| e.byte_offset);
        spec.display_props.sort_by_key(|d| d.byte_offset);

        Some(spec)
    }
}

#[cfg(test)]
#[path = "display_row_test.rs"]
mod tests;
