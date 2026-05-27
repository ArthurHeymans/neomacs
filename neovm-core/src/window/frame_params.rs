//! GNU-shaped frame parameter keys.
//!
//! GNU Emacs keeps the meaningful GUI frame parameters in `frame_parms[]`
//! in `src/frame.c`.  Lisp can still attach arbitrary frame parameters, but
//! known parameters get routed through dedicated semantic handlers.  Keep the
//! same split here: typed known parameters for Rust internals, plus dynamic
//! symbol keys for Lisp-visible unknown parameters.

use crate::emacs_core::intern::{SymId, resolve_sym};
use crate::emacs_core::value::Value;
use strum::{EnumString, IntoStaticStr};

pub const GNU_FRAME_PARAM_COUNT: usize = 54;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum FrameParam {
    AutoRaise = 0,
    AutoLower,
    BackgroundColor,
    BorderColor,
    BorderWidth,
    CursorColor,
    CursorType,
    Font,
    ForegroundColor,
    IconName,
    IconType,
    ChildFrameBorderWidth,
    InternalBorderWidth,
    RightDividerWidth,
    BottomDividerWidth,
    MenuBarLines,
    MouseColor,
    Name,
    ScrollBarWidth,
    ScrollBarHeight,
    Title,
    Unsplittable,
    VerticalScrollBars,
    HorizontalScrollBars,
    Visibility,
    TabBarLines,
    ToolBarLines,
    ScrollBarForeground,
    ScrollBarBackground,
    ScreenGamma,
    LineSpacing,
    LeftFringe,
    RightFringe,
    WaitForWm,
    Fullscreen,
    FontBackend,
    Alpha,
    Sticky,
    ToolBarPosition,
    InhibitDoubleBuffering,
    Undecorated,
    ParentFrame,
    SkipTaskbar,
    NoFocusOnMap,
    NoAcceptFocus,
    ZGroup,
    OverrideRedirect,
    NoSpecialGlyphs,
    AlphaBackground,
    BordersRespectAlphaBackground,
    UseFrameSynchronization,
    Shaded,
    NsAppearance,
    NsTransparentTitlebar,
}

pub const GNU_FRAME_PARAMS: [FrameParam; GNU_FRAME_PARAM_COUNT] = [
    FrameParam::AutoRaise,
    FrameParam::AutoLower,
    FrameParam::BackgroundColor,
    FrameParam::BorderColor,
    FrameParam::BorderWidth,
    FrameParam::CursorColor,
    FrameParam::CursorType,
    FrameParam::Font,
    FrameParam::ForegroundColor,
    FrameParam::IconName,
    FrameParam::IconType,
    FrameParam::ChildFrameBorderWidth,
    FrameParam::InternalBorderWidth,
    FrameParam::RightDividerWidth,
    FrameParam::BottomDividerWidth,
    FrameParam::MenuBarLines,
    FrameParam::MouseColor,
    FrameParam::Name,
    FrameParam::ScrollBarWidth,
    FrameParam::ScrollBarHeight,
    FrameParam::Title,
    FrameParam::Unsplittable,
    FrameParam::VerticalScrollBars,
    FrameParam::HorizontalScrollBars,
    FrameParam::Visibility,
    FrameParam::TabBarLines,
    FrameParam::ToolBarLines,
    FrameParam::ScrollBarForeground,
    FrameParam::ScrollBarBackground,
    FrameParam::ScreenGamma,
    FrameParam::LineSpacing,
    FrameParam::LeftFringe,
    FrameParam::RightFringe,
    FrameParam::WaitForWm,
    FrameParam::Fullscreen,
    FrameParam::FontBackend,
    FrameParam::Alpha,
    FrameParam::Sticky,
    FrameParam::ToolBarPosition,
    FrameParam::InhibitDoubleBuffering,
    FrameParam::Undecorated,
    FrameParam::ParentFrame,
    FrameParam::SkipTaskbar,
    FrameParam::NoFocusOnMap,
    FrameParam::NoAcceptFocus,
    FrameParam::ZGroup,
    FrameParam::OverrideRedirect,
    FrameParam::NoSpecialGlyphs,
    FrameParam::AlphaBackground,
    FrameParam::BordersRespectAlphaBackground,
    FrameParam::UseFrameSynchronization,
    FrameParam::Shaded,
    FrameParam::NsAppearance,
    FrameParam::NsTransparentTitlebar,
];

