use crate::display_item::{
    DisplayItemLayout, DisplayLength, DisplayLengthExpr, DisplayLengthSymbol,
    DisplayMediaReplacement, DisplayStretch, DisplayStretchWidth, DisplayXwidgetItem,
};
use crate::display_space::{DisplaySpaceKey, is_display_space_spec};
use crate::display_spec::{DisplaySpecHead, parse_display_xwidget_layout};
use neovm_core::emacs_core::Value;
use neovm_core::emacs_core::value::list_to_vec;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DisplayPropertyClassification {
    pub(crate) replacement: Option<DisplayReplacementProperty>,
    pub(crate) modifiers: DisplayTextPropertyModifiers,
}

impl DisplayPropertyClassification {
    pub(crate) fn replacement(&self) -> Option<&DisplayReplacementProperty> {
        self.replacement.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayReplacementProperty {
    String,
    Stretch(DisplayStretch),
    Media(DisplayMediaReplacementProperty),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DisplayMediaReplacementProperty {
    Image,
    Video,
    Xwidget(DisplayMediaReplacement),
    Webkit,
}

impl DisplayMediaReplacementProperty {
    pub(crate) fn direct_replacement(&self) -> Option<DisplayMediaReplacement> {
        match self {
            Self::Xwidget(media) => Some(*media),
            Self::Image | Self::Video | Self::Webkit => None,
        }
    }

    pub(crate) fn accepts_media_replacement(&self, media: &DisplayMediaReplacement) -> bool {
        matches!(
            (self, media.kind),
            (
                Self::Image,
                crate::display_item::DisplayMediaReplacementKind::Image { .. }
            ) | (
                Self::Video,
                crate::display_item::DisplayMediaReplacementKind::Video { .. }
            ) | (
                Self::Webkit,
                crate::display_item::DisplayMediaReplacementKind::Xwidget { .. }
            )
        )
    }

    pub(crate) fn uses_xwidget_cursor_extents(&self) -> bool {
        matches!(self, Self::Xwidget(_))
    }

    pub(crate) fn media_fallback_placeholder(&self) -> Option<&'static str> {
        match self {
            Self::Image => Some("[img]"),
            Self::Video | Self::Webkit => Some("     "),
            Self::Xwidget(_) => None,
        }
    }
}

pub(crate) type DisplayTextPropertyModifiers = DisplayItemLayout;

pub(crate) fn classify_display_property(value: Value) -> DisplayPropertyClassification {
    let replacement = if value.is_string() {
        Some(DisplayReplacementProperty::String)
    } else if is_display_space_spec(&value) {
        parse_display_space(value).map(DisplayReplacementProperty::Stretch)
    } else if DisplaySpecHead::Image.is_head_of(&value) {
        Some(DisplayReplacementProperty::Media(
            DisplayMediaReplacementProperty::Image,
        ))
    } else if DisplaySpecHead::Video.is_head_of(&value) {
        Some(DisplayReplacementProperty::Media(
            DisplayMediaReplacementProperty::Video,
        ))
    } else if DisplaySpecHead::Xwidget.is_head_of(&value) {
        parse_display_xwidget_layout(&value).map(|layout| {
            DisplayReplacementProperty::Media(DisplayMediaReplacementProperty::Xwidget(
                DisplayMediaReplacement::xwidget(DisplayXwidgetItem {
                    xwidget_id: layout.xwidget_id.min(i32::MAX as u32) as i32,
                    width: layout.width,
                    height: layout.height,
                }),
            ))
        })
    } else if DisplaySpecHead::Webkit.is_head_of(&value) {
        Some(DisplayReplacementProperty::Media(
            DisplayMediaReplacementProperty::Webkit,
        ))
    } else {
        None
    };

    let modifiers = if replacement.is_some() {
        DisplayTextPropertyModifiers::default()
    } else {
        DisplayTextPropertyModifiers {
            raise: parse_display_raise_factor(value),
            height: parse_display_height_factor(value),
        }
    };

    DisplayPropertyClassification {
        replacement,
        modifiers,
    }
}

fn parse_display_space(value: Value) -> Option<DisplayStretch> {
    let items = list_to_vec(&value)?;
    let mut width = None;
    let mut height = None;
    let mut ascent = None;
    let mut i = 1usize;
    while i + 1 < items.len() {
        let key = items[i];
        let val = items[i + 1];
        match DisplaySpaceKey::from_lisp_value(key) {
            Some(DisplaySpaceKey::Width | DisplaySpaceKey::RelativeWidth) => {
                width = parse_display_length(val).map(DisplayStretchWidth::Length);
            }
            Some(DisplaySpaceKey::AlignTo) => {
                width = parse_display_length_expr(val).map(DisplayStretchWidth::AlignTo);
            }
            Some(DisplaySpaceKey::Height | DisplaySpaceKey::RelativeHeight) => {
                height = parse_display_length(val);
            }
            Some(DisplaySpaceKey::Ascent) => {
                ascent = parse_display_length(val);
            }
            None => {}
        }
        i += 2;
    }

    Some(DisplayStretch {
        width: width.unwrap_or(DisplayStretchWidth::Length(DisplayLength::Em(1.0))),
        height,
        ascent,
    })
}

fn parse_display_length(value: Value) -> Option<DisplayLength> {
    if let Some(number) = lisp_number(value) {
        return Some(DisplayLength::Em(number));
    }
    parse_display_length_expr(value).map(DisplayLength::Expr)
}

pub(crate) fn parse_display_length_expr(value: Value) -> Option<DisplayLengthExpr> {
    if let Some(number) = lisp_number(value) {
        return Some(DisplayLengthExpr::Em(number));
    }

    if value.is_symbol() {
        let name = value.as_symbol_name()?;
        return Some(
            display_length_symbol(name)
                .map(DisplayLengthExpr::Symbol)
                .unwrap_or_else(|| DisplayLengthExpr::Variable(name.into())),
        );
    }

    if !value.is_cons() {
        return None;
    }

    let items = list_to_vec(&value)?;
    let head = items.first()?;
    if head.is_symbol_named("+") {
        return items
            .iter()
            .skip(1)
            .copied()
            .map(parse_display_length_expr)
            .collect::<Option<Vec<_>>>()
            .map(DisplayLengthExpr::Add);
    }
    if head.is_symbol_named("-") {
        return items
            .iter()
            .skip(1)
            .copied()
            .map(parse_display_length_expr)
            .collect::<Option<Vec<_>>>()
            .map(DisplayLengthExpr::Sub);
    }
    if items.len() == 1
        && let Some(number) = lisp_number(items[0])
    {
        return Some(DisplayLengthExpr::Pixels(number));
    }

    None
}

fn display_length_symbol(name: &str) -> Option<DisplayLengthSymbol> {
    match name {
        "height" => Some(DisplayLengthSymbol::Height),
        "width" => Some(DisplayLengthSymbol::Width),
        "text" => Some(DisplayLengthSymbol::Text),
        "left" => Some(DisplayLengthSymbol::Left),
        "right" => Some(DisplayLengthSymbol::Right),
        "center" => Some(DisplayLengthSymbol::Center),
        "left-fringe" => Some(DisplayLengthSymbol::LeftFringe),
        "right-fringe" => Some(DisplayLengthSymbol::RightFringe),
        "left-margin" => Some(DisplayLengthSymbol::LeftMargin),
        "right-margin" => Some(DisplayLengthSymbol::RightMargin),
        "scroll-bar" => Some(DisplayLengthSymbol::ScrollBar),
        _ => None,
    }
}

fn parse_display_raise_factor(value: Value) -> Option<f32> {
    if value.is_cons() {
        let car = value.cons_car();
        let cdr = value.cons_cdr();
        if car.is_symbol_named("raise") {
            if cdr.is_cons() {
                return cdr.cons_car().as_number_f64().map(|factor| factor as f32);
            }
            return cdr.as_number_f64().map(|factor| factor as f32);
        }
    }

    plist_number(value, ":raise")
}

fn parse_display_height_factor(value: Value) -> Option<f32> {
    if value.is_cons() {
        let car = value.cons_car();
        let cdr = value.cons_cdr();
        if car.is_symbol_named("height") {
            if cdr.is_cons() {
                return cdr.cons_car().as_number_f64().map(|factor| factor as f32);
            }
            return cdr.as_number_f64().map(|factor| factor as f32);
        }
    }

    plist_number(value, ":height")
}

fn plist_number(value: Value, key_name: &str) -> Option<f32> {
    let items = list_to_vec(&value)?;
    let mut i = 0;
    while i + 1 < items.len() {
        if items[i].is_symbol_named(key_name) {
            return items[i + 1].as_number_f64().map(|factor| factor as f32);
        }
        i += 1;
    }
    None
}

fn lisp_number(value: Value) -> Option<f32> {
    value
        .as_float()
        .or_else(|| value.as_fixnum().map(|number| number as f64))
        .filter(|number| number.is_finite())
        .map(|number| number as f32)
}

#[cfg(test)]
#[path = "display_property_test.rs"]
mod tests;
