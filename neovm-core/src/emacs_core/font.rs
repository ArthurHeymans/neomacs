//! Font and face builtins for the Elisp interpreter.
//!
//! Font builtins:
//! - `fontp`, `font-spec`, `font-get`, `font-put`, `list-fonts`, `find-font`,
//!   `clear-font-cache`, `font-family-list`, `font-xlfd-name`
//!
//! Face builtins:
//! - `internal-make-lisp-face`, `internal-lisp-face-p`, `internal-copy-lisp-face`,
//!   `internal-set-lisp-face-attribute`, `internal-get-lisp-face-attribute`,
//!   `internal-merge-in-global-face`, `face-attribute-relative-p`,
//!   `merge-face-attribute`, `face-list`, `color-defined-p`, `color-values`,
//!   `defined-colors`, `face-id`, `face-font`, `internal-face-x-get-resource`,
//!   `internal-set-font-selection-order`,
//!   `internal-set-alternative-font-family-alist`,
//!   `internal-set-alternative-font-registry-alist`

use crate::emacs_core::error::LispCondition;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{OnceLock, RwLock};

use num_enum::{IntoPrimitive, TryFromPrimitive};
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};

use super::error::{EvalResult, Flow, signal};
use super::intern::{intern, resolve_sym};
use super::value::*;
use crate::buffer::{Buffer, CharPos0, EmacsBytePos, LispCharPos1};
use crate::emacs_core::SymId;
use crate::face::{
    BoxStyle, Face as RuntimeFace, FaceHeight, FaceRemapping, FontSlant, FontWeight, FontWidth,
    LFACE_ATTRS, LFACE_VECTOR_SIZE, LFaceAttr, UnderlineStyle,
};
use crate::heap_types::LispString;
use crate::tagged::header::store_value_atomic;
use crate::window::{FRAME_ID_BASE, FrameId, FrameManager, FrameParam, WindowId};

type AlternativeFontFamilyAlist = Vec<(SymId, Vec<SymId>)>;
type AlternativeFontRegistryAlist = Vec<(LispString, Vec<LispString>)>;

const FONT_WEIGHT_STYLE_TABLE: &[(i64, &[&str])] = &[
    (0, &["thin"]),
    (
        40,
        &["ultra-light", "ultralight", "extra-light", "extralight"],
    ),
    (50, &["light"]),
    (55, &["semi-light", "semilight", "demilight"]),
    (80, &["regular", "normal", "unspecified", "book"]),
    (100, &["medium"]),
    (
        180,
        &["semi-bold", "semibold", "demibold", "demi-bold", "demi"],
    ),
    (200, &["bold"]),
    (205, &["extra-bold", "extrabold", "ultra-bold", "ultrabold"]),
    (210, &["black", "heavy"]),
    (250, &["ultra-heavy", "ultraheavy"]),
];

const FONT_SLANT_STYLE_TABLE: &[(i64, &[&str])] = &[
    (0, &["reverse-oblique", "ro"]),
    (10, &["reverse-italic", "ri"]),
    (100, &["normal", "r", "unspecified"]),
    (200, &["italic", "i", "ot"]),
    (210, &["oblique", "o"]),
];

const FONT_WIDTH_STYLE_TABLE: &[(i64, &[&str])] = &[
    (50, &["ultra-condensed", "ultracondensed"]),
    (63, &["extra-condensed", "extracondensed"]),
    (75, &["condensed", "compressed", "narrow"]),
    (87, &["semi-condensed", "semicondensed", "demicondensed"]),
    (100, &["normal", "medium", "regular", "unspecified"]),
    (113, &["semi-expanded", "semiexpanded", "demiexpanded"]),
    (125, &["expanded"]),
    (150, &["extra-expanded", "extraexpanded"]),
    (200, &["ultra-expanded", "ultraexpanded", "wide"]),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
enum FontSpacing {
    #[strum(serialize = "p", serialize = "P")]
    Proportional = 0,
    #[strum(serialize = "d", serialize = "D")]
    Dual = 90,
    #[strum(serialize = "m", serialize = "M")]
    Mono = 100,
    #[strum(serialize = "c", serialize = "C")]
    Charcell = 110,
}

impl FontSpacing {
    const MAX_GNU_CODE: i64 = 110;

    fn from_symbol_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    #[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
    fn from_gnu_code(code: i64) -> Option<Self> {
        let code = i32::try_from(code).ok()?;
        Self::try_from(code).ok()
    }

    fn gnu_code(self) -> i32 {
        self.into()
    }

    fn xlfd_letter(self) -> &'static str {
        match self {
            Self::Proportional => "p",
            Self::Dual => "d",
            Self::Mono => "m",
            Self::Charcell => "c",
        }
    }

    fn xlfd_bucket_for_gnu_code(code: i64) -> Option<Self> {
        match code {
            0 => Some(Self::Proportional),
            1..=90 => Some(Self::Dual),
            91..=100 => Some(Self::Mono),
            101..=Self::MAX_GNU_CODE => Some(Self::Charcell),
            _ => None,
        }
    }

    fn xlfd_letter_for_gnu_code(code: i64) -> Option<&'static str> {
        Self::xlfd_bucket_for_gnu_code(code).map(Self::xlfd_letter)
    }
}

static ALTERNATIVE_FONT_FAMILY_ALIST: OnceLock<RwLock<AlternativeFontFamilyAlist>> =
    OnceLock::new();
static ALTERNATIVE_FONT_REGISTRY_ALIST: OnceLock<RwLock<AlternativeFontRegistryAlist>> =
    OnceLock::new();

fn alternative_font_family_alist() -> &'static RwLock<AlternativeFontFamilyAlist> {
    ALTERNATIVE_FONT_FAMILY_ALIST.get_or_init(|| RwLock::new(Vec::new()))
}

fn alternative_font_registry_alist() -> &'static RwLock<AlternativeFontRegistryAlist> {
    ALTERNATIVE_FONT_REGISTRY_ALIST.get_or_init(|| RwLock::new(Vec::new()))
}

fn font_style_table(entries: &[(i64, &[&str])]) -> Value {
    Value::vector(
        entries
            .iter()
            .map(|(numeric, names)| {
                let mut row = Vec::with_capacity(names.len() + 1);
                row.push(Value::fixnum(*numeric));
                row.extend(names.iter().map(|name| Value::symbol(*name)));
                Value::vector(row)
            })
            .collect(),
    )
}

pub(crate) fn init_font_vars(obarray: &mut super::symbol::Obarray) {
    for (name, value) in [
        (
            "font-weight-table",
            font_style_table(FONT_WEIGHT_STYLE_TABLE),
        ),
        ("font-slant-table", font_style_table(FONT_SLANT_STYLE_TABLE)),
        ("font-width-table", font_style_table(FONT_WIDTH_STYLE_TABLE)),
    ] {
        obarray.set_symbol_value(name, value);
        obarray.make_special(name);
        obarray.set_constant(name);
    }

    obarray.set_symbol_value("font-log", Value::T);
    obarray.make_special("font-log");
}

pub fn alternative_font_families(family: &str) -> Vec<String> {
    let lookup = family.trim();
    if lookup.is_empty() {
        return Vec::new();
    }

    let Ok(alist) = alternative_font_family_alist().read() else {
        return vec![lookup.to_string()];
    };

    alist
        .iter()
        .find_map(|(name, families)| {
            // Issue #131: compare/return font-family names over their real Emacs
            // bytes (resolve_sym_lisp_string), so raw-unibyte families are not
            // confused with the PUA-sentinel storage form.
            crate::emacs_core::intern::resolve_sym_lisp_string(*name)
                .as_bytes()
                .eq_ignore_ascii_case(lookup.as_bytes())
                .then(|| {
                    families
                        .iter()
                        .map(|sym| {
                            crate::emacs_core::emacs_char::to_utf8_lossy(
                                crate::emacs_core::intern::resolve_sym_lisp_string(*sym).as_bytes(),
                            )
                        })
                        .collect()
                })
        })
        .unwrap_or_else(|| vec![lookup.to_string()])
}

pub fn alternative_font_registries(registry: &str) -> Vec<String> {
    let lookup = registry.trim();
    if lookup.is_empty() {
        return Vec::new();
    }

    let Ok(alist) = alternative_font_registry_alist().read() else {
        return vec![lookup.to_ascii_lowercase()];
    };

    alist
        .iter()
        .find_map(|(name, registries)| {
            name.as_bytes()
                .eq_ignore_ascii_case(lookup.as_bytes())
                .then(|| {
                    registries
                        .iter()
                        .map(|text| {
                            // Issue #131: font registry names are ASCII identifiers; render the
                            // string's Emacs bytes faithfully rather than via storage sentinels.
                            crate::emacs_core::emacs_char::to_utf8_lossy(text.as_bytes())
                        })
                        .collect()
                })
        })
        .unwrap_or_else(|| vec![lookup.to_ascii_lowercase()])
}

// ---------------------------------------------------------------------------
// Argument helpers (local to this module)
// ---------------------------------------------------------------------------

fn expect_args(name: &str, args: &[Value], n: usize) -> Result<(), Flow> {
    if args.len() != n {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_min_args(name: &str, args: &[Value], min: usize) -> Result<(), Flow> {
    if args.len() < min {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_max_args(name: &str, args: &[Value], max: usize) -> Result<(), Flow> {
    if args.len() > max {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn live_frame_designator_in_state(frames: &FrameManager, value: &Value) -> bool {
    match value.kind() {
        ValueKind::Fixnum(id) if id >= 0 => frames.get(FrameId(id as u64)).is_some(),
        ValueKind::Veclike(VecLikeType::Frame) => {
            frames.get(FrameId(value.as_frame_id().unwrap())).is_some()
        }
        _ => false,
    }
}

fn frame_id_from_designator(value: &Value) -> Option<FrameId> {
    match value.kind() {
        ValueKind::Fixnum(id) if id >= 0 => Some(FrameId(id as u64)),
        ValueKind::Veclike(VecLikeType::Frame) => Some(FrameId(value.as_frame_id().unwrap())),
        _ => None,
    }
}

fn font_string_text(value: &Value) -> Option<String> {
    // Issue #131: read the value's real Emacs bytes (lossy UTF-8 view) rather than
    // the PUA-sentinel storage form. Font/color/property names are ASCII, where
    // this is exact; raw-byte family names are interned faithfully elsewhere.
    value
        .as_lisp_string()
        .map(|ls| crate::emacs_core::emacs_char::to_utf8_lossy(ls.as_bytes()))
}

fn font_value_text(value: &Value) -> Option<String> {
    match value.kind() {
        ValueKind::String => font_string_text(value),
        ValueKind::Symbol(id) => Some(resolve_sym(id).to_owned()),
        _ => None,
    }
}

fn font_value_text_lisp_string(value: &Value) -> Option<LispString> {
    match value.kind() {
        ValueKind::String => value.as_lisp_string().cloned(),
        ValueKind::Symbol(id) => Some(LispString::from_utf8(resolve_sym(id))),
        _ => None,
    }
}

struct LiveFrameFontResolution {
    font_value: Value,
    realized: Option<super::eval::ResolvedFrameFont>,
}

fn face_from_named_font_string(name: &str) -> Option<RuntimeFace> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut face = RuntimeFace::new("default");

    if !trimmed.starts_with('-') {
        if let Some((family, size)) = trimmed.rsplit_once('-')
            && !family.trim().is_empty()
            && size.chars().all(|ch| ch.is_ascii_digit())
            && let Ok(points) = size.parse::<i32>()
            && points > 0
        {
            face.family = Some(Value::string(family.trim().to_string()));
            face.height = Some(FaceHeight::Absolute(points * 10));
            return Some(face);
        }
        face.family = Some(Value::string(trimmed.to_string()));
        return Some(face);
    }

    let fields = trimmed.split('-').collect::<Vec<_>>();
    if fields.len() < 12 {
        return None;
    }

    let foundry = fields[1];
    let family = fields[2];
    let weight = fields[3];
    let slant = fields[4];
    let set_width = fields[5];
    let pixel = fields[7];

    if foundry != "*" && !foundry.is_empty() {
        face.foundry = Some(Value::string(foundry.to_string()));
    }
    if family != "*" && !family.is_empty() {
        face.family = Some(Value::string(family.to_string()));
    }
    if let Some(parsed_weight) = FontWeight::from_symbol(weight) {
        face.weight = Some(parsed_weight);
    }
    face.slant = match slant {
        "i" | "italic" => Some(FontSlant::Italic),
        "o" | "oblique" => Some(FontSlant::Oblique),
        "ri" | "reverse-italic" => Some(FontSlant::ReverseItalic),
        "ro" | "reverse-oblique" => Some(FontSlant::ReverseOblique),
        "r" | "normal" | "*" => Some(FontSlant::Normal),
        _ => None,
    };
    face.width = match set_width {
        "normal" | "*" => Some(FontWidth::Normal),
        other => FontWidth::from_symbol(other),
    };
    if pixel.chars().all(|ch| ch.is_ascii_digit())
        && let Ok(size_px) = pixel.parse::<i32>()
        && size_px > 0
    {
        face.height = Some(FaceHeight::Absolute(size_px * 10));
    }

    Some(face)
}

fn face_from_font_value(value: &Value) -> Option<RuntimeFace> {
    if let Some(text) = font_value_text(value) {
        return face_from_named_font_string(&text);
    }
    if !is_font(value) {
        return None;
    }

    let font_spec = is_font_spec(value);
    let elems = value.as_vector_data().unwrap().clone();
    let mut face = RuntimeFace::new("default");

    face.family = font_vector_get_flexible(&elems, "family")
        .and_then(|value| font_value_text(&value))
        .map(Value::string);
    face.foundry = font_vector_get_flexible(&elems, "foundry")
        .and_then(|value| font_value_text(&value))
        .map(Value::string);
    face.weight = font_vector_get_flexible(&elems, "weight").and_then(font_weight_from_value);
    face.slant = font_vector_get_flexible(&elems, "slant").and_then(font_slant_from_value);
    face.width = font_vector_get_flexible(&elems, "width").and_then(|value| match value.kind() {
        ValueKind::Symbol(id) => FontWidth::from_symbol(resolve_sym(id)),
        _ => None,
    });
    face.height = if let Some(value) = font_vector_get_flexible(&elems, "height") {
        face_height_from_value(value)
    } else if let Some(value) = font_vector_get_flexible(&elems, "size") {
        if font_spec {
            font_spec_size_to_face_height(value).and_then(face_height_from_value)
        } else {
            face_height_from_value(value)
        }
    } else {
        None
    };

    Some(face)
}

fn face_height_from_value(value: Value) -> Option<FaceHeight> {
    match value.kind() {
        ValueKind::Fixnum(n) if n > 0 => Some(FaceHeight::Absolute(n as i32)),
        ValueKind::Float if value.xfloat() > 0.0 => Some(FaceHeight::Relative(value.xfloat())),
        _ => None,
    }
}

fn build_frame_font_object_from_resolution(
    requested_face: &RuntimeFace,
    resolved: &super::eval::ResolvedFrameFont,
) -> Value {
    let mut selected = requested_face.clone();
    selected.family = Some(Value::heap_string(resolved.family.clone()));
    selected.foundry = resolved
        .foundry
        .clone()
        .map(Value::heap_string)
        .or(requested_face.foundry);
    selected.weight = Some(resolved.weight);
    selected.slant = Some(resolved.slant);
    selected.width = Some(resolved.width);
    selected.height = match requested_face.height {
        Some(FaceHeight::Absolute(height)) => Some(FaceHeight::Absolute(height)),
        Some(FaceHeight::Relative(scale)) => Some(FaceHeight::Relative(scale)),
        None => Some(FaceHeight::Absolute(resolved.height_tenths)),
    };

    build_font_object(&selected)
}

fn resolve_live_frame_font_request(
    eval: &mut super::eval::Context,
    frame_id: FrameId,
    requested: &Value,
) -> LiveFrameFontResolution {
    resolve_live_frame_font_request_in_state(
        &eval.frames,
        &mut eval.display_host,
        frame_id,
        requested,
    )
}

fn resolve_live_frame_font_request_in_state(
    frames: &FrameManager,
    display_host: &mut Option<Box<dyn super::eval::DisplayHost>>,
    frame_id: FrameId,
    requested: &Value,
) -> LiveFrameFontResolution {
    if is_font_object(requested) {
        return LiveFrameFontResolution {
            font_value: *requested,
            realized: None,
        };
    }

    if let Some(frame) = frames.get(frame_id)
        && font_value_matches_frame_font_parameter(frame, requested)
        && let Some(font_value) = frame.parameter("font-parameter")
        && is_font(&font_value)
    {
        return LiveFrameFontResolution {
            font_value,
            realized: None,
        };
    }

    let Some(requested_face) = face_from_font_value(requested) else {
        return LiveFrameFontResolution {
            font_value: *requested,
            realized: None,
        };
    };

    let realized = display_host
        .as_mut()
        .and_then(|host| {
            host.resolve_frame_font(frame_id, requested_face.clone())
                .ok()
        })
        .flatten();
    let font_value = realized
        .as_ref()
        .map(|resolved| build_frame_font_object_from_resolution(&requested_face, resolved))
        .unwrap_or_else(|| build_font_object(&requested_face));

    LiveFrameFontResolution {
        font_value,
        realized,
    }
}

fn sync_live_frame_font_state(
    eval: &mut super::eval::Context,
    frame_id: FrameId,
    requested: &Value,
    resolution: &LiveFrameFontResolution,
) {
    sync_live_frame_font_state_in_state(
        &mut eval.frames,
        &mut eval.display_host,
        frame_id,
        requested,
        resolution,
    );
}

fn sync_live_frame_font_state_in_state(
    frames: &mut FrameManager,
    display_host: &mut Option<Box<dyn super::eval::DisplayHost>>,
    frame_id: FrameId,
    requested: &Value,
    resolution: &LiveFrameFontResolution,
) {
    let Some(frame) = frames.get_mut(frame_id) else {
        return;
    };

    let public_font_name = if requested.is_string() {
        *requested
    } else {
        font_name_value(&resolution.font_value).unwrap_or(*requested)
    };

    frame.set_known_parameter(FrameParam::Font, public_font_name);
    frame.set_parameter(Value::symbol("font-parameter"), resolution.font_value);

    let mut geometry_hints = None;
    if let Some(realized) = &resolution.realized {
        frame.font_pixel_size = realized.font_size_px.max(1.0);
        frame.char_width = realized.char_width.max(1.0);
        frame.char_height = realized.line_height.max(1.0);
        let is_top_level_gui_frame =
            frame.effective_window_system().is_some() && frame.parent_frame.as_frame_id().is_none();
        if is_top_level_gui_frame {
            frame.defer_next_gui_parameter_resize();
            geometry_hints = Some(frame.gui_geometry_hints());
        }
    }

    if let Some(geometry_hints) = geometry_hints
        && let Some(host) = display_host.as_mut()
        && let Err(err) = host.set_gui_frame_geometry_hints(frame_id, geometry_hints)
    {
        tracing::warn!(
            "failed to update live frame geometry hints after font change for frame 0x{:x}: {}",
            frame_id.0,
            err
        );
    }
}

pub(crate) fn sync_live_frame_font_parameter_in_state(
    frames: &mut FrameManager,
    display_host: &mut Option<Box<dyn super::eval::DisplayHost>>,
    frame_id: FrameId,
    requested: Value,
) {
    let resolution =
        resolve_live_frame_font_request_in_state(frames, display_host, frame_id, &requested);
    sync_live_frame_font_state_in_state(frames, display_host, frame_id, &requested, &resolution);
}

fn default_face_font_attr_affects_frame_font(attr: LFaceAttr) -> bool {
    matches!(
        attr,
        LFaceAttr::Font
            | LFaceAttr::Family
            | LFaceAttr::Foundry
            | LFaceAttr::Height
            | LFaceAttr::Weight
            | LFaceAttr::Slant
            | LFaceAttr::Width
    )
}

fn sync_live_default_face_font_state(eval: &mut super::eval::Context, frame_id: FrameId) {
    if eval
        .frames
        .get(frame_id)
        .is_none_or(|frame| frame.effective_window_system().is_none())
    {
        return;
    }

    let Some(vector) = lookup_frame_lisp_face_vector(eval, frame_id, "default") else {
        return;
    };
    let requested_face = runtime_face_from_lisp_face_vector("default", vector);
    let realized = eval
        .display_host
        .as_mut()
        .and_then(|host| {
            host.resolve_frame_font(frame_id, requested_face.clone())
                .ok()
        })
        .flatten();
    let font_value = realized
        .as_ref()
        .map(|resolved| build_frame_font_object_from_resolution(&requested_face, resolved))
        .unwrap_or_else(|| build_font_object(&requested_face));
    let resolution = LiveFrameFontResolution {
        font_value,
        realized,
    };

    sync_live_frame_font_state(eval, frame_id, &font_value, &resolution);
}

fn expect_optional_frame_designator_in_state(
    frames: &FrameManager,
    value: Option<&Value>,
) -> Result<(), Flow> {
    if let Some(frame) = value
        && !frame.is_nil()
        && !live_frame_designator_in_state(frames, frame)
    {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), *frame],
        ));
    }
    Ok(())
}

fn frame_device_designator_p(value: &Value) -> bool {
    match value.kind() {
        ValueKind::Fixnum(id) => id >= FRAME_ID_BASE as i64,
        ValueKind::Veclike(VecLikeType::Frame) => value.as_frame_id().unwrap() >= FRAME_ID_BASE,
        _ => false,
    }
}

fn live_frame_id_for_face_update(
    eval: &mut super::eval::Context,
    frame: Option<&Value>,
) -> Result<Option<FrameId>, Flow> {
    match frame {
        None => Ok(Some(super::window_cmds::ensure_selected_frame_id(eval))),
        Some(v) if v.is_nil() || v.as_fixnum() == Some(0) => {
            Ok(Some(super::window_cmds::ensure_selected_frame_id(eval)))
        }
        Some(v) if v.is_t() => Ok(None),
        Some(value) if live_frame_designator_in_state(&eval.frames, value) => Ok(Some(
            frame_id_from_designator(value)
                .expect("live frame designator should decode to frame id"),
        )),
        Some(other) => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), *other],
        )),
    }
}

fn set_frame_face_color_from_frame_parameter(
    eval: &mut super::eval::Context,
    frame_id: FrameId,
    face_name: &str,
    attr: LFaceAttr,
    value: Value,
) -> Result<(), crate::emacs_core::error::Flow> {
    builtin_internal_set_lisp_face_attribute(
        eval,
        vec![
            Value::symbol(face_name),
            Value::symbol(attr.keyword()),
            value,
            Value::make_frame(frame_id.0),
        ],
    )?;
    Ok(())
}

pub(crate) fn update_face_from_frame_parameter(
    eval: &mut super::eval::Context,
    frame_id: FrameId,
    param: FrameParam,
    new_value: Value,
) -> Result<(), crate::emacs_core::error::Flow> {
    match param {
        FrameParam::ForegroundColor => {
            set_frame_face_color_from_frame_parameter(
                eval,
                frame_id,
                "default",
                LFaceAttr::Foreground,
                new_value,
            )?;
        }
        FrameParam::BackgroundColor => {
            if let Some(function) = eval.obarray().symbol_function("frame-set-background-mode") {
                let _ = eval.apply(function, vec![Value::make_frame(frame_id.0)])?;
            }
            set_frame_face_color_from_frame_parameter(
                eval,
                frame_id,
                "default",
                LFaceAttr::Background,
                new_value,
            )?;
        }
        _ => {}
    }
    Ok(())
}

/// Seed the selected frame's authoritative `default` Lisp face specification
/// from its `font-parameter` without mutating Lisp override state.
///
/// GNU keeps the defface for `default` empty and realizes the actual frame
/// font through the face subsystem in C.  Redisplay later derives the runtime
/// face table from this frame-local specification.
pub fn seed_live_frame_default_face_from_font_parameter(
    eval: &mut super::eval::Context,
    frame_id: FrameId,
) {
    let Some(font_value) = eval
        .frames
        .get(frame_id)
        .and_then(|frame| frame.parameter("font-parameter"))
    else {
        return;
    };

    let Some(vector) =
        ensure_frame_lisp_face_vector(eval, frame_id, "default", FrameFaceInitial::SelectedBase)
    else {
        return;
    };
    for (attr_name, attr_value) in derived_face_attrs_from_font_value(&font_value) {
        set_lisp_face_vector_attr(vector, attr_name, attr_value);
    }
    eval.face_change_count += 1;
}

// ---------------------------------------------------------------------------
// Font-spec helpers
// ---------------------------------------------------------------------------

/// The tag keyword used to identify font-spec vectors: `:font-spec`.
const FONT_SPEC_TAG: &str = "font-spec";
const FONT_ENTITY_TAG: &str = "font-entity";
const FONT_OBJECT_TAG: &str = "font-object";

fn is_tagged_font_vector(val: &Value, tag: &str) -> bool {
    match val.kind() {
        ValueKind::Veclike(VecLikeType::Vector) => {
            let elems = val.as_vector_data().unwrap().clone();
            elems
                .first()
                .and_then(|v| v.as_symbol_name())
                .is_some_and(|name| name.trim_start_matches(':') == tag)
        }
        _ => false,
    }
}

/// Check whether a Value is a font-spec (a vector whose first element is
/// the tag symbol/keyword `font-spec` / `:font-spec`.
fn is_font_spec(val: &Value) -> bool {
    is_tagged_font_vector(val, FONT_SPEC_TAG)
}

/// Check whether a value is represented as a font-object vector.
fn is_font_object(val: &Value) -> bool {
    is_tagged_font_vector(val, FONT_OBJECT_TAG)
}

/// Check whether a value is represented as a font-entity vector.
fn is_font_entity(val: &Value) -> bool {
    is_tagged_font_vector(val, FONT_ENTITY_TAG)
}

fn is_font(val: &Value) -> bool {
    is_font_spec(val) || is_font_entity(val) || is_font_object(val)
}

/// The `type-of`/`cl-type-of` symbol for a font value, mirroring GNU's
/// `PVEC_FONT` size discrimination (`font-spec` < `font-entity` <
/// `font-object`, src/font.h FONT_*_MAX). Neomacs represents fonts as
/// tag-keyword vectors, so the type predicates must recognize them
/// explicitly. `None` for non-font values.
pub(crate) fn font_value_type_symbol(val: &Value) -> Option<&'static str> {
    if is_font_spec(val) {
        Some(FONT_SPEC_TAG)
    } else if is_font_entity(val) {
        Some(FONT_ENTITY_TAG)
    } else if is_font_object(val) {
        Some(FONT_OBJECT_TAG)
    } else {
        None
    }
}

/// Extract a property from a tagged font vector.
///
/// Property lookup is strict: keys only match if they are exactly equal to
/// `prop` (keyword vs symbol distinction is preserved).
fn font_vector_get(vec_elems: &[Value], prop: &Value) -> Value {
    // Skip the tag at index 0; scan remaining pairs.
    let mut i = 1;
    while i + 1 < vec_elems.len() {
        if vec_elems[i] == *prop {
            return vec_elems[i + 1];
        }
        i += 2;
    }
    Value::NIL
}

/// Get a property from a tagged font vector while accepting both `family` and `:family`
/// style keys, and both keyword and symbol keys.
fn font_vector_get_flexible(vec_elems: &[Value], prop: &str) -> Option<Value> {
    let prop_norm = prop.trim_start_matches(':');
    let mut i = 1;
    while i + 1 < vec_elems.len() {
        let key = &vec_elems[i];
        let key_text = match key.kind() {
            ValueKind::Symbol(k) => resolve_sym(k),
            _ => {
                i += 2;
                continue;
            }
        };
        let key_norm = key_text.trim_start_matches(':');
        if key_norm == prop_norm {
            return Some(vec_elems[i + 1]);
        }
        i += 2;
    }
    None
}

fn font_spec_field_to_string(value: &Value) -> String {
    match value.kind() {
        ValueKind::String => font_string_text(value).expect("checked string"),
        ValueKind::Symbol(id) => resolve_sym(id).to_owned(),
        _ => "*".to_string(),
    }
}

