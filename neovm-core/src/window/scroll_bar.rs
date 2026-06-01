use crate::emacs_core::value::Value;
use strum::{EnumString, IntoStaticStr};

/// GNU vertical scroll-bar type symbols accepted by window and frame code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum VerticalScrollBarType {
    Left,
    Right,
}

impl VerticalScrollBarType {
    pub fn from_symbol_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    pub fn from_symbol_value(value: &Value) -> Option<Self> {
        Self::from_symbol_name(value.as_symbol_name()?)
    }

    pub fn name(self) -> &'static str {
        self.into()
    }

    pub fn symbol(self) -> Value {
        Value::symbol(self.name())
    }
}

/// GNU horizontal scroll-bar type symbols accepted by window code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum HorizontalScrollBarType {
    Bottom,
}

impl HorizontalScrollBarType {
    pub fn from_symbol_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    pub fn from_symbol_value(value: &Value) -> Option<Self> {
        Self::from_symbol_name(value.as_symbol_name()?)
    }

    pub fn name(self) -> &'static str {
        self.into()
    }

    pub fn symbol(self) -> Value {
        Value::symbol(self.name())
    }
}

pub fn is_valid_vertical_scroll_bar_value(value: Value) -> bool {
    value.is_nil()
        || value == Value::T
        || VerticalScrollBarType::from_symbol_value(&value).is_some()
}

pub fn is_valid_horizontal_scroll_bar_value(value: Value) -> bool {
    value.is_nil()
        || value == Value::T
        || HorizontalScrollBarType::from_symbol_value(&value).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_bar_domains_match_gnu_symbols() {
        assert_eq!(
            VerticalScrollBarType::from_symbol_value(&Value::symbol("left")),
            Some(VerticalScrollBarType::Left)
        );
        assert_eq!(
            VerticalScrollBarType::from_symbol_value(&Value::symbol("right")),
            Some(VerticalScrollBarType::Right)
        );
        assert_eq!(VerticalScrollBarType::from_symbol_name("bottom"), None);
        assert!(is_valid_vertical_scroll_bar_value(Value::NIL));
        assert!(is_valid_vertical_scroll_bar_value(Value::T));
        assert!(!is_valid_vertical_scroll_bar_value(Value::symbol("bottom")));

        assert_eq!(
            HorizontalScrollBarType::from_symbol_value(&Value::symbol("bottom")),
            Some(HorizontalScrollBarType::Bottom)
        );
        assert_eq!(HorizontalScrollBarType::from_symbol_name("top"), None);
        assert!(is_valid_horizontal_scroll_bar_value(Value::NIL));
        assert!(is_valid_horizontal_scroll_bar_value(Value::T));
        assert!(!is_valid_horizontal_scroll_bar_value(Value::symbol("left")));
    }
}
