use crate::neovm_bridge::{LayoutBufferView, RustBufferAccess};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowSource {
    window_start: i64,
    text_start_byte: usize,
    bytes_read: usize,
}

impl BufferTextWindowSource {
    pub(crate) const fn window_start(self) -> i64 {
        self.window_start
    }

    pub(crate) const fn text_start_byte(self) -> usize {
        self.text_start_byte
    }

    pub(crate) const fn bytes_read(self) -> usize {
        self.bytes_read
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BufferTextWindowSourceRequest {
    requested_window_start: i64,
    previous_window_end: Option<i64>,
    point_charpos: i64,
    accessible_start: i64,
    accessible_end: i64,
    max_rows: usize,
    window_width_px: i64,
    is_minibuffer: bool,
}

impl BufferTextWindowSourceRequest {
    pub(crate) fn new(
        requested_window_start: i64,
        previous_window_end: Option<i64>,
        point_charpos: i64,
        accessible_start: i64,
        accessible_end: i64,
        max_rows: usize,
        window_width_px: f32,
        is_minibuffer: bool,
    ) -> Self {
        Self {
            requested_window_start,
            previous_window_end,
            point_charpos,
            accessible_start,
            accessible_end,
            max_rows,
            window_width_px: window_width_px.max(1.0) as i64,
            is_minibuffer,
        }
    }

    pub(crate) fn read_into<B: LayoutBufferView>(
        self,
        access: &RustBufferAccess<'_, B>,
        out: &mut Vec<u8>,
    ) -> BufferTextWindowSource {
        let window_start =
            self.resolve_window_start(|charpos| access.byte_at(access.charpos_to_bytepos(charpos)));
        let text_start_byte = access.charpos_to_bytepos(window_start) as usize;
        let read_chars = self.accessible_end - window_start + 1;
        let bytes_read = if read_chars <= 0 {
            out.clear();
            0
        } else {
            let text_end = (window_start + read_chars).min(self.accessible_end);
            let byte_to = access.charpos_to_bytepos(text_end);
            access.copy_text(text_start_byte as i64, byte_to, out);
            out.len()
        };

        BufferTextWindowSource {
            window_start,
            text_start_byte,
            bytes_read,
        }
    }

    fn resolve_window_start(self, byte_at_charpos: impl Fn(i64) -> Option<u8>) -> i64 {
        let mut window_start = self.requested_window_start.max(self.accessible_start);

        if window_start > self.accessible_start {
            let remaining_chars = self.accessible_end - window_start;
            if remaining_chars < self.max_rows as i64 && self.accessible_end > self.max_rows as i64
            {
                window_start =
                    self.scan_back_from_point((self.max_rows / 2).max(1), &byte_at_charpos);
            }
        }

        if self.point_charpos >= self.accessible_start && self.point_charpos < window_start {
            let adjusted = self.scan_back_from_point((self.max_rows / 4).max(1), &byte_at_charpos);
            tracing::debug!(
                "layout_window_rust: adjusted window_start {} -> {} (point={})",
                self.requested_window_start,
                adjusted,
                self.point_charpos
            );
            return adjusted;
        }

        if self.should_forward_scroll_without_layout(window_start) {
            let adjusted =
                self.scan_back_from_point(((self.max_rows * 3) / 4).max(1), &byte_at_charpos);
            tracing::debug!(
                "layout_window_rust: forward-adjusted window_start {} -> {} (point={}, prev_end={})",
                self.requested_window_start,
                adjusted,
                self.point_charpos,
                self.previous_window_end.unwrap_or(0)
            );
            return adjusted;
        }

        window_start
    }

    fn should_forward_scroll_without_layout(self, window_start: i64) -> bool {
        if self.point_charpos <= 0 || self.is_minibuffer {
            return false;
        }
        let has_prev_end = self
            .previous_window_end
            .is_some_and(|end| self.point_charpos > end);
        let max_visible_chars = (self.max_rows.max(1) as i64) * self.window_width_px;
        let far_below_without_prev_end = self.previous_window_end.is_none()
            && self.point_charpos - window_start > max_visible_chars;
        has_prev_end || far_below_without_prev_end
    }

    fn scan_back_from_point(
        self,
        target_rows_above: usize,
        byte_at_charpos: &impl Fn(i64) -> Option<u8>,
    ) -> i64 {
        let mut lines_back = 0usize;
        let mut scan_pos = self.point_charpos.max(self.accessible_start);
        while scan_pos > self.accessible_start && lines_back < target_rows_above {
            scan_pos -= 1;
            if byte_at_charpos(scan_pos) == Some(b'\n') {
                lines_back += 1;
            }
        }
        scan_pos.max(self.accessible_start)
    }
}

#[cfg(test)]
#[path = "display_buffer_text_source_test.rs"]
mod tests;