fn xlfd_size_field(size_val: &Value) -> Option<String> {
    match size_val.kind() {
        ValueKind::Fixnum(size) => {
            if size > 0 {
                Some(format!("{}-*", size))
            } else {
                Some("*-*".to_string())
            }
        }
        ValueKind::Float => {
            let f = size_val.xfloat();
            let scaled = f * 10.0;
            if scaled.is_finite() {
                Some(format!("*-{}", scaled.round() as i64))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn fold_xlfd_wildcards(mut name: String) -> String {
    while let Some(pos) = name.find("-*-*") {
        name.replace_range(pos + 1..pos + 3, "");
    }
    name
}

fn normalize_registry_field(value: &Option<Value>) -> String {
    match value {
        None => "*-*".to_string(),
        Some(v) => match v.kind() {
            ValueKind::String => {
                let s = font_string_text(v).expect("checked string");
                if !s.contains('-') {
                    format!("{}-*", s)
                } else {
                    s
                }
            }
            ValueKind::Symbol(id) => {
                let s = resolve_sym(id);
                if !s.contains('-') {
                    format!("{}-*", s)
                } else {
                    s.to_owned()
                }
            }
            _ => "*-*".to_string(),
        },
    }
}

fn sanitize_style_field(value: &Value) -> String {
    match value.kind() {
        ValueKind::Symbol(id) => resolve_sym(id)
            .chars()
            .filter(|ch| *ch != '-' && *ch != '?' && *ch != ',' && *ch != '"')
            .collect(),
        ValueKind::String => {
            let s = font_string_text(value).expect("checked string");
            s.chars()
                .filter(|ch| *ch != '-' && *ch != '?' && *ch != ',' && *ch != '"')
                .collect()
        }
        _ => "*".to_string(),
    }
}

fn spacing_field(value: Option<&Value>) -> String {
    match value {
        None => "*".to_string(),
        Some(v) if v.is_fixnum() => {
            let spacing = v.as_fixnum().unwrap();
            FontSpacing::xlfd_letter_for_gnu_code(spacing)
                .unwrap_or("*")
                .to_string()
        }
        Some(v) => sanitize_style_field(v),
    }
}

fn avg_width_field(value: Option<&Value>) -> String {
    match value {
        Some(v) => match v.kind() {
            ValueKind::Fixnum(n) => n.to_string(),
            ValueKind::String => font_string_text(v).expect("checked string"),
            ValueKind::Symbol(id) => resolve_sym(id).to_owned(),
            _ => "*".to_string(),
        },
        None => "*".to_string(),
    }
}

fn xlfd_pixel_field(size: Option<&Value>) -> String {
    match size {
        Some(value) => xlfd_size_field(value).unwrap_or("*-*".to_string()),
        None => "*-*".to_string(),
    }
}

fn xlfd_resolution_field(dpi: Option<&Value>) -> String {
    match dpi {
        Some(v) if v.is_fixnum() => {
            let size = v.as_fixnum().unwrap();
            format!("{}-{}", size, size)
        }
        _ => "*-*".to_string(),
    }
}

fn xlfd_fields_from_font_vector(
    v: &[Value],
) -> (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
) {
    let foundry = font_vector_get_flexible(v, "foundry")
        .map(|value| font_spec_field_to_string(&value))
        .unwrap_or_else(|| "*".to_string());
    let family = font_vector_get_flexible(v, "family")
        .map(|value| font_spec_field_to_string(&value))
        .unwrap_or_else(|| "*".to_string());
    let weight = font_vector_get_flexible(v, "weight")
        .map(|value| sanitize_style_field(&value))
        .unwrap_or_else(|| "*".to_string());
    let slant = font_vector_get_flexible(v, "slant")
        .map(|value| sanitize_style_field(&value))
        .unwrap_or_else(|| "*".to_string());
    let set_width = font_vector_get_flexible(v, "set-width")
        .or_else(|| font_vector_get_flexible(v, "setwidth"))
        .or_else(|| font_vector_get_flexible(v, "width"))
        .map(|value| font_spec_field_to_string(&value))
        .unwrap_or_else(|| "*".to_string());
    let adstyle = font_vector_get_flexible(v, "adstyle")
        .map(|value| font_spec_field_to_string(&value))
        .unwrap_or_else(|| "*".to_string());

    let size = font_vector_get_flexible(v, "size");
    let dpi = font_vector_get_flexible(v, "dpi");
    let spacing = font_vector_get_flexible(v, "spacing");
    let avg_width = font_vector_get_flexible(v, "average_width")
        .or_else(|| font_vector_get_flexible(v, "avg_width"))
        .or_else(|| font_vector_get_flexible(v, "avg-width"));
    let registry = font_vector_get_flexible(v, "registry");

    let pixel = xlfd_pixel_field(size.as_ref());
    let resx = xlfd_resolution_field(dpi.as_ref());
    let spacing = spacing_field(spacing.as_ref());
    let avg_width = avg_width_field(avg_width.as_ref());
    let registry = normalize_registry_field(&registry);

    (
        foundry, family, weight, slant, set_width, adstyle, pixel, resx, spacing, avg_width,
        registry,
    )
}

/// Set (or add) a property in a font-spec in place.
fn font_spec_put(vec_elems: &mut Vec<Value>, prop: &Value, val: &Value) -> EvalResult {
    let normalized = normalize_font_prop_value(prop, val)?;
    let mut i = 1;
    while i + 1 < vec_elems.len() {
        if vec_elems[i] == *prop {
            vec_elems[i + 1] = normalized;
            return Ok(normalized);
        }
        i += 2;
    }
    vec_elems.push(*prop);
    vec_elems.push(normalized);
    Ok(normalized)
}

fn invalid_font_property(prop: &Value, val: &Value) -> Flow {
    signal(
        "error",
        vec![
            Value::string("invalid font property"),
            Value::cons(*prop, *val),
        ],
    )
}

fn font_style_table_for_key(key: &str) -> Option<&'static [(i64, &'static [&'static str])]> {
    match key {
        "weight" => Some(FONT_WEIGHT_STYLE_TABLE),
        "slant" => Some(FONT_SLANT_STYLE_TABLE),
        "width" => Some(FONT_WIDTH_STYLE_TABLE),
        _ => None,
    }
}

/// GNU `font_style_symbolic (font, prop, for_face=true)` (font.c:471-490):
/// canonicalize a stored weight/slant/width symbol to the first ("preferred")
/// name of its style-table row -- the value behind `AREF (elt, 1)`, i.e.
/// `names[0]`. This is what `Ffont_face_attributes` uses (heavy -> black,
/// ultra-bold -> extra-bold, normal -> regular). `font-get`/`font-spec`
/// storage keep the matched alias verbatim (`for_face=false`), so this is
/// applied only at the face-read boundary. Returns `None` for a symbol that is
/// not a known style word.
fn font_style_canonical_for_face(key: &str, name: &str) -> Option<&'static str> {
    let table = font_style_table_for_key(key)?;
    table
        .iter()
        .find(|(_, names)| names.iter().any(|alias| alias.eq_ignore_ascii_case(name)))
        .and_then(|(_, names)| names.first().copied())
}

fn font_style_symbol_from_gnu_code(
    table: &'static [(i64, &'static [&'static str])],
    code: i64,
) -> Option<&'static str> {
    let code = u16::try_from(code).ok()?;
    let numeric = i64::from(code >> 8);
    let row = usize::from((code >> 4) & 0x0f);
    let alias = usize::from(code & 0x0f);
    let (row_numeric, names) = table.get(row)?;
    if *row_numeric == numeric {
        names.get(alias).copied()
    } else {
        None
    }
}

fn font_style_symbol_from_name(
    table: &'static [(i64, &'static [&'static str])],
    name: &str,
) -> Option<&'static str> {
    table
        .iter()
        .flat_map(|(_, names)| names.iter().copied())
        .find(|candidate| *candidate == name)
        .or_else(|| {
            table
                .iter()
                .flat_map(|(_, names)| names.iter().copied())
                .find(|candidate| candidate.eq_ignore_ascii_case(name))
        })
}

fn validate_font_style_prop(key: &str, prop: &Value, val: &Value) -> EvalResult {
    if val.is_nil() {
        return Ok(*val);
    }
    match val.kind() {
        ValueKind::Symbol(id) => {
            let name = resolve_sym(id);
            font_style_table_for_key(key)
                .and_then(|table| font_style_symbol_from_name(table, name))
                .map(Value::symbol)
                .ok_or_else(|| invalid_font_property(prop, val))
        }
        ValueKind::Fixnum(n) => font_style_table_for_key(key)
            .and_then(|table| font_style_symbol_from_gnu_code(table, n))
            .map(Value::symbol)
            .ok_or_else(|| invalid_font_property(prop, val)),
        _ => Err(invalid_font_property(prop, val)),
    }
}

fn validate_non_negative_font_prop(prop: &Value, val: &Value) -> EvalResult {
    if val.is_nil()
        || matches!(val.kind(), ValueKind::Fixnum(n) if n >= 0)
        || matches!(val.kind(), ValueKind::Float if val.xfloat() >= 0.0)
    {
        Ok(*val)
    } else {
        Err(invalid_font_property(prop, val))
    }
}

fn validate_spacing_font_prop(prop: &Value, val: &Value) -> EvalResult {
    if val.is_nil() {
        return Ok(*val);
    }
    match val.kind() {
        ValueKind::Fixnum(n) if (0..=FontSpacing::MAX_GNU_CODE).contains(&n) => Ok(*val),
        ValueKind::Symbol(id) => FontSpacing::from_symbol_name(resolve_sym(id))
            .map(|spacing| Value::fixnum(i64::from(spacing.gnu_code())))
            .ok_or_else(|| invalid_font_property(prop, val)),
        _ => Err(invalid_font_property(prop, val)),
    }
}

fn normalize_font_prop_value(prop: &Value, val: &Value) -> EvalResult {
    let key = match prop.kind() {
        ValueKind::Symbol(id) => resolve_sym(id).trim_start_matches(':'),
        _ => return Ok(*val),
    };

    match key {
        "family" | "foundry" | "lang" | "adstyle" | "type" | "script" => match val.kind() {
            ValueKind::String => font_string_text(val)
                .map(|text| Value::from_sym_id(intern(&text)))
                .map(Ok)
                .unwrap_or(Ok(*val)),
            ValueKind::Symbol(_) | ValueKind::Nil => Ok(*val),
            _ => Err(invalid_font_property(prop, val)),
        },
        "registry" => match val.kind() {
            ValueKind::String => font_string_text(val)
                .map(|text| Value::from_sym_id(intern(&text.to_ascii_lowercase())))
                .map(Ok)
                .unwrap_or(Ok(*val)),
            ValueKind::Symbol(id) => Ok(Value::from_sym_id(intern(
                &resolve_sym(id).to_ascii_lowercase(),
            ))),
            ValueKind::Nil => Ok(*val),
            _ => Err(invalid_font_property(prop, val)),
        },
        "weight" | "slant" | "width" => validate_font_style_prop(key, prop, val),
        "size" | "dpi" | "avgwidth" | "average-width" | "avg-width" => {
            validate_non_negative_font_prop(prop, val)
        }
        "spacing" => validate_spacing_font_prop(prop, val),
        _ => Ok(*val),
    }
}

// ===========================================================================
// Font name parsing (fontconfig / XLFD)
//
// Ports GNU Emacs `font_parse_name` (src/font.c) which dispatches between
// `font_parse_xlfd` (names starting with '-' or containing '*'/'?') and
// `font_parse_fcname` (fontconfig "Family-Size:key=val" names).  The parsed
// properties are stored into a font-spec property vector using keyword keys,
// matching the layout produced by `font-spec`/`font-put`.
// ===========================================================================

/// Set a basic font-spec property (`:family`, `:size`, etc.) on a property
/// vector, replacing any existing entry.  Mirrors GNU's `ASET (font, IDX, val)`.
fn font_parse_set(elems: &mut Vec<Value>, key: &str, val: Value) {
    let prop = Value::keyword(key);
    let mut i = 1;
    while i + 1 < elems.len() {
        if elems[i]
            .as_symbol_name()
            .map(|name| name.trim_start_matches(':'))
            == Some(key)
        {
            elems[i + 1] = val;
            return;
        }
        i += 2;
    }
    elems.push(prop);
    elems.push(val);
}

/// Canonicalize and store a weight/slant/width style word the way GNU's
/// `FONT_SET_STYLE` does: look the word up in the style table and store the
/// canonical symbol (neomacs stores the symbol; `font-face-attributes` reads it
/// back directly, matching GNU's `font_style_symbolic`).
fn font_parse_set_style(elems: &mut Vec<Value>, key: &str, word: &str) {
    if let Some(name) =
        font_style_table_for_key(key).and_then(|table| font_style_symbol_from_name(table, word))
    {
        font_parse_set(elems, key, Value::symbol(name));
    }
}

/// Try to interpret a fontconfig property word as a weight, slant or spacing
/// keyword (the bare-word case from GNU `font_parse_fcname`).
fn font_parse_fcname_enum_word(elems: &mut Vec<Value>, word: &str) {
    match word {
        "thin" | "ultra-light" | "light" | "semi-light" | "book" | "medium" | "normal"
        | "semibold" | "demibold" | "bold" | "ultra-bold" | "black" | "heavy" | "ultra-heavy" => {
            font_parse_set_style(elems, "weight", word);
        }
        "roman" | "italic" | "oblique" => {
            font_parse_set_style(elems, "slant", word);
        }
        "charcell" => font_parse_set(elems, "spacing", Value::fixnum(110)),
        "mono" => font_parse_set(elems, "spacing", Value::fixnum(100)),
        "proportional" => font_parse_set(elems, "spacing", Value::fixnum(0)),
        _ => {}
    }
}

/// Store a `key=val` fontconfig property.  Recognized keys map to basic
/// font-spec slots; unknown keys are dropped (GNU would route them to the
/// font driver's `filter_properties`, which has no effect on a bare spec).
fn font_parse_fcname_keyval(elems: &mut Vec<Value>, key: &str, val: &str) {
    match key {
        "pixelsize" => {
            if let Ok(n) = val.parse::<i64>() {
                font_parse_set(elems, "size", Value::fixnum(n));
            }
        }
        "size" => {
            if let Ok(f) = val.parse::<f64>() {
                font_parse_set(elems, "size", Value::make_float(f));
            } else if let Ok(n) = val.parse::<i64>() {
                font_parse_set(elems, "size", Value::fixnum(n));
            }
        }
        "weight" | "slant" | "width" => font_parse_set_style(elems, key, val),
        "spacing" => {
            if let Some(spacing) = FontSpacing::from_symbol_name(val) {
                font_parse_set(
                    elems,
                    "spacing",
                    Value::fixnum(i64::from(spacing.gnu_code())),
                );
            } else if let Ok(n) = val.parse::<i64>() {
                font_parse_set(elems, "spacing", Value::fixnum(n));
            }
        }
        "foundry" | "family" | "adstyle" | "lang" | "script" => {
            font_parse_set(elems, key, Value::symbol(val));
        }
        "registry" => font_parse_set(elems, "registry", Value::symbol(val.to_ascii_lowercase())),
        "dpi" => {
            if let Ok(n) = val.parse::<i64>() {
                font_parse_set(elems, "dpi", Value::fixnum(n));
            }
        }
        _ => {}
    }
}

/// Port of GNU `font_parse_fcname` (src/font.c): parse a fontconfig-style name
/// such as `"Monospace-10"`, `"Family:weight=bold"`, or `"Family-12:bold"` into
/// font-spec properties.  Returns `false` on an empty name (GNU `-1`).
fn font_parse_fcname(elems: &mut Vec<Value>, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let bytes = name.as_bytes();
    let mut family_end: Option<usize> = None;
    let mut size_beg: Option<usize> = None;
    let mut props_beg: Option<usize> = None;

    // Scan forward for the first ':' (property data) or a '-NN[.NN]' size run.
    let mut p = 0;
    while p < bytes.len() {
        let c = bytes[p];
        if c == b'\\' && p + 1 < bytes.len() {
            p += 2;
            continue;
        } else if c == b':' {
            props_beg = Some(p);
            family_end = Some(p);
            break;
        } else if c == b'-' {
            // Everything up to the next ':' must be digits (and at most one '.').
            let mut decimal = false;
            let mut size_found = true;
            let mut q = p + 1;
            while q < bytes.len() && bytes[q] != b':' {
                let cq = bytes[q];
                if !cq.is_ascii_digit() {
                    if cq != b'.' || decimal {
                        size_found = false;
                        break;
                    }
                    decimal = true;
                }
                q += 1;
            }
            // GNU requires at least one char after '-' to count as a size.
            if size_found && q > p + 1 {
                family_end = Some(p);
                size_beg = Some(p + 1);
                break;
            }
        }
        p += 1;
    }

    let Some(family_end) = family_end else {
        // No size and no property data: a plain family name (possibly GTK-style
        // with trailing style words / size separated by spaces).
        return font_parse_fcname_plain(elems, name);
    };

    // Family.
    if family_end > 0 {
        let family = unescape_fcname(&name[..family_end]);
        font_parse_set(elems, "family", Value::symbol(&family));
    }

    // Point size (stored as a float, matching GNU `make_float`).
    if let Some(size_beg) = size_beg {
        // Read the numeric run starting at size_beg.
        let rest = &name[size_beg..];
        let end = rest.find(':').unwrap_or(rest.len());
        let size_str = &rest[..end];
        if let Ok(f) = size_str.parse::<f64>() {
            font_parse_set(elems, "size", Value::make_float(f));
        }
        // If a ':' follows the size, properties start there.
        if size_beg + end < bytes.len() && bytes[size_beg + end] == b':' {
            props_beg = Some(size_beg + end);
        }
    }

    // Parse ":KEY=VAL" / ":enumword" properties.
    if let Some(props_beg) = props_beg {
        for segment in name[props_beg..].split(':') {
            if segment.is_empty() {
                continue;
            }
            if let Some(eq) = segment.find('=') {
                let key = &segment[..eq];
                let val = &segment[eq + 1..];
                font_parse_fcname_keyval(elems, key, val);
            } else {
                font_parse_fcname_enum_word(elems, segment);
            }
        }
    }

    true
}

/// Strip fontconfig quoting backslashes from a family name.
fn unescape_fcname(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// GTK / plain fontconfig name with no size or property delimiters, e.g.
/// `"Monospace"`, `"DejaVu Sans Bold 12"`.  Ported from the `else` branch of
/// GNU `font_parse_fcname`: scan backwards for a numeric size, then for known
/// style words, the remainder being the family.
fn font_parse_fcname_plain(elems: &mut Vec<Value>, name: &str) -> bool {
    let bytes = name.as_bytes();
    let len = bytes.len();

    // Scan backwards for a trailing numeric size (preceded by a space or BOS).
    let mut p = len;
    let mut size: Option<f64> = None;
    {
        let mut i = len;
        while i > 0 && bytes[i - 1].is_ascii_digit() {
            i -= 1;
        }
        if i < len
            && (i == 0 || bytes[i - 1] == b' ')
            && let Ok(f) = name[i..].parse::<f64>()
        {
            size = Some(f);
            // Drop the size (and a preceding space) from the family scan.
            p = if i > 0 { i - 1 } else { i };
        }
    }

    // Scan backwards over space-separated words, recognizing style keywords.
    let mut weight: Option<&str> = None;
    let mut slant: Option<&str> = None;
    let mut width: Option<&str> = None;
    let mut family_end = p;
    while p > 0 {
        // Find the start of the current word.
        let mut q = p;
        while q > 0 {
            if q > 1 && bytes[q - 2] == b'\\' {
                q -= 1;
            } else if bytes[q - 1] == b' ' {
                break;
            }
            q -= 1;
        }
        let word = &name[q..p];
        let matched = match word {
            "Ultra-Light" => {
                weight.get_or_insert("ultra-light");
                true
            }
            "Light" => {
                weight.get_or_insert("light");
                true
            }
            "Book" => {
                weight.get_or_insert("book");
                true
            }
            "Medium" => {
                weight.get_or_insert("medium");
                true
            }
            "Semi-Bold" => {
                weight.get_or_insert("semi-bold");
                true
            }
            "Bold" => {
                weight.get_or_insert("bold");
                true
            }
            "Italic" => {
                slant.get_or_insert("italic");
                true
            }
            "Oblique" => {
                slant.get_or_insert("oblique");
                true
            }
            "Semi-Condensed" => {
                width.get_or_insert("semi-condensed");
                true
            }
            "Condensed" => {
                width.get_or_insert("condensed");
                true
            }
            _ => false,
        };
        if !matched {
            family_end = p;
            break;
        }
        // Move past the space before this word.
        p = if q > 0 { q - 1 } else { 0 };
        family_end = q;
        if q == 0 {
            break;
        }
    }

    if family_end > 0 {
        font_parse_set(
            elems,
            "family",
            Value::symbol(unescape_fcname(&name[..family_end])),
        );
    }
    if let Some(f) = size {
        font_parse_set(elems, "size", Value::make_float(f));
    }
    if let Some(w) = weight {
        font_parse_set_style(elems, "weight", w);
    }
    if let Some(s) = slant {
        font_parse_set_style(elems, "slant", s);
    }
    if let Some(w) = width {
        font_parse_set_style(elems, "width", w);
    }
    true
}

/// XLFD field indices (GNU `enum xlfd_field_index`).
const XLFD_FOUNDRY: usize = 0;
const XLFD_FAMILY: usize = 1;
const XLFD_WEIGHT: usize = 2;
const XLFD_SLANT: usize = 3;
const XLFD_SWIDTH: usize = 4;
const XLFD_ADSTYLE: usize = 5;
const XLFD_PIXEL: usize = 6;
const XLFD_POINT: usize = 7;
const XLFD_RESX: usize = 8;
const XLFD_RESY: usize = 9;
const XLFD_SPACING: usize = 10;
const XLFD_AVGWIDTH: usize = 11;
const XLFD_REGISTRY: usize = 12;
const XLFD_ENCODING: usize = 13;
const XLFD_LAST: usize = 14;

/// Port of GNU `font_parse_xlfd` (src/font.c): parse a hyphen-delimited XLFD
/// name such as `"-misc-fixed-medium-r-normal--13-120-..."`.  Only the
/// fully-specified (14-field) form is handled here, which covers the names
/// `font-spec :name` is given in practice.  Returns `false` on parse failure.
fn font_parse_xlfd(elems: &mut Vec<Value>, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Split into fields on '-'.  GNU treats a leading "*-" specially; for the
    // fully-specified form we simply split on '-'.
    let fields: Vec<&str> = name.split('-').collect();

    // A fully specified XLFD has a leading '-', so split() yields an empty
    // first element followed by exactly 14 fields.
    if fields.len() != XLFD_LAST + 1 || !fields[0].is_empty() {
        return false;
    }
    let f = &fields[1..]; // 14 fields, indices XLFD_FOUNDRY..XLFD_ENCODING

    let intern_field = |idx: usize| -> &str { f[idx] };

    // Foundry / family (interned as symbols).
    if !f[XLFD_FOUNDRY].is_empty() && f[XLFD_FOUNDRY] != "*" {
        font_parse_set(elems, "foundry", Value::symbol(f[XLFD_FOUNDRY]));
    }
    if !f[XLFD_FAMILY].is_empty() && f[XLFD_FAMILY] != "*" {
        font_parse_set(elems, "family", Value::symbol(f[XLFD_FAMILY]));
    }

    // Weight / slant / width style fields.
    for (xlfd_idx, key) in [
        (XLFD_WEIGHT, "weight"),
        (XLFD_SLANT, "slant"),
        (XLFD_SWIDTH, "width"),
    ] {
        let word = intern_field(xlfd_idx);
        if !word.is_empty() && word != "*" {
            font_parse_set_style(elems, key, word);
        }
    }

    // Adstyle: GNU stores the interned field unconditionally for a fully
    // specified XLFD (an empty field becomes the empty symbol `##`).
    let adstyle = intern_field(XLFD_ADSTYLE);
    if adstyle != "*" {
        font_parse_set(elems, "adstyle", Value::symbol(adstyle));
    }

    // Registry-encoding: "registry-encoding" combined.
    let registry = intern_field(XLFD_REGISTRY);
    let encoding = intern_field(XLFD_ENCODING);
    if !(registry == "*" && encoding == "*") {
        let combined = format!("{registry}-{encoding}");
        font_parse_set(
            elems,
            "registry",
            Value::symbol(combined.to_ascii_lowercase()),
        );
    }

    // Size: prefer pixel size (fixnum), else point size / 10 (float).
    let pixel = intern_field(XLFD_PIXEL);
    if let Ok(px) = pixel.parse::<i64>() {
        if px > 0 {
            font_parse_set(elems, "size", Value::fixnum(px));
        }
    } else {
        let point = intern_field(XLFD_POINT);
        if let Ok(pt) = point.parse::<i64>() {
            font_parse_set(elems, "size", Value::make_float(pt as f64 / 10.0));
        }
    }

    // DPI (resolution-y).
    let resy = intern_field(XLFD_RESY);
    if let Ok(dpi) = resy.parse::<i64>() {
        font_parse_set(elems, "dpi", Value::fixnum(dpi));
    }
    let _ = intern_field(XLFD_RESX);

    // Spacing letter (p/d/m/c).
    let spacing = intern_field(XLFD_SPACING);
    if let Some(sp) = FontSpacing::from_symbol_name(spacing) {
        font_parse_set(elems, "spacing", Value::fixnum(i64::from(sp.gnu_code())));
    }

    // Average width.
    let avg = intern_field(XLFD_AVGWIDTH).trim_start_matches('~');
    if let Ok(n) = avg.parse::<i64>() {
        font_parse_set(elems, "avgwidth", Value::fixnum(n));
    }

    true
}

/// Port of GNU `font_parse_name` (src/font.c): dispatch a font NAME string to
/// the XLFD or fontconfig parser and store the parsed properties into ELEMS
/// (a font-spec property vector).  Returns `false` if the name cannot be parsed.
fn font_parse_name(elems: &mut Vec<Value>, name: &str) -> bool {
    if name.starts_with('-') || name.contains('*') || name.contains('?') {
        font_parse_xlfd(elems, name)
    } else {
        font_parse_fcname(elems, name)
    }
}

/// Build a font-spec from a font NAME string (GNU `font_spec_from_name`):
/// parse NAME, then record it under `:name`.  Returns `None` on parse failure.
fn font_spec_from_name(name: &str) -> Option<Value> {
    let mut elems = vec![Value::keyword(FONT_SPEC_TAG)];
    if !font_parse_name(&mut elems, name) {
        return None;
    }
    font_parse_set(&mut elems, "name", Value::string(name.to_string()));
    Some(Value::vector(elems))
}

/// `(font-face-attributes FONT &optional FRAME)` -- return a plist of face
/// attributes generated by FONT.  Port of GNU `Ffont_face_attributes`
/// (src/font.c): FONT may be a font name string (parsed via
/// `font_spec_from_name`), a font-spec, font-entity, or font-object.  The result
/// is `(:family F :height H :weight W :slant S :width WD)` with absent keys
/// omitted.
pub(crate) fn builtin_font_face_attributes(args: Vec<Value>) -> EvalResult {
    expect_min_args("font-face-attributes", &args, 1)?;
    expect_max_args("font-face-attributes", &args, 2)?;

    let font = if args[0].is_string() {
        let name = font_string_text(&args[0]).unwrap_or_default();
        match font_spec_from_name(&name) {
            Some(spec) => spec,
            None => {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid font name"), args[0]],
                ));
            }
        }
    } else if is_font(&args[0]) {
        args[0]
    } else {
        return Err(signal(
            "error",
            vec![Value::string("Invalid font object"), args[0]],
        ));
    };

    let elems = font.as_vector_data().unwrap().clone();
    let mut plist: Vec<Value> = Vec::with_capacity(10);

    // :family (symbol name -> string).
    if let Some(family) = font_vector_get_flexible(&elems, "family")
        && !family.is_nil()
    {
        let family_str = match family.kind() {
            ValueKind::Symbol(id) => Value::string(resolve_sym(id).to_owned()),
            ValueKind::String => family,
            _ => Value::NIL,
        };
        if !family_str.is_nil() {
            plist.push(Value::keyword("family"));
            plist.push(family_str);
        }
    }

    // :height -- GNU maps the font size to a face height (10 * point size).
    // A fixnum size is a pixel size converted via PIXEL_TO_POINT; with no
    // display DPI here we follow GNU's float path (point size) for parsed
    // names, where size is stored as a float.
    if let Some(size) = font_vector_get_flexible(&elems, "size") {
        match size.kind() {
            ValueKind::Float => {
                let pts = size.xfloat();
                if pts > 0.0 {
                    plist.push(Value::keyword("height"));
                    plist.push(Value::fixnum(10 * (pts as i64)));
                }
            }
            ValueKind::Fixnum(px) if px > 0 => {
                // Pixel size: GNU converts via the frame resolution.  Without a
                // live display we approximate point size == pixel size (the
                // common 72-dpi identity used in batch contexts).
                plist.push(Value::keyword("height"));
                plist.push(Value::fixnum(px * 10));
            }
            _ => {}
        }
    }

    // :weight / :slant / :width -- GNU `Ffont_face_attributes` reads these via
    // the FONT_*_FOR_FACE macros (font_style_symbolic with for_face=true), which
    // canonicalize the stored alias to its row's preferred name
    // (heavy -> black, ultra-bold -> extra-bold, normal -> regular). The
    // storage path keeps the alias verbatim (matching `font-get`), so the
    // canonicalization happens here, at the face-read boundary.
    for key in ["weight", "slant", "width"] {
        if let Some(val) = font_vector_get_flexible(&elems, key)
            && !val.is_nil()
        {
            let canonical = val
                .as_symbol_name()
                .and_then(|name| font_style_canonical_for_face(key, name))
                .map(Value::symbol)
                .unwrap_or(val);
            plist.push(Value::keyword(key));
            plist.push(canonical);
        }
    }

    Ok(Value::list(plist))
}

// ===========================================================================
// Font builtins (pure)
// ===========================================================================

/// `(fontp OBJECT &optional EXTRA-TYPE)` -- return t if OBJECT is a font-spec,
/// font-entity, or font-object.  We represent all of these as tagged vectors
/// with `:font-spec` keyword at position 0.
pub(crate) fn builtin_fontp(args: Vec<Value>) -> EvalResult {
    expect_max_args("fontp", &args, 2)?;
    expect_min_args("fontp", &args, 1)?;
    let object = &args[0];
    let extra_type = args.get(1).copied().unwrap_or(Value::NIL);
    let value = if extra_type.is_nil() {
        is_font(object)
    } else if extra_type.is_symbol_named("font-spec") {
        is_font_spec(object)
    } else if extra_type.is_symbol_named("font-object") {
        is_font_object(object)
    } else if extra_type.is_symbol_named("font-entity") {
        is_font_entity(object)
    } else {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("font-extra-type"), extra_type],
        ));
    };
    Ok(Value::bool_val(value))
}

