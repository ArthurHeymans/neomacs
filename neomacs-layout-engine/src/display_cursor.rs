use crate::types::WindowParams;
use crate::unicode::{decode_utf8, is_cluster_extender, is_wide_char};
use neomacs_display_protocol::frame_glyphs::CursorStyle;

#[inline]
pub(crate) fn next_tab_stop_col(
    current_col: usize,
    tab_width: i32,
    tab_stop_list: &[i32],
) -> usize {
    if !tab_stop_list.is_empty() {
        if let Some(&stop) = tab_stop_list
            .iter()
            .find(|&&stop| (stop as usize) > current_col)
        {
            return stop as usize;
        }
        let last = *tab_stop_list.last().unwrap() as usize;
        let tab_w = tab_width.max(1) as usize;
        if current_col >= last {
            return last + ((current_col - last) / tab_w + 1) * tab_w;
        }
        return last;
    }

    let tab_w = tab_width.max(1) as usize;
    ((current_col / tab_w) + 1) * tab_w
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CursorSlotWidthPolicy {
    ExplicitPixels(f32),
    GlyphColumns(usize),
    TabClamp { frame_char_width: f32 },
}

pub(crate) struct CursorSlotWidthRequest<'a> {
    style: CursorStyle,
    text: &'a [u8],
    byte_idx: usize,
    col: i32,
    tab_width: i32,
    tab_stop_list: &'a [i32],
    x_stretch_cursor: bool,
    frame_char_width: f32,
}

impl<'a> CursorSlotWidthRequest<'a> {
    pub(crate) fn from_window_params(
        style: CursorStyle,
        text: &'a [u8],
        byte_idx: usize,
        col: i32,
        params: &'a WindowParams,
    ) -> Self {
        Self {
            style,
            text,
            byte_idx,
            col,
            tab_width: params.tab_width,
            tab_stop_list: &params.tab_stop_list,
            x_stretch_cursor: params.x_stretch_cursor,
            frame_char_width: params.char_width,
        }
    }

    pub(crate) fn point_columns(&self) -> usize {
        if self.byte_idx >= self.text.len() {
            return 1;
        }

        let (ch, _) = decode_utf8(&self.text[self.byte_idx..]);
        match ch {
            '\t' => {
                let col_usize = self.col.max(0) as usize;
                let next_tab = next_tab_stop_col(col_usize, self.tab_width, self.tab_stop_list)
                    .max(col_usize + 1);
                next_tab - col_usize
            }
            '\n' | '\r' => 1,
            _ if is_cluster_extender(ch) => 0,
            _ if is_wide_char(ch) => 2,
            _ => 1,
        }
    }

    pub(crate) fn width_policy(&self) -> CursorSlotWidthPolicy {
        match self.style {
            CursorStyle::Bar(width) => CursorSlotWidthPolicy::ExplicitPixels(width),
            CursorStyle::Hbar(_) => CursorSlotWidthPolicy::GlyphColumns(self.point_columns()),
            CursorStyle::FilledBox | CursorStyle::Hollow => {
                if !self.x_stretch_cursor && self.byte_idx < self.text.len() {
                    let (ch, _) = decode_utf8(&self.text[self.byte_idx..]);
                    if ch == '\t' {
                        return CursorSlotWidthPolicy::TabClamp {
                            frame_char_width: self.frame_char_width,
                        };
                    }
                }
                CursorSlotWidthPolicy::GlyphColumns(self.point_columns())
            }
        }
    }

    pub(crate) fn width_px(&self, face_char_w: f32) -> f32 {
        self.width_policy().width_px(face_char_w)
    }
}

impl CursorSlotWidthPolicy {
    pub(crate) fn width_px(self, face_char_w: f32) -> f32 {
        match self {
            Self::ExplicitPixels(width) => width,
            Self::GlyphColumns(columns) => columns as f32 * face_char_w,
            Self::TabClamp { frame_char_width } => frame_char_width.max(1.0),
        }
    }
}
