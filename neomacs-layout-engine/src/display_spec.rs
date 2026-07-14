//! Display property parsing for replacement glyphs.
//!
//! GNU xdisp treats display specs as a small typed domain: strings, images,
//! spaces, and xwidgets.  Neomacs adds native video and retains a temporary
//! WebKit convenience spec.  Keep symbol/plist parsing in this module so layout
//! code consumes typed requests instead of open-coding display-spec shapes.

use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::eval::{
    VideoResolveRequest, VideoResolveSource, WebKitResolveRequest, WebKitResolveSource,
};
use neovm_core::emacs_core::image::ImageSpecKey;
use neovm_core::emacs_core::image_catalog::{ImageResolveRequest, ImageResolveSource};
use neovm_core::emacs_core::value::{ValueKind, list_to_vec};
use neovm_core::face::Color as LispColor;
use strum::{EnumString, IntoStaticStr};

#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub(crate) enum DisplaySpecHead {
    Image,
    Video,
    Webkit,
    Xwidget,
}

impl DisplaySpecHead {
    pub(crate) fn is_head_of(self, value: &Value) -> bool {
        value.is_cons() && value.cons_car().is_symbol_named(self.into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumString, IntoStaticStr)]
#[strum(prefix = ":", serialize_all = "kebab-case")]
enum DisplayMediaKey {
    File,
    Uri,
    Width,
    Height,
    Loop,
    LoopCount,
    Autoplay,
    Xwidget,
}

impl DisplayMediaKey {
    fn from_lisp_value(value: Value) -> Option<Self> {
        value.as_symbol_name()?.strip_prefix(':')?.parse().ok()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayImageLayout {
    pub(crate) request: ImageResolveRequest,
    pub(crate) scale: f32,
    pub(crate) ascent: DisplayImageAscentPolicy,
    pub(crate) margin: DisplayImageMargin,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DisplayImageMargin {
    pub(crate) horizontal: f32,
    pub(crate) vertical: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DisplayImageAscentPolicy {
    Percent(f32),
    Center,
}

impl Default for DisplayImageAscentPolicy {
    fn default() -> Self {
        Self::Percent(50.0)
    }
}

impl DisplayImageAscentPolicy {
    pub(crate) fn resolve(self, image_height: f32, text_height: f32, text_ascent: f32) -> f32 {
        match self {
            Self::Percent(percent) => image_height * (percent / 100.0),
            Self::Center => {
                let text_descent = (text_height - text_ascent).max(0.0);
                ((image_height + text_ascent - text_descent + 1.0) / 2.0).floor()
            }
        }
    }
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

#[derive(Clone, Debug)]
pub(crate) struct DisplayXwidgetLayout {
    pub(crate) xwidget_id: u32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

/// Which fringe a `(left-fringe …)` / `(right-fringe …)` display spec targets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DisplayFringeSide {
    Left,
    Right,
}

/// Parsed `(left-fringe BITMAP FACE)` / `(right-fringe BITMAP FACE)` display
/// spec. The bitmap is kept as the raw symbol `Value` (resolved to a registry
/// index later, where the evaluator is available); FACE is the optional face
/// symbol the spec requests (a `set-fringe-bitmap-face` override wins over it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayFringeLayout {
    pub(crate) bitmap: Value,
    pub(crate) side: DisplayFringeSide,
    pub(crate) face: Option<Value>,
}

/// Parse a `(left-fringe BITMAP [FACE])` / `(right-fringe BITMAP [FACE])` spec.
/// Returns `None` if the head is not a fringe symbol or BITMAP is missing.
pub(crate) fn parse_display_fringe_layout(value: &Value) -> Option<DisplayFringeLayout> {
    if !value.is_cons() {
        return None;
    }
    let side = match value.cons_car().as_symbol_name()? {
        "left-fringe" => DisplayFringeSide::Left,
        "right-fringe" => DisplayFringeSide::Right,
        _ => return None,
    };
    let items = list_to_vec(value)?;
    // items[0] = head, items[1] = BITMAP, items[2] = optional FACE.
    let bitmap = *items.get(1)?;
    if bitmap.is_nil() {
        return None;
    }
    let face = items.get(2).copied().filter(|face| !face.is_nil());
    Some(DisplayFringeLayout { bitmap, side, face })
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
    let mut ascent = DisplayImageAscentPolicy::default();
    let mut margin = DisplayImageMargin::default();
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
            Some(ImageSpecKey::Ascent) => {
                ascent = parse_image_ascent(value).unwrap_or(ascent);
            }
            Some(ImageSpecKey::Margin) => {
                margin = parse_image_margin(value).unwrap_or(margin);
            }
            Some(ImageSpecKey::Foreground) => {
                fg_color = parse_image_color_pixel(value).unwrap_or(fg_color);
            }
            Some(ImageSpecKey::Background) => {
                if let Some(pixel) = parse_image_color_pixel(value) {
                    bg_color = pixel;
                }
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
        ascent,
        margin,
    })
}

fn parse_image_margin(value: Value) -> Option<DisplayImageMargin> {
    let component = |value: Value| {
        value
            .as_int()
            .filter(|value| *value >= 0)
            .map(|value| value as f32)
    };
    if let Some(margin) = component(value) {
        return Some(DisplayImageMargin {
            horizontal: margin,
            vertical: margin,
        });
    }
    if !value.is_cons() {
        return None;
    }
    Some(DisplayImageMargin {
        horizontal: component(value.cons_car())?,
        vertical: component(value.cons_cdr())?,
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
        let value = items[i + 1];
        match DisplayMediaKey::from_lisp_value(items[i]) {
            Some(DisplayMediaKey::File) => {
                source = value
                    .as_lisp_string()
                    .cloned()
                    .map(VideoResolveSource::File);
            }
            Some(DisplayMediaKey::Uri) => {
                source = value.as_lisp_string().cloned().map(VideoResolveSource::Uri);
            }
            Some(DisplayMediaKey::Width) => {
                if let Some(parsed) = parse_image_dimension(value) {
                    width = parsed.max(1) as f32;
                }
            }
            Some(DisplayMediaKey::Height) => {
                if let Some(parsed) = parse_image_dimension(value) {
                    height = parsed.max(1) as f32;
                }
            }
            Some(DisplayMediaKey::Loop | DisplayMediaKey::LoopCount) => {
                loop_count = parse_video_loop_count(value);
            }
            Some(DisplayMediaKey::Autoplay) => {
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
        let value = items[i + 1];
        match DisplayMediaKey::from_lisp_value(items[i]) {
            Some(DisplayMediaKey::File) => {
                source = value
                    .as_lisp_string()
                    .cloned()
                    .map(WebKitResolveSource::File);
            }
            Some(DisplayMediaKey::Uri) => {
                source = value
                    .as_lisp_string()
                    .cloned()
                    .map(WebKitResolveSource::Uri);
            }
            Some(DisplayMediaKey::Width) => {
                if let Some(parsed) = parse_image_dimension(value) {
                    width = parsed.max(1) as f32;
                }
            }
            Some(DisplayMediaKey::Height) => {
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

pub(crate) fn parse_display_xwidget_layout(prop_val: &Value) -> Option<DisplayXwidgetLayout> {
    let items = list_to_vec(prop_val)?;
    if items.first()?.as_symbol_name() != Some(DisplaySpecHead::Xwidget.into()) {
        return None;
    }

    let mut xwidget = None;
    let mut i = 1usize;
    while i + 1 < items.len() {
        if DisplayMediaKey::from_lisp_value(items[i]) == Some(DisplayMediaKey::Xwidget) {
            xwidget = items[i + 1].as_xwidget();
            break;
        }
        i += 2;
    }

    let xwidget = xwidget?;
    Some(DisplayXwidgetLayout {
        xwidget_id: xwidget.xwidget_id,
        width: xwidget.width.max(0) as f32,
        height: xwidget.height.max(0) as f32,
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

fn parse_image_ascent(value: Value) -> Option<DisplayImageAscentPolicy> {
    if value.is_symbol_named("center") {
        return Some(DisplayImageAscentPolicy::Center);
    }
    let percent = match value.kind() {
        ValueKind::Fixnum(_) => value.as_int()? as f32,
        _ => return None,
    };
    (percent.is_finite() && (0.0..=100.0).contains(&percent))
        .then_some(DisplayImageAscentPolicy::Percent(percent))
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumString, IntoStaticStr)]
#[strum(prefix = ":", serialize_all = "kebab-case")]
pub(crate) enum DisplaySpaceKey {
    Width,
    RelativeWidth,
    AlignTo,
    Height,
    RelativeHeight,
    Ascent,
}

impl DisplaySpaceKey {
    pub(crate) fn from_lisp_value(value: Value) -> Option<Self> {
        Self::from_keyword(value.as_symbol_name()?)
    }

    pub(crate) fn from_keyword(name: &str) -> Option<Self> {
        name.strip_prefix(':')?.parse().ok()
    }

    #[cfg(test)]
    pub(crate) fn keyword(self) -> &'static str {
        self.into()
    }

    #[cfg(test)]
    pub(crate) fn value(self) -> Value {
        Value::keyword(self.keyword())
    }
}

pub(crate) fn is_display_space_spec(value: &Value) -> bool {
    value.is_cons() && value.cons_car().is_symbol_named("space")
}

/// `(left-fringe BITMAP FACE)` / `(right-fringe BITMAP FACE)` display spec: a
/// list whose HEAD symbol is `left-fringe`/`right-fringe`. GNU (`src/xdisp.c`
/// `handle_display_spec` → `handle_single_display_spec`) treats this as a
/// fringe-bitmap replacement that shows nothing inline. This is distinct from
/// the `left-fringe`/`right-fringe` *length units* (`DisplayLengthSymbol`),
/// which only ever appear as a bare symbol or inside a `space` `:width`/
/// `:align-to` pixel expression — never as the head of the `display` value.
pub(crate) fn is_display_fringe_spec(value: &Value) -> bool {
    value.is_cons()
        && (value.cons_car().is_symbol_named("left-fringe")
            || value.cons_car().is_symbol_named("right-fringe"))
}

/// Recognized heads of a SINGLE `display` spec — the symbols GNU's
/// `handle_display_spec` (src/xdisp.c) tests before deciding a list is a
/// *list of display specs* rather than one spec. A `display` value that is a
/// cons whose car is NONE of these (and not nil) is iterated element-by-element
/// (e.g. diff-hl's `((left-fringe BITMAP FACE))`).
///
/// We omit only the eval/control heads NeoMacs does not yet implement
/// (`when`/`slice`/`disable-eval`); those still get treated as single specs by
/// `is_display_spec_list` returning false, matching GNU's "not a list" branch.
const SINGLE_DISPLAY_SPEC_HEADS: &[&str] = &[
    "image",
    "xwidget",
    "space",
    "when",
    "slice",
    "space-width",
    "height",
    "raise",
    "left-fringe",
    "right-fringe",
    "min-width",
    // NeoMacs-only convenience heads handled as single specs.
    "video",
    "webkit",
];

/// Mirror of GNU `handle_display_spec`'s list-of-specs test (src/xdisp.c):
/// a `display` value is a LIST OF DISPLAY SPECS (to be iterated, each element
/// handled as its own single spec) when it is a cons whose car is neither a
/// recognized single-spec head symbol, nor a `(margin …)` marginal-area spec,
/// nor nil. Otherwise it is a single spec.
///
/// This is what classifies diff-hl/flycheck/git-gutter's list-wrapped
/// `((left-fringe BITMAP FACE))` so the inner `(left-fringe …)` is reached.
pub(crate) fn is_display_spec_list(value: &Value) -> bool {
    if !value.is_cons() {
        return false;
    }
    let car = value.cons_car();
    if car.is_nil() {
        return false;
    }
    // `(margin …)` marginal-area spec: car is itself `(margin . _)`.
    if car.is_cons() && car.cons_car().is_symbol_named("margin") {
        return false;
    }
    if let Some(name) = car.as_symbol_name() {
        // A recognized single-spec head symbol => single spec.
        if SINGLE_DISPLAY_SPEC_HEADS.contains(&name) {
            return false;
        }
        // A keyword head (`:raise`/`:height`/…) is a flat property list, not a
        // list of display specs. NeoMacs accepts the keyword-plist convenience
        // form `(:raise 0.2 :height 1.4)` as a SINGLE spec whose modifiers are
        // parsed from the whole plist; iterating it element-by-element would
        // discard those modifiers (a regression GNU never hits — GNU has no such
        // keyword-plist form). Treat any keyword-headed list as a single spec.
        if name.starts_with(':') {
            return false;
        }
        // Any other symbol head (unknown to us) => GNU iterates it as a list.
        return true;
    }
    // car is a cons (e.g. an inner `(left-fringe …)`) or other non-symbol:
    // GNU iterates it as a list of specs.
    true
}

pub(crate) fn display_space_positive_number(value: Value) -> Option<f32> {
    value
        .as_float()
        .or_else(|| value.as_int().map(|integer| integer as f64))
        .filter(|number| number.is_finite() && *number > 0.0)
        .map(|number| number as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_spec(ascent: Option<Value>) -> Value {
        let mut items = vec![
            Value::symbol("image"),
            Value::keyword("type"),
            Value::symbol("svg"),
            Value::keyword("file"),
            Value::string("/tmp/icon.svg"),
        ];
        if let Some(ascent) = ascent {
            items.push(Value::keyword("ascent"));
            items.push(ascent);
        }
        Value::list(items)
    }

    fn parsed_image_ascent(value: Option<Value>) -> DisplayImageAscentPolicy {
        parse_display_image_layout(&image_spec(value), 0, 0)
            .expect("valid image spec")
            .ascent
    }

    #[test]
    fn image_ascent_parses_gnu_domain_and_defaults_invalid_values() {
        let _eval = neovm_core::emacs_core::Context::new();

        assert_eq!(
            parsed_image_ascent(None),
            DisplayImageAscentPolicy::Percent(50.0)
        );
        assert_eq!(
            parsed_image_ascent(Some(Value::symbol("center"))),
            DisplayImageAscentPolicy::Center
        );
        assert_eq!(
            parsed_image_ascent(Some(Value::fixnum(0))),
            DisplayImageAscentPolicy::Percent(0.0)
        );
        assert_eq!(
            parsed_image_ascent(Some(Value::fixnum(100))),
            DisplayImageAscentPolicy::Percent(100.0)
        );
        assert_eq!(
            parsed_image_ascent(Some(Value::fixnum(101))),
            DisplayImageAscentPolicy::Percent(50.0)
        );
        assert_eq!(
            parsed_image_ascent(Some(Value::make_float(75.0))),
            DisplayImageAscentPolicy::Percent(50.0)
        );
    }

    #[test]
    fn image_margin_preserves_gnu_scalar_and_pair_geometry() {
        let mut eval = neovm_core::emacs_core::Context::new();
        eval.setup_thread_locals();
        let scalar = Value::list(vec![
            Value::symbol("image"),
            Value::keyword("file"),
            Value::string("/tmp/icon.svg"),
            Value::keyword("margin"),
            Value::fixnum(2),
        ]);
        let pair = Value::list(vec![
            Value::symbol("image"),
            Value::keyword("file"),
            Value::string("/tmp/icon.svg"),
            Value::keyword("margin"),
            Value::cons(Value::fixnum(3), Value::fixnum(4)),
        ]);

        assert_eq!(
            parse_display_image_layout(&scalar, 0, 0).unwrap().margin,
            DisplayImageMargin {
                horizontal: 2.0,
                vertical: 2.0,
            }
        );
        assert_eq!(
            parse_display_image_layout(&pair, 0, 0).unwrap().margin,
            DisplayImageMargin {
                horizontal: 3.0,
                vertical: 4.0,
            }
        );
    }

    #[test]
    fn image_background_is_only_decoder_input_not_opacity_evidence() {
        let mut eval = neovm_core::emacs_core::Context::new();
        eval.setup_thread_locals();
        let explicit = Value::list(vec![
            Value::symbol("image"),
            Value::keyword("file"),
            Value::string("/tmp/icon.svg"),
            Value::keyword("background"),
            Value::string("#123456"),
        ]);

        assert_eq!(
            parse_display_image_layout(&explicit, 0, 0)
                .unwrap()
                .request
                .bg_color,
            0x12_34_56
        );
    }

    #[test]
    fn parse_display_fringe_layout_left_with_face() {
        let _eval = neovm_core::emacs_core::Context::new();
        let layout = parse_display_fringe_layout(&Value::list(vec![
            Value::symbol("left-fringe"),
            Value::symbol("magit-fringe-bitmap>"),
            Value::symbol("magit-section-heading"),
        ]))
        .expect("left fringe layout");
        assert_eq!(layout.side, DisplayFringeSide::Left);
        assert!(layout.bitmap.is_symbol_named("magit-fringe-bitmap>"));
        assert!(
            layout
                .face
                .is_some_and(|f| f.is_symbol_named("magit-section-heading"))
        );
    }

    #[test]
    fn parse_display_fringe_layout_right_without_face() {
        let _eval = neovm_core::emacs_core::Context::new();
        let layout = parse_display_fringe_layout(&Value::list(vec![
            Value::symbol("right-fringe"),
            Value::symbol("right-arrow"),
        ]))
        .expect("right fringe layout");
        assert_eq!(layout.side, DisplayFringeSide::Right);
        assert!(layout.bitmap.is_symbol_named("right-arrow"));
        assert!(layout.face.is_none());
    }

    #[test]
    fn parse_display_fringe_layout_rejects_non_fringe_and_missing_bitmap() {
        let _eval = neovm_core::emacs_core::Context::new();
        // Not a fringe head.
        assert!(
            parse_display_fringe_layout(&Value::list(vec![
                Value::symbol("space"),
                Value::keyword(":width"),
            ]))
            .is_none()
        );
        // Missing BITMAP.
        assert!(
            parse_display_fringe_layout(&Value::list(vec![Value::symbol("left-fringe")])).is_none()
        );
    }

    #[test]
    fn display_space_keys_match_gnu_keyword_domain() {
        let keys = [
            (DisplaySpaceKey::Width, ":width"),
            (DisplaySpaceKey::RelativeWidth, ":relative-width"),
            (DisplaySpaceKey::AlignTo, ":align-to"),
            (DisplaySpaceKey::Height, ":height"),
            (DisplaySpaceKey::RelativeHeight, ":relative-height"),
            (DisplaySpaceKey::Ascent, ":ascent"),
        ];

        for (key, keyword) in keys {
            assert_eq!(key.keyword(), keyword);
            assert_eq!(DisplaySpaceKey::from_keyword(keyword), Some(key));
            assert_eq!(DisplaySpaceKey::from_lisp_value(key.value()), Some(key));
        }

        assert_eq!(DisplaySpaceKey::from_keyword("width"), None);
        assert_eq!(DisplaySpaceKey::from_keyword(":foreground"), None);
        assert_eq!(
            DisplaySpaceKey::from_lisp_value(Value::symbol("width")),
            None
        );
    }

    #[test]
    fn display_media_keys_match_lisp_keyword_domain() {
        let keys = [
            (DisplayMediaKey::File, ":file"),
            (DisplayMediaKey::Uri, ":uri"),
            (DisplayMediaKey::Width, ":width"),
            (DisplayMediaKey::Height, ":height"),
            (DisplayMediaKey::Loop, ":loop"),
            (DisplayMediaKey::LoopCount, ":loop-count"),
            (DisplayMediaKey::Autoplay, ":autoplay"),
            (DisplayMediaKey::Xwidget, ":xwidget"),
        ];

        for (key, name) in keys {
            assert_eq!(
                DisplayMediaKey::from_lisp_value(Value::symbol(name)),
                Some(key)
            );
            let serialized: &'static str = key.into();
            assert_eq!(serialized, name);
        }

        assert_eq!(
            DisplayMediaKey::from_lisp_value(Value::symbol("width")),
            None
        );
        assert_eq!(
            DisplayMediaKey::from_lisp_value(Value::symbol(":foreground")),
            None
        );
    }
}