/// `(font-spec &rest ARGS)` -- create a font spec from keyword args.
///
/// Usage: `(font-spec :family "Monospace" :weight 'normal :size 12)`
///
/// Returns a vector `[:font-spec :family "Monospace" :weight normal :size 12]`.
pub(crate) fn builtin_font_spec(args: Vec<Value>) -> EvalResult {
    let mut elems: Vec<Value> = Vec::with_capacity(1 + args.len());
    elems.push(Value::keyword(FONT_SPEC_TAG));

    for pair_index in (0..args.len()).step_by(2) {
        let key = &args[pair_index];
        let value = args.get(pair_index + 1);

        let Some(value) = value else {
            if key.is_keyword() || key.is_symbol() || key.is_nil() {
                let key_name = match key.kind() {
                    ValueKind::Symbol(id) => resolve_sym(id).to_owned(),
                    ValueKind::Nil => "nil".to_string(),
                    _ => "nil".to_string(),
                };
                return Err(signal(
                    "error",
                    vec![Value::string(format!("No value for key ‘{}’", key_name))],
                ));
            }
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("symbolp"), *key],
            ));
        };

        if key.is_nil() {
            return Err(signal(
                "error",
                vec![
                    Value::string("invalid font property"),
                    Value::list(vec![Value::cons(Value::keyword("type"), *value)]),
                ],
            ));
        }

        if !(key.is_keyword() || key.is_symbol()) {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("symbolp"), *key],
            ));
        }

        // GNU `Ffont_spec`: a `:name` argument is a font name string that is
        // parsed via `font_parse_name` into the spec's basic slots; the name
        // itself is also recorded under `:name`.
        if key
            .as_symbol_name()
            .map(|name| name.trim_start_matches(':'))
            == Some("name")
        {
            let Some(name) = font_string_text(value) else {
                return Err(signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("stringp"), *value],
                ));
            };
            if !font_parse_name(&mut elems, &name) {
                return Err(signal(
                    "error",
                    vec![Value::string(format!("Invalid font name: {name}"))],
                ));
            }
            font_parse_set(&mut elems, "name", *value);
            continue;
        }

        elems.push(*key);
        elems.push(normalize_font_prop_value(key, value)?);
    }

    Ok(Value::vector(elems))
}

/// `(font-get FONT PROP)` -- get a property value from a font-spec.
pub(crate) fn builtin_font_get(args: Vec<Value>) -> EvalResult {
    expect_args("font-get", &args, 2)?;
    if !is_font(&args[0]) {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("font"), args[0]],
        ));
    }
    if !(args[1].is_keyword() || args[1].is_symbol()) {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("symbolp"), args[1]],
        ));
    }

    match args[0].kind() {
        ValueKind::Veclike(VecLikeType::Vector) => {
            let elems = args[0].as_vector_data().unwrap().clone();
            let exact = font_vector_get(&elems, &args[1]);
            if !exact.is_nil() {
                return Ok(exact);
            }

            if let Some(id) = args[1].as_keyword_id() {
                return Ok(font_vector_get_flexible(&elems, resolve_sym(id)).unwrap_or(Value::NIL));
            }

            Ok(Value::NIL)
        }
        _ => unreachable!("font check above guarantees vector"),
    }
}

/// `(font-put FONT PROP VAL)` -- set a property in a font-spec and return VAL.
pub(crate) fn builtin_font_put(args: Vec<Value>) -> EvalResult {
    expect_args("font-put", &args, 3)?;
    if !is_font_spec(&args[0]) {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("font-spec"), args[0]],
        ));
    }
    match args[0].kind() {
        ValueKind::Veclike(VecLikeType::Vector) => {
            let mut elems = args[0]
                .as_vector_data()
                .map(|items| items.to_vec())
                .unwrap_or_default();
            let normalized = font_spec_put(&mut elems, &args[1], &args[2])?;
            let _ = args[0].replace_vector_data(elems);
            Ok(normalized)
        }
        _ => unreachable!("font-spec check above guarantees vector"),
    }
}

/// Context-aware variant of `list-fonts`.
///
/// Accepts live frame designators in the optional FRAME slot.
pub(crate) fn builtin_list_fonts(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("list-fonts", &args, 1)?;
    expect_max_args("list-fonts", &args, 4)?;
    if !is_font_spec(&args[0]) {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("font-spec"), args[0]],
        ));
    }
    expect_optional_frame_designator_in_state(&eval.frames, args.get(1))?;
    Ok(Value::NIL)
}

fn font_weight_from_value(value: Value) -> Option<FontWeight> {
    match value.kind() {
        ValueKind::Symbol(id) => FontWeight::from_symbol(resolve_sym(id)),
        _ => None,
    }
}

fn font_slant_from_value(value: Value) -> Option<FontSlant> {
    match value.kind() {
        ValueKind::Symbol(id) => FontSlant::from_symbol(resolve_sym(id)),
        _ => None,
    }
}

fn find_font_frame_id(
    eval: &mut super::eval::Context,
    frame: Option<&Value>,
) -> Result<FrameId, Flow> {
    match frame {
        None => Ok(super::window_cmds::ensure_selected_frame_id(eval)),
        Some(v) if v.is_nil() => Ok(super::window_cmds::ensure_selected_frame_id(eval)),
        Some(value) if live_frame_designator_in_state(&eval.frames, value) => {
            frame_id_from_designator(value).ok_or_else(|| {
                signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("frame-live-p"), *value],
                )
            })
        }
        Some(other) => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), *other],
        )),
    }
}

fn font_spec_resolve_request(
    eval: &mut super::eval::Context,
    font_spec: &Value,
    frame: Option<&Value>,
) -> Result<super::eval::FontSpecResolveRequest, Flow> {
    if !font_spec.is_vector() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("font-spec"), *font_spec],
        ));
    };

    let elems = font_spec.as_vector_data().unwrap().clone();
    let family = font_vector_get_flexible(&elems, "family")
        .and_then(|value| font_value_text_lisp_string(&value));
    let registry = font_vector_get_flexible(&elems, "registry")
        .and_then(|value| font_value_text_lisp_string(&value));
    let lang = font_vector_get_flexible(&elems, "lang")
        .and_then(|value| font_value_text_lisp_string(&value));
    let weight = font_vector_get_flexible(&elems, "weight").and_then(font_weight_from_value);
    let slant = font_vector_get_flexible(&elems, "slant").and_then(font_slant_from_value);
    let width = font_vector_get_flexible(&elems, "width").and_then(|value| match value.kind() {
        ValueKind::Symbol(id) => FontWidth::from_symbol(resolve_sym(id)),
        _ => None,
    });

    Ok(super::eval::FontSpecResolveRequest {
        frame_id: find_font_frame_id(eval, frame)?,
        family,
        registry,
        lang,
        weight,
        slant,
        width,
    })
}

/// Context-aware variant of `find-font`.
///
/// Accepts live frame designators in the optional FRAME slot.
pub(crate) fn builtin_find_font(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("find-font", &args, 1)?;
    expect_max_args("find-font", &args, 2)?;
    if !is_font_spec(&args[0]) {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("font-spec"), args[0]],
        ));
    }

    let request = font_spec_resolve_request(eval, &args[0], args.get(1))?;
    let Some(host) = eval.display_host.as_mut() else {
        return Ok(Value::NIL);
    };
    let matched = host
        .resolve_font_for_spec(request)
        .map_err(|err| signal("error", vec![Value::string(err)]))?;
    let Some(matched) = matched else {
        return Ok(Value::NIL);
    };
    Ok(build_font_entity_for_spec_match(&matched))
}

/// `(clear-font-cache)` -- reset internal font/face caches and return nil.
pub(crate) fn builtin_clear_font_cache(args: Vec<Value>) -> EvalResult {
    expect_max_args("clear-font-cache", &args, 0)?;
    clear_font_cache_state();
    Ok(Value::NIL)
}

/// Context-aware variant of `font-family-list`.
///
/// Accepts live frame designators in the optional FRAME slot.
pub(crate) fn builtin_font_family_list(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_max_args("font-family-list", &args, 1)?;
    expect_optional_frame_designator_in_state(&eval.frames, args.first())?;
    Ok(Value::NIL)
}

/// `(font-xlfd-name FONT &optional FOLD-WILDCARDS)` -- render font-spec fields
/// into an XLFD string; wildcard folding is supported in compatibility mode.
pub(crate) fn builtin_font_xlfd_name(args: Vec<Value>) -> EvalResult {
    expect_min_args("font-xlfd-name", &args, 1)?;
    expect_max_args("font-xlfd-name", &args, 3)?;
    if !is_font(&args[0]) {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("font"), args[0]],
        ));
    }

    let fields = match args[0].kind() {
        ValueKind::Veclike(VecLikeType::Vector) => {
            let elems = args[0].as_vector_data().unwrap().clone();
            if is_font_object(&args[0])
                && font_vector_get_flexible(&elems, "name").is_some_and(|v| v.is_string())
            {
                let font_name = font_vector_get_flexible(&elems, "name")
                    .unwrap()
                    .as_utf8_str()
                    .unwrap()
                    .to_owned();
                if font_name.starts_with('-') {
                    return Ok(Value::string(
                        if args.get(1).is_some_and(|v| v.is_truthy()) {
                            fold_xlfd_wildcards(font_name)
                        } else {
                            font_name
                        },
                    ));
                }
            }
            xlfd_fields_from_font_vector(&elems)
        }
        _ => (
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*-*".to_string(),
            "*-*".to_string(),
            "*".to_string(),
            "*".to_string(),
            "*-*".to_string(),
        ),
    };

    let (
        foundry,
        family,
        weight,
        slant,
        set_width,
        adstyle,
        pixel,
        resx,
        spacing,
        avg_width,
        registry,
    ) = fields;
    let rendered = if args.get(1).is_some_and(|v| v.is_truthy()) {
        let name = format!(
            "-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}",
            foundry,
            family,
            weight,
            slant,
            set_width,
            adstyle,
            pixel,
            resx,
            spacing,
            avg_width,
            registry
        );
        fold_xlfd_wildcards(name)
    } else {
        format!(
            "-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}-{}",
            foundry,
            family,
            weight,
            slant,
            set_width,
            adstyle,
            pixel,
            resx,
            spacing,
            avg_width,
            registry
        )
    };
    Ok(Value::string(rendered))
}

/// `(close-font FONT-OBJECT &optional FRAME)` -- close an open font object.
///
/// NeoVM currently has no runtime font-object handles, so this validates the
/// argument shape and returns nil for accepted objects.
pub(crate) fn builtin_close_font(args: Vec<Value>) -> EvalResult {
    expect_min_args("close-font", &args, 1)?;
    expect_max_args("close-font", &args, 2)?;
    if !is_font_object(&args[0]) {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("font-object"), args[0]],
        ));
    }
    Ok(Value::NIL)
}

#[derive(Clone, Debug)]
enum FaceLayer {
    Named(Vec<String>),
    Inline(RuntimeFace),
}

fn window_id_from_designator(value: &Value) -> Option<WindowId> {
    match value.kind() {
        ValueKind::Veclike(VecLikeType::Window) => Some(WindowId(value.as_window_id().unwrap())),
        ValueKind::Fixnum(n) if n >= 0 => Some(WindowId(n as u64)),
        _ => None,
    }
}

fn resolve_live_window_for_font_at(
    eval: &mut super::eval::Context,
    value: Option<&Value>,
) -> Result<(FrameId, WindowId), Flow> {
    match value {
        None => {
            let frame_id = super::window_cmds::ensure_selected_frame_id(eval);
            let frame = eval
                .frames
                .get(frame_id)
                .ok_or_else(|| signal("error", vec![Value::string("No selected frame")]))?;
            Ok((frame_id, frame.selected_window))
        }
        Some(v) if v.is_nil() => {
            let frame_id = super::window_cmds::ensure_selected_frame_id(eval);
            let frame = eval
                .frames
                .get(frame_id)
                .ok_or_else(|| signal("error", vec![Value::string("No selected frame")]))?;
            Ok((frame_id, frame.selected_window))
        }
        Some(other) => {
            let Some(window_id) = window_id_from_designator(other) else {
                return Err(signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("window-live-p"), *other],
                ));
            };
            let Some(frame_id) = eval.frames.find_window_frame_id(window_id) else {
                return Err(signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("window-live-p"), *other],
                ));
            };
            Ok((frame_id, window_id))
        }
    }
}

fn resolve_face_layers_from_value(value: &Value) -> Vec<FaceLayer> {
    match value.kind() {
        ValueKind::Nil => Vec::new(),
        ValueKind::Symbol(_) => value
            .as_symbol_name()
            .filter(|name| *name != "nil")
            .map(|name| vec![FaceLayer::Named(vec![name.to_string()])])
            .unwrap_or_default(),
        ValueKind::Cons => {
            let Some(items) = list_to_vec(value) else {
                return Vec::new();
            };
            if items.first().is_some_and(|item| item.is_keyword()) {
                vec![FaceLayer::Inline(RuntimeFace::from_plist(
                    "--font-at--",
                    &items,
                ))]
            } else {
                let names = items
                    .iter()
                    .filter_map(|item| {
                        item.as_symbol_name()
                            .filter(|name| *name != "nil")
                            .map(|name| name.to_string())
                    })
                    .collect::<Vec<_>>();
                if names.is_empty() {
                    Vec::new()
                } else {
                    vec![FaceLayer::Named(names)]
                }
            }
        }
        _ => Vec::new(),
    }
}

/// Extract the `face-remapping-alist` for a specific buffer.
///
/// Checks the buffer-local binding first; falls back to the global value.
fn face_remapping_for_buffer(eval: &super::eval::Context, buffer: &Buffer) -> FaceRemapping {
    // Buffer-local binding takes priority
    let value = buffer
        .get_buffer_local("face-remapping-alist")
        .or_else(|| eval.obarray().symbol_value("face-remapping-alist").copied())
        .unwrap_or(Value::NIL);

    if value.is_nil() {
        FaceRemapping::new()
    } else {
        FaceRemapping::from_lisp(&value)
    }
}

/// Extract the `face-remapping-alist` from the current buffer (if any).
fn face_remapping_for_current_buffer(eval: &super::eval::Context) -> FaceRemapping {
    if let Some(buf) = eval.buffers.current_buffer() {
        face_remapping_for_buffer(eval, buf)
    } else {
        let value = eval
            .obarray()
            .symbol_value("face-remapping-alist")
            .copied()
            .unwrap_or(Value::NIL);
        if value.is_nil() {
            FaceRemapping::new()
        } else {
            FaceRemapping::from_lisp(&value)
        }
    }
}

fn apply_face_layers_with_remapping(
    face_table: &crate::face::FaceTable,
    layers: &[FaceLayer],
    remapping: &FaceRemapping,
) -> RuntimeFace {
    let mut face = if remapping.is_empty() {
        face_table.resolve("default")
    } else {
        face_table.resolve_with_remapping("default", remapping)
    };
    for layer in layers {
        match layer {
            FaceLayer::Named(names) => {
                let refs = names.iter().map(String::as_str).collect::<Vec<_>>();
                let merged = if remapping.is_empty() {
                    face_table.merge_faces(&refs)
                } else {
                    face_table.merge_faces_with_remapping(&refs, remapping)
                };
                face = face.merge(&merged);
            }
            FaceLayer::Inline(inline_face) => {
                face = face.merge(inline_face);
            }
        }
    }
    face
}

fn resolved_face_at_buffer_byte(
    eval: &super::eval::Context,
    face_table: &crate::face::FaceTable,
    buffer: &Buffer,
    bytepos: EmacsBytePos,
) -> RuntimeFace {
    let mut layers = Vec::new();

    let face_prop =
        buffer.text_props_get_property_at_emacs_byte_pos(bytepos, Value::symbol("face"));
    let font_lock_face_prop =
        buffer.text_props_get_property_at_emacs_byte_pos(bytepos, Value::symbol("font-lock-face"));
    if let Some(value) = face_prop.or(font_lock_face_prop) {
        layers.extend(resolve_face_layers_from_value(&value));
    }

    let mut overlay_layers = Vec::new();
    for overlay_id in buffer.overlays.overlays_at_emacs_byte_pos(bytepos) {
        let priority = buffer
            .overlays
            .overlay_get_named(overlay_id, Value::symbol("priority"))
            .and_then(|value| value.as_int())
            .unwrap_or(0);
        if let Some(value) = buffer
            .overlays
            .overlay_get_named(overlay_id, Value::symbol("face"))
        {
            let resolved = resolve_face_layers_from_value(&value);
            if !resolved.is_empty() {
                overlay_layers.push((priority, resolved));
            }
        }
    }
    overlay_layers.sort_by_key(|(priority, _)| *priority);
    for (_, resolved) in overlay_layers {
        layers.extend(resolved);
    }

    // Consult buffer-local face-remapping-alist
    let remapping = face_remapping_for_buffer(eval, buffer);
    apply_face_layers_with_remapping(face_table, &layers, &remapping)
}

fn resolved_face_at_string_char_pos(
    eval: &super::eval::Context,
    face_table: &crate::face::FaceTable,
    str_value: Value,
    char_pos: CharPos0,
) -> RuntimeFace {
    let mut layers = Vec::new();
    if let Some(table) = get_string_text_properties_table_for_value(str_value) {
        let face_prop = table.get_property_at_char_pos(char_pos, Value::symbol("face"));
        let font_lock_face_prop =
            table.get_property_at_char_pos(char_pos, Value::symbol("font-lock-face"));
        if let Some(value) = face_prop.or(font_lock_face_prop) {
            layers.extend(resolve_face_layers_from_value(&value));
        }
    }
    // Use face-remapping-alist from the current buffer (strings inherit
    // the buffer context they're displayed in).
    let remapping = face_remapping_for_current_buffer(eval);
    apply_face_layers_with_remapping(face_table, &layers, &remapping)
}

fn face_height_to_font_value(height: &FaceHeight) -> Value {
    match height {
        FaceHeight::Absolute(n) => Value::fixnum(*n as i64),
        FaceHeight::Relative(f) => Value::make_float(*f),
    }
}

fn font_weight_symbol(weight: FontWeight) -> &'static str {
    weight.symbol_name()
}

fn build_font_object(face: &RuntimeFace) -> Value {
    build_font_object_with_pixel_size(face, None)
}

/// GNU font objects carry the OPENED pixel size in FONT_SIZE (the XLFD's
/// pixel field prints it); pass `pixel_size` when the resolver knows it.
fn build_font_object_with_pixel_size(face: &RuntimeFace, pixel_size: Option<i64>) -> Value {
    let mut elems = vec![Value::keyword(FONT_OBJECT_TAG)];

    let mut push_field = |name: &str, value: Value| {
        elems.push(Value::keyword(name));
        elems.push(value);
    };

    if let Some(foundry) = face
        .foundry
        .as_ref()
        .and_then(font_value_text)
        .map(|text| Value::from_sym_id(intern(&text)))
    {
        push_field("foundry", foundry);
    }
    if let Some(family) = face
        .family
        .as_ref()
        .and_then(font_value_text)
        .map(|text| Value::from_sym_id(intern(&text)))
    {
        push_field("family", family);
    }
    // GNU's canonical style-table first names, as on entities.
    if let Some(weight) = face.weight {
        let name = font_weight_symbol(weight);
        let name = gnu_style_first_name(GNU_WEIGHT_TABLE, name).unwrap_or(name);
        push_field("weight", Value::symbol(name));
    }
    if let Some(slant) = face.slant {
        let name = slant.symbol_name();
        let name = gnu_style_first_name(GNU_SLANT_TABLE, name).unwrap_or(name);
        push_field("slant", Value::symbol(name));
    }
    if let Some(width) = face.width {
        let name = width.symbol_name();
        let name = gnu_style_first_name(GNU_WIDTH_TABLE, name).unwrap_or(name);
        push_field("width", Value::symbol(name));
    }
    if let Some(height) = &face.height {
        push_field("height", face_height_to_font_value(height));
    }
    if let Some(px) = pixel_size {
        push_field("size", Value::fixnum(px));
    } else if let Some(height) = &face.height {
        push_field("size", face_height_to_font_value(height));
    }
    if pixel_size.is_some() {
        // A resolver-opened font: like GNU's opened font objects, carry the
        // entity registry and the scalable avg-width 0 so the object XLFD
        // ends "-0-iso10646-1", not "-*-*".
        push_field("registry", Value::from_sym_id(intern("iso10646-1")));
        push_field("avg-width", Value::fixnum(0));
    }

    let font_object = Value::vector(elems);
    let xlfd = builtin_font_xlfd_name(vec![font_object]).unwrap_or(Value::NIL);
    if font_object.is_vector() {
        let mut items = font_object
            .as_vector_data()
            .map(|items| items.to_vec())
            .unwrap_or_default();
        items.push(Value::keyword("name"));
        items.push(if xlfd.is_nil() { Value::NIL } else { xlfd });
        let _ = font_object.replace_vector_data(items);
    }
    font_object
}

fn build_font_entity_for_spec_match(matched: &super::eval::ResolvedFontSpecMatch) -> Value {
    let mut elems = vec![Value::keyword(FONT_ENTITY_TAG)];

    let mut push_field = |name: &str, value: Value| {
        elems.push(Value::keyword(name));
        elems.push(value);
    };

    // GNU orders entity fields foundry-first (XLFD order); the foundry is
    // a symbol (e.g. GOOG) read from fontconfig FC_FOUNDRY.
    if let Some(foundry) = &matched.foundry {
        push_field(
            "foundry",
            Value::from_sym_id(intern(foundry.as_utf8_str().unwrap_or_default())),
        );
    }
    push_field(
        "family",
        Value::from_sym_id(intern(matched.family.as_utf8_str().unwrap_or_default())),
    );
    if let Some(registry) = &matched.registry {
        push_field(
            "registry",
            Value::from_sym_id(intern(registry.as_utf8_str().unwrap_or_default())),
        );
    }
    // Style symbols use GNU's canonical (first) style-table name —
    // font-get on a GNU entity reports e.g. `ultra-light`, never the
    // `extralight` alias; the XLFD's dashless spelling falls out of
    // `sanitize_style_field` stripping the dash.
    if let Some(weight) = matched.weight {
        let name = font_weight_symbol(weight);
        let name = gnu_style_first_name(GNU_WEIGHT_TABLE, name).unwrap_or(name);
        push_field("weight", Value::symbol(name));
    }
    if let Some(slant) = matched.slant {
        let name = slant.symbol_name();
        let name = gnu_style_first_name(GNU_SLANT_TABLE, name).unwrap_or(name);
        push_field("slant", Value::symbol(name));
    }
    if let Some(width) = matched.width {
        let name = width.symbol_name();
        let name = gnu_style_first_name(GNU_WIDTH_TABLE, name).unwrap_or(name);
        push_field("width", Value::symbol(name));
    }
    if let Some(spacing) = matched.spacing {
        push_field("spacing", Value::fixnum(spacing as i64));
    }
    if let Some(postscript_name) = &matched.postscript_name {
        push_field(
            "postscript-name",
            Value::heap_string(postscript_name.clone()),
        );
    }
    if let Some(file) = &matched.file {
        push_field("file", Value::heap_string(file.clone()));
    }
    // Scalable entities carry average width 0 (GNU src/ftfont.c sets
    // FONT_AVGWIDTH_INDEX to 0); the XLFD renders it as "0", not "*".
    push_field("avg-width", Value::fixnum(0));

    Value::vector(elems)
}

fn font_vector_with_file(font: Value, file: &Option<LispString>) -> Value {
    let Some(file) = file else {
        return font;
    };
    if font.is_vector() {
        let mut items = font
            .as_vector_data()
            .map(|items| items.to_vec())
            .unwrap_or_default();
        items.push(Value::keyword("file"));
        items.push(Value::heap_string(file.clone()));
        let _ = font.replace_vector_data(items);
    }
    font
}

fn build_font_object_for_match(
    face: &RuntimeFace,
    matched: &super::eval::ResolvedFontMatch,
) -> Value {
    let mut selected = face.clone();
    selected.family = Some(Value::from_sym_id(intern(
        matched.family.as_utf8_str().unwrap_or_default(),
    )));
    selected.foundry = matched
        .foundry
        .as_ref()
        .map(|foundry| Value::from_sym_id(intern(foundry.as_utf8_str().unwrap_or_default())))
        .or(face.foundry);
    selected.weight = Some(matched.weight);
    selected.slant = Some(matched.slant);
    selected.width = Some(matched.width);
    font_vector_with_file(
        build_font_object_with_pixel_size(&selected, Some(matched.pixel_size_px.max(1) as i64)),
        &matched.file,
    )
}

fn font_name_value(font_like: &Value) -> Option<Value> {
    match font_like.kind() {
        ValueKind::String => Some(*font_like),
        ValueKind::Veclike(VecLikeType::Vector) if is_font(font_like) => {
            let elems = font_like.as_vector_data().unwrap().clone();
            if let Some(value) = font_vector_get_flexible(&elems, "name") {
                return match value.kind() {
                    ValueKind::String => Some(value),
                    ValueKind::Symbol(sym) => Some(Value::string(resolve_sym(sym).to_owned())),
                    _ => None,
                };
            }
            match builtin_font_xlfd_name(vec![*font_like]) {
                Ok(v) if v.is_string() => Some(v),
                _ => None,
            }
        }
        _ => None,
    }
}

pub(crate) fn public_frame_font_parameter_value(font_like: Value) -> Value {
    if is_font(&font_like) {
        font_name_value(&font_like).unwrap_or(font_like)
    } else {
        font_like
    }
}

fn font_value_matches_frame_font_parameter(
    frame: &crate::window::Frame,
    requested: &Value,
) -> bool {
    let Some(frame_font) = frame.known_parameter(FrameParam::Font) else {
        return false;
    };
    match (frame_font.kind(), requested.kind()) {
        (ValueKind::String, ValueKind::String) => {
            frame_font.as_lisp_string() == requested.as_lisp_string()
        }
        _ => false,
    }
}

fn public_live_frame_font_value(font_value: Value) -> Value {
    if !font_value.is_vector() {
        return font_value;
    };
    if !is_font(&font_value) {
        return font_value;
    }

    let elems = font_value.as_vector_data().unwrap().clone();
    let mut filtered = Vec::with_capacity(elems.len());
    let mut idx = 0;
    while idx < elems.len() {
        if idx == 0 {
            filtered.push(elems[idx]);
            idx += 1;
            continue;
        }

        if idx + 1 >= elems.len() {
            filtered.push(elems[idx]);
            break;
        }

        let key_name = elems[idx]
            .as_symbol_id()
            .or_else(|| elems[idx].as_keyword_id())
            .map(|id_| resolve_sym(id_).trim_start_matches(':').to_string());
        let keep = key_name.as_deref() != Some("height");
        if keep {
            filtered.push(elems[idx]);
            let value = match key_name.as_deref() {
                Some("family") | Some("foundry")
                    if elems[idx + 1]
                        .as_symbol_name()
                        .is_some_and(|name| !name.is_empty()) =>
                {
                    Value::string(
                        elems[idx + 1]
                            .as_symbol_name()
                            .expect("checked above")
                            .to_string(),
                    )
                }
                _ => elems[idx + 1],
            };
            filtered.push(value);
        }
        idx += 2;
    }

    Value::vector(filtered)
}

fn live_frame_font_attribute_fallback(
    eval: &super::eval::Context,
    frame_id: FrameId,
    attr: LFaceAttr,
) -> Option<Value> {
    let frame = eval.frames.get(frame_id)?;
    let font_value = frame.parameter("font-parameter")?;
    if !is_font(&font_value) {
        return None;
    }

    if attr == LFaceAttr::Font {
        return Some(public_live_frame_font_value(font_value));
    }

    derived_face_attrs_from_font_value(&font_value)
        .into_iter()
        .find_map(|(derived_attr, derived_value)| (derived_attr == attr).then_some(derived_value))
}

/// GNU font.c style tables (weight/slant/width). Each row is the aliases of
/// one numeric style value; `font_style_symbolic` reports the FIRST name,
/// which is what `font_unparse_fcname` prints in the fontconfig-style full
/// name (e.g. "extra-bold").
const GNU_WEIGHT_TABLE: &[&[&str]] = &[
    &["thin"],
    &["ultra-light", "ultralight", "extra-light", "extralight"],
    &["light"],
    &["semi-light", "semilight", "demilight"],
    &["regular", "normal", "unspecified", "book"],
    &["medium"],
    &["semi-bold", "semibold", "demibold", "demi-bold", "demi"],
    &["bold"],
    &["extra-bold", "extrabold", "ultra-bold", "ultrabold"],
    &["black", "heavy"],
    &["ultra-heavy", "ultraheavy"],
];
const GNU_SLANT_TABLE: &[&[&str]] = &[
    &["reverse-oblique", "ro"],
    &["reverse-italic", "ri"],
    &["normal", "r", "unspecified"],
    &["italic", "i", "ot"],
    &["oblique", "o"],
];
const GNU_WIDTH_TABLE: &[&[&str]] = &[
    &["ultra-condensed", "ultracondensed"],
    &["extra-condensed", "extracondensed"],
    &["condensed", "compressed", "narrow"],
    &["semi-condensed", "semicondensed", "demicondensed"],
    &["normal", "medium", "regular", "unspecified"],
    &["semi-expanded", "semiexpanded", "demiexpanded"],
    &["expanded"],
    &["extra-expanded", "extraexpanded"],
    &["ultra-expanded", "ultraexpanded", "wide"],
];

/// Map a style symbol name to GNU's canonical (first) table name.
fn gnu_style_first_name(
    table: &'static [&'static [&'static str]],
    name: &str,
) -> Option<&'static str> {
    table
        .iter()
        .find(|row| row.contains(&name))
        .map(|row| row[0])
}

