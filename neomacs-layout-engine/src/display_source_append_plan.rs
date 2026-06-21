use crate::display_item::{DisplayItem, DisplayItemKind};
use crate::display_row_append_context::DisplayRowAppendFrame;
use crate::display_row_builder::DisplayRowItemMeasurement;
use crate::display_row_render_policy::DisplayRowRenderPolicy;
use crate::display_text_run_measurement::DisplayTextRunMeasurementPlan;
use crate::font_metrics::FontMetricsService;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NaturalDisplayRowAppendRenderPolicy;

impl DisplayRowRenderPolicy for NaturalDisplayRowAppendRenderPolicy {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedSourceAdvanceRenderPolicy {
    advance_px: f32,
}

impl ResolvedSourceAdvanceRenderPolicy {
    pub(crate) fn new(advance_px: f32) -> Self {
        Self { advance_px }
    }

    fn measurement_for_text(&self, text: &str) -> DisplayRowItemMeasurement {
        DisplayRowItemMeasurement::TextRun(
            DisplayTextRunMeasurementPlan::from_resolved_source_advance(text, self.advance_px),
        )
    }
}

impl DisplayRowRenderPolicy for ResolvedSourceAdvanceRenderPolicy {
    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        _face_id: u32,
        _font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        match &item.kind {
            DisplayItemKind::TextRun(run) => self.measurement_for_text(&run.text),
            DisplayItemKind::SourceMappedText(text) => self.measurement_for_text(&text.text),
            _ => DisplayRowItemMeasurement::Default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DisplaySourceAppendRenderPolicy {
    Natural(NaturalDisplayRowAppendRenderPolicy),
    Resolved(ResolvedSourceAdvanceRenderPolicy),
}

impl DisplaySourceAppendRenderPolicy {
    pub(crate) fn natural() -> Self {
        Self::Natural(NaturalDisplayRowAppendRenderPolicy)
    }

    pub(crate) fn resolved_advance(advance_px: f32) -> Self {
        Self::Resolved(ResolvedSourceAdvanceRenderPolicy::new(advance_px))
    }
}

impl DisplayRowRenderPolicy for DisplaySourceAppendRenderPolicy {
    fn measurement_for(
        &mut self,
        item: &DisplayItem,
        face_id: u32,
        font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        match self {
            Self::Natural(policy) => policy.measurement_for(item, face_id, font_metrics),
            Self::Resolved(policy) => policy.measurement_for(item, face_id, font_metrics),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DisplaySourceFallbackWidth {
    columns: usize,
}

impl DisplaySourceFallbackWidth {
    pub(crate) fn columns(columns: usize) -> Self {
        Self { columns }
    }

    #[cfg(test)]
    pub(crate) fn column_count(self) -> usize {
        self.columns
    }

    pub(crate) fn resolve_to_text_row(self, frame: &DisplayRowAppendFrame) -> f32 {
        frame.width_for_columns(self.columns)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplaySourceAppendMeasurementKind {
    NaturalRenderedSource,
    ResolvedComplexRun,
}

impl DisplaySourceAppendMeasurementKind {
    pub(crate) fn for_char(ch: char) -> Self {
        if crate::composition::needs_complex_shaping(ch) {
            Self::ResolvedComplexRun
        } else {
            Self::NaturalRenderedSource
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplaySourceAppendRenderPlan {
    advance_px: f32,
    policy: DisplaySourceAppendRenderPolicy,
}

impl DisplaySourceAppendRenderPlan {
    pub(crate) fn natural(advance_px: f32) -> Self {
        Self {
            advance_px,
            policy: DisplaySourceAppendRenderPolicy::natural(),
        }
    }

    pub(crate) fn resolved_advance(advance_px: f32) -> Self {
        Self {
            advance_px,
            policy: DisplaySourceAppendRenderPolicy::resolved_advance(advance_px),
        }
    }

    pub(crate) fn advance_px(self) -> f32 {
        self.advance_px
    }

    pub(crate) fn render_policy(self) -> DisplaySourceAppendRenderPolicy {
        self.policy
    }
}