impl FrameParam {
    pub fn gnu_index(self) -> usize {
        self as usize
    }

    pub fn from_gnu_index(index: usize) -> Option<Self> {
        GNU_FRAME_PARAMS.get(index).copied()
    }

    pub fn name(self) -> &'static str {
        self.into()
    }

    pub fn symbol(self) -> Value {
        Value::symbol(self.name())
    }

    pub fn from_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    pub fn from_symbol_id(symbol: SymId) -> Option<Self> {
        Self::from_name(resolve_sym(symbol))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameParamKey {
    Known(FrameParam),
    Unknown(SymId),
}

impl FrameParamKey {
    pub fn from_symbol_id(symbol: SymId) -> Self {
        FrameParam::from_symbol_id(symbol)
            .map(Self::Known)
            .unwrap_or(Self::Unknown(symbol))
    }

    pub fn from_symbol_value(value: Value) -> Option<Self> {
        value.as_symbol_id().map(Self::from_symbol_id)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Known(param) => param.name(),
            Self::Unknown(symbol) => resolve_sym(symbol),
        }
    }

    pub fn symbol(self) -> Value {
        match self {
            Self::Known(param) => param.symbol(),
            Self::Unknown(symbol) => Value::from_sym_id(symbol),
        }
    }
}

impl From<FrameParam> for FrameParamKey {
    fn from(value: FrameParam) -> Self {
        Self::Known(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnu_frame_params_preserve_gnu_order() {
        let names: Vec<&str> = GNU_FRAME_PARAMS
            .iter()
            .copied()
            .map(FrameParam::name)
            .collect();
        assert_eq!(
            names,
            vec![
                "auto-raise",
                "auto-lower",
                "background-color",
                "border-color",
                "border-width",
                "cursor-color",
                "cursor-type",
                "font",
                "foreground-color",
                "icon-name",
                "icon-type",
                "child-frame-border-width",
                "internal-border-width",
                "right-divider-width",
                "bottom-divider-width",
                "menu-bar-lines",
                "mouse-color",
                "name",
                "scroll-bar-width",
                "scroll-bar-height",
                "title",
                "unsplittable",
                "vertical-scroll-bars",
                "horizontal-scroll-bars",
                "visibility",
                "tab-bar-lines",
                "tool-bar-lines",
                "scroll-bar-foreground",
                "scroll-bar-background",
                "screen-gamma",
                "line-spacing",
                "left-fringe",
                "right-fringe",
                "wait-for-wm",
                "fullscreen",
                "font-backend",
                "alpha",
                "sticky",
                "tool-bar-position",
                "inhibit-double-buffering",
                "undecorated",
                "parent-frame",
                "skip-taskbar",
                "no-focus-on-map",
                "no-accept-focus",
                "z-group",
                "override-redirect",
                "no-special-glyphs",
                "alpha-background",
                "borders-respect-alpha-background",
                "use-frame-synchronization",
                "shaded",
                "ns-appearance",
                "ns-transparent-titlebar",
            ]
        );
        for (index, param) in GNU_FRAME_PARAMS.iter().copied().enumerate() {
            assert_eq!(param.gnu_index(), index);
            assert_eq!(FrameParam::from_gnu_index(index), Some(param));
            assert_eq!(FrameParam::from_name(param.name()), Some(param));
        }
        assert_eq!(FrameParam::from_gnu_index(GNU_FRAME_PARAM_COUNT), None);
    }
}