/// `font-info` for a font ENTITY, following GNU font.c `Ffont_info`: open
/// the entity via `font_open_entity` (a scalable entity's size 0 probes
/// upward from 1px until the font is "manageable") and report the OPENED
/// font's metrics — the tiny pixelsize=1 numbers — not the frame's realized
/// font. Names: element 0 is the entity XLFD with the probed pixel size,
/// element 1 the fontconfig-style name `font_unparse_fcname` builds.
fn font_info_vector_for_entity(eval: &mut super::eval::Context, entity: &Value) -> Option<Value> {
    let elems = entity.as_vector_data()?.clone();
    let file_value = font_vector_get_flexible(&elems, "file").filter(|value| value.is_string())?;
    let file = file_value.as_utf8_str()?.to_owned();
    let px = font_vector_get_flexible(&elems, "size")
        .and_then(|value| match value.kind() {
            ValueKind::Fixnum(n) if n > 0 => Some(n as u32),
            _ => None,
        })
        .unwrap_or(0);
    // Variable fonts: probe the value's weight instance (OT wght axis units
    // are CSS weights).
    let wght = font_vector_get_flexible(&elems, "weight")
        .and_then(|value| value.as_symbol_name())
        .and_then(|name| name.trim_start_matches(':').parse::<FontWeight>().ok())
        .map(|weight| f32::from(weight.css_weight()));
    let probe = eval
        .display_host
        .as_mut()
        .and_then(|host| host.probe_font_px_metrics(&file, 0, px, wght).ok())
        .flatten()?;
    // Element 14: (opentype GSUB . GPOS) like GNU's
    // `Fcons (Qopentype, otf_capability (font))` (font.c Ffont_info).
    let capability = otf_capability_lisp(eval, &file);

    let (
        foundry,
        family,
        weight,
        slant,
        set_width,
        adstyle,
        _pixel,
        resx,
        spacing_field,
        avg_width,
        registry,
    ) = xlfd_fields_from_font_vector(&elems);
    let opened_name = format!(
        "-{}-{}-{}-{}-{}-{}-{}-*-{}-{}-{}-{}",
        foundry,
        family,
        weight,
        slant,
        set_width,
        adstyle,
        probe.pixel_size,
        resx,
        spacing_field,
        avg_width,
        registry
    );

    // font_unparse_fcname: family:pixelsize=N[:foundry=F][:weight=W]
    // [:slant=S][:width=W][:spacing=N]:scalable=true (avgwidth 0).
    let mut full_name = String::new();
    full_name.push_str(&family);
    full_name.push_str(&format!(":pixelsize={}", probe.pixel_size));
    if foundry != "*" {
        full_name.push_str(&format!(":foundry={foundry}"));
    }
    let style = |key: &str, table: &'static [&'static [&'static str]]| -> Option<&'static str> {
        font_vector_get_flexible(&elems, key)
            .and_then(|value| value.as_symbol_name())
            .and_then(|name| gnu_style_first_name(table, name.trim_start_matches(':')))
    };
    if let Some(name) = style("weight", GNU_WEIGHT_TABLE) {
        full_name.push_str(&format!(":weight={name}"));
    }
    if let Some(name) = style("slant", GNU_SLANT_TABLE).or(Some("normal")) {
        full_name.push_str(&format!(":slant={name}"));
    }
    full_name.push_str(&format!(
        ":width={}",
        style("width", GNU_WIDTH_TABLE).unwrap_or("normal")
    ));
    if let Some(spacing) =
        font_vector_get_flexible(&elems, "spacing").and_then(|value| match value.kind() {
            ValueKind::Fixnum(n) => Some(n),
            _ => None,
        })
    {
        full_name.push_str(&format!(":spacing={spacing}"));
    }
    full_name.push_str(":scalable=true");

    Some(Value::vector(vec![
        Value::string(opened_name),
        Value::string(full_name),
        Value::fixnum(probe.pixel_size as i64),
        Value::fixnum(probe.height as i64),
        Value::fixnum(0),
        Value::fixnum(0),
        Value::fixnum(0),
        Value::fixnum(probe.max_width as i64),
        Value::fixnum(probe.ascent as i64),
        Value::fixnum(probe.descent as i64),
        Value::fixnum(probe.space_width as i64),
        Value::fixnum(probe.average_width as i64),
        file_value,
        capability,
    ]))
}

/// `(opentype GSUB . GPOS)` for a font file, or nil when unavailable.
fn otf_capability_lisp(eval: &mut super::eval::Context, file: &str) -> Value {
    eval.display_host
        .as_mut()
        .and_then(|host| host.font_otf_capability(file, 0).ok())
        .flatten()
        .map(|caps| {
            Value::cons(
                Value::symbol("opentype"),
                Value::cons(otf_side_to_lisp(&caps.gsub), otf_side_to_lisp(&caps.gpos)),
            )
        })
        .unwrap_or(Value::NIL)
}

/// Capability for any font VALUE carrying a `:file`, else nil.
fn font_value_otf_capability(eval: &mut super::eval::Context, font_like: &Value) -> Value {
    let Some(file) = font_like
        .as_vector_data()
        .and_then(|elems| font_vector_get_flexible(elems, "file"))
        .filter(|value| value.is_string())
        .and_then(|value| value.as_utf8_str().map(|s| s.to_owned()))
    else {
        return Value::NIL;
    };
    otf_capability_lisp(eval, &file)
}

/// Lisp form of one GSUB/GPOS side: list of `(SCRIPT (LANGSYS FEATURES...)
/// ...)`, default langsys printed as `nil`; `nil` for an empty side —
/// mirroring GNU `hbfont_otf_features`.
fn otf_side_to_lisp(side: &super::eval::OtfSideCapability) -> Value {
    let scripts: Vec<Value> = side
        .iter()
        .map(|(script, lang_syses)| {
            let langsys_values: Vec<Value> = lang_syses
                .iter()
                .map(|(tag, features)| {
                    let feature_values: Vec<Value> = features
                        .iter()
                        .map(|feature| Value::from_sym_id(intern(feature)))
                        .collect();
                    Value::cons(
                        tag.as_deref()
                            .map(|tag| Value::from_sym_id(intern(tag)))
                            .unwrap_or(Value::NIL),
                        Value::list(feature_values),
                    )
                })
                .collect();
            Value::cons(
                Value::from_sym_id(intern(script)),
                Value::list(langsys_values),
            )
        })
        .collect();
    Value::list(scripts)
}

fn font_info_vector_for_runtime_font(
    font_like: &Value,
    frame: &crate::window::Frame,
    capability: Value,
) -> Value {
    let opened_name = font_name_value(font_like).unwrap_or_else(|| Value::string(""));
    let full_name = opened_name;
    let file = match font_like.kind() {
        ValueKind::Veclike(VecLikeType::Vector) if is_font(font_like) => font_like
            .as_vector_data()
            .and_then(|elems| font_vector_get_flexible(elems, "file"))
            .filter(|value| value.is_string())
            .unwrap_or(Value::NIL),
        _ => Value::NIL,
    };
    let size = frame.font_pixel_size.max(1.0).round() as i64;
    let height = frame.char_height.max(1.0).round() as i64;
    let average_width = frame.char_width.max(1.0).round() as i64;
    let space_width = average_width;
    let max_width = average_width;
    let ascent = ((height as f32) * 0.75).round() as i64;
    let descent = (height - ascent).max(0);
    let default_ascent = ascent;

    Value::vector(vec![
        opened_name,
        full_name,
        Value::fixnum(size),
        Value::fixnum(height),
        Value::fixnum(0),
        Value::fixnum(0),
        Value::fixnum(default_ascent),
        Value::fixnum(max_width),
        Value::fixnum(ascent),
        Value::fixnum(descent),
        Value::fixnum(space_width),
        Value::fixnum(average_width),
        file,
        capability,
    ])
}

fn resolve_font_match(
    eval: &mut super::eval::Context,
    frame_id: FrameId,
    character: char,
    face: &RuntimeFace,
) -> Option<super::eval::ResolvedFontMatch> {
    eval.display_host
        .as_mut()
        .and_then(|host| {
            host.resolve_font_for_char(super::eval::FontResolveRequest {
                frame_id,
                character,
                face: face.clone(),
            })
            .ok()
        })
        .flatten()
}

/// `(font-at POSITION &optional WINDOW STRING)` -- resolve the effective font
/// object for the target buffer or string position.
pub(crate) fn builtin_font_at(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("font-at", &args, 1)?;
    expect_max_args("font-at", &args, 3)?;

    let (frame_id, window_id) = resolve_live_window_for_font_at(eval, args.get(1))?;
    let (window_buffer_id, has_window_system) = {
        let frame = eval
            .frames
            .get(frame_id)
            .ok_or_else(|| signal("error", vec![Value::string("No selected frame")]))?;
        let window = frame
            .find_window(window_id)
            .ok_or_else(|| signal("error", vec![Value::string("Window not found")]))?;
        (
            window.buffer_id(),
            frame.effective_window_system().is_some(),
        )
    };

    if let Some(string_value) = args.get(2)
        && !string_value.is_nil()
    {
        if !string_value.is_string() {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("stringp"), *string_value],
            ));
        };
        let pos = match args[0].kind() {
            ValueKind::Fixnum(n) => n,
            _other => {
                return Err(signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("fixnump"), args[0]],
                ));
            }
        };
        let string = string_value
            .as_lisp_string()
            .expect("string object must carry LispString payload");
        let char_len = string.schars() as i64;
        if !(0 <= pos && pos < char_len) {
            return Err(signal(
                LispCondition::ArgsOutOfRange,
                vec![*string_value, Value::fixnum(pos)],
            ));
        }
        if !has_window_system {
            return Ok(Value::NIL);
        }
        let face_table = runtime_face_table_from_frame_lisp_faces(eval, frame_id, true);
        let char_pos = usize::try_from(pos).expect("validated non-negative string position");
        let bytepos = if string.is_multibyte() {
            crate::emacs_core::emacs_char::char_to_byte_pos(string.as_bytes(), char_pos)
        } else {
            char_pos
        };
        let face = resolved_face_at_string_char_pos(
            eval,
            &face_table,
            *string_value,
            CharPos0::new(char_pos),
        );
        let code = if string.is_multibyte() {
            crate::emacs_core::emacs_char::string_char(&string.as_bytes()[bytepos..]).0
        } else {
            string.as_bytes()[bytepos] as u32
        };
        let Some(character) = char::from_u32(code) else {
            return Ok(build_font_object(&face));
        };
        if let Some(matched) = resolve_font_match(eval, frame_id, character, &face) {
            return Ok(build_font_object_for_match(&face, &matched));
        }
        return Ok(build_font_object(&face));
    }

    let current_buffer_id = eval
        .buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    if window_buffer_id != Some(current_buffer_id) {
        return Err(signal(
            "error",
            vec![Value::string(
                "Specified window is not displaying the current buffer",
            )],
        ));
    }

    let pos =
        crate::emacs_core::builtins::expect_integer_or_marker_in_buffers(&eval.buffers, &args[0])?;
    let buffer = eval
        .buffers
        .get(current_buffer_id)
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    let beg = buffer.point_min_lisp_char_pos().as_i64();
    let end = buffer.point_max_lisp_char_pos().as_i64();
    if !(beg <= pos && pos < end) {
        return Err(signal(
            LispCondition::ArgsOutOfRange,
            vec![args[0], Value::fixnum(beg), Value::fixnum(end)],
        ));
    }

    if !has_window_system {
        return Ok(Value::NIL);
    }

    let face_table = runtime_face_table_from_frame_lisp_faces(eval, frame_id, true);
    let bytepos = buffer.lisp_pos_to_accessible_emacs_byte_pos(LispCharPos1::new(pos));
    let face = resolved_face_at_buffer_byte(eval, &face_table, buffer, bytepos);
    let character = buffer.char_at_emacs_byte_pos(bytepos).ok_or_else(|| {
        signal(
            LispCondition::ArgsOutOfRange,
            vec![args[0], Value::fixnum(beg), Value::fixnum(end)],
        )
    })?;
    if let Some(matched) = resolve_font_match(eval, frame_id, character, &face) {
        return Ok(build_font_object_for_match(&face, &matched));
    }
    Ok(build_font_object(&face))
}

/// `(internal-char-font POSITION &optional CH)` -- the `(FONT-OBJECT . GLYPH-CODE)`
/// that `describe-char` uses for its "display:" line and character-code-property
/// section. A non-nil POSITION resolves the character and face at that buffer
/// position (like `font-at`); a nil POSITION resolves CH in the default face.
/// Returns nil on a non-window frame or when no font can be found.
pub(crate) fn builtin_internal_char_font(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("internal-char-font", &args, 1)?;
    expect_max_args("internal-char-font", &args, 2)?;
    let position = args[0];
    let ch_arg = args.get(1).copied().unwrap_or(Value::NIL);

    let frame_id = super::window_cmds::ensure_selected_frame_id(eval);
    let has_window_system = eval
        .frames
        .get(frame_id)
        .is_some_and(|frame| frame.effective_window_system().is_some());

    let (character, face) = if position.is_nil() {
        let code = crate::emacs_core::builtins::expect_character_code(&ch_arg)?;
        let Some(character) = char::from_u32(code as u32) else {
            return Ok(Value::NIL);
        };
        if !has_window_system {
            return Ok(Value::NIL);
        }
        let face_table = runtime_face_table_from_frame_lisp_faces(eval, frame_id, true);
        (character, face_table.resolve("default"))
    } else {
        if !ch_arg.is_nil() {
            let _ = crate::emacs_core::builtins::expect_character_code(&ch_arg)?;
        }
        let current_buffer_id = eval
            .buffers
            .current_buffer_id()
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let pos = crate::emacs_core::builtins::expect_integer_or_marker_in_buffers(
            &eval.buffers,
            &args[0],
        )?;
        let buffer = eval
            .buffers
            .get(current_buffer_id)
            .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
        let beg = buffer.point_min_lisp_char_pos().as_i64();
        let end = buffer.point_max_lisp_char_pos().as_i64();
        if !(beg <= pos && pos < end) {
            return Err(signal(
                LispCondition::ArgsOutOfRange,
                vec![args[0], Value::fixnum(beg), Value::fixnum(end)],
            ));
        }
        if !has_window_system {
            return Ok(Value::NIL);
        }
        let face_table = runtime_face_table_from_frame_lisp_faces(eval, frame_id, true);
        let bytepos = buffer.lisp_pos_to_accessible_emacs_byte_pos(LispCharPos1::new(pos));
        let face = resolved_face_at_buffer_byte(eval, &face_table, buffer, bytepos);
        let character = buffer.char_at_emacs_byte_pos(bytepos).ok_or_else(|| {
            signal(
                LispCondition::ArgsOutOfRange,
                vec![args[0], Value::fixnum(beg), Value::fixnum(end)],
            )
        })?;
        (character, face)
    };

    let Some(matched) = resolve_font_match(eval, frame_id, character, &face) else {
        return Ok(Value::NIL);
    };
    let font_object = build_font_object_for_match(&face, &matched);
    // GNU's cdr is the font-driver glyph code; `describe-char` formats it as a
    // hex number, so fall back to 0 (the `.notdef` slot) rather than nil.
    let glyph_code = i64::from(matched.glyph_code.unwrap_or(0));
    Ok(Value::cons(font_object, Value::fixnum(glyph_code)))
}

// ===========================================================================
// Face builtins (pure)
// ===========================================================================

/// Lisp face IDs assigned during GNU Emacs `-Q` loadup.
///
/// These IDs are the symbol `face` property returned by `(face-id FACE)`.
/// They are distinct from GNU's realized display-cache `enum face_id`.
#[repr(i64)]
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    EnumString,
    IntoPrimitive,
    IntoStaticStr,
    TryFromPrimitive,
)]
#[strum(serialize_all = "kebab-case")]
enum GnuBootstrapLispFaceId {
    Default = 0,
    Bold = 1,
    Italic = 2,
    BoldItalic = 3,
    Underline = 4,
    FixedPitch = 5,
    FixedPitchSerif = 6,
    VariablePitch = 7,
    VariablePitchText = 8,
    Shadow = 9,
    Link = 10,
    LinkVisited = 11,
    Highlight = 12,
    Region = 13,
    SecondarySelection = 14,
    TrailingWhitespace = 15,
    LineNumber = 16,
    LineNumberCurrentLine = 17,
    LineNumberMajorTick = 18,
    LineNumberMinorTick = 19,
    FillColumnIndicator = 20,
    EscapeGlyph = 21,
    Homoglyph = 22,
    NobreakSpace = 23,
    NobreakHyphen = 24,
    ModeLine = 25,
    ModeLineActive = 26,
    ModeLineInactive = 27,
    ModeLineHighlight = 28,
    ModeLineEmphasis = 29,
    ModeLineBufferId = 30,
    HeaderLine = 31,
    HeaderLineHighlight = 32,
    HeaderLineActive = 33,
    HeaderLineInactive = 34,
    VerticalBorder = 35,
    WindowDivider = 36,
    WindowDividerFirstPixel = 37,
    WindowDividerLastPixel = 38,
    InternalBorder = 39,
    ChildFrameBorder = 40,
    MinibufferPrompt = 41,
    Margin = 42,
    Fringe = 43,
    ScrollBar = 44,
    Border = 45,
    Cursor = 46,
    Mouse = 47,
    ToolBar = 48,
    TabBar = 49,
    TabLine = 50,
    TabLineActive = 51,
    TabLineInactive = 52,
    Menu = 53,
    HelpArgumentName = 54,
    HelpKeyBinding = 55,
    GlyphlessChar = 56,
    Error = 57,
    Warning = 58,
    Success = 59,
    ReadMultipleChoiceFace = 60,
    TtyMenuEnabledFace = 61,
    TtyMenuDisabledFace = 62,
    TtyMenuSelectedFace = 63,
    ShowParenMatch = 64,
    ShowParenMatchExpression = 65,
    ShowParenMismatch = 66,
    Button = 67,
    AbbrevTableName = 68,
    HelpForHelpHeader = 69,
    ConfusinglyReordered = 70,
    NextError = 71,
    NextErrorMessage = 72,
    SeparatorLine = 73,
    BlinkMatchingParenOffscreen = 74,
    CompletionsGroupTitle = 75,
    CompletionsGroupSeparator = 76,
    CompletionsAnnotations = 77,
    CompletionsHighlight = 78,
    CompletionsFirstDifference = 79,
    CompletionsCommonPart = 80,
    MinibufferNonselected = 81,
    FontLockCommentFace = 82,
    FontLockCommentDelimiterFace = 83,
    FontLockStringFace = 84,
    FontLockDocFace = 85,
    FontLockDocMarkupFace = 86,
    FontLockKeywordFace = 87,
    FontLockBuiltinFace = 88,
    FontLockFunctionNameFace = 89,
    FontLockFunctionCallFace = 90,
    FontLockVariableNameFace = 91,
    FontLockVariableUseFace = 92,
    FontLockTypeFace = 93,
    FontLockConstantFace = 94,
    FontLockWarningFace = 95,
    FontLockNegationCharFace = 96,
    FontLockPreprocessorFace = 97,
    FontLockRegexpFace = 98,
    FontLockRegexpGroupingBackslash = 99,
    FontLockRegexpGroupingConstruct = 100,
    FontLockEscapeFace = 101,
    FontLockNumberFace = 102,
    FontLockOperatorFace = 103,
    FontLockPropertyNameFace = 104,
    FontLockPropertyUseFace = 105,
    FontLockPunctuationFace = 106,
    FontLockBracketFace = 107,
    FontLockDelimiterFace = 108,
    FontLockMiscPunctuationFace = 109,
    MouseDragAndDropRegion = 110,
    Isearch = 111,
    IsearchFail = 112,
    LazyHighlight = 113,
    #[strum(to_string = "isearch-group-1")]
    IsearchGroup1 = 114,
    #[strum(to_string = "isearch-group-2")]
    IsearchGroup2 = 115,
    FileNameShadow = 116,
    TabBarTab = 117,
    TabBarTabInactive = 118,
    TabBarTabGroupCurrent = 119,
    TabBarTabGroupInactive = 120,
    TabBarTabUngrouped = 121,
    TabBarTabHighlight = 122,
    QueryReplace = 123,
    Match = 124,
    TabulatedListFakeHeader = 125,
    BufferMenuBuffer = 126,
    ElispSymbolAtMouse = 127,
    ElispFreeVariable = 128,
    ElispSpecialVariableDeclaration = 129,
    ElispCondition = 130,
    ElispMajorModeName = 131,
    ElispFace = 132,
    ElispSymbolRole = 133,
    ElispSymbolRoleDefinition = 134,
    ElispFunction = 135,
    ElispNonLocalExit = 136,
    ElispUnknownCall = 137,
    ElispMacro = 138,
    ElispSpecialForm = 139,
    ElispThrowTag = 140,
    ElispFeature = 141,
    ElispRx = 142,
    ElispTheme = 143,
    ElispBindingVariable = 144,
    ElispBoundVariable = 145,
    ElispShadowingVariable = 146,
    ElispShadowedVariable = 147,
    ElispVariableAtPoint = 148,
    ElispWarningType = 149,
    ElispFunctionPropertyDeclaration = 150,
    ElispThing = 151,
    ElispSlot = 152,
    ElispWidgetType = 153,
    ElispType = 154,
    ElispGroup = 155,
    ElispNnooBackend = 156,
    ElispAmpersand = 157,
    ElispConstant = 158,
    ElispDefun = 159,
    ElispDefmacro = 160,
    ElispDefvar = 161,
    ElispDefface = 162,
    ElispIcon = 163,
    ElispDeficon = 164,
    ElispOclosure = 165,
    ElispDefoclosure = 166,
    ElispCoding = 167,
    ElispDefcoding = 168,
    ElispCharset = 169,
    ElispDefcharset = 170,
    ElispCompletionCategory = 171,
    ElispCompletionCategoryDefinition = 172,
    VcStateBase = 173,
    VcUpToDateState = 174,
    VcNeedsUpdateState = 175,
    VcLockedState = 176,
    VcLocallyAddedState = 177,
    VcConflictState = 178,
    VcRemovedState = 179,
    VcMissingState = 180,
    VcEditedState = 181,
    VcIgnoredState = 182,
    ElispShorthandFontLockFace = 183,
    EldocHighlightFunctionArgument = 184,
    Tooltip = 185,
}

const GNU_BOOTSTRAP_LISP_FACES: &[GnuBootstrapLispFaceId] = &[
    GnuBootstrapLispFaceId::Default,
    GnuBootstrapLispFaceId::Bold,
    GnuBootstrapLispFaceId::Italic,
    GnuBootstrapLispFaceId::BoldItalic,
    GnuBootstrapLispFaceId::Underline,
    GnuBootstrapLispFaceId::FixedPitch,
    GnuBootstrapLispFaceId::FixedPitchSerif,
    GnuBootstrapLispFaceId::VariablePitch,
    GnuBootstrapLispFaceId::VariablePitchText,
    GnuBootstrapLispFaceId::Shadow,
    GnuBootstrapLispFaceId::Link,
    GnuBootstrapLispFaceId::LinkVisited,
    GnuBootstrapLispFaceId::Highlight,
    GnuBootstrapLispFaceId::Region,
    GnuBootstrapLispFaceId::SecondarySelection,
    GnuBootstrapLispFaceId::TrailingWhitespace,
    GnuBootstrapLispFaceId::LineNumber,
    GnuBootstrapLispFaceId::LineNumberCurrentLine,
    GnuBootstrapLispFaceId::LineNumberMajorTick,
    GnuBootstrapLispFaceId::LineNumberMinorTick,
    GnuBootstrapLispFaceId::FillColumnIndicator,
    GnuBootstrapLispFaceId::EscapeGlyph,
    GnuBootstrapLispFaceId::Homoglyph,
    GnuBootstrapLispFaceId::NobreakSpace,
    GnuBootstrapLispFaceId::NobreakHyphen,
    GnuBootstrapLispFaceId::ModeLine,
    GnuBootstrapLispFaceId::ModeLineActive,
    GnuBootstrapLispFaceId::ModeLineInactive,
    GnuBootstrapLispFaceId::ModeLineHighlight,
    GnuBootstrapLispFaceId::ModeLineEmphasis,
    GnuBootstrapLispFaceId::ModeLineBufferId,
    GnuBootstrapLispFaceId::HeaderLine,
    GnuBootstrapLispFaceId::HeaderLineHighlight,
    GnuBootstrapLispFaceId::HeaderLineActive,
    GnuBootstrapLispFaceId::HeaderLineInactive,
    GnuBootstrapLispFaceId::VerticalBorder,
    GnuBootstrapLispFaceId::WindowDivider,
    GnuBootstrapLispFaceId::WindowDividerFirstPixel,
    GnuBootstrapLispFaceId::WindowDividerLastPixel,
    GnuBootstrapLispFaceId::InternalBorder,
    GnuBootstrapLispFaceId::ChildFrameBorder,
    GnuBootstrapLispFaceId::MinibufferPrompt,
    GnuBootstrapLispFaceId::Margin,
    GnuBootstrapLispFaceId::Fringe,
    GnuBootstrapLispFaceId::ScrollBar,
    GnuBootstrapLispFaceId::Border,
    GnuBootstrapLispFaceId::Cursor,
    GnuBootstrapLispFaceId::Mouse,
    GnuBootstrapLispFaceId::ToolBar,
    GnuBootstrapLispFaceId::TabBar,
    GnuBootstrapLispFaceId::TabLine,
    GnuBootstrapLispFaceId::TabLineActive,
    GnuBootstrapLispFaceId::TabLineInactive,
    GnuBootstrapLispFaceId::Menu,
    GnuBootstrapLispFaceId::HelpArgumentName,
    GnuBootstrapLispFaceId::HelpKeyBinding,
    GnuBootstrapLispFaceId::GlyphlessChar,
    GnuBootstrapLispFaceId::Error,
    GnuBootstrapLispFaceId::Warning,
    GnuBootstrapLispFaceId::Success,
    GnuBootstrapLispFaceId::ReadMultipleChoiceFace,
    GnuBootstrapLispFaceId::TtyMenuEnabledFace,
    GnuBootstrapLispFaceId::TtyMenuDisabledFace,
    GnuBootstrapLispFaceId::TtyMenuSelectedFace,
    GnuBootstrapLispFaceId::ShowParenMatch,
    GnuBootstrapLispFaceId::ShowParenMatchExpression,
    GnuBootstrapLispFaceId::ShowParenMismatch,
    GnuBootstrapLispFaceId::Button,
    GnuBootstrapLispFaceId::AbbrevTableName,
    GnuBootstrapLispFaceId::HelpForHelpHeader,
    GnuBootstrapLispFaceId::ConfusinglyReordered,
    GnuBootstrapLispFaceId::NextError,
    GnuBootstrapLispFaceId::NextErrorMessage,
    GnuBootstrapLispFaceId::SeparatorLine,
    GnuBootstrapLispFaceId::BlinkMatchingParenOffscreen,
    GnuBootstrapLispFaceId::CompletionsGroupTitle,
    GnuBootstrapLispFaceId::CompletionsGroupSeparator,
    GnuBootstrapLispFaceId::CompletionsAnnotations,
    GnuBootstrapLispFaceId::CompletionsHighlight,
    GnuBootstrapLispFaceId::CompletionsFirstDifference,
    GnuBootstrapLispFaceId::CompletionsCommonPart,
    GnuBootstrapLispFaceId::MinibufferNonselected,
    GnuBootstrapLispFaceId::FontLockCommentFace,
    GnuBootstrapLispFaceId::FontLockCommentDelimiterFace,
    GnuBootstrapLispFaceId::FontLockStringFace,
    GnuBootstrapLispFaceId::FontLockDocFace,
    GnuBootstrapLispFaceId::FontLockDocMarkupFace,
    GnuBootstrapLispFaceId::FontLockKeywordFace,
    GnuBootstrapLispFaceId::FontLockBuiltinFace,
    GnuBootstrapLispFaceId::FontLockFunctionNameFace,
    GnuBootstrapLispFaceId::FontLockFunctionCallFace,
    GnuBootstrapLispFaceId::FontLockVariableNameFace,
    GnuBootstrapLispFaceId::FontLockVariableUseFace,
    GnuBootstrapLispFaceId::FontLockTypeFace,
    GnuBootstrapLispFaceId::FontLockConstantFace,
    GnuBootstrapLispFaceId::FontLockWarningFace,
    GnuBootstrapLispFaceId::FontLockNegationCharFace,
    GnuBootstrapLispFaceId::FontLockPreprocessorFace,
    GnuBootstrapLispFaceId::FontLockRegexpFace,
    GnuBootstrapLispFaceId::FontLockRegexpGroupingBackslash,
    GnuBootstrapLispFaceId::FontLockRegexpGroupingConstruct,
    GnuBootstrapLispFaceId::FontLockEscapeFace,
    GnuBootstrapLispFaceId::FontLockNumberFace,
    GnuBootstrapLispFaceId::FontLockOperatorFace,
    GnuBootstrapLispFaceId::FontLockPropertyNameFace,
    GnuBootstrapLispFaceId::FontLockPropertyUseFace,
    GnuBootstrapLispFaceId::FontLockPunctuationFace,
    GnuBootstrapLispFaceId::FontLockBracketFace,
    GnuBootstrapLispFaceId::FontLockDelimiterFace,
    GnuBootstrapLispFaceId::FontLockMiscPunctuationFace,
    GnuBootstrapLispFaceId::MouseDragAndDropRegion,
    GnuBootstrapLispFaceId::Isearch,
    GnuBootstrapLispFaceId::IsearchFail,
    GnuBootstrapLispFaceId::LazyHighlight,
    GnuBootstrapLispFaceId::IsearchGroup1,
    GnuBootstrapLispFaceId::IsearchGroup2,
    GnuBootstrapLispFaceId::FileNameShadow,
    GnuBootstrapLispFaceId::TabBarTab,
    GnuBootstrapLispFaceId::TabBarTabInactive,
    GnuBootstrapLispFaceId::TabBarTabGroupCurrent,
    GnuBootstrapLispFaceId::TabBarTabGroupInactive,
    GnuBootstrapLispFaceId::TabBarTabUngrouped,
    GnuBootstrapLispFaceId::TabBarTabHighlight,
    GnuBootstrapLispFaceId::QueryReplace,
    GnuBootstrapLispFaceId::Match,
    GnuBootstrapLispFaceId::TabulatedListFakeHeader,
    GnuBootstrapLispFaceId::BufferMenuBuffer,
    GnuBootstrapLispFaceId::ElispSymbolAtMouse,
    GnuBootstrapLispFaceId::ElispFreeVariable,
    GnuBootstrapLispFaceId::ElispSpecialVariableDeclaration,
    GnuBootstrapLispFaceId::ElispCondition,
    GnuBootstrapLispFaceId::ElispMajorModeName,
    GnuBootstrapLispFaceId::ElispFace,
    GnuBootstrapLispFaceId::ElispSymbolRole,
    GnuBootstrapLispFaceId::ElispSymbolRoleDefinition,
    GnuBootstrapLispFaceId::ElispFunction,
    GnuBootstrapLispFaceId::ElispNonLocalExit,
    GnuBootstrapLispFaceId::ElispUnknownCall,
    GnuBootstrapLispFaceId::ElispMacro,
    GnuBootstrapLispFaceId::ElispSpecialForm,
    GnuBootstrapLispFaceId::ElispThrowTag,
    GnuBootstrapLispFaceId::ElispFeature,
    GnuBootstrapLispFaceId::ElispRx,
    GnuBootstrapLispFaceId::ElispTheme,
    GnuBootstrapLispFaceId::ElispBindingVariable,
    GnuBootstrapLispFaceId::ElispBoundVariable,
    GnuBootstrapLispFaceId::ElispShadowingVariable,
    GnuBootstrapLispFaceId::ElispShadowedVariable,
    GnuBootstrapLispFaceId::ElispVariableAtPoint,
    GnuBootstrapLispFaceId::ElispWarningType,
    GnuBootstrapLispFaceId::ElispFunctionPropertyDeclaration,
    GnuBootstrapLispFaceId::ElispThing,
    GnuBootstrapLispFaceId::ElispSlot,
    GnuBootstrapLispFaceId::ElispWidgetType,
    GnuBootstrapLispFaceId::ElispType,
    GnuBootstrapLispFaceId::ElispGroup,
    GnuBootstrapLispFaceId::ElispNnooBackend,
    GnuBootstrapLispFaceId::ElispAmpersand,
    GnuBootstrapLispFaceId::ElispConstant,
    GnuBootstrapLispFaceId::ElispDefun,
    GnuBootstrapLispFaceId::ElispDefmacro,
    GnuBootstrapLispFaceId::ElispDefvar,
    GnuBootstrapLispFaceId::ElispDefface,
    GnuBootstrapLispFaceId::ElispIcon,
    GnuBootstrapLispFaceId::ElispDeficon,
    GnuBootstrapLispFaceId::ElispOclosure,
    GnuBootstrapLispFaceId::ElispDefoclosure,
    GnuBootstrapLispFaceId::ElispCoding,
    GnuBootstrapLispFaceId::ElispDefcoding,
    GnuBootstrapLispFaceId::ElispCharset,
    GnuBootstrapLispFaceId::ElispDefcharset,
    GnuBootstrapLispFaceId::ElispCompletionCategory,
    GnuBootstrapLispFaceId::ElispCompletionCategoryDefinition,
    GnuBootstrapLispFaceId::VcStateBase,
    GnuBootstrapLispFaceId::VcUpToDateState,
    GnuBootstrapLispFaceId::VcNeedsUpdateState,
    GnuBootstrapLispFaceId::VcLockedState,
    GnuBootstrapLispFaceId::VcLocallyAddedState,
    GnuBootstrapLispFaceId::VcConflictState,
    GnuBootstrapLispFaceId::VcRemovedState,
    GnuBootstrapLispFaceId::VcMissingState,
    GnuBootstrapLispFaceId::VcEditedState,
    GnuBootstrapLispFaceId::VcIgnoredState,
    GnuBootstrapLispFaceId::ElispShorthandFontLockFace,
    GnuBootstrapLispFaceId::EldocHighlightFunctionArgument,
    GnuBootstrapLispFaceId::Tooltip,
];

