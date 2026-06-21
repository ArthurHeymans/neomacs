use crate::display_item::DisplayItem;
use crate::display_row_builder::DisplayRowItemMeasurement;
use crate::font_metrics::FontMetricsService;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayRowRenderClipBehavior {
    PreserveRemainderAndStop,
    Stop,
    Continue,
}

pub(crate) trait DisplayRowRenderPolicy {
    fn stop_before_item(&mut self, _item: &DisplayItem) -> bool {
        false
    }

    fn measurement_for(
        &mut self,
        _item: &DisplayItem,
        _face_id: u32,
        _font_metrics: &mut Option<FontMetricsService>,
    ) -> DisplayRowItemMeasurement {
        DisplayRowItemMeasurement::Default
    }

    fn clipped_behavior(&mut self, _item: &DisplayItem) -> DisplayRowRenderClipBehavior {
        DisplayRowRenderClipBehavior::PreserveRemainderAndStop
    }
}

pub(crate) struct NaturalDisplayRowRenderPolicy;

impl DisplayRowRenderPolicy for NaturalDisplayRowRenderPolicy {}
