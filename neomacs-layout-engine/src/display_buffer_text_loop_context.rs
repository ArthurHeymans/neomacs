//! Buffer text loop request builders.

use crate::display_buffer_text_face_resolution::BufferSourceItemLayoutResolutionContext;
use crate::display_buffer_text_overflow::{
    BufferTextSourceItemRenderContext, BufferTextSourceItemRenderRequest,
};
use crate::display_buffer_text_row_lifecycle::{
    BufferEndOfBufferTailRenderContext, BufferEndOfBufferTailRenderRequest,
    BufferHscrollSkipRenderContext, BufferHscrollSkipRenderRequest,
    BufferInvisibleTextRenderContext, BufferInvisibleTextRenderRequest,
    BufferSelectiveDisplayTailRenderContext, BufferSelectiveDisplayTailRenderRequest,
    BufferTextLineBreakRenderContext, BufferTextLineBreakRenderRequest,
};
use crate::display_buffer_text_source::BufferTextSourceStepChar;
use crate::display_buffer_text_source_consumption::BufferTextSourceItem;
use crate::display_row::DisplayRowActiveFaceState;
use crate::display_row_append_context::DisplayRowAppendSurface;
use crate::display_row_geometry::{
    DisplayRowGeometryDefaults, DisplayRowLimit, DisplayRowVisibilityLimit,
};
use crate::display_row_overlay_string::BufferOverlayStringTextRowRenderContext;
use crate::types::WindowParams;
use neovm_core::buffer::BufferId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BufferTextWindowLoopRequestContext {
    buffer_id: BufferId,
    text_start_byte: usize,
    accessible_end: i64,
    point_charpos: i64,
    selective_display: i32,
    tab_width: i32,
    extra_line_spacing: f32,
    content_x: f32,
    has_prefix: bool,
    default_face_ascent: f32,
    char_height: f32,
    char_width: f32,
    row_visibility_limit: DisplayRowVisibilityLimit,
    row_geometry_defaults: DisplayRowGeometryDefaults,
    display_text_row_base: usize,
    max_rows: usize,
    row_limit: DisplayRowLimit,
}