const FIRST_DYNAMIC_FACE_ID: i64 = 186;

impl GnuBootstrapLispFaceId {
    fn from_name(name: &str) -> Option<Self> {
        // `name.parse()` (strum `FromStr`) is a linear scan over all ~183
        // variants.  Doom sets hundreds of mostly-non-builtin faces, so every
        // lookup scanned -- and failed -- the whole list (a startup hot spot).
        // Build the name->variant map once and look up in O(1).
        static BY_NAME: OnceLock<rustc_hash::FxHashMap<&'static str, GnuBootstrapLispFaceId>> =
            OnceLock::new();
        BY_NAME
            .get_or_init(|| {
                GnuBootstrapLispFaceId::iter()
                    .map(|variant| (<&'static str>::from(variant), variant))
                    .collect()
            })
            .get(name)
            .copied()
    }

    fn id(self) -> i64 {
        self.into()
    }

    fn name(self) -> &'static str {
        self.into()
    }
}

fn is_known_lisp_face_name(name: &str) -> bool {
    GnuBootstrapLispFaceId::from_name(name).is_some()
}

fn known_face_id(name: &str) -> Option<i64> {
    GnuBootstrapLispFaceId::from_name(name).map(GnuBootstrapLispFaceId::id)
}

const LISP_FACE_VECTOR_LEN: usize = LFACE_VECTOR_SIZE;

#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::EnumString, strum::IntoStaticStr)]
#[strum(prefix = ":", serialize_all = "kebab-case")]
enum SetFaceAttrAlias {
    Bold,
    Italic,
}

impl SetFaceAttrAlias {
    fn from_keyword(name: &str) -> Option<Self> {
        name.strip_prefix(':')?.parse().ok()
    }

    fn keyword(self) -> &'static str {
        self.into()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SetFaceAttr {
    LFace(LFaceAttr),
    Alias(SetFaceAttrAlias),
}

impl SetFaceAttr {
    fn from_keyword(name: &str) -> Option<Self> {
        LFaceAttr::from_keyword(name)
            .map(Self::LFace)
            .or_else(|| SetFaceAttrAlias::from_keyword(name).map(Self::Alias))
    }
}

fn valid_face_weight_symbol(name: &str) -> bool {
    FontWeight::from_symbol(name).is_some()
}

fn valid_face_slant_symbol(name: &str) -> bool {
    FontSlant::from_symbol(name).is_some()
}

fn valid_face_width_symbol(name: &str) -> bool {
    FontWidth::from_symbol(name).is_some()
}

fn non_empty_lisp_string(value: Value) -> bool {
    value
        .as_lisp_string()
        .is_some_and(|string| !string.is_empty())
}

fn valid_face_underline_value(value: Value) -> bool {
    if value.is_nil() || value == Value::T {
        return true;
    }
    if matches!(value.kind(), ValueKind::String) {
        return non_empty_lisp_string(value);
    }
    if !value.is_cons() {
        return false;
    }

    let mut list = value;
    while list.is_cons() {
        let key = list.cons_car();
        if key.is_nil() {
            break;
        }
        list = list.cons_cdr();
        let val = if list.is_cons() {
            let value = list.cons_car();
            list = list.cons_cdr();
            value
        } else {
            Value::NIL
        };

        if key.is_nil() || (val.is_nil() && !key.is_symbol_named(":position")) {
            return false;
        }
        if key.is_symbol_named(":color")
            && !(val.is_symbol_named("foreground-color") || non_empty_lisp_string(val))
        {
            return false;
        }
        if key.is_symbol_named(":style")
            && val
                .as_symbol_name()
                .and_then(UnderlineStyle::from_symbol)
                .is_none()
        {
            return false;
        }
    }
    true
}

fn valid_box_line_width(value: Value) -> bool {
    if let Some(width) = value.as_fixnum() {
        return width != 0;
    }
    value.is_cons()
        && value.cons_car().as_fixnum().is_some_and(|width| width != 0)
        && value.cons_cdr().as_fixnum().is_some_and(|width| width != 0)
}

fn valid_face_box_value(value: Value) -> bool {
    if value == Value::T || value.is_nil() {
        return true;
    }
    if let Some(width) = value.as_fixnum() {
        return width != 0;
    }
    if matches!(value.kind(), ValueKind::String) {
        return non_empty_lisp_string(value);
    }
    if value.is_cons() && value.cons_car().is_fixnum() && value.cons_cdr().is_fixnum() {
        return true;
    }
    if !value.is_cons() {
        return false;
    }

    let mut list = value;
    while !list.is_nil() {
        if !list.is_cons() {
            return false;
        }
        let key = list.cons_car();
        list = list.cons_cdr();
        if !list.is_cons() {
            return false;
        }
        let val = list.cons_car();
        list = list.cons_cdr();

        if key.is_symbol_named(":line-width") {
            if !valid_box_line_width(val) {
                return false;
            }
        } else if key.is_symbol_named(":color") {
            if !val.is_nil() && !non_empty_lisp_string(val) {
                return false;
            }
        } else if key.is_symbol_named(":style") {
            if !val.is_nil()
                && val
                    .as_symbol_name()
                    .and_then(BoxStyle::from_symbol)
                    .is_none()
            {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

#[derive(Default)]
struct FaceAttrState {
    selected_created: HashSet<SymId>,
    selected_overrides: HashMap<SymId, HashMap<LFaceAttr, Value>>,
    defaults_overrides: HashMap<SymId, HashMap<LFaceAttr, Value>>,
}

thread_local! {
    static CREATED_LISP_FACES: RefCell<HashSet<SymId>> = RefCell::new(HashSet::new());
    static CREATED_FACE_IDS: RefCell<HashMap<SymId, i64>> = RefCell::new(HashMap::new());
    static NEXT_CREATED_FACE_ID: RefCell<i64> = const { RefCell::new(FIRST_DYNAMIC_FACE_ID) };
    static FACE_ATTR_STATE: RefCell<FaceAttrState> = RefCell::new(FaceAttrState::default());
    /// Generation counter bumped whenever the defined-face set
    /// (`CREATED_LISP_FACES`) changes.  Keys `FACE_NAME_LIST_CACHE`.
    static FACE_SET_GENERATION: Cell<u64> = const { Cell::new(0) };
    /// Cached sorted face-name list, valid while `FACE_SET_GENERATION` is
    /// unchanged.  Doom calls `face-list` and seeds face tables hundreds of
    /// times during startup with an unchanging face set; recomputing the sort
    /// (with a per-comparison `face_id_for_name`) each time dominated the face
    /// path in the startup profile.
    static FACE_NAME_LIST_CACHE: RefCell<Option<(u64, Rc<[String]>)>> = const { RefCell::new(None) };
}

/// Invalidate the cached face-name list after the defined-face set changes.
fn bump_face_set_generation() {
    FACE_SET_GENERATION.with(|generation| generation.set(generation.get().wrapping_add(1)));
}

fn face_symbol_id(name: &str) -> SymId {
    intern(name)
}

fn face_attr_id(name: &str) -> SymId {
    intern(name)
}

pub(crate) fn clear_font_cache_state() {
    CREATED_LISP_FACES.with(|slot| slot.borrow_mut().clear());
    CREATED_FACE_IDS.with(|slot| slot.borrow_mut().clear());
    NEXT_CREATED_FACE_ID.with(|slot| *slot.borrow_mut() = FIRST_DYNAMIC_FACE_ID);
    FACE_ATTR_STATE.with(|slot| *slot.borrow_mut() = FaceAttrState::default());
    bump_face_set_generation();
}

/// Collect GC roots from face attribute overrides.
pub(crate) fn collect_font_gc_roots(roots: &mut Vec<Value>) {
    FACE_ATTR_STATE.with(|slot| {
        let state = slot.borrow();
        for attrs in state.selected_overrides.values() {
            roots.extend(attrs.values().copied());
        }
        for attrs in state.defaults_overrides.values() {
            roots.extend(attrs.values().copied());
        }
    });
}

fn is_created_lisp_face(name: &str) -> bool {
    CREATED_LISP_FACES.with(|slot| slot.borrow().contains(&face_symbol_id(name)))
}

/// Restore the `CREATED_LISP_FACES` set from an evaluator's face table.
/// Called after pdump load to re-populate the thread-local face name set
/// that was lost during serialization.
pub(crate) fn restore_created_faces_from_table(face_names: &[String]) {
    CREATED_LISP_FACES.with(|slot| {
        let mut set = slot.borrow_mut();
        for name in face_names {
            if !is_known_lisp_face_name(name) {
                set.insert(face_symbol_id(name));
            }
        }
    });
    bump_face_set_generation();
}

fn mark_created_lisp_face(name: &str) {
    let inserted = CREATED_LISP_FACES.with(|slot| slot.borrow_mut().insert(face_symbol_id(name)));
    if inserted {
        ensure_dynamic_face_id(name);
        bump_face_set_generation();
    }
}

pub(crate) fn ensure_lisp_face_id_property(
    eval: &mut super::eval::Context,
    face_name: &str,
) -> Result<(), Flow> {
    ensure_dynamic_face_id(face_name);
    if let Some(face_id) = face_id_for_name(face_name) {
        eval.obarray_mut()
            .put_property(face_name, "face", Value::fixnum(face_id))?;
    }
    Ok(())
}

fn ensure_dynamic_face_id(name: &str) {
    if known_face_id(name).is_some() {
        return;
    }
    let face = face_symbol_id(name);
    CREATED_FACE_IDS.with(|slot| {
        let mut ids = slot.borrow_mut();
        if ids.contains_key(&face) {
            return;
        }
        NEXT_CREATED_FACE_ID.with(|next_slot| {
            let mut next = next_slot.borrow_mut();
            ids.insert(face, *next);
            *next += 1;
        });
    });
}

fn dynamic_face_id(name: &str) -> Option<i64> {
    CREATED_FACE_IDS.with(|slot| slot.borrow().get(&face_symbol_id(name)).copied())
}

pub(crate) fn face_id_for_name(name: &str) -> Option<i64> {
    if let Some(id) = known_face_id(name) {
        return Some(id);
    }
    if is_known_lisp_face_name(name) {
        ensure_dynamic_face_id(name);
    }
    dynamic_face_id(name)
}

pub(crate) fn all_defined_face_names_sorted_by_id_desc() -> Rc<[String]> {
    let generation = FACE_SET_GENERATION.with(|generation| generation.get());
    if let Some(cached) = FACE_NAME_LIST_CACHE.with(|cache| {
        cache
            .borrow()
            .as_ref()
            .filter(|(cached_generation, _)| *cached_generation == generation)
            .map(|(_, names)| Rc::clone(names))
    }) {
        return cached;
    }

    let names: Rc<[String]> = Rc::from(compute_face_names_sorted_by_id_desc());
    FACE_NAME_LIST_CACHE.with(|cache| {
        *cache.borrow_mut() = Some((generation, Rc::clone(&names)));
    });
    names
}

fn compute_face_names_sorted_by_id_desc() -> Vec<String> {
    // Dedup by interned symbol id (O(n)) rather than a linear string scan
    // (O(n^2)).  Bootstrap and created faces share the global obarray, so equal
    // names map to the same `SymId`.
    let mut seen: HashSet<SymId> = HashSet::new();
    let mut names: Vec<String> = Vec::new();
    for face in GNU_BOOTSTRAP_LISP_FACES.iter() {
        let name = face.name();
        if seen.insert(face_symbol_id(name)) {
            names.push(name.to_string());
        }
    }
    CREATED_LISP_FACES.with(|slot| {
        for symbol in slot.borrow().iter() {
            if seen.insert(*symbol) {
                names.push(resolve_sym(*symbol).to_string());
            }
        }
    });
    // Decorate-sort-undecorate: resolve each face id once (O(n)) instead of
    // inside the comparator (O(n log n) `face_id_for_name` lookups, which
    // dominated the face path in the startup profile).
    let mut keyed: Vec<(i64, String)> = names
        .into_iter()
        .map(|name| (face_id_for_name(&name).unwrap_or(i64::MAX), name))
        .collect();
    keyed.sort_by(|(left_id, left_name), (right_id, right_name)| {
        right_id
            .cmp(left_id)
            .then_with(|| left_name.cmp(right_name))
    });
    keyed.into_iter().map(|(_, name)| name).collect()
}

fn is_selected_created_lisp_face(name: &str) -> bool {
    FACE_ATTR_STATE.with(|slot| {
        slot.borrow()
            .selected_created
            .contains(&face_symbol_id(name))
    })
}

fn mark_selected_created_lisp_face(name: &str) {
    FACE_ATTR_STATE.with(|slot| {
        slot.borrow_mut()
            .selected_created
            .insert(face_symbol_id(name));
    });
}

fn face_exists_for_domain(name: &str, defaults_frame: bool) -> bool {
    if is_known_lisp_face_name(name) {
        return true;
    }
    // A face created via defface/internal-make-lisp-face exists for all
    // domains. GNU Emacs uses a single hash table for face lookup —
    // there is no distinction between "defaults" and "selected" existence.
    if is_created_lisp_face(name) {
        return true;
    }
    if !defaults_frame {
        is_selected_created_lisp_face(name)
    } else {
        false
    }
}

fn get_face_override(face_name: &str, attr: LFaceAttr, defaults_frame: bool) -> Option<Value> {
    let face = face_symbol_id(face_name);
    FACE_ATTR_STATE.with(|slot| {
        let state = slot.borrow();
        let map = if defaults_frame {
            &state.defaults_overrides
        } else {
            &state.selected_overrides
        };
        map.get(&face).and_then(|attrs| attrs.get(&attr)).copied()
    })
}

fn set_face_override(face_name: &str, attr: LFaceAttr, value: Value, defaults_frame: bool) {
    let face = face_symbol_id(face_name);
    FACE_ATTR_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        let map = if defaults_frame {
            &mut state.defaults_overrides
        } else {
            &mut state.selected_overrides
        };
        map.entry(face).or_default().insert(attr, value);
    });
}

fn clear_face_overrides(face_name: &str, defaults_frame: bool) {
    let face = face_symbol_id(face_name);
    FACE_ATTR_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        if defaults_frame {
            state.defaults_overrides.remove(&face);
        } else {
            state.selected_overrides.remove(&face);
        }
    });
}

pub(crate) fn clear_created_lisp_face(name: &str) {
    let face = face_symbol_id(name);
    CREATED_LISP_FACES.with(|slot| {
        slot.borrow_mut().remove(&face);
    });
    FACE_ATTR_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        state.selected_created.remove(&face);
        state.defaults_overrides.remove(&face);
        state.selected_overrides.remove(&face);
    });
}

fn merge_defaults_overrides_into_selected(face_name: &str) {
    let face = face_symbol_id(face_name);
    FACE_ATTR_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        let defaults = state.defaults_overrides.get(&face).cloned();
        if let Some(attrs) = defaults {
            let selected = state.selected_overrides.entry(face).or_default();
            for (attr, value) in attrs {
                if value.is_symbol_named("unspecified") || value.is_symbol_named("relative") {
                    continue;
                }
                selected.insert(attr, value);
            }
        }
    });
}

fn symbol_name_for_face_value(face: &Value) -> Option<String> {
    match face.kind() {
        ValueKind::Nil => Some("nil".to_string()),
        ValueKind::T => Some("t".to_string()),
        ValueKind::Symbol(id) => Some(resolve_sym(id).to_owned()),
        _ => None,
    }
}

fn require_symbol_face_name(face: &Value) -> Result<String, Flow> {
    symbol_name_for_face_value(face).ok_or_else(|| {
        signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("symbolp"), *face],
        )
    })
}

/// An interned face name after following every `face-alias` edge.
///
/// Keeping this distinct from an arbitrary Lisp `Value` prevents face-table
/// callers from accidentally looking up an unresolved alias.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResolvedFaceName(SymId);

impl ResolvedFaceName {
    fn from_symbol(value: Value) -> Option<Self> {
        value.as_symbol_id().map(Self)
    }

    fn symbol(self) -> Value {
        Value::from_sym_id(self.0)
    }

