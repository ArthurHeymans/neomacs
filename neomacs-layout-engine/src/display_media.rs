use crate::display_item::{
    DisplayImageItem, DisplayMediaReplacement, DisplayVideoItem, DisplayXwidgetItem,
};
use crate::display_row_metrics::DisplayRowFallbackMetrics;
use crate::display_row_width::DisplayRowCharWidthPolicy;
use crate::display_spec::{
    parse_display_image_layout, parse_display_video_layout, parse_display_webkit_layout,
};
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::DisplayHost;

#[derive(Clone, Copy)]
pub(crate) struct DisplayMediaResolveParams<'a> {
    pub(crate) display_host: &'a dyn DisplayHost,
    pub(crate) default_fg: u32,
    pub(crate) default_bg: u32,
    pub(crate) fallback_metrics: DisplayRowFallbackMetrics,
}

pub(crate) fn resolve_display_media_property(
    display_prop: &Value,
    params: DisplayMediaResolveParams<'_>,
) -> Option<DisplayMediaReplacement> {
    resolve_image_display_property(display_prop, params)
        .or_else(|| resolve_video_display_property(display_prop, params))
        .or_else(|| resolve_webkit_display_property(display_prop, params))
}

fn resolve_image_display_property(
    display_prop: &Value,
    params: DisplayMediaResolveParams<'_>,
) -> Option<DisplayMediaReplacement> {
    let spec = parse_display_image_layout(display_prop, params.default_fg, params.default_bg)?;
    let scale = spec.scale;
    let resolved = params
        .display_host
        .request_image(spec.request)
        .ok()
        .flatten()?;
    let mut width = resolved.width.max(1) as f32;
    let mut height = resolved.height.max(1) as f32;
    if (scale - 1.0).abs() > f32::EPSILON && scale.is_finite() && scale > 0.0 {
        width = (width * scale).round().max(1.0);
        height = (height * scale).round().max(1.0);
    }
    Some(DisplayMediaReplacement::image(DisplayImageItem {
        image_id: display_media_id(resolved.image_id),
        width,
        height,
    }))
}

fn resolve_video_display_property(
    display_prop: &Value,
    params: DisplayMediaResolveParams<'_>,
) -> Option<DisplayMediaReplacement> {
    let spec = parse_display_video_layout(
        display_prop,
        DisplayRowCharWidthPolicy::new(params.fallback_metrics.char_width()).fallback() * 40.0,
        params.fallback_metrics.row_height() * 12.0,
    )?;
    let resolved = params
        .display_host
        .request_video(spec.request.clone())
        .ok()
        .flatten()?;
    Some(DisplayMediaReplacement::video(DisplayVideoItem {
        video_id: display_media_id(resolved.video_id),
        width: spec.width.max(1.0),
        height: spec.height.max(1.0),
        loop_count: spec.loop_count,
        autoplay: spec.autoplay,
    }))
}

fn resolve_webkit_display_property(
    display_prop: &Value,
    params: DisplayMediaResolveParams<'_>,
) -> Option<DisplayMediaReplacement> {
    let spec = parse_display_webkit_layout(
        display_prop,
        DisplayRowCharWidthPolicy::new(params.fallback_metrics.char_width()).fallback() * 40.0,
        params.fallback_metrics.row_height() * 12.0,
    )?;
    let resolved = params
        .display_host
        .request_webkit(spec.request.clone())
        .ok()
        .flatten()?;
    Some(DisplayMediaReplacement::xwidget(DisplayXwidgetItem {
        xwidget_id: display_media_id(resolved.webkit_id),
        width: spec.width.max(1.0),
        height: spec.height.max(1.0),
    }))
}

fn display_media_id(id: u32) -> i32 {
    id.min(i32::MAX as u32) as i32
}
