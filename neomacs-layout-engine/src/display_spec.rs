//! Display property parsing for replacement glyphs.
//!
//! GNU xdisp treats display specs as a small typed domain: strings, images,
//! spaces, and xwidgets.  Neomacs adds native video and a temporary WebKit
//! convenience spec.  Keep symbol/plist parsing in this module so layout code
//! consumes typed requests instead of open-coding display-spec shapes.

use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::{
    ImageResolveRequest, ImageResolveSource, VideoResolveRequest, VideoResolveSource,
    WebKitResolveRequest, WebKitResolveSource,
};
use neovm_core::emacs_core::image::ImageSpecKey;
use neovm_core::emacs_core::value::{ValueKind, list_to_vec};
use neovm_core::face::Color as LispColor;
use strum::{EnumString, IntoStaticStr};

#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum DisplaySpecHead {
    Image,
    Video,
    Webkit,
}

impl DisplaySpecHead {
    pub(crate) fn is_head_of(self, value: &Value) -> bool {
        value.is_cons() && value.cons_car().is_symbol_named(self.into())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayImageLayout {
    pub(crate) request: ImageResolveRequest,
    pub(crate) scale: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayVideoLayout {
    pub(crate) request: VideoResolveRequest,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) loop_count: i32,
    pub(crate) autoplay: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayWebKitLayout {
    pub(crate) request: WebKitResolveRequest,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

pub(crate) fn parse_display_image_layout(
    prop_val: &Value,
    default_fg: u32,
    default_bg: u32,
) -> Option<DisplayImageLayout> {
    let items = list_to_vec(prop_val)?;
    if items.first()?.as_symbol_name() != Some(DisplaySpecHead::Image.into()) {
        return None;
    }

    let mut source = None;
    let mut max_width = 0u32;
    let mut max_height = 0u32;
    let mut scale = 1.0f32;
    let mut fg_color = default_fg;
    let mut bg_color = default_bg;

    let mut i = 1usize;
    while i + 1 < items.len() {
        let value = items[i + 1];
        match ImageSpecKey::from_lisp_value(items[i]) {
            Some(ImageSpecKey::File) => {
                source = value
                    .as_lisp_string()
                    .cloned()
                    .map(ImageResolveSource::File);
            }
            Some(ImageSpecKey::Data) => {
                source = value
                    .as_lisp_string()
                    .map(|data| ImageResolveSource::Data(data.as_bytes().to_vec()));
            }
            Some(ImageSpecKey::Width | ImageSpecKey::MaxWidth) => {
                max_width = parse_image_dimension(value).unwrap_or(max_width);
            }
            Some(ImageSpecKey::Height | ImageSpecKey::MaxHeight) => {
                max_height = parse_image_dimension(value).unwrap_or(max_height);
            }
            Some(ImageSpecKey::Scale) => {
                scale = parse_image_scale(value).unwrap_or(scale);
            }
            Some(ImageSpecKey::Foreground) => {
                fg_color = parse_image_color_pixel(value).unwrap_or(fg_color);
            }
            Some(ImageSpecKey::Background) => {
                bg_color = parse_image_color_pixel(value).unwrap_or(bg_color);
            }
            _ => {}
        }
        i += 2;
    }

    Some(DisplayImageLayout {
        request: ImageResolveRequest {
            source: source?,
            max_width,
            max_height,
            fg_color,
            bg_color,
        },
        scale,
    })
}

pub(crate) fn parse_display_video_layout(
    prop_val: &Value,
    fallback_width: f32,
    fallback_height: f32,
) -> Option<DisplayVideoLayout> {
    let items = list_to_vec(prop_val)?;
    if items.first()?.as_symbol_name() != Some(DisplaySpecHead::Video.into()) {
        return None;
    }

    let mut source = None;
    let mut width = fallback_width.max(1.0);
    let mut height = fallback_height.max(1.0);
    let mut loop_count = 0;
    let mut autoplay = false;

    let mut i = 1usize;
    while i + 1 < items.len() {
        let key = items[i].as_symbol_name();
        let value = items[i + 1];
        match key {
            Some(":file") => {
                source = value
                    .as_lisp_string()
                    .cloned()
                    .map(VideoResolveSource::File);
            }
            Some(":uri") => {
                source = value.as_lisp_string().cloned().map(VideoResolveSource::Uri);
            }
            Some(":width") => {
                if let Some(parsed) = parse_image_dimension(value) {
                    width = parsed.max(1) as f32;
                }
            }
            Some(":height") => {
                if let Some(parsed) = parse_image_dimension(value) {
                    height = parsed.max(1) as f32;
                }
            }
            Some(":loop") | Some(":loop-count") => {
                loop_count = parse_video_loop_count(value);
            }
            Some(":autoplay") => {
                autoplay = parse_boolish(value);
            }
            _ => {}
        }
        i += 2;
    }

    Some(DisplayVideoLayout {
        request: VideoResolveRequest {
            source: source?,
            loop_count,
            autoplay,
        },
        width,
        height,
        loop_count,
        autoplay,
    })
}

pub(crate) fn parse_display_webkit_layout(
    prop_val: &Value,
    fallback_width: f32,
    fallback_height: f32,
) -> Option<DisplayWebKitLayout> {
    let items = list_to_vec(prop_val)?;
    if items.first()?.as_symbol_name() != Some(DisplaySpecHead::Webkit.into()) {
        return None;
    }

    let mut source = None;
    let mut width = fallback_width.max(1.0);
    let mut height = fallback_height.max(1.0);

    let mut i = 1usize;
    while i + 1 < items.len() {
        let key = items[i].as_symbol_name();
        let value = items[i + 1];
        match key {
            Some(":file") => {
                source = value
                    .as_lisp_string()
                    .cloned()
                    .map(WebKitResolveSource::File);
            }
            Some(":uri") => {
                source = value
                    .as_lisp_string()
                    .cloned()
                    .map(WebKitResolveSource::Uri);
            }
            Some(":width") => {
                if let Some(parsed) = parse_image_dimension(value) {
                    width = parsed.max(1) as f32;
                }
            }
            Some(":height") => {
                if let Some(parsed) = parse_image_dimension(value) {
                    height = parsed.max(1) as f32;
                }
            }
            _ => {}
        }
        i += 2;
    }

    Some(DisplayWebKitLayout {
        request: WebKitResolveRequest {
            source: source?,
            width: width.round().max(1.0) as u32,
            height: height.round().max(1.0) as u32,
        },
        width,
        height,
    })
}

fn parse_image_dimension(value: Value) -> Option<u32> {
    match value.kind() {
        ValueKind::Fixnum(_) => Some(value.as_int()?.max(0) as u32),
        ValueKind::Float => Some(value.as_float()?.max(0.0).round() as u32),
        _ => None,
    }
}

fn parse_image_scale(value: Value) -> Option<f32> {
    if value.is_symbol_named("default") {
        return None;
    }
    match value.kind() {
        ValueKind::Fixnum(_) => Some(value.as_int()?.max(0) as f32),
        ValueKind::Float => Some(value.as_float()?.max(0.0) as f32),
        _ => None,
    }
}

fn parse_image_color_pixel(value: Value) -> Option<u32> {
    let color = value
        .as_lisp_string()
        .and_then(|name| LispColor::parse(name.as_utf8_str()?))?;
    Some(((color.r as u32) << 16) | ((color.g as u32) << 8) | color.b as u32)
}

fn parse_boolish(value: Value) -> bool {
    !value.is_nil()
}

fn parse_video_loop_count(value: Value) -> i32 {
    if value.is_nil() {
        return 0;
    }
    if value.is_symbol_named("t") {
        return -1;
    }
    value.as_int().unwrap_or(-1) as i32
}