    fn name(self) -> &'static str {
        resolve_sym(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResolvedFaceDesignator {
    Symbol(ResolvedFaceName),
    String(ResolvedFaceName),
    Other(Value),
}

impl ResolvedFaceDesignator {
    fn name(self) -> Option<ResolvedFaceName> {
        match self {
            Self::Symbol(name) | Self::String(name) => Some(name),
            Self::Other(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FaceDesignatorKind {
    Symbol,
    String,
}

impl FaceDesignatorKind {
    fn resolved(self, name: ResolvedFaceName) -> ResolvedFaceDesignator {
        match self {
            Self::Symbol => ResolvedFaceDesignator::Symbol(name),
            Self::String => ResolvedFaceDesignator::String(name),
        }
    }
}

/// GNU's `resolve_face_name` uses two different cycle contracts: predicates and
/// attribute access signal, while create-on-miss paths fall back to `default`.
/// Make callers choose instead of hiding that semantic difference in a bool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FaceAliasCyclePolicy {
    Signal,
    UseDefault,
}

fn face_alias_target(
    eval: &super::eval::Context,
    face: ResolvedFaceName,
) -> Option<ResolvedFaceName> {
    if face.symbol().is_nil() {
        return None;
    }
    let target = eval.obarray().get_property(face.name(), "face-alias")?;
    if target.is_nil() {
        return None;
    }
    ResolvedFaceName::from_symbol(target)
}

/// Follow `face-alias` properties exactly like GNU xfaces.c
/// `resolve_face_name`, including constant-space cycle detection.
fn resolve_face_designator(
    eval: &super::eval::Context,
    face: Value,
    cycle_policy: FaceAliasCyclePolicy,
) -> Result<ResolvedFaceDesignator, Flow> {
    let (kind, origin) = match face.kind() {
        ValueKind::String => {
            let name = font_string_text(&face).expect("checked string");
            (
                FaceDesignatorKind::String,
                ResolvedFaceName(
                    Value::symbol(&name)
                        .as_symbol_id()
                        .expect("interned face name must be a symbol"),
                ),
            )
        }
        _ => {
            let Some(name) = ResolvedFaceName::from_symbol(face) else {
                return Ok(ResolvedFaceDesignator::Other(face));
            };
            (FaceDesignatorKind::Symbol, name)
        }
    };

    let mut tortoise = origin;
    let mut hare = origin;
    loop {
        let face_name = hare;
        let Some(first_hop) = face_alias_target(eval, hare) else {
            return Ok(kind.resolved(face_name));
        };

        let face_name = first_hop;
        let Some(second_hop) = face_alias_target(eval, first_hop) else {
            return Ok(kind.resolved(face_name));
        };

        hare = second_hop;
        tortoise = face_alias_target(eval, tortoise)
            .expect("hare cannot advance twice unless tortoise can advance once");
        if hare == tortoise {
            return match cycle_policy {
                FaceAliasCyclePolicy::Signal => {
                    Err(signal(LispCondition::CircularList, vec![origin.symbol()]))
                }
                FaceAliasCyclePolicy::UseDefault => Ok(kind.resolved(ResolvedFaceName(
                    Value::symbol("default")
                        .as_symbol_id()
                        .expect("default must be an interned symbol"),
                ))),
            };
        }
    }
}

fn known_resolved_face_name(resolved: ResolvedFaceDesignator) -> Option<ResolvedFaceName> {
    let name = resolved.name()?;
    if is_known_lisp_face_name(name.name()) || is_created_lisp_face(name.name()) {
        Some(name)
    } else {
        None
    }
}

fn resolve_copy_source_face_symbol(
    eval: &super::eval::Context,
    face: &Value,
) -> Result<String, Flow> {
    let _ = require_symbol_face_name(face)?;
    let name = resolve_face_designator(eval, *face, FaceAliasCyclePolicy::Signal)?
        .name()
        .expect("a required symbol resolves to a named face");
    if is_known_lisp_face_name(name.name()) || is_created_lisp_face(name.name()) {
        return Ok(name.name().to_owned());
    }
    Err(invalid_face_error(*face))
}

fn resolve_face_name_for_domain(
    eval: &super::eval::Context,
    face: &Value,
    defaults_frame: bool,
) -> Result<String, Flow> {
    match resolve_face_designator(eval, *face, FaceAliasCyclePolicy::Signal)? {
        ResolvedFaceDesignator::String(name) => {
            if face_exists_for_domain(name.name(), defaults_frame) {
                Err(signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("symbolp"), *face],
                ))
            } else {
                Err(signal(
                    "error",
                    vec![Value::string("Invalid face"), Value::symbol(name.name())],
                ))
            }
        }
        ResolvedFaceDesignator::Symbol(name) => {
            if face_exists_for_domain(name.name(), defaults_frame) {
                Ok(name.name().to_owned())
            } else {
                Err(invalid_face_error(*face))
            }
        }
        ResolvedFaceDesignator::Other(_) => Err(invalid_face_error(*face)),
    }
}

fn resolve_face_name_for_merge(eval: &super::eval::Context, face: &Value) -> Result<String, Flow> {
    match resolve_face_designator(eval, *face, FaceAliasCyclePolicy::Signal)? {
        ResolvedFaceDesignator::String(name) => {
            if face_exists_for_domain(name.name(), true) {
                Ok(name.name().to_owned())
            } else {
                Err(signal(
                    "error",
                    vec![Value::string("Invalid face"), Value::symbol(name.name())],
                ))
            }
        }
        ResolvedFaceDesignator::Symbol(name) => {
            if face_exists_for_domain(name.name(), true) {
                Ok(name.name().to_owned())
            } else {
                Err(invalid_face_error(*face))
            }
        }
        ResolvedFaceDesignator::Other(_) => Err(invalid_face_error(*face)),
    }
}

fn invalid_face_error(face: Value) -> Flow {
    let mut data = vec![Value::string("Invalid face")];
    if let Some(items) = list_to_vec(&face) {
        data.extend(items);
    } else {
        data.push(face);
    }
    signal("error", data)
}

/// The `unspecified' symbol used to fill empty Lisp face slots.  GNU keeps it
/// as the staticpro'd `Qunspecified' and never re-interns it; cache the
/// interned `SymId' once so realising a face does not re-intern "unspecified"
/// for every slot.  `make_lisp_face_vector' fills ~30 slots and Doom realises
/// hundreds of faces, so this was ~5% of startup CPU.  Safe at runtime: the
/// cache is first populated after the pdump is loaded (same pattern as
/// `cached_symbol_id!' in eval.rs).
fn unspecified_face_symbol() -> Value {
    static ID: OnceLock<SymId> = OnceLock::new();
    Value::from_sym_id(*ID.get_or_init(|| intern("unspecified")))
}

/// The leading `face' tag symbol stored in slot 0 of a Lisp face vector.
fn face_tag_symbol() -> Value {
    static ID: OnceLock<SymId> = OnceLock::new();
    Value::from_sym_id(*ID.get_or_init(|| intern("face")))
}

pub(crate) fn make_lisp_face_vector() -> Value {
    let unspecified = unspecified_face_symbol();
    let mut values = Vec::with_capacity(LISP_FACE_VECTOR_LEN);
    values.push(face_tag_symbol());
    values.extend((1..LISP_FACE_VECTOR_LEN).map(|_| unspecified));
    Value::vector(values)
}

fn reset_lisp_face_vector(vector: Value) {
    let unspecified = unspecified_face_symbol();
    let _ = vector.with_vector_data_mut(|slots| {
        if slots.len() != LISP_FACE_VECTOR_LEN {
            *slots = vec![unspecified; LISP_FACE_VECTOR_LEN];
        }
        store_value_atomic(&mut slots[0], face_tag_symbol());
        for slot in slots.iter_mut().take(LISP_FACE_VECTOR_LEN).skip(1) {
            store_value_atomic(slot, unspecified);
        }
    });
}

fn copy_lisp_face_vector_slots(from: Value, to: Value) {
    let Some(source) = from.as_vector_data() else {
        return;
    };
    let _ = to.replace_vector_data(source.clone());
}

fn lisp_face_vector_attr(vector: Value, attr: LFaceAttr) -> Option<Value> {
    vector
        .as_vector_data()
        .and_then(|slots| slots.get(attr as usize).copied())
}

fn set_lisp_face_vector_attr(vector: Value, attr: LFaceAttr, value: Value) {
    let _ = vector.set_vector_slot(attr as usize, value);
}

fn set_lisp_face_vector_attr_with_font_derivatives(
    face_name: &str,
    vector: Value,
    attr: LFaceAttr,
    attr_value: Value,
    font_derivation_value: Value,
) -> Result<(), Flow> {
    set_lisp_face_vector_attr(vector, attr, attr_value);
    if attr == LFaceAttr::Font && !is_reset_like_face_attr_value(&attr_value) {
        for (derived_attr, derived_value) in
            derived_face_attrs_from_font_value(&font_derivation_value)
        {
            let (canonical_attr, canonical_value) = normalize_face_attr_for_set(
                face_name,
                SetFaceAttr::LFace(derived_attr),
                derived_value,
            )?;
            set_lisp_face_vector_attr(vector, canonical_attr, canonical_value);
        }
    }
    Ok(())
}

fn sync_face_overrides_from_lisp_face_vector(face_name: &str, vector: Value, defaults_frame: bool) {
    clear_face_overrides(face_name, defaults_frame);
    let Some(slots) = vector.as_vector_data() else {
        return;
    };
    for attr in LFACE_ATTRS {
        let value = slots
            .get(attr as usize)
            .copied()
            .unwrap_or_else(|| Value::symbol("unspecified"));
        if !value.is_symbol_named("unspecified") {
            set_face_override(face_name, attr, value, defaults_frame);
        }
    }
}

fn make_lisp_face_vector_for_domain(face_name: &str, defaults_frame: bool) -> Value {
    let mut values = Vec::with_capacity(LISP_FACE_VECTOR_LEN);
    values.push(Value::symbol("face"));
    values.extend(
        LFACE_ATTRS
            .iter()
            .map(|attr| lisp_face_attribute_value(face_name, *attr, defaults_frame)),
    );
    Value::vector(values)
}

pub(crate) fn make_lisp_face_vector_for_frame(face_name: &str) -> Value {
    make_lisp_face_vector_for_domain(face_name, false)
}

fn face_hash_entry_lisp_vector(entry: Value) -> Option<Value> {
    if entry.is_vector() {
        Some(entry)
    } else if entry.is_cons() {
        let vector = entry.cons_cdr();
        vector.is_vector().then_some(vector)
    } else {
        None
    }
}

fn runtime_unspecified_lisp_face_attr(attr: LFaceAttr, value: Value) -> bool {
    value.is_symbol_named("unspecified")
        || value.is_symbol_named(":ignore-defface")
        || value.is_symbol_named("reset")
        || (attr == LFaceAttr::Foreground && value.as_utf8_str() == Some("unspecified-fg"))
        || (attr == LFaceAttr::Background && value.as_utf8_str() == Some("unspecified-bg"))
}

fn frame_lisp_face_table_entries(
    eval: &super::eval::Context,
    frame_id: FrameId,
) -> Vec<(String, Value)> {
    let Some(table) = eval
        .frames
        .get(frame_id)
        .map(|frame| frame.face_hash_table())
    else {
        return Vec::new();
    };
    let Some(hash_table) = table.as_hash_table() else {
        return Vec::new();
    };

    hash_table
        .data
        .iter()
        .filter_map(|(key, entry)| match key {
            HashKey::Symbol(symbol) => face_hash_entry_lisp_vector(*entry)
                .map(|vector| (resolve_sym(*symbol).to_string(), vector)),
            _ => None,
        })
        .collect()
}

fn frame_parameter_color_or_tty_default(
    eval: &super::eval::Context,
    frame_id: FrameId,
    param: FrameParam,
    tty_default: &str,
) -> Value {
    eval.frames
        .get(frame_id)
        .and_then(|frame| frame.known_parameter(param))
        .filter(|value| value.is_string())
        .unwrap_or_else(|| Value::string(tty_default))
}

fn default_face_has_explicit_font_attr(attr: LFaceAttr) -> bool {
    get_face_override("default", attr, false).is_some()
        || get_face_override("default", attr, true).is_some()
}

fn realize_default_lisp_face_for_frame(eval: &mut super::eval::Context, frame_id: FrameId) {
    let Some(vector) =
        ensure_frame_lisp_face_vector(eval, frame_id, "default", FrameFaceInitial::SelectedBase)
    else {
        return;
    };
    let Some(frame) = eval.frames.get(frame_id) else {
        return;
    };
    let window_system = frame.effective_window_system();

    if window_system.is_none() {
        set_lisp_face_vector_attr(vector, LFaceAttr::Family, Value::string("default"));
        set_lisp_face_vector_attr(vector, LFaceAttr::Foundry, Value::string("default"));
        set_lisp_face_vector_attr(vector, LFaceAttr::Width, Value::symbol("normal"));
        set_lisp_face_vector_attr(vector, LFaceAttr::Height, Value::fixnum(1));
        if lisp_face_vector_attr(vector, LFaceAttr::Weight)
            .is_none_or(|value| runtime_unspecified_lisp_face_attr(LFaceAttr::Weight, value))
        {
            set_lisp_face_vector_attr(vector, LFaceAttr::Weight, Value::symbol("normal"));
        }
        if lisp_face_vector_attr(vector, LFaceAttr::Slant)
            .is_none_or(|value| runtime_unspecified_lisp_face_attr(LFaceAttr::Slant, value))
        {
            set_lisp_face_vector_attr(vector, LFaceAttr::Slant, Value::symbol("normal"));
        }
        if lisp_face_vector_attr(vector, LFaceAttr::Fontset)
            .is_none_or(|value| runtime_unspecified_lisp_face_attr(LFaceAttr::Fontset, value))
        {
            set_lisp_face_vector_attr(vector, LFaceAttr::Fontset, Value::NIL);
        }
    } else {
        for attr in [
            LFaceAttr::Family,
            LFaceAttr::Foundry,
            LFaceAttr::Width,
            LFaceAttr::Height,
            LFaceAttr::Weight,
            LFaceAttr::Slant,
        ] {
            if default_face_has_explicit_font_attr(attr) {
                continue;
            }
            let fallback = live_frame_font_attribute_fallback(eval, frame_id, attr);
            if let Some(value) = fallback {
                set_lisp_face_vector_attr(vector, attr, value);
            }
        }
    }

    for attr in [
        LFaceAttr::Extend,
        LFaceAttr::Underline,
        LFaceAttr::Overline,
        LFaceAttr::StrikeThrough,
        LFaceAttr::Box,
        LFaceAttr::InverseVideo,
        LFaceAttr::Stipple,
    ] {
        if lisp_face_vector_attr(vector, attr)
            .is_none_or(|value| runtime_unspecified_lisp_face_attr(attr, value))
        {
            set_lisp_face_vector_attr(vector, attr, Value::NIL);
        }
    }

    if lisp_face_vector_attr(vector, LFaceAttr::Foreground)
        .is_none_or(|value| runtime_unspecified_lisp_face_attr(LFaceAttr::Foreground, value))
    {
        let value = frame_parameter_color_or_tty_default(
            eval,
            frame_id,
            FrameParam::ForegroundColor,
            "unspecified-fg",
        );
        set_lisp_face_vector_attr(vector, LFaceAttr::Foreground, value);
    }
    if lisp_face_vector_attr(vector, LFaceAttr::Background)
        .is_none_or(|value| runtime_unspecified_lisp_face_attr(LFaceAttr::Background, value))
    {
        let value = frame_parameter_color_or_tty_default(
            eval,
            frame_id,
            FrameParam::BackgroundColor,
            "unspecified-bg",
        );
        set_lisp_face_vector_attr(vector, LFaceAttr::Background, value);
    }
}

fn runtime_face_from_lisp_face_vector(face_name: &str, vector: Value) -> RuntimeFace {
    let mut face = RuntimeFace::new(face_name);
    for attr in LFACE_ATTRS {
        let Some(value) = lisp_face_vector_attr(vector, attr) else {
            continue;
        };
        if runtime_unspecified_lisp_face_attr(attr, value) {
            continue;
        }
        if let Some(face_attr) = lisp_value_to_face_attr(attr, value) {
            face.set_attribute(attr, face_attr);
        }
    }
    face
}

/// Materialize a frame's authoritative Lisp face specifications into an
/// isolated runtime table.  This is a derived value: callers may use it for a
/// Lisp query or install it as redisplay's cache, but must never mutate it as
/// face-definition state.
fn runtime_face_table_from_frame_lisp_faces(
    eval: &super::eval::Context,
    frame_id: FrameId,
    preserve_default_baseline: bool,
) -> crate::face::FaceTable {
    // Preserve the already-established default face baseline.  In particular,
    // Lisp `font-at` queries retain relative inline heights until actual font
    // realization; replacing the baseline with the frame's concrete default
    // height here would prematurely collapse that semantic distinction.
    let mut table = if preserve_default_baseline {
        eval.face_table.clone()
    } else {
        crate::face::FaceTable::new()
    };
    for (face_name, vector) in frame_lisp_face_table_entries(eval, frame_id) {
        if preserve_default_baseline && face_name == "default" {
            continue;
        }
        table.define(
            &face_name,
            runtime_face_from_lisp_face_vector(&face_name, vector),
        );
    }
    table
}

/// Rebuild the display-facing runtime face cache from GNU-shaped frame-local
/// Lisp face vectors.
///
/// GNU stores face definitions as Lisp vectors in `frame->face_hash_table` and
/// realizes renderable `struct face` entries from those vectors during
/// redisplay. Neomacs still has a Rust `FaceTable` for the layout bridge; keep
/// it as a derived cache so redisplay follows the same ownership boundary.
pub(crate) fn sync_runtime_face_table_from_frame_lisp_faces(
    eval: &mut super::eval::Context,
    frame_id: FrameId,
) {
    realize_default_lisp_face_for_frame(eval, frame_id);
    eval.face_table = runtime_face_table_from_frame_lisp_faces(eval, frame_id, false);
}

#[derive(Clone, Copy)]
enum FrameFaceInitial {
    Empty,
    SelectedBase,
}

fn ensure_global_lisp_face_vector(
    eval: &mut super::eval::Context,
    face_name: &str,
) -> Option<Value> {
    crate::emacs_core::xfaces::face_new_frame_defaults_vector(eval, face_name)
}

fn lookup_frame_lisp_face_vector(
    eval: &super::eval::Context,
    frame_id: FrameId,
    face_name: &str,
) -> Option<Value> {
    let table = eval.frames.get(frame_id)?.face_hash_table();
    crate::emacs_core::xfaces::lookup_frame_face_hash_entry(table, Value::symbol(face_name))
}

/// Symbol-keyed frame face lookup for callers that already hold the interned
/// face symbol, avoiding the `&str` -> `Value::symbol` re-intern in
/// `lookup_frame_lisp_face_vector`. `key` must be an interned symbol `Value`.
fn lookup_frame_lisp_face_vector_by_symbol(
    eval: &super::eval::Context,
    frame_id: FrameId,
    key: Value,
) -> Option<Value> {
    let table = eval.frames.get(frame_id)?.face_hash_table();
    crate::emacs_core::xfaces::lookup_frame_face_hash_entry(table, key)
}

fn ensure_frame_lisp_face_vector(
    eval: &mut super::eval::Context,
    frame_id: FrameId,
    face_name: &str,
    initial: FrameFaceInitial,
) -> Option<Value> {
    if let Some(vector) = lookup_frame_lisp_face_vector(eval, frame_id, face_name) {
        return Some(vector);
    }
    let vector = match initial {
        FrameFaceInitial::Empty => make_lisp_face_vector(),
        FrameFaceInitial::SelectedBase => make_lisp_face_vector_for_domain(face_name, false),
    };
    let frame = eval.frames.get_mut(frame_id)?;
    crate::emacs_core::xfaces::upsert_frame_face_hash_entry(
        frame.face_hash_table(),
        Value::symbol(face_name),
        vector,
    );
    Some(vector)
}

fn apply_lisp_face_vector_update_for_frame_arg(
    eval: &mut super::eval::Context,
    face_name: &str,
    attr: LFaceAttr,
    attr_value: Value,
    font_derivation_value: Value,
    frame_arg: Option<&Value>,
) -> Result<(), Flow> {
    match frame_arg {
        Some(frame) if frame.is_t() => {
            if let Some(vector) = ensure_global_lisp_face_vector(eval, face_name) {
                set_lisp_face_vector_attr_with_font_derivatives(
                    face_name,
                    vector,
                    attr,
                    attr_value,
                    font_derivation_value,
                )?;
            }
        }
        Some(frame) if frame.as_fixnum() == Some(0) => {
            if let Some(vector) = ensure_global_lisp_face_vector(eval, face_name) {
                set_lisp_face_vector_attr_with_font_derivatives(
                    face_name,
                    vector,
                    attr,
                    attr_value,
                    font_derivation_value,
                )?;
            }
            for frame_id in eval.frames.frame_list() {
                if let Some(vector) = ensure_frame_lisp_face_vector(
                    eval,
                    frame_id,
                    face_name,
                    FrameFaceInitial::Empty,
                ) {
                    set_lisp_face_vector_attr_with_font_derivatives(
                        face_name,
                        vector,
                        attr,
                        attr_value,
                        font_derivation_value,
                    )?;
                }
            }
        }
        Some(frame) if live_frame_designator_in_state(&eval.frames, frame) => {
            let frame_id =
                frame_id_from_designator(frame).expect("live frame designator should decode");
            if let Some(vector) =
                ensure_frame_lisp_face_vector(eval, frame_id, face_name, FrameFaceInitial::Empty)
            {
                set_lisp_face_vector_attr_with_font_derivatives(
                    face_name,
                    vector,
                    attr,
                    attr_value,
                    font_derivation_value,
                )?;
            }
        }
        None => {
            let frame_id = super::window_cmds::ensure_selected_frame_id(eval);
            if let Some(vector) =
                ensure_frame_lisp_face_vector(eval, frame_id, face_name, FrameFaceInitial::Empty)
            {
                set_lisp_face_vector_attr_with_font_derivatives(
                    face_name,
                    vector,
                    attr,
                    attr_value,
                    font_derivation_value,
                )?;
            }
        }
        Some(frame) if frame.is_nil() => {
            let frame_id = super::window_cmds::ensure_selected_frame_id(eval);
            if let Some(vector) =
                ensure_frame_lisp_face_vector(eval, frame_id, face_name, FrameFaceInitial::Empty)
            {
                set_lisp_face_vector_attr_with_font_derivatives(
                    face_name,
                    vector,
                    attr,
                    attr_value,
                    font_derivation_value,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn normalize_face_attribute_name(attr: &Value) -> Result<LFaceAttr, Flow> {
    let name = match attr.kind() {
        ValueKind::Symbol(id) => resolve_sym(id),
        ValueKind::Nil => "nil",
        ValueKind::T => "t",
        _ => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("symbolp"), *attr],
            ));
        }
    };

    if let Some(attr) = LFaceAttr::from_keyword(name) {
        Ok(attr)
    } else if attr.is_nil() {
        Err(signal(
            "error",
            vec![Value::string("Invalid face attribute name")],
        ))
    } else {
        Err(signal(
            "error",
            vec![Value::string("Invalid face attribute name"), *attr],
        ))
    }
}

fn normalize_set_face_attribute_name(attr: &Value) -> Result<SetFaceAttr, Flow> {
    let name = match attr.kind() {
        ValueKind::Symbol(id) => resolve_sym(id),
        ValueKind::Nil => "nil",
        ValueKind::T => "t",
        _ => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("symbolp"), *attr],
            ));
        }
    };

    if let Some(attr) = SetFaceAttr::from_keyword(name) {
        Ok(attr)
    } else if attr.is_nil() {
        Err(signal(
            "error",
            vec![Value::string("Invalid face attribute name")],
        ))
    } else {
        Err(signal(
            "error",
            vec![Value::string("Invalid face attribute name"), *attr],
        ))
    }
}

fn default_face_attribute_value(attr: LFaceAttr) -> Value {
    match attr {
        LFaceAttr::Family | LFaceAttr::Foundry => Value::string("default"),
        LFaceAttr::Height => Value::fixnum(1),
        LFaceAttr::Weight | LFaceAttr::Slant | LFaceAttr::Width => Value::symbol("normal"),
        LFaceAttr::Underline
        | LFaceAttr::Overline
        | LFaceAttr::StrikeThrough
        | LFaceAttr::Box
        | LFaceAttr::InverseVideo
        | LFaceAttr::Stipple
        | LFaceAttr::Inherit
        | LFaceAttr::Extend
        | LFaceAttr::Fontset => Value::NIL,
        LFaceAttr::Foreground => Value::string("unspecified-fg"),
        LFaceAttr::Background => Value::string("unspecified-bg"),
        LFaceAttr::DistantForeground | LFaceAttr::Font => Value::symbol("unspecified"),
    }
}

fn is_reset_like_face_attr_value(value: &Value) -> bool {
    value.as_symbol_id().is_some_and(|id| {
        let s = resolve_sym(id);
        s == "unspecified" || s == ":ignore-defface" || s == "reset"
    })
}

fn font_spec_size_to_face_height(size: Value) -> Option<Value> {
    match size.kind() {
        ValueKind::Float if size.xfloat() > 0.0 => Some(Value::fixnum(10 * (size.xfloat() as i64))),
        ValueKind::Fixnum(px) if px > 0 => Some(Value::fixnum(px * 10)),
        _ => None,
    }
}

fn derived_face_attrs_from_font_value(value: &Value) -> Vec<(LFaceAttr, Value)> {
    if !value.is_vector() {
        return Vec::new();
    };
    if !is_font(value) {
        return Vec::new();
    }

    let font_spec = is_font_spec(value);
    let elems = value.as_vector_data().unwrap().clone();
    let mut derived = Vec::new();

    for (field, attr) in [
        ("family", LFaceAttr::Family),
        ("foundry", LFaceAttr::Foundry),
    ] {
        if let Some(v) = font_vector_get_flexible(&elems, field)
            && let Some(text) = font_value_text(&v)
        {
            derived.push((attr, Value::string(text)));
        }
    }

    for (field, attr) in [
        ("weight", LFaceAttr::Weight),
        ("slant", LFaceAttr::Slant),
        ("width", LFaceAttr::Width),
    ] {
        if let Some(v) = font_vector_get_flexible(&elems, field) {
            derived.push((attr, v));
        }
    }

    if let Some(v) = font_vector_get_flexible(&elems, "height") {
        derived.push((LFaceAttr::Height, v));
    } else if let Some(v) = font_vector_get_flexible(&elems, "size") {
        if font_spec {
            if let Some(height) = font_spec_size_to_face_height(v) {
                derived.push((LFaceAttr::Height, height));
            }
        } else {
            derived.push((LFaceAttr::Height, v));
        }
    }

    derived
}

fn apply_derived_font_face_overrides(
    face_name: &str,
    font_value: &Value,
    defaults_frame: bool,
) -> Result<(), Flow> {
    for (attr_name, attr_value) in derived_face_attrs_from_font_value(font_value) {
        let (canonical_attr, canonical_value) =
            normalize_face_attr_for_set(face_name, SetFaceAttr::LFace(attr_name), attr_value)?;
        set_face_override(face_name, canonical_attr, canonical_value, defaults_frame);
    }
    Ok(())
}

fn lisp_face_attribute_base_value(face: &str, attr: LFaceAttr, defaults_frame: bool) -> Value {
    if defaults_frame {
        return Value::symbol("unspecified");
    }
    if face == "default" {
        return default_face_attribute_value(attr);
    }
    match (face, attr) {
        ("bold", LFaceAttr::Weight) => Value::symbol("bold"),
        ("italic", LFaceAttr::Slant) => Value::symbol("italic"),
        ("underline", LFaceAttr::Underline) => Value::T,
        ("highlight", LFaceAttr::InverseVideo) => Value::T,
        ("region", LFaceAttr::InverseVideo) => Value::T,
        ("mode-line", LFaceAttr::InverseVideo) => Value::T,
        ("mode-line-active", LFaceAttr::Inherit) => Value::symbol("mode-line"),
        ("mode-line-highlight", LFaceAttr::Inherit) => Value::symbol("highlight"),
        ("mode-line-emphasis", LFaceAttr::Weight) => Value::symbol("bold"),
        ("mode-line-buffer-id", LFaceAttr::Weight) => Value::symbol("bold"),
        ("mode-line-inactive", LFaceAttr::Inherit) => Value::symbol("mode-line"),
        ("header-line", LFaceAttr::Inherit) => Value::symbol("mode-line"),
        ("header-line-highlight", LFaceAttr::Inherit) => Value::symbol("mode-line-highlight"),
        ("header-line-active", LFaceAttr::Inherit) => Value::symbol("header-line"),
        ("header-line-inactive", LFaceAttr::Inherit) => Value::symbol("header-line"),
        ("fringe", LFaceAttr::Background) => Value::string("gray"),
        ("cursor", LFaceAttr::Background) => Value::string("white"),
        ("vertical-border", LFaceAttr::Inherit) => Value::symbol("mode-line-inactive"),
        ("tool-bar", LFaceAttr::Foreground) => Value::string("black"),
        ("tool-bar", LFaceAttr::Box) => Value::symbol("t"),
        ("tab-bar", LFaceAttr::Inherit) => Value::symbol("variable-pitch"),
        ("tab-line", LFaceAttr::Inherit) => Value::symbol("variable-pitch"),
        _ => Value::symbol("unspecified"),
    }
}

fn lisp_face_attribute_value(face: &str, attr: LFaceAttr, defaults_frame: bool) -> Value {
    if let Some(value) = get_face_override(face, attr, defaults_frame) {
        return value;
    }
    lisp_face_attribute_base_value(face, attr, defaults_frame)
}

fn resolve_known_face_name_for_compare(
    eval: &super::eval::Context,
    face: &Value,
    defaults_frame: bool,
) -> Result<String, Flow> {
    match resolve_face_designator(eval, *face, FaceAliasCyclePolicy::Signal)? {
        ResolvedFaceDesignator::Symbol(name) | ResolvedFaceDesignator::String(name) => {
            if face_exists_for_domain(name.name(), defaults_frame) {
                Ok(name.name().to_owned())
            } else {
                Err(signal(
                    "error",
                    vec![Value::string("Invalid face"), Value::symbol(name.name())],
                ))
            }
        }
        ResolvedFaceDesignator::Other(_) => Err(invalid_face_error(*face)),
    }
}

fn face_attr_value_name(attr: &Value) -> Result<SymId, Flow> {
    match attr.kind() {
        ValueKind::Symbol(id) => {
            let s = resolve_sym(id);
            if s.starts_with(':') {
                Ok(face_attr_id(s))
            } else {
                Ok(face_attr_id(&format!(":{s}")))
            }
        }
        ValueKind::Nil => Ok(face_attr_id("nil")),
        ValueKind::T => Ok(face_attr_id("t")),
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("symbolp"), *attr],
        )),
    }
}

fn frame_defaults_flag(frame: Option<&Value>) -> Result<bool, Flow> {
    match frame {
        None => Ok(false),
        Some(v) if v.is_nil() => Ok(false),
        Some(v) if v.is_t() => Ok(true),
        Some(v) if frame_device_designator_p(v) => Ok(false),
        Some(v) => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), *v],
        )),
    }
}

fn proper_list_to_vec_or_listp_error(value: &Value) -> Result<Vec<Value>, Flow> {
    let mut out = Vec::new();
    let mut cursor = *value;
    loop {
        match cursor.kind() {
            ValueKind::Nil => return Ok(out),
            ValueKind::Cons => {
                let cell_car = cursor.cons_car();
                let cell_cdr = cursor.cons_cdr();
                out.push(cell_car);
                cursor = cell_cdr;
            }
            _other => {
                return Err(signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("listp"), cursor],
                ));
            }
        }
    }
}

fn check_non_empty_string(value: &Value, empty_message: &str) -> Result<(), Flow> {
    match value.kind() {
        ValueKind::String => {
            if value
                .as_lisp_string()
                .expect("ValueKind::String must carry LispString payload")
                .is_empty()
            {
                Err(signal("error", vec![Value::string(empty_message), *value]))
            } else {
                Ok(())
            }
        }
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), *value],
        )),
    }
}

fn symbol_name_or_type_error(value: &Value) -> Result<String, Flow> {
    match value.kind() {
        ValueKind::Nil => Ok("nil".to_string()),
        ValueKind::T => Ok("t".to_string()),
        ValueKind::Symbol(id) => Ok(resolve_sym(id).to_owned()),
        _ => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("symbolp"), *value],
        )),
    }
}

fn normalize_face_attr_for_set(
    face_name: &str,
    attr: SetFaceAttr,
    value: Value,
) -> Result<(LFaceAttr, Value), Flow> {
    normalize_face_attr_for_set_with_eval(None, face_name, attr, value)
}

fn merge_face_height_value(
    eval: Option<&mut super::eval::Context>,
    from: Value,
    to: Value,
    invalid: Value,
) -> Value {
    match from.kind() {
        ValueKind::Fixnum(_) => from,
        ValueKind::Float => match to.kind() {
            ValueKind::Fixnum(height) => Value::fixnum((from.xfloat() * height as f64) as i64),
            ValueKind::Float => Value::make_float(from.xfloat() * to.xfloat()),
            _ if is_reset_like_face_attr_value(&to) => from,
            _ => invalid,
        },
        _ => {
            let Some(eval) = eval else {
                return invalid;
            };
            match eval.funcall_general(from, vec![to]) {
                Ok(result) if !to.is_fixnum() || result.is_fixnum() => result,
                Ok(_) | Err(_) => invalid,
            }
        }
    }
}

fn normalize_face_attr_for_set_with_eval(
    eval: Option<&mut super::eval::Context>,
    face_name: &str,
    attr: SetFaceAttr,
    value: Value,
) -> Result<(LFaceAttr, Value), Flow> {
    let attr_name = match attr {
        SetFaceAttr::LFace(attr) => attr.keyword(),
        SetFaceAttr::Alias(alias) => alias.keyword(),
    };
    let mut normalized = match attr_name {
        ":foreground" | ":background" | ":distant-foreground" if value.is_nil() => {
            Value::symbol("unspecified")
        }
        _ => value,
    };
    let is_reset_like = is_reset_like_face_attr_value(&normalized);

    match attr_name {
        ":family" | ":foundry" => {
            if !is_reset_like {
                match normalized.kind() {
                    ValueKind::String
                        if !normalized
                            .as_lisp_string()
                            .expect("ValueKind::String must carry LispString payload")
                            .is_empty() => {}
                    ValueKind::String => {
                        let msg = if attr_name == ":family" {
                            "Invalid face family"
                        } else {
                            "Invalid face foundry"
                        };
                        return Err(signal("error", vec![Value::string(msg), normalized]));
                    }
                    _ => {
                        return Err(signal(
                            LispCondition::WrongTypeArgument,
                            vec![Value::symbol("stringp"), normalized],
                        ));
                    }
                }
            }
        }
        ":height" => {
            if !is_reset_like {
                if face_name == "default" {
                    match normalized.kind() {
                        ValueKind::Fixnum(n) if n > 0 => {}
                        _ => {
                            return Err(signal(
                                "error",
                                vec![
                                    Value::string("Default face height not absolute and positive"),
                                    normalized,
                                ],
                            ));
                        }
                    }
                } else {
                    match normalized.kind() {
                        ValueKind::Fixnum(n) if n > 0 => {}
                        ValueKind::Float if normalized.xfloat() > 0.0 => {}
                        _ => {
                            let test = merge_face_height_value(
                                eval,
                                normalized,
                                Value::fixnum(10),
                                Value::NIL,
                            );
                            if test.as_int().is_none_or(|n| n <= 0) {
                                return Err(signal(
                                    "error",
                                    vec![
                                        Value::string(
                                            "Face height does not produce a positive integer",
                                        ),
                                        normalized,
                                    ],
                                ));
                            }
                        }
                    }
                }
            }
        }
        ":weight" => {
            if !is_reset_like {
                let sym = symbol_name_or_type_error(&normalized)?;
                if !valid_face_weight_symbol(&sym) {
                    return Err(signal(
                        "error",
                        vec![Value::string("Invalid face weight"), normalized],
                    ));
                }
            }
        }
        ":slant" => {
            if !is_reset_like {
                let sym = symbol_name_or_type_error(&normalized)?;
                if !valid_face_slant_symbol(&sym) {
                    return Err(signal(
                        "error",
                        vec![Value::string("Invalid face slant"), normalized],
                    ));
                }
            }
        }
        ":width" => {
            if !is_reset_like {
                let sym = symbol_name_or_type_error(&normalized)?;
                if !valid_face_width_symbol(&sym) {
                    return Err(signal(
                        "error",
                        vec![Value::string("Invalid face width"), normalized],
                    ));
                }
            }
        }
        ":foreground" | ":background" | ":distant-foreground" => {
            if !is_reset_like {
                // Doom themes and some Emacs themes store colours as cons cells
                // (dark . light).  Resolve to a plain string so downstream
                // consumers (lisp_value_to_face_attr) receive a valid colour.
                if let ValueKind::Cons = normalized.kind() {
                    normalized = normalized.cons_car();
                }
                let check_msg = match attr_name {
                    ":foreground" => "Empty foreground color value",
                    ":background" => "Empty background color value",
                    _ => "Empty distant-foreground color value",
                };
                check_non_empty_string(&normalized, check_msg)?;
            }
        }
        ":inverse-video" => {
            if !is_reset_like {
                let sym = symbol_name_or_type_error(&normalized)?;
                if sym != "t" && sym != "nil" {
                    return Err(signal(
                        "error",
                        vec![
                            Value::string("Invalid inverse-video face attribute value"),
                            normalized,
                        ],
                    ));
                }
            }
        }
        ":extend" => {
            if !is_reset_like {
                let sym = symbol_name_or_type_error(&normalized)?;
                if sym != "t" && sym != "nil" {
                    return Err(signal(
                        "error",
                        vec![
                            Value::string("Invalid extend face attribute value"),
                            normalized,
                        ],
                    ));
                }
            }
        }
        ":underline" => {
            if !is_reset_like && !valid_face_underline_value(normalized) {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid face underline"), normalized],
                ));
            }
        }
        ":box" => {
            // GNU xfaces.c `internal-set-lisp-face-attribute` (QCbox arm):
            // `t` means a simple box of width 1 in the face's foreground
            // color and is canonicalized to the fixnum 1 *before* validation
            // and storage, so `face-attribute` later reports 1, not t.
            if normalized == Value::T {
                normalized = Value::fixnum(1);
            }
            if !is_reset_like && !valid_face_box_value(normalized) {
                return Err(signal(
                    "error",
                    vec![Value::string("Invalid face box"), normalized],
                ));
            }
        }
        ":inherit" => {
            // Accept any face_ref: nil / symbol / list of face_refs /
            // plist of attributes. Matches GNU `merge_face_ref`
            // (xfaces.c:2700-3025) which accepts any value and
            // dispatches by shape at resolution time.
            let valid = matches!(
                normalized.kind(),
                ValueKind::Nil | ValueKind::T | ValueKind::Symbol(_) | ValueKind::Cons
            );
            if !valid {
                let mut payload = vec![Value::string("Invalid face inheritance")];
                payload.push(normalized);
                return Err(signal("error", payload));
            }
        }
        ":bold" => {
            let mapped = if normalized.is_nil() {
                Value::symbol("normal")
            } else {
                Value::symbol("bold")
            };
            return Ok((LFaceAttr::Weight, mapped));
        }
        ":italic" => {
            let mapped = if normalized.is_nil() {
                Value::symbol("normal")
            } else {
                Value::symbol("italic")
            };
            return Ok((LFaceAttr::Slant, mapped));
        }
        _ => {}
    }

    match attr {
        SetFaceAttr::LFace(attr) => Ok((attr, normalized)),
        SetFaceAttr::Alias(_) => unreachable!("aliases returned above"),
    }
}

/// `(internal-lisp-face-p FACE &optional FRAME)` -- return a face descriptor
/// vector for known faces, nil otherwise.
pub(crate) fn builtin_internal_lisp_face_p(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("internal-lisp-face-p", &args, 1)?;
    expect_max_args("internal-lisp-face-p", &args, 2)?;

    // Fast path mirroring GNU's `Finternal_lisp_face_p`: resolve the
    // `face-alias` chain, then perform an allocation-free, symbol-keyed lookup
    // in the frame face table (2-arg live frame) or the global
    // `face--new-frame-defaults` table (null frame). GNU never allocates, seeds,
    // or creates here; neither does this path. The known-face/ensure gate is
    // retained only as a cold fallback for a table miss.
    let resolved = resolve_face_designator(eval, args[0], FaceAliasCyclePolicy::Signal)?;
    let key = resolved.name().map(ResolvedFaceName::symbol);

    if let Some(frame) = args.get(1)
        && !frame.is_nil()
    {
        if !live_frame_designator_in_state(&eval.frames, frame) {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("frame-live-p"), *frame],
            ));
        }
        let frame_id = frame_id_from_designator(frame)
            .expect("live frame designator should decode to frame id");
        if let Some(vector) =
            key.and_then(|k| lookup_frame_lisp_face_vector_by_symbol(eval, frame_id, k))
        {
            return Ok(vector);
        }
        return Ok(match known_resolved_face_name(resolved) {
            Some(face_name) => lookup_frame_lisp_face_vector(eval, frame_id, face_name.name())
                .unwrap_or(Value::NIL),
            None => Value::NIL,
        });
    }

    // Null-frame (or omitted-frame) global path.
    if let Some(vector) =
        key.and_then(|k| crate::emacs_core::xfaces::lookup_face_new_frame_defaults_vector(eval, k))
    {
        return Ok(vector);
    }
    Ok(match known_resolved_face_name(resolved) {
        Some(face_name) => {
            ensure_global_lisp_face_vector(eval, face_name.name()).unwrap_or(Value::NIL)
        }
        None => Value::NIL,
    })
}

