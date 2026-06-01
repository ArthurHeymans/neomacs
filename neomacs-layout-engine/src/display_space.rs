use neovm_core::emacs_core::Value;
use strum::{EnumString, IntoStaticStr};

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
}