impl BufferTextWindowLoopRequestContext {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer_id: BufferId,
        text_start_byte: usize,
        accessible_end: i64,
        point_charpos: i64,
        params: &WindowParams,
        content_x: f32,
        has_prefix: bool,
        default_face_ascent: f32,
        char_height: f32,
        char_width: f32,
        row_visibility_limit: DisplayRowVisibilityLimit,
        row_geometry_defaults: DisplayRowGeometryDefaults,
        display_text_row_base: usize,
        max_rows: usize,
        row_limit: DisplayRowLimit,
    ) -> Self {
        Self {
            buffer_id,
            text_start_byte,
            accessible_end,
            point_charpos,
            selective_display: params.selective_display,
            tab_width: params.tab_width,
            extra_line_spacing: params.extra_line_spacing,
            content_x,
            has_prefix,
            default_face_ascent,
            char_height,
            char_width,
            row_visibility_limit,
            row_geometry_defaults,
            display_text_row_base,
            max_rows,
            row_limit,
        }
    }

    pub(crate) fn invisible_text_request<'a>(
        self,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
    ) -> BufferInvisibleTextRenderRequest<'a> {
        BufferInvisibleTextRenderRequest::new(BufferInvisibleTextRenderContext::new(
            text,
            self.accessible_end,
            self.point_charpos,
            append_surface,
            overlay_context,
            active_face_state,
            glyph_y_offset,
            self.default_face_ascent,
            self.char_height,
            self.char_width,
        ))
    }

    pub(crate) fn hscroll_skip_request<'a>(
        self,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
    ) -> BufferHscrollSkipRenderRequest<'a> {
        BufferHscrollSkipRenderRequest::new(BufferHscrollSkipRenderContext::new(
            text,
            self.tab_width,
            self.content_x,
            append_surface,
            active_face_state,
            self.default_face_ascent,
            self.char_height,
            self.char_width,
            self.point_charpos,
            self.has_prefix,
            self.row_geometry_defaults,
            self.display_text_row_base,
            self.max_rows,
            self.row_limit,
        ))
    }

    pub(crate) fn selective_display_tail_request<'a>(
        self,
        source_step_char: BufferTextSourceStepChar,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        active_face_state: &'a DisplayRowActiveFaceState,
        glyph_y_offset: f32,
    ) -> BufferSelectiveDisplayTailRenderRequest<'a> {
        BufferSelectiveDisplayTailRenderRequest::new(
            source_step_char,
            BufferSelectiveDisplayTailRenderContext::new(
                text,
                self.text_start_byte,
                self.selective_display,
                self.tab_width,
                append_surface,
                active_face_state,
                glyph_y_offset,
                self.default_face_ascent,
                self.char_height,
                self.char_width,
                self.content_x,
                self.has_prefix,
                self.row_geometry_defaults,
                self.display_text_row_base,
                self.max_rows,
                self.row_limit,
            ),
        )
    }

    pub(crate) fn line_break_request<'a>(
        self,
        source_char: BufferTextSourceStepChar,
        text: &'a [u8],
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
    ) -> BufferTextLineBreakRenderRequest<'a> {
        BufferTextLineBreakRenderRequest::new(
            source_char,
            BufferTextLineBreakRenderContext::new(
                text,
                self.text_start_byte,
                self.selective_display,
                self.tab_width,
                active_face_state,
                self.point_charpos,
                self.char_height,
                self.extra_line_spacing,
                self.content_x,
                self.has_prefix,
                self.row_geometry_defaults,
                self.display_text_row_base,
                self.max_rows,
                self.row_limit,
                overlay_context,
            ),
        )
    }

    pub(crate) fn source_item_request<'a>(
        self,
        layout_resolution_context: BufferSourceItemLayoutResolutionContext<'a>,
        source_item: BufferTextSourceItem,
        text: &'a [u8],
        append_surface: &'a DisplayRowAppendSurface,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
        params: &'a WindowParams,
        glyph_y_offset: f32,
    ) -> BufferTextSourceItemRenderRequest<'a> {
        BufferTextSourceItemRenderRequest::new(
            source_item,
            BufferTextSourceItemRenderContext::new(
                layout_resolution_context,
                text,
                self.text_start_byte,
                self.buffer_id,
                append_surface,
                overlay_context,
                active_face_state,
                params,
                glyph_y_offset,
                self.char_height,
                self.point_charpos,
                self.row_visibility_limit,
                self.content_x,
                self.has_prefix,
                self.row_geometry_defaults,
                self.display_text_row_base,
                self.max_rows,
                self.row_limit,
            ),
        )
    }

    pub(crate) fn end_of_buffer_tail_request<'a>(
        self,
        byte_idx: usize,
        charpos: i64,
        overlay_context: BufferOverlayStringTextRowRenderContext<'a>,
        active_face_state: &'a DisplayRowActiveFaceState,
    ) -> BufferEndOfBufferTailRenderRequest<'a> {
        BufferEndOfBufferTailRenderRequest::new(BufferEndOfBufferTailRenderContext::new(
            byte_idx,
            charpos,
            self.accessible_end,
            self.point_charpos,
            overlay_context,
            active_face_state,
        ))
    }

    pub(crate) fn buffer_id(self) -> BufferId {
        self.buffer_id
    }

    pub(crate) fn text_start_byte(self) -> usize {
        self.text_start_byte
    }

    pub(crate) fn content_x(self) -> f32 {
        self.content_x
    }

    pub(crate) fn char_height(self) -> f32 {
        self.char_height
    }

    pub(crate) fn point_charpos(self) -> i64 {
        self.point_charpos
    }

    pub(crate) fn row_visibility_limit(self) -> DisplayRowVisibilityLimit {
        self.row_visibility_limit
    }

    #[cfg(test)]
    pub(crate) fn accessible_end(self) -> i64 {
        self.accessible_end
    }

    #[cfg(test)]
    pub(crate) fn selective_display(self) -> i32 {
        self.selective_display
    }

    #[cfg(test)]
    pub(crate) fn tab_width(self) -> i32 {
        self.tab_width
    }

    #[cfg(test)]
    pub(crate) fn row_limit(self) -> DisplayRowLimit {
        self.row_limit
    }
}