/// Eval-backed version of `internal-make-lisp-face` that also ensures the face
/// exists in the evaluator's `FaceTable`.
pub(crate) fn builtin_internal_make_lisp_face(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("internal-make-lisp-face", &args, 1)?;
    expect_max_args("internal-make-lisp-face", &args, 2)?;
    let _ = require_symbol_face_name(&args[0])?;
    let face_name = resolve_face_designator(eval, args[0], FaceAliasCyclePolicy::UseDefault)?
        .name()
        .expect("a required symbol resolves to a named face")
        .name()
        .to_owned();
    if let Some(frame) = args.get(1)
        && !frame.is_nil()
        && !live_frame_designator_in_state(&eval.frames, frame)
    {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), *frame],
        ));
    }
    mark_created_lisp_face(&face_name);
    ensure_lisp_face_id_property(eval, &face_name)?;
    let _ = ensure_global_lisp_face_vector(eval, &face_name);
    let result = if let Some(frame) = args.get(1).filter(|frame| !frame.is_nil()) {
        let frame_id = frame_id_from_designator(frame)
            .expect("validated frame designator should decode to frame id");
        let vector =
            ensure_frame_lisp_face_vector(eval, frame_id, &face_name, FrameFaceInitial::Empty)
                .unwrap_or_else(make_lisp_face_vector);
        reset_lisp_face_vector(vector);
        clear_face_overrides(&face_name, false);
        vector
    } else {
        let vector =
            ensure_global_lisp_face_vector(eval, &face_name).unwrap_or_else(make_lisp_face_vector);
        reset_lisp_face_vector(vector);
        clear_face_overrides(&face_name, true);
        vector
    };
    eval.face_table.ensure_face(&face_name);
    eval.face_change_count += 1;
    Ok(result)
}

/// Eval-backed version of `internal-copy-lisp-face`.
///
/// The copied Lisp vector remains authoritative; redisplay derives its
/// runtime representation after observing `face_change_count`.
pub(crate) fn builtin_internal_copy_lisp_face(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("internal-copy-lisp-face", &args, 4)?;
    let _ = require_symbol_face_name(&args[0])?;
    let _ = require_symbol_face_name(&args[1])?;
    let to_name = resolve_face_designator(eval, args[1], FaceAliasCyclePolicy::UseDefault)?
        .name()
        .expect("a required symbol resolves to a named face")
        .name()
        .to_owned();
    let copy_defaults_domain = args[2].is_t();
    if !copy_defaults_domain && !live_frame_designator_in_state(&eval.frames, &args[2]) {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), args[2]],
        ));
    }
    if !copy_defaults_domain
        && !args[3].is_nil()
        && !live_frame_designator_in_state(&eval.frames, &args[3])
    {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), args[3]],
        ));
    }
    let from_name = resolve_copy_source_face_symbol(eval, &args[0])?;
    mark_created_lisp_face(&to_name);
    ensure_lisp_face_id_property(eval, &to_name)?;
    let _ = ensure_global_lisp_face_vector(eval, &to_name);
    let (src_vector, dst_vector, defaults_frame) = if copy_defaults_domain {
        let src_vector = ensure_global_lisp_face_vector(eval, &from_name)
            .ok_or_else(|| invalid_face_error(args[0]))?;
        let dst_vector =
            ensure_global_lisp_face_vector(eval, &to_name).unwrap_or_else(make_lisp_face_vector);
        (src_vector, dst_vector, true)
    } else {
        let frame_id = frame_id_from_designator(&args[2])
            .expect("validated frame designator should decode to frame id");
        let new_frame_id = if args[3].is_nil() {
            frame_id
        } else {
            frame_id_from_designator(&args[3])
                .expect("validated frame designator should decode to frame id")
        };
        let src_vector = ensure_frame_lisp_face_vector(
            eval,
            frame_id,
            &from_name,
            FrameFaceInitial::SelectedBase,
        )
        .ok_or_else(|| invalid_face_error(args[0]))?;
        let dst_vector =
            ensure_frame_lisp_face_vector(eval, new_frame_id, &to_name, FrameFaceInitial::Empty)
                .unwrap_or_else(make_lisp_face_vector);
        (src_vector, dst_vector, false)
    };
    copy_lisp_face_vector_slots(src_vector, dst_vector);
    sync_face_overrides_from_lisp_face_vector(&to_name, dst_vector, defaults_frame);

    let result = args[1];

    eval.face_change_count += 1;

    Ok(result)
}

/// Eval-backed version of `internal-set-lisp-face-attribute`.
///
/// Like GNU Emacs' `Finternal_set_lisp_face_attribute`, this mutates the
/// authoritative Lisp face specification and marks face state changed.  It
/// deliberately does not materialize the display-facing `FaceTable` or a
/// per-frame runtime face: redisplay owns those derived representations.
pub(crate) fn builtin_internal_set_lisp_face_attribute(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    // Pure logic (FACE_ATTR_STATE storage + validation)
    expect_min_args("internal-set-lisp-face-attribute", &args, 3)?;
    expect_max_args("internal-set-lisp-face-attribute", &args, 4)?;
    let face = &args[0];
    let _ = require_symbol_face_name(face)?;
    let resolved_face = resolve_face_designator(eval, *face, FaceAliasCyclePolicy::Signal)?
        .name()
        .expect("a required symbol resolves to a named face");
    let face_name = resolved_face.name().to_owned();
    let face_symbol = resolved_face.symbol();
    let attr_name = normalize_set_face_attribute_name(&args[1])?;
    let value = args[2];
    if let Some(frame) = args.get(3)
        && !frame.is_nil()
        && !frame.is_t()
        && frame.as_fixnum() != Some(0)
        && !live_frame_designator_in_state(&eval.frames, frame)
    {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), *frame],
        ));
    }

    {
        let mut apply_set = |defaults_frame: bool| -> Result<(), Flow> {
            if defaults_frame {
                if !face_exists_for_domain(&face_name, true) {
                    if face.is_nil() {
                        return Err(signal("error", vec![Value::string("Invalid face")]));
                    }
                    return Err(signal(
                        "error",
                        vec![Value::string("Invalid face"), face_symbol],
                    ));
                }
            } else if !face_exists_for_domain(&face_name, false) {
                mark_selected_created_lisp_face(&face_name);
                mark_created_lisp_face(&face_name);
                // GNU Emacs `Finternal_set_lisp_face_attribute` calls
                // `lface_from_face_name` which calls `Finternal_make_lisp_face`,
                // which stores the internal face ID as the symbol's `face`
                // property.  Without this `check-face` / `face-id` fail.
                ensure_lisp_face_id_property(eval, &face_name)?;
            }

            let (canonical_attr, mut canonical_value) =
                normalize_face_attr_for_set_with_eval(Some(eval), &face_name, attr_name, value)?;
            // GNU Emacs: when updating face--new-frame-defaults, convert
            // `unspecified' to `:ignore-defface' so the defface spec
            // doesn't override the explicitly unspecified value
            // (xfaces.c:3262, Finternal_set_lisp_face_attribute).
            if defaults_frame
                && is_reset_like_face_attr_value(&canonical_value)
                && canonical_value.is_symbol_named("unspecified")
            {
                canonical_value = Value::symbol(":ignore-defface");
            }
            set_face_override(&face_name, canonical_attr, canonical_value, defaults_frame);
            if defaults_frame {
                if let Some(vector) = ensure_global_lisp_face_vector(eval, &face_name) {
                    set_lisp_face_vector_attr_with_font_derivatives(
                        &face_name,
                        vector,
                        canonical_attr,
                        canonical_value,
                        canonical_value,
                    )?;
                }
            } else {
                let frame_ids = match args.get(3) {
                    Some(v) if v.as_fixnum() == Some(0) => eval.frames.frame_list(),
                    Some(frame) if live_frame_designator_in_state(&eval.frames, frame) => {
                        frame_id_from_designator(frame)
                            .map(|frame_id| vec![frame_id])
                            .unwrap_or_default()
                    }
                    _ => vec![super::window_cmds::ensure_selected_frame_id(eval)],
                };
                let initial = if is_known_lisp_face_name(&face_name) {
                    FrameFaceInitial::SelectedBase
                } else {
                    FrameFaceInitial::Empty
                };
                for frame_id in frame_ids {
                    if let Some(vector) =
                        ensure_frame_lisp_face_vector(eval, frame_id, &face_name, initial)
                    {
                        set_lisp_face_vector_attr_with_font_derivatives(
                            &face_name,
                            vector,
                            canonical_attr,
                            canonical_value,
                            canonical_value,
                        )?;
                    }
                }
            }
            if canonical_attr == LFaceAttr::Font && !is_reset_like_face_attr_value(&canonical_value)
            {
                apply_derived_font_face_overrides(&face_name, &canonical_value, defaults_frame)?;
            }
            Ok(())
        };

        match args.get(3) {
            None => apply_set(false)?,
            Some(v) if v.is_nil() => apply_set(false)?,
            Some(v) if v.is_t() => apply_set(true)?,
            Some(v) if v.as_fixnum() == Some(0) => {
                apply_set(true)?;
                apply_set(false)?;
            }
            Some(_) => apply_set(false)?,
        }
    }

    let result = face_symbol;

    // Preserve GNU-visible live-frame font/default-parameter side effects,
    // but leave conversion to render-facing face attributes to redisplay.
    if args.len() >= 3 {
        let value = args[2];

        if let Ok(attr_name) = normalize_set_face_attribute_name(&args[1]) {
            let (canonical_attr, canonical_value) =
                normalize_face_attr_for_set_with_eval(Some(eval), &face_name, attr_name, value)?;
            let attr_name_str = canonical_attr.keyword();
            let live_frame_id = live_frame_id_for_face_update(eval, args.get(3))?;
            let font_resolution = if canonical_attr == LFaceAttr::Font {
                live_frame_id
                    .map(|frame_id| resolve_live_frame_font_request(eval, frame_id, &value))
            } else {
                None
            };
            let effective_value = font_resolution
                .as_ref()
                .map_or(canonical_value, |resolution| resolution.font_value);
            let public_effective_value = if canonical_attr == LFaceAttr::Font {
                public_live_frame_font_value(effective_value)
            } else {
                effective_value
            };

            if canonical_attr == LFaceAttr::Font && effective_value != value {
                set_face_override(&face_name, canonical_attr, public_effective_value, false);
            }
            if canonical_attr == LFaceAttr::Font {
                apply_lisp_face_vector_update_for_frame_arg(
                    eval,
                    &face_name,
                    canonical_attr,
                    public_effective_value,
                    effective_value,
                    args.get(3),
                )?;
            }

            if canonical_attr == LFaceAttr::Font {
                for (derived_attr, derived_value) in
                    derived_face_attrs_from_font_value(&effective_value)
                {
                    set_face_override(&face_name, derived_attr, derived_value, false);
                }
            }

            if canonical_attr == LFaceAttr::Font && face_name == "default" {
                if let (Some(frame_id), Some(resolution)) =
                    (live_frame_id, font_resolution.as_ref())
                {
                    sync_live_frame_font_state(eval, frame_id, &value, resolution);
                }
            } else if face_name == "default"
                && default_face_font_attr_affects_frame_font(canonical_attr)
                && let Some(frame_id) = live_frame_id
            {
                sync_live_default_face_font_state(eval, frame_id);
            }

            if let Some(frame_id) = live_frame_id
                && face_name == "default"
            {
                let frame_param = match attr_name_str {
                    ":foreground" => Some(FrameParam::ForegroundColor),
                    ":background" => Some(FrameParam::BackgroundColor),
                    _ => None,
                };
                if let Some(param) = frame_param {
                    if let Some(frame) = eval.frames.get_mut(frame_id) {
                        frame.set_known_parameter(param, public_effective_value);
                    }
                    if attr_name_str == ":background"
                        && let Some(function) =
                            eval.obarray().symbol_function("frame-set-background-mode")
                    {
                        let _ = eval.apply(function, vec![Value::make_frame(frame_id.0)])?;
                    }
                }
            }
        }
    }

    eval.face_change_count += 1;

    Ok(result)
}

/// Convert a Lisp face attribute value to `FaceAttrValue` for `FaceTable`.
fn lisp_value_to_face_attr(attr: LFaceAttr, value: Value) -> Option<crate::face::FaceAttrValue> {
    use crate::face::{
        BoxBorder, BoxStyle, Color, FaceAttrValue, FaceHeight, FontSlant, FontWeight, FontWidth,
        Underline, UnderlineStyle,
    };

    // "unspecified" symbol = reset the attribute
    if value.is_symbol_named("unspecified") {
        return Some(FaceAttrValue::Unspecified);
    }

    match attr {
        LFaceAttr::Foreground | LFaceAttr::Background | LFaceAttr::DistantForeground => {
            let s = match value.kind() {
                ValueKind::Cons => value.cons_car().as_utf8_str(),
                _ => value.as_utf8_str().or_else(|| value.as_symbol_name()),
            };
            let s = s?;
            let c = Color::from_name(s).or_else(|| Color::from_hex(s))?;
            Some(FaceAttrValue::Color(c))
        }
        LFaceAttr::Weight => {
            let name = value.as_symbol_name()?;
            Some(FaceAttrValue::Weight(FontWeight::from_symbol(name)?))
        }
        LFaceAttr::Slant => {
            let name = value.as_symbol_name()?;
            Some(FaceAttrValue::Slant(FontSlant::from_symbol(name)?))
        }
        LFaceAttr::Width => {
            let name = value.as_symbol_name()?;
            Some(FaceAttrValue::Width(FontWidth::from_symbol(name)?))
        }
        LFaceAttr::Height => match value.kind() {
            ValueKind::Fixnum(n) => Some(FaceAttrValue::Height(FaceHeight::Absolute(n as i32))),
            ValueKind::Float => Some(FaceAttrValue::Height(FaceHeight::Relative(value.xfloat()))),
            _ => None,
        },
        LFaceAttr::Family | LFaceAttr::Foundry => {
            if value.is_string() {
                Some(FaceAttrValue::Text(value))
            } else {
                None
            }
        }
        LFaceAttr::Underline => {
            if value.is_nil() {
                return Some(FaceAttrValue::Bool(false));
            }
            if value.is_t() {
                return Some(FaceAttrValue::Bool(true));
            }
            if let Some(s) = value.as_utf8_str() {
                let color = Color::from_name(s).or_else(|| Color::from_hex(s));
                return Some(FaceAttrValue::Underline(Underline {
                    style: UnderlineStyle::Line,
                    color,
                    position: None,
                }));
            }
            // Plist form: (:style STYLE :color COLOR :position POS)
            if let Some(plist) = super::value::list_to_vec(&value) {
                let mut style = UnderlineStyle::Line;
                let mut color = None;
                let mut position = None;
                let mut i = 0;
                while i + 1 < plist.len() {
                    let key = plist[i].as_symbol_name().unwrap_or("");
                    let val = &plist[i + 1];
                    match key {
                        ":style" => {
                            style = val
                                .as_symbol_name()
                                .and_then(UnderlineStyle::from_symbol)
                                .unwrap_or(UnderlineStyle::Line);
                        }
                        ":color" => {
                            if let Some(s) = val.as_utf8_str().or_else(|| val.as_symbol_name()) {
                                color = Color::from_name(s).or_else(|| Color::from_hex(s));
                            }
                        }
                        ":position" => {
                            if let Some(n) = val.as_fixnum() {
                                position = Some(n as i32);
                            }
                        }
                        _ => {}
                    }
                    i += 2;
                }
                return Some(FaceAttrValue::Underline(Underline {
                    style,
                    color,
                    position,
                }));
            }
            Some(FaceAttrValue::Bool(true))
        }
        LFaceAttr::Overline | LFaceAttr::StrikeThrough => {
            if value.is_nil() {
                return Some(FaceAttrValue::Bool(false));
            }
            if value.is_t() {
                return Some(FaceAttrValue::Bool(true));
            }
            if let Some(s) = value.as_utf8_str() {
                let c = Color::from_name(s).or_else(|| Color::from_hex(s))?;
                return Some(FaceAttrValue::Color(c));
            }
            Some(FaceAttrValue::Bool(value.is_truthy()))
        }
        LFaceAttr::Box => {
            if value.is_nil() {
                return Some(FaceAttrValue::Unspecified);
            }
            if value.is_t() {
                return Some(FaceAttrValue::Box(BoxBorder {
                    color: None,
                    width: 1,
                    style: BoxStyle::Flat,
                }));
            }
            if let Some(n) = value.as_fixnum() {
                return Some(FaceAttrValue::Box(BoxBorder {
                    color: None,
                    width: n as i32,
                    style: BoxStyle::Flat,
                }));
            }
            // Color string shorthand
            if let Some(s) = value.as_utf8_str() {
                let color = Color::from_name(s).or_else(|| Color::from_hex(s));
                return Some(FaceAttrValue::Box(BoxBorder {
                    color,
                    width: 1,
                    style: BoxStyle::Flat,
                }));
            }
            // Plist form: (:line-width WIDTH :color COLOR :style STYLE)
            if let Some(plist) = super::value::list_to_vec(&value) {
                let mut border = BoxBorder {
                    color: None,
                    width: 1,
                    style: BoxStyle::Flat,
                };
                let mut i = 0;
                while i + 1 < plist.len() {
                    let key = plist[i].as_symbol_name().unwrap_or("");
                    let val = &plist[i + 1];
                    match key {
                        ":line-width" => {
                            if let Some(n) = val.as_fixnum() {
                                border.width = n as i32;
                            }
                        }
                        ":color" => {
                            if let Some(s) = val.as_utf8_str().or_else(|| val.as_symbol_name()) {
                                border.color = Color::from_name(s).or_else(|| Color::from_hex(s));
                            }
                        }
                        ":style" => {
                            border.style = val
                                .as_symbol_name()
                                .and_then(BoxStyle::from_symbol)
                                .unwrap_or(BoxStyle::Flat);
                        }
                        _ => {}
                    }
                    i += 2;
                }
                return Some(FaceAttrValue::Box(border));
            }
            Some(FaceAttrValue::Box(BoxBorder {
                color: None,
                width: 1,
                style: BoxStyle::Flat,
            }))
        }
        LFaceAttr::InverseVideo | LFaceAttr::Extend => Some(FaceAttrValue::Bool(value.is_truthy())),
        LFaceAttr::Inherit => {
            // Store raw face_ref. Matches GNU's `LFACE_INHERIT_INDEX`
            // slot which holds any face_ref (symbol / list / plist);
            // `merge_face_ref` dispatches on shape at resolution time.
            if value.is_nil() || value.is_symbol_named("nil") {
                return Some(FaceAttrValue::Inherit(None));
            }
            Some(FaceAttrValue::Inherit(Some(value)))
        }
        LFaceAttr::Stipple => {
            // Store the raw stipple spec (a `(W H DATA)` cons, a bitmap file
            // string, or a symbol). GNU keeps it in `LFACE_STIPPLE_INDEX` and
            // realizes it to a pixmap at draw time; neomacs realizes it to a
            // `StipplePattern` in the layout bridge (`realize_face`).
            if value.is_nil() || value.is_symbol_named("nil") {
                Some(FaceAttrValue::Stipple(None))
            } else {
                Some(FaceAttrValue::Stipple(Some(value)))
            }
        }
        LFaceAttr::Font | LFaceAttr::Fontset => None,
    }
}
pub(crate) fn builtin_internal_get_lisp_face_attribute(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("internal-get-lisp-face-attribute", &args, 2)?;
    expect_max_args("internal-get-lisp-face-attribute", &args, 3)?;
    let defaults_frame = if let Some(frame) = args.get(2) {
        if frame.is_nil() {
            false
        } else if frame.is_t() {
            true
        } else if live_frame_designator_in_state(&eval.frames, frame) {
            false
        } else {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("frame-live-p"), *frame],
            ));
        }
    } else {
        false
    };

    let face_name = resolve_face_name_for_domain(eval, &args[0], defaults_frame)?;
    let attr_name = normalize_face_attribute_name(&args[1])?;

    if defaults_frame {
        if let Some(vector) = ensure_global_lisp_face_vector(eval, &face_name)
            && let Some(value) = lisp_face_vector_attr(vector, attr_name)
        {
            return Ok(value);
        }
        return Ok(lisp_face_attribute_value(&face_name, attr_name, true));
    }

    let frame_id = match args.get(2) {
        None => Some(super::window_cmds::ensure_selected_frame_id(eval)),
        Some(v) if v.is_nil() => Some(super::window_cmds::ensure_selected_frame_id(eval)),
        Some(frame) if live_frame_designator_in_state(&eval.frames, frame) => {
            frame_id_from_designator(frame)
        }
        _ => None,
    };

    if face_name == "default"
        && get_face_override(&face_name, attr_name, false).is_none()
        && matches!(
            attr_name,
            LFaceAttr::Font
                | LFaceAttr::Family
                | LFaceAttr::Foundry
                | LFaceAttr::Weight
                | LFaceAttr::Slant
                | LFaceAttr::Width
                | LFaceAttr::Height
        )
        && let Some(frame_id) = frame_id
        && let Some(fallback) = live_frame_font_attribute_fallback(eval, frame_id, attr_name)
    {
        return Ok(fallback);
    }

    let lisp_value = frame_id
        .and_then(|frame_id| {
            let initial = if is_known_lisp_face_name(&face_name) {
                FrameFaceInitial::SelectedBase
            } else {
                FrameFaceInitial::Empty
            };
            ensure_frame_lisp_face_vector(eval, frame_id, &face_name, initial)
        })
        .and_then(|vector| lisp_face_vector_attr(vector, attr_name))
        .unwrap_or_else(|| lisp_face_attribute_value(&face_name, attr_name, false));
    // GNU `internal-get-lisp-face-attribute` (xfaces.c) returns the LISP face
    // attribute (`LFACE_*` of `lface_from_face_name`), never the *realized*
    // face. Do NOT fall back to the runtime realized face here: the realized
    // face on this batch/mono frame still carries colors realized for a
    // color-capable display during the bootstrap image build (e.g. `error`
    // :foreground "#ff0000"), whereas GNU returns `unspecified` because no
    // display clause of the defface spec matched a mono terminal. The lisp face
    // value above (frame lisp vector slot, falling back to the base/override
    // value) is the GNU-faithful answer.
    Ok(lisp_value)
}

/// `(internal-lisp-face-attribute-values ATTR)` -- return valid discrete values
/// for known boolean-like face attributes.
pub(crate) fn builtin_internal_lisp_face_attribute_values(args: Vec<Value>) -> EvalResult {
    expect_args("internal-lisp-face-attribute-values", &args, 1)?;
    let attr_name = face_attr_value_name(&args[0])?;
    if LFaceAttr::from_keyword(resolve_sym(attr_name)).is_some_and(LFaceAttr::is_discrete_boolean) {
        Ok(Value::list(vec![Value::T, Value::NIL]))
    } else {
        Ok(Value::NIL)
    }
}

/// `(internal-lisp-face-equal-p FACE1 FACE2 &optional FRAME)` -- return t if
/// FACE1 and FACE2 resolve to equal face attributes in the selected frame or in
/// default face definitions when FRAME is t.
pub(crate) fn builtin_internal_lisp_face_equal_p(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("internal-lisp-face-equal-p", &args, 2)?;
    expect_max_args("internal-lisp-face-equal-p", &args, 3)?;
    let defaults_frame = frame_defaults_flag(args.get(2))?;
    let face1 = resolve_known_face_name_for_compare(eval, &args[0], defaults_frame)?;
    let face2 = resolve_known_face_name_for_compare(eval, &args[1], defaults_frame)?;
    for attr in LFACE_ATTRS {
        let v1 = lisp_face_attribute_value(&face1, attr, defaults_frame);
        let v2 = lisp_face_attribute_value(&face2, attr, defaults_frame);
        if v1 != v2 {
            return Ok(Value::NIL);
        }
    }
    Ok(Value::T)
}

/// `(internal-lisp-face-empty-p FACE &optional FRAME)` -- return t if FACE has
/// only unspecified attributes in selected/default face definitions.
pub(crate) fn builtin_internal_lisp_face_empty_p(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("internal-lisp-face-empty-p", &args, 1)?;
    expect_max_args("internal-lisp-face-empty-p", &args, 2)?;
    let defaults_frame = frame_defaults_flag(args.get(1))?;
    let face = resolve_known_face_name_for_compare(eval, &args[0], defaults_frame)?;
    for attr in LFACE_ATTRS {
        let v = lisp_face_attribute_value(&face, attr, defaults_frame);
        if !v.is_symbol_named("unspecified") {
            return Ok(Value::NIL);
        }
    }
    Ok(Value::T)
}

pub(crate) fn builtin_internal_merge_in_global_face(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("internal-merge-in-global-face", &args, 2)?;
    if !frame_device_designator_p(&args[1]) {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), args[1]],
        ));
    }
    let face_name = resolve_face_name_for_merge(eval, &args[0])?;
    if !is_known_lisp_face_name(&face_name) {
        mark_created_lisp_face(&face_name);
        mark_selected_created_lisp_face(&face_name);
    }
    merge_defaults_overrides_into_selected(&face_name);
    let frame_id = frame_id_from_designator(&args[1])
        .expect("validated frame designator should decode to frame id");
    if let (Some(global_vector), Some(local_vector)) = (
        ensure_global_lisp_face_vector(eval, &face_name),
        ensure_frame_lisp_face_vector(eval, frame_id, &face_name, FrameFaceInitial::Empty),
    ) {
        for attr in LFACE_ATTRS {
            let Some(global_value) = lisp_face_vector_attr(global_vector, attr) else {
                continue;
            };
            if global_value.is_symbol_named(":ignore-defface") {
                set_lisp_face_vector_attr(local_vector, attr, Value::symbol("unspecified"));
            } else if !global_value.is_symbol_named("unspecified") {
                set_lisp_face_vector_attr(local_vector, attr, global_value);
            }
        }
        sync_face_overrides_from_lisp_face_vector(&face_name, local_vector, false);
    }

    eval.face_change_count += 1;
    Ok(Value::NIL)
}

/// `(face-attribute-relative-p ATTRIBUTE VALUE)` -- return t if VALUE is the
/// value is a relative form for ATTRIBUTE.
pub(crate) fn builtin_face_attribute_relative_p(args: Vec<Value>) -> EvalResult {
    expect_args("face-attribute-relative-p", &args, 2)?;
    let value_is_relative_reset = args[1]
        .as_symbol_id()
        .or_else(|| args[1].as_keyword_id())
        .is_some_and(|id_| {
            matches!(
                resolve_sym(id_),
                "unspecified" | ":ignore-defface" | "ignore-defface"
            )
        });
    if value_is_relative_reset {
        return Ok(Value::T);
    }

    let height_attr = match args[0].kind() {
        ValueKind::Symbol(id) => {
            let n = resolve_sym(id);
            n == "height" || n == ":height"
        }
        _ => false,
    };
    if !height_attr {
        return Ok(Value::NIL);
    }

    Ok(Value::bool_val(
        !(args[1].is_fixnum() || args[1].as_char().is_some()),
    ))
}

/// `(merge-face-attribute ATTRIBUTE VALUE1 VALUE2)` -- return VALUE1 unless it
/// is the symbol `unspecified`, in which case return VALUE2.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_merge_face_attribute(args: Vec<Value>) -> EvalResult {
    expect_args("merge-face-attribute", &args, 3)?;
    Ok(merge_face_attribute_impl(None, &args))
}

pub(crate) fn builtin_merge_face_attribute_with_eval(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("merge-face-attribute", &args, 3)?;
    Ok(merge_face_attribute_impl(Some(eval), &args))
}

fn merge_face_attribute_impl(eval: Option<&mut super::eval::Context>, args: &[Value]) -> Value {
    let value1_is_relative_reset = args[1]
        .as_symbol_id()
        .or_else(|| args[1].as_keyword_id())
        .is_some_and(|id_| {
            matches!(
                resolve_sym(id_),
                "unspecified" | ":ignore-defface" | "ignore-defface"
            )
        });
    if value1_is_relative_reset {
        return args[2];
    }

    let height_attr = args[0]
        .as_symbol_id()
        .or_else(|| args[0].as_keyword_id())
        .is_some_and(|id_| matches!(resolve_sym(id_), "height" | ":height"));
    if height_attr {
        return merge_face_height_value(eval, args[1], args[2], args[1]);
    }

    args[1]
}

/// `(face-list &optional FRAME)` -- return list of known face names.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_face_list(args: Vec<Value>) -> EvalResult {
    expect_max_args("face-list", &args, 1)?;
    Ok(Value::list(
        all_defined_face_names_sorted_by_id_desc()
            .iter()
            .map(|name| Value::symbol(name.as_str()))
            .collect(),
    ))
}

fn expect_color_string(value: &Value) -> Result<String, Flow> {
    match value.kind() {
        ValueKind::String => Ok(font_string_text(value).expect("checked string")),
        _other => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), *value],
        )),
    }
}

fn expect_optional_color_frame_arg(args: &[Value], idx: usize) -> Result<(), Flow> {
    if let Some(frame) = args.get(idx)
        && !frame.is_nil()
        && !frame.is_frame()
    {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("framep"), *frame],
        ));
    }
    Ok(())
}

