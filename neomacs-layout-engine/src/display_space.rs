use neovm_core::emacs_core::Value;
use strum::{EnumString, IntoStaticStr};

#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumString, IntoStaticStr)]
pub(crate) enum DisplaySpaceKey {
    #[strum(to_string = ":width")]
    Width,
    #[strum(to_string = ":relative-width")]
    RelativeWidth,
    #[strum(to_string = ":align-to")]
    AlignTo,
    #[strum(to_string = ":height")]
    Height,
    #[strum(to_string = ":relative-height")]
    RelativeHeight,
    #[strum(to_string = ":ascent")]
    Ascent,
}

impl DisplaySpaceKey {
    pub(crate) fn from_lisp_value(value: Value) -> Option<Self> {
        value.as_symbol_name().and_then(|name| name.parse().ok())
    }
}

pub(crate) fn is_display_space_spec(value: &Value) -> bool {
    value.is_cons() && value.cons_car().is_symbol_named("space")
}

pub(crate) fn display_space_positive_number(value: Value) -> Option<f32> {
    value
        .as_float()
        .or_else(|| value.as_int().map(|integer| integer as f64))
        .filter(|number| number.is_finite() && *number > 0.0)
        .map(|number| number as f32)
}