fn selected_or_designated_live_frame_id(
    frames: &FrameManager,
    frame: Option<&Value>,
) -> Result<FrameId, Flow> {
    match frame {
        None => frames
            .selected_frame()
            .map(|frame| frame.id)
            .ok_or_else(|| signal("error", vec![Value::string("No selected frame")])),
        Some(v) if v.is_nil() => frames
            .selected_frame()
            .map(|frame| frame.id)
            .ok_or_else(|| signal("error", vec![Value::string("No selected frame")])),
        Some(value) if live_frame_designator_in_state(frames, value) => {
            Ok(frame_id_from_designator(value)
                .expect("live frame designator should decode to frame id"))
        }
        Some(other) => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), *other],
        )),
    }
}

fn graphic_color_target_frame_id(
    ctx: &super::eval::Context,
    frame: Option<&Value>,
) -> Result<Option<FrameId>, Flow> {
    let frame_id = selected_or_designated_live_frame_id(&ctx.frames, frame)?;
    Ok(ctx
        .frames
        .get(frame_id)
        .and_then(|frame| frame.effective_window_system())
        .filter(|window_system| super::display::gui_window_system_active_value(*window_system))
        .map(|_| frame_id))
}

fn parse_color_16bit_any(color_name: &str) -> Option<(i64, i64, i64)> {
    let lower = color_name.trim().to_lowercase();
    if let Some(hex) = lower.strip_prefix('#') {
        parse_hex_color_16bit(hex)
    } else {
        parse_named_color_16bit(&lower)
    }
}

/// `(color-defined-p COLOR &optional FRAME)` -- nil if unknown; otherwise truthy
/// for known RGB/hex and supported terminal color names.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_color_defined_p(args: Vec<Value>) -> EvalResult {
    expect_min_args("color-defined-p", &args, 1)?;
    expect_max_args("color-defined-p", &args, 2)?;
    expect_optional_color_device_arg(&args, 1)?;
    match args[0].kind() {
        ValueKind::String => Ok(Value::bool_val(
            !builtin_color_values(vec![args[0]])?.is_nil(),
        )),
        _ => Ok(Value::NIL),
    }
}

pub(crate) fn builtin_xw_color_defined_p_ctx(
    ctx: &super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("xw-color-defined-p", &args, 1)?;
    expect_max_args("xw-color-defined-p", &args, 2)?;
    expect_optional_color_frame_arg(&args, 1)?;
    if graphic_color_target_frame_id(ctx, args.get(1))?.is_none() {
        return Ok(Value::NIL);
    }
    let color_name = match args[0].kind() {
        ValueKind::String => font_string_text(&args[0]).expect("checked string"),
        _ => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("stringp"), args[0]],
            ));
        }
    };
    Ok(Value::bool_val(
        parse_color_16bit_any(&color_name).is_some(),
    ))
}

/// `(color-values COLOR &optional FRAME)` -- resolve COLOR and return a
/// terminal-compatible `(R G B)` list with 16-bit component values.
///
/// In batch/TTY compatibility mode we approximate resolved colors to the
/// nearest entry in the 8-color terminal palette.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_color_values(args: Vec<Value>) -> EvalResult {
    expect_min_args("color-values", &args, 1)?;
    expect_max_args("color-values", &args, 2)?;
    expect_optional_color_device_arg(&args, 1)?;
    let color_name = match args[0].kind() {
        ValueKind::String => font_string_text(&args[0]).expect("checked string"),
        _ => return Ok(Value::NIL),
    };
    let lower = color_name.trim().to_lowercase();
    let resolved = if let Some(hex) = lower.strip_prefix('#') {
        parse_hex_color_16bit(hex)
    } else {
        parse_named_color_16bit(&lower)
    };
    let Some((r, g, b)) = resolved.map(approximate_tty_color) else {
        return Ok(Value::NIL);
    };
    Ok(Value::list(vec![
        Value::fixnum(r),
        Value::fixnum(g),
        Value::fixnum(b),
    ]))
}

pub(crate) fn builtin_xw_color_values_ctx(
    ctx: &super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("xw-color-values", &args, 1)?;
    expect_max_args("xw-color-values", &args, 2)?;
    expect_optional_color_frame_arg(&args, 1)?;
    if graphic_color_target_frame_id(ctx, args.get(1))?.is_none() {
        return Ok(Value::NIL);
    }
    let color_name = match args[0].kind() {
        ValueKind::String => font_string_text(&args[0]).expect("checked string"),
        _ => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("stringp"), args[0]],
            ));
        }
    };
    let Some((r, g, b)) = parse_color_16bit_any(&color_name) else {
        return Ok(Value::NIL);
    };
    Ok(Value::list(vec![
        Value::fixnum(r),
        Value::fixnum(g),
        Value::fixnum(b),
    ]))
}

/// `(color-values-from-color-spec COLOR-SPEC)` -- parse hex color spec and
/// return raw `(R G B)` 16-bit channel values.
pub(crate) fn builtin_color_values_from_color_spec(args: Vec<Value>) -> EvalResult {
    expect_args("color-values-from-color-spec", &args, 1)?;
    let color_spec = expect_color_string(&args[0])?;
    let lower = color_spec.trim().to_lowercase();
    let Some(hex) = lower.strip_prefix('#') else {
        return Ok(Value::NIL);
    };
    let Some((r, g, b)) = parse_hex_color_16bit(hex) else {
        return Ok(Value::NIL);
    };
    Ok(Value::list(vec![
        Value::fixnum(r),
        Value::fixnum(g),
        Value::fixnum(b),
    ]))
}

/// `(color-gray-p COLOR &optional FRAME)` -- t if COLOR resolves to equal RGB
/// channels, nil otherwise.
pub(crate) fn builtin_color_gray_p(ctx: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("color-gray-p", &args, 1)?;
    expect_max_args("color-gray-p", &args, 2)?;
    let _ = expect_color_string(&args[0])?;
    expect_optional_color_frame_arg(&args, 1)?;
    // GNU `Fcolor_gray_p` -> `face_color_gray_p` (xfaces.c:1214-1235) resolves
    // the name through the frame's colour hook -- the same `tty-color-desc'
    // path as `color-values'/`color-distance', not a private name table -- and
    // treats an unresolvable colour as simply "not gray" (false), never an
    // error.
    let graphic = graphic_color_target_frame_id(ctx, args.get(1))
        .map(|id| id.is_some())
        .unwrap_or(false);
    let Ok((r, g, b)) = resolve_color_distance_rgb(ctx, &args[0], graphic) else {
        return Ok(Value::NIL);
    };
    Ok(Value::bool_val(color_is_gray(r, g, b)))
}

/// GNU `face_color_gray_p` (xfaces.c:1214-1235): a colour is "gray" if it is
/// close to black (every 16-bit channel < 5000) or its channels are within 5%
/// (`max/20`) of one another.
fn color_is_gray(r: i64, g: i64, b: i64) -> bool {
    if r < 5000 && g < 5000 && b < 5000 {
        return true;
    }
    (r - g).abs() < r.max(g) / 20 && (g - b).abs() < g.max(b) / 20 && (b - r).abs() < b.max(r) / 20
}

/// `(color-supported-p COLOR &optional FRAME BACKGROUND-P)` -- t if COLOR
/// resolves on this build's color parser.
pub(crate) fn builtin_color_supported_p(args: Vec<Value>) -> EvalResult {
    expect_min_args("color-supported-p", &args, 1)?;
    expect_max_args("color-supported-p", &args, 3)?;
    let color = expect_color_string(&args[0])?;
    expect_optional_color_frame_arg(&args, 1)?;
    let _ = args.get(2);
    Ok(Value::bool_val(parse_color_16bit_any(&color).is_some()))
}

fn expect_optional_color_distance_frame_arg(args: &[Value], idx: usize) -> Result<(), Flow> {
    if let Some(frame) = args.get(idx)
        && !frame.is_nil()
        && !frame.is_frame()
    {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("frame-live-p"), *frame],
        ));
    }
    Ok(())
}

fn invalid_color_error(value: &Value) -> Flow {
    signal("error", vec![Value::string("Invalid color"), *value])
}

/// Parse an `(R G B)` list of fixnums, mirroring GNU `parse_rgb_list`
/// (`src/xfaces.c`). Returns None unless the value is a 3+ element list whose
/// first three elements are fixnums.
fn parse_rgb_list(value: &Value) -> Option<(i64, i64, i64)> {
    let items = list_to_vec(value)?;
    if items.len() < 3 {
        return None;
    }
    Some((
        items[0].as_fixnum()?,
        items[1].as_fixnum()?,
        items[2].as_fixnum()?,
    ))
}

/// Resolve a `color-distance` argument to a 16-bit `(R G B)` triple, mirroring
/// GNU `Fcolor_distance` (`src/xfaces.c:4792`): an `(R G B)` list parses
/// directly via `parse_rgb_list`; a string is resolved through the frame
/// terminal's `defined_color_hook`. On a graphic frame that hook parses the
/// raw RGB; on a TTY frame (`tty_defined_color`) it calls `tty_lookup_color`,
/// which dispatches to the Lisp `tty-color-desc` to find the nearest entry in
/// the active terminal palette. We reproduce that TTY path so batch results
/// match GNU (e.g. "#808080" and "#c0c0c0" both quantize to white).
fn resolve_color_distance_rgb(
    ctx: &mut super::eval::Context,
    value: &Value,
    graphic: bool,
) -> Result<(i64, i64, i64), Flow> {
    if let Some(rgb) = parse_rgb_list(value) {
        return Ok(rgb);
    }
    if !value.is_string() {
        return Err(invalid_color_error(value));
    }
    let color = font_string_text(value).expect("checked string");
    if graphic {
        return parse_color_16bit_any(&color).ok_or_else(|| invalid_color_error(value));
    }
    // TTY frame: resolve via `tty-color-desc' -> (NAME INDEX R G B), exactly as
    // GNU's `tty_lookup_color' does. GNU guards this with `Ffboundp
    // (Qtty_color_desc)' and treats a failed lookup as "not resolved" (false),
    // never an error; mirror that so a bare environment (e.g. unit tests
    // without term/tty-colors.el loaded, where the call may signal) falls back
    // to a coarse quantization instead of propagating the signal.
    if ctx.obarray.fboundp("tty-color-desc")
        && let Ok(desc) = ctx.funcall_general(Value::symbol("tty-color-desc"), vec![*value])
        && let Some(items) = list_to_vec(&desc)
        && items.len() >= 5
        && let (Some(r), Some(g), Some(b)) = (
            items[2].as_fixnum(),
            items[3].as_fixnum(),
            items[4].as_fixnum(),
        )
    {
        return Ok((r, g, b));
    }
    parse_color_16bit_any(&color)
        .map(approximate_tty_color)
        .ok_or_else(|| invalid_color_error(value))
}

fn color_distance_metric(lhs: (i64, i64, i64), rhs: (i64, i64, i64)) -> i64 {
    // GNU `color_distance` (xfaces.c): the Riemersma colour metric over 16-bit
    // channels (the inputs here are already 0..65535). This is a more even
    // approximation of L*u*v* than the 8-bit redmean variant.
    // See https://www.compuphase.com/cmetric.htm
    let r = lhs.0 - rhs.0;
    let g = lhs.1 - rhs.1;
    let b = lhs.2 - rhs.2;
    let r_mean = (lhs.0 + rhs.0) >> 1;
    ((((2 * 65536 + r_mean) * r * r) >> 16)
        + 4 * g * g
        + (((2 * 65536 + 65535 - r_mean) * b * b) >> 16))
        >> 16
}

/// `(color-distance COLOR1 COLOR2 &optional FRAME METRIC)` -- return a
/// perceptual distance between colors. Mirrors GNU `Fcolor_distance`.
pub(crate) fn builtin_color_distance(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("color-distance", &args, 2)?;
    expect_max_args("color-distance", &args, 4)?;
    expect_optional_color_distance_frame_arg(&args, 2)?;
    // GNU resolves the frame's terminal type (graphic vs TTY) to pick the
    // colour-definition hook. When no frame is available (e.g. a bare
    // headless context), default to the TTY path, which is also the
    // batch/`--batch' default.
    let graphic = graphic_color_target_frame_id(ctx, args.get(2))
        .map(|id| id.is_some())
        .unwrap_or(false);
    let lhs = resolve_color_distance_rgb(ctx, &args[0], graphic)?;
    let rhs = resolve_color_distance_rgb(ctx, &args[1], graphic)?;
    if let Some(metric) = args.get(3).filter(|m| !m.is_nil()) {
        // GNU calls METRIC with two (RED GREEN BLUE) lists.
        let metric = *metric;
        return ctx.funcall_general(
            metric,
            vec![
                Value::list(vec![
                    Value::fixnum(lhs.0),
                    Value::fixnum(lhs.1),
                    Value::fixnum(lhs.2),
                ]),
                Value::list(vec![
                    Value::fixnum(rhs.0),
                    Value::fixnum(rhs.1),
                    Value::fixnum(rhs.2),
                ]),
            ],
        );
    }
    Ok(Value::fixnum(color_distance_metric(lhs, rhs)))
}

fn parse_hex_color_16bit(hex: &str) -> Option<(i64, i64, i64)> {
    match hex.len() {
        3 => {
            let r = i64::from(hex[0..1].chars().next()?.to_digit(16)? as u16);
            let g = i64::from(hex[1..2].chars().next()?.to_digit(16)? as u16);
            let b = i64::from(hex[2..3].chars().next()?.to_digit(16)? as u16);
            Some((
                r | (r << 4) | (r << 8) | (r << 12),
                g | (g << 4) | (g << 8) | (g << 12),
                b | (b << 4) | (b << 8) | (b << 12),
            ))
        }
        6 => Some((
            i64::from(u16::from_str_radix(&hex[0..2], 16).ok()?) * 257,
            i64::from(u16::from_str_radix(&hex[2..4], 16).ok()?) * 257,
            i64::from(u16::from_str_radix(&hex[4..6], 16).ok()?) * 257,
        )),
        12 => Some((
            i64::from(u16::from_str_radix(&hex[0..4], 16).ok()?),
            i64::from(u16::from_str_radix(&hex[4..8], 16).ok()?),
            i64::from(u16::from_str_radix(&hex[8..12], 16).ok()?),
        )),
        _ => None,
    }
}

fn parse_named_color_16bit(name: &str) -> Option<(i64, i64, i64)> {
    let color = crate::face::Color::from_name(name)?;
    Some((
        i64::from(color.r) * 257,
        i64::from(color.g) * 257,
        i64::from(color.b) * 257,
    ))
}

fn approximate_tty_color((r, g, b): (i64, i64, i64)) -> (i64, i64, i64) {
    // Emacs batch/TTY behavior is effectively a coarse 8-color quantization.
    // A narrow channel spread is treated as gray, otherwise channels are
    // quantized relative to the local min/max midpoint.
    const GRAY_BAND: i64 = 0x1111;
    const BRIGHT_THRESHOLD: i64 = 0x8888;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max - min <= GRAY_BAND {
        return if max >= BRIGHT_THRESHOLD {
            (65535, 65535, 65535)
        } else {
            (0, 0, 0)
        };
    }

    let mid = (max + min) / 2;
    (
        if r >= mid { 65535 } else { 0 },
        if g >= mid { 65535 } else { 0 },
        if b >= mid { 65535 } else { 0 },
    )
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn invalid_get_device_terminal_error(value: &Value) -> Flow {
    signal(
        "error",
        vec![Value::string(format!(
            "Invalid argument {} in 'get-device-terminal'",
            super::print::print_value(value)
        ))],
    )
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn color_device_designator_p(value: &Value) -> bool {
    match value.kind() {
        ValueKind::Nil => true,
        _ => frame_device_designator_p(value),
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn expect_optional_color_device_arg(args: &[Value], idx: usize) -> Result<(), Flow> {
    if let Some(value) = args.get(idx)
        && !color_device_designator_p(value)
    {
        return Err(invalid_get_device_terminal_error(value));
    }
    Ok(())
}

/// `(defined-colors &optional FRAME)` -- return a list of defined color names.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_defined_colors(args: Vec<Value>) -> EvalResult {
    expect_max_args("defined-colors", &args, 1)?;
    expect_optional_color_device_arg(&args, 0)?;
    let colors = vec![
        "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
    ];
    Ok(Value::list(colors.into_iter().map(Value::string).collect()))
}

/// `(face-id FACE &optional FRAME)` -- return numeric face id for known and
/// dynamically created faces.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_face_id(args: Vec<Value>) -> EvalResult {
    expect_min_args("face-id", &args, 1)?;
    expect_max_args("face-id", &args, 2)?;
    if args[0].is_string() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("symbolp"), args[0]],
        ));
    }

    if let Some(name) = symbol_name_for_face_value(&args[0]) {
        if let Some(id) = face_id_for_name(&name) {
            return Ok(Value::fixnum(id));
        }
        if is_created_lisp_face(&name) {
            ensure_dynamic_face_id(&name);
            if let Some(id) = face_id_for_name(&name) {
                return Ok(Value::fixnum(id));
            }
        }
    }
    let rendered = super::print::print_value(&args[0]);
    Err(signal(
        "error",
        vec![Value::string(format!("Not a face: {rendered}"))],
    ))
}

pub(crate) fn builtin_face_font(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("face-font", &args, 1)?;
    expect_max_args("face-font", &args, 3)?;

    let defaults_frame = args.get(1).is_some_and(|v| v.is_t());
    if defaults_frame {
        let face_name = resolve_face_name_for_domain(eval, &args[0], true)?;
        let mut styles = Vec::new();
        let weight = lisp_face_attribute_value(&face_name, LFaceAttr::Weight, true);
        if matches!(weight.as_symbol_name(), Some(name) if name != "normal" && name != "unspecified")
        {
            styles.push(Value::symbol("bold"));
        }
        let slant = lisp_face_attribute_value(&face_name, LFaceAttr::Slant, true);
        if matches!(slant.as_symbol_name(), Some(name) if name != "normal" && name != "unspecified")
        {
            styles.push(Value::symbol("italic"));
        }
        return if styles.is_empty() {
            Ok(Value::NIL)
        } else {
            Ok(Value::list(styles))
        };
    }

    let frame_id = match args.get(1) {
        None => super::window_cmds::ensure_selected_frame_id(eval),
        Some(v) if v.is_nil() => super::window_cmds::ensure_selected_frame_id(eval),
        Some(frame) if live_frame_designator_in_state(&eval.frames, frame) => {
            frame_id_from_designator(frame)
                .expect("live frame designator should decode to frame id")
        }
        Some(other) => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("frame-live-p"), *other],
            ));
        }
    };
    let frame = eval
        .frames
        .get(frame_id)
        .ok_or_else(|| signal("error", vec![Value::string("No selected frame")]))?;
    if frame.window_system.is_none() {
        // GNU `Fface_font` (xfaces.c) calls `lookup_named_face (..., true)`,
        // which signals "Invalid face" only when the name does not resolve to
        // any face. A face created at runtime via `make-face`/`defface` is a
        // valid (but on a TTY frame, unrealized) face, so it must return nil
        // rather than error. Use the full existence check, not just the
        // bootstrap built-in table.
        return match resolve_face_designator(eval, args[0], FaceAliasCyclePolicy::Signal)? {
            ResolvedFaceDesignator::Symbol(name) | ResolvedFaceDesignator::String(name) => {
                if face_exists_for_domain(name.name(), false) {
                    Ok(Value::NIL)
                } else if name.symbol().is_nil() {
                    Err(signal("error", vec![Value::string("Invalid face")]))
                } else {
                    Err(signal(
                        "error",
                        vec![Value::string("Invalid face"), name.symbol()],
                    ))
                }
            }
            ResolvedFaceDesignator::Other(value) => {
                Err(signal("error", vec![Value::string("Invalid face"), value]))
            }
        };
    }

    let face_name = resolve_face_name_for_domain(eval, &args[0], false)?;
    let remapping = face_remapping_for_current_buffer(eval);
    let face = if remapping.is_empty() {
        eval.face_table.resolve(&face_name)
    } else {
        eval.face_table
            .resolve_with_remapping(&face_name, &remapping)
    };
    if let Some(character) = args.get(2).filter(|value| !value.is_nil()) {
        let code = super::builtins::expect_character_code(character)? as u32;
        let Some(ch) = char::from_u32(code) else {
            return Ok(font_name_value(&build_font_object(&face)).unwrap_or(Value::NIL));
        };
        if let Some(matched) = resolve_font_match(eval, frame_id, ch, &face) {
            return Ok(
                font_name_value(&build_font_object_for_match(&face, &matched))
                    .unwrap_or(Value::NIL),
            );
        }
    }

    Ok(font_name_value(&build_font_object(&face)).unwrap_or(Value::NIL))
}

pub(crate) fn builtin_font_info(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("font-info", &args, 1)?;
    expect_max_args("font-info", &args, 2)?;

    if !(args[0].is_string()
        || is_font(&args[0])
        || is_font_entity(&args[0])
        || is_font_object(&args[0]))
    {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), args[0]],
        ));
    }

    let frame_id = match args.get(1) {
        None => super::window_cmds::ensure_selected_frame_id(eval),
        Some(v) if v.is_nil() => super::window_cmds::ensure_selected_frame_id(eval),
        Some(frame) if live_frame_designator_in_state(&eval.frames, frame) => {
            frame_id_from_designator(frame)
                .expect("live frame designator should decode to frame id")
        }
        Some(other) => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("frame-live-p"), *other],
            ));
        }
    };
    let has_window_system = eval
        .frames
        .get(frame_id)
        .ok_or_else(|| signal("error", vec![Value::string("No selected frame")]))?
        .window_system
        .is_some();
    if !has_window_system {
        return Ok(Value::NIL);
    }

    if is_font_entity(&args[0]) || is_font_object(&args[0]) {
        // GNU opens the font itself (font_open_entity for entities; a
        // font-at object is already opened at its pixel size) and reports
        // the OPENED font's metrics; only fall back to the frame font when
        // the value can't be probed (no file, unreadable, ...).
        if let Some(info) = font_info_vector_for_entity(eval, &args[0]) {
            return Ok(info);
        }
    }
    // GNU attaches (opentype . caps) to font-info for OPENED fonts too
    // (font-at objects); compute it from the font's file before borrowing
    // the frame.
    let capability = font_value_otf_capability(eval, &args[0]);
    let frame = eval
        .frames
        .get(frame_id)
        .ok_or_else(|| signal("error", vec![Value::string("No selected frame")]))?;
    if args[0].is_string() || is_font(&args[0]) {
        Ok(font_info_vector_for_runtime_font(
            &args[0], frame, capability,
        ))
    } else {
        Ok(Value::NIL)
    }
}

/// `(internal-face-x-get-resource RESOURCE CLASS FRAME)` -- validate arguments and
/// return nil (font resource lookup is not implemented).
pub(crate) fn builtin_internal_face_x_get_resource(args: Vec<Value>) -> EvalResult {
    expect_min_args("internal-face-x-get-resource", &args, 2)?;
    expect_max_args("internal-face-x-get-resource", &args, 3)?;
    for arg in args.iter().take(2) {
        if !arg.is_string() {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("stringp"), *arg],
            ));
        }
    }
    Ok(Value::NIL)
}

/// `(internal-set-font-selection-order ORDER)` -- validate order list shape and return nil.
pub(crate) fn builtin_internal_set_font_selection_order(args: Vec<Value>) -> EvalResult {
    expect_args("internal-set-font-selection-order", &args, 1)?;
    let order = &args[0];
    if !order.is_nil() && !order.is_cons() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("listp"), *order],
        ));
    }

    let valid_keywords = [":width", ":height", ":weight", ":slant"];
    let valid = if let Some(values) = list_to_vec(order) {
        if values.len() == valid_keywords.len() {
            let mut seen = HashSet::new();
            values.iter().all(|value| {
                if let Some(id) = value.as_keyword_id() {
                    let s = resolve_sym(id);
                    let key = if s.starts_with(':') {
                        s.to_owned()
                    } else {
                        format!(":{s}")
                    };
                    valid_keywords.contains(&key.as_str()) && seen.insert(key)
                } else {
                    false
                }
            })
        } else {
            false
        }
    } else {
        false
    };

    if valid {
        return Ok(Value::NIL);
    }

    if let Some(values) = list_to_vec(order) {
        if values.is_empty() {
            return Err(signal(
                "error",
                vec![Value::string("Invalid font sort order")],
            ));
        }
        let mut payload = vec![Value::string("Invalid font sort order")];
        payload.extend(values);
        return Err(signal("error", payload));
    }

    Err(signal(
        "error",
        vec![Value::string("Invalid font sort order"), *order],
    ))
}

/// `(internal-set-alternative-font-family-alist ALIST)` -- normalize string
/// entries to symbols and return the normalized list.
pub(crate) fn builtin_internal_set_alternative_font_family_alist(args: Vec<Value>) -> EvalResult {
    expect_args("internal-set-alternative-font-family-alist", &args, 1)?;
    let entries = proper_list_to_vec_or_listp_error(&args[0])?;
    let mut normalized = Vec::with_capacity(entries.len());
    let mut alist = Vec::with_capacity(entries.len());
    for entry in entries {
        let members = proper_list_to_vec_or_listp_error(&entry)?;
        let mut converted = Vec::with_capacity(members.len());
        let mut names = Vec::with_capacity(members.len());
        for member in members {
            match member.kind() {
                ValueKind::String => {
                    // Issue #131: intern the family name faithfully (real Emacs
                    // bytes) rather than via the PUA-sentinel storage form.
                    let sym = crate::emacs_core::intern::intern_lisp_string(
                        member.as_lisp_string().expect("checked string"),
                    );
                    converted.push(Value::from_sym_id(sym));
                    names.push(sym);
                }
                _other => {
                    return Err(signal(
                        LispCondition::WrongTypeArgument,
                        vec![Value::symbol("stringp"), member],
                    ));
                }
            }
        }
        if let Some(name) = names.first().copied() {
            alist.push((name, names));
        }
        normalized.push(Value::list(converted));
    }
    if let Ok(mut state) = alternative_font_family_alist().write() {
        *state = alist;
    }
    clear_font_cache_state();
    Ok(Value::list(normalized))
}

/// `(internal-set-alternative-font-registry-alist ALIST)` -- downcase string
/// entries and return the normalized list.
pub(crate) fn builtin_internal_set_alternative_font_registry_alist(args: Vec<Value>) -> EvalResult {
    expect_args("internal-set-alternative-font-registry-alist", &args, 1)?;
    let entries = proper_list_to_vec_or_listp_error(&args[0])?;
    let mut normalized = Vec::with_capacity(entries.len());
    let mut alist = Vec::with_capacity(entries.len());
    for entry in entries {
        let members = proper_list_to_vec_or_listp_error(&entry)?;
        let mut converted = Vec::with_capacity(members.len());
        let mut names = Vec::with_capacity(members.len());
        for member in members {
            let downcased = crate::emacs_core::builtins::builtin_downcase(vec![member])?;
            if let Some(text) = downcased.as_lisp_string() {
                names.push(text.clone());
            }
            converted.push(downcased);
        }
        if names.len() == converted.len()
            && let Some(name) = names.first().cloned()
        {
            alist.push((name, names));
        }
        normalized.push(Value::list(converted));
    }
    if let Ok(mut state) = alternative_font_registry_alist().write() {
        *state = alist;
    }
    clear_font_cache_state();
    Ok(Value::list(normalized))
}

// ---------------------------------------------------------------------------
// xfaces.c: x-load-color-file
// ---------------------------------------------------------------------------

/// `(x-load-color-file FILENAME)` — read an RGB color file (rgb.txt format)
/// and return an alist of `(NAME R G B)` entries.
///
/// Each line has the format `R G B  name` where R/G/B are 0-255 decimal.
/// Lines starting with `!` or `#` are comments and are skipped.
pub(crate) fn builtin_x_load_color_file(args: Vec<Value>) -> EvalResult {
    expect_args("x-load-color-file", &args, 1)?;
    let filename = match args[0].kind() {
        ValueKind::String => font_string_text(&args[0]).expect("checked string"),
        _other => {
            return Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("stringp"), args[0]],
            ));
        }
    };

    // Expand the filename (resolve ~, relative paths, etc.)
    let expanded = super::fileio::expand_file_name(&filename, None);
    let contents = match std::fs::read_to_string(&expanded) {
        Ok(s) => s,
        Err(_) => return Ok(Value::NIL),
    };

    let mut result = Value::NIL;
    // Build alist in reverse order, then reverse (or build in correct order
    // by collecting into vec and reversing).
    let mut entries: Vec<Value> = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('!') || trimmed.starts_with('#') {
            continue;
        }
        // Parse: R G B  color-name
        let mut parts = trimmed.splitn(4, char::is_whitespace);
        let r_str = match parts.next() {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };
        // Skip whitespace between fields
        let g_str = loop {
            match parts.next() {
                Some(s) if !s.is_empty() => break s,
                Some(_) => continue,
                None => break "",
            }
        };
        if g_str.is_empty() {
            continue;
        }
        let b_str = loop {
            match parts.next() {
                Some(s) if !s.is_empty() => break s,
                Some(_) => continue,
                None => break "",
            }
        };
        if b_str.is_empty() {
            continue;
        }
        let name_part = loop {
            match parts.next() {
                Some(s) if !s.is_empty() => break s,
                Some(_) => continue,
                None => break "",
            }
        };
        if name_part.is_empty() {
            continue;
        }

        let r: u16 = match r_str.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let g: u16 = match g_str.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let b: u16 = match b_str.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Scale 0-255 to 0-65535 (same as Emacs: val * 257)
        let r16 = (r as i64) * 257;
        let g16 = (g as i64) * 257;
        let b16 = (b as i64) * 257;

        // Build (NAME R G B) as a proper list
        let color_entry = Value::cons(
            Value::string(name_part),
            Value::cons(
                Value::fixnum(r16),
                Value::cons(
                    Value::fixnum(g16),
                    Value::cons(Value::fixnum(b16), Value::NIL),
                ),
            ),
        );
        entries.push(color_entry);
    }

    // Build alist from entries (preserve file order)
    for entry in entries.into_iter().rev() {
        result = Value::cons(entry, result);
    }

    Ok(result)
}

// ===========================================================================
// Tests
// ===========================================================================
#[cfg(test)]
#[path = "font_test.rs"]
mod tests;
