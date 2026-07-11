//! Coordinate-safe frame-level chrome layout.
//!
//! A frame chrome band owns one absolute frame rectangle. Everything inside
//! the band is band-local and can be translated to frame coordinates only via
//! [`FrameRect::place`].

use crate::frame_glyphs::{DisplaySlotId, GlyphRowRole};
use crate::glyph_matrix::GlyphRow;
use crate::types::{Color, ImageId, Rect, VideoId, XwidgetId};
use crate::ui_types::{MenuBarItem, ToolBarItem};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FrameSize {
    width: f32,
    height: f32,
}

impl FrameSize {
    pub fn new(width: f32, height: f32) -> Result<Self, ChromeLayoutError> {
        if !valid_extent(width) || !valid_extent(height) {
            return Err(ChromeLayoutError::InvalidFrameSize);
        }
        Ok(Self { width, height })
    }

    pub fn width(self) -> f32 {
        self.width
    }

    pub fn height(self) -> f32 {
        self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FrameRect(Rect);

impl FrameRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self, ChromeLayoutError> {
        if !valid_origin(x) || !valid_origin(y) || !valid_extent(width) || !valid_extent(height) {
            return Err(ChromeLayoutError::InvalidRect);
        }
        Ok(Self(Rect::new(x, y, width, height)))
    }

    pub fn x(self) -> f32 {
        self.0.x
    }

    pub fn y(self) -> f32 {
        self.0.y
    }

    pub fn width(self) -> f32 {
        self.0.width
    }

    pub fn height(self) -> f32 {
        self.0.height
    }

    pub fn bottom(self) -> f32 {
        self.y() + self.height()
    }

    pub fn raw(self) -> Rect {
        self.0
    }

    pub fn place(self, local: BandRect) -> Result<Self, ChromeLayoutError> {
        let local = local.raw();
        if local.x + local.width > self.width() || local.y + local.height > self.height() {
            return Err(ChromeLayoutError::ContentExceedsBand);
        }
        Self::new(
            self.x() + local.x,
            self.y() + local.y,
            local.width,
            local.height,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct BandRect(Rect);

impl BandRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Result<Self, ChromeLayoutError> {
        if !valid_origin(x) || !valid_origin(y) || !valid_extent(width) || !valid_extent(height) {
            return Err(ChromeLayoutError::InvalidRect);
        }
        Ok(Self(Rect::new(x, y, width, height)))
    }

    pub fn raw(self) -> Rect {
        self.0
    }
}

fn valid_origin(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_extent(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FrameChromeKind {
    MenuBar,
    ToolBar,
    CompactBar,
    TabBar,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ChromeBandId(u32);

impl ChromeBandId {
    fn from_position(position: usize) -> Self {
        Self(position as u32)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ChromeAction {
    OpenMenu { index: u32, key: String },
    InvokeToolBarItem { index: u32 },
    SelectTab { index: u32 },
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChromeHitRegion {
    local_bounds: BandRect,
    action: ChromeAction,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaterializedChromeHitRegion {
    bounds: FrameRect,
    action: ChromeAction,
}

impl MaterializedChromeHitRegion {
    pub fn bounds(&self) -> FrameRect {
        self.bounds
    }

    pub fn action(&self) -> &ChromeAction {
        &self.action
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PositionedChromeItem<T> {
    local_bounds: BandRect,
    item: T,
    action: ChromeAction,
}

impl<T> PositionedChromeItem<T> {
    pub fn new(local_bounds: BandRect, item: T, action: ChromeAction) -> Self {
        Self {
            local_bounds,
            item,
            action,
        }
    }

    pub fn local_bounds(&self) -> BandRect {
        self.local_bounds
    }

    pub fn item(&self) -> &T {
        &self.item
    }

    pub fn action(&self) -> &ChromeAction {
        &self.action
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MenuBarContent {
    items: Vec<PositionedChromeItem<MenuBarItem>>,
    foreground: Color,
    background: Color,
}

impl MenuBarContent {
    pub fn new(
        items: Vec<PositionedChromeItem<MenuBarItem>>,
        foreground: Color,
        background: Color,
    ) -> Self {
        Self {
            items,
            foreground,
            background,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new(), Color::WHITE, Color::BLACK)
    }

    pub fn items(&self) -> &[PositionedChromeItem<MenuBarItem>] {
        &self.items
    }

    pub fn foreground(&self) -> Color {
        self.foreground
    }

    pub fn background(&self) -> Color {
        self.background
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolBarContent {
    items: Vec<PositionedChromeItem<ToolBarItem>>,
    foreground: Color,
    background: Color,
}

impl ToolBarContent {
    pub fn new(
        items: Vec<PositionedChromeItem<ToolBarItem>>,
        foreground: Color,
        background: Color,
    ) -> Self {
        Self {
            items,
            foreground,
            background,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new(), Color::WHITE, Color::BLACK)
    }

    pub fn items(&self) -> &[PositionedChromeItem<ToolBarItem>] {
        &self.items
    }

    pub fn foreground(&self) -> Color {
        self.foreground
    }

    pub fn background(&self) -> Color {
        self.background
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompactBarContent {
    menu_items: Vec<PositionedChromeItem<MenuBarItem>>,
    tool_items: Vec<PositionedChromeItem<ToolBarItem>>,
    menu_foreground: Color,
    menu_background: Color,
    tool_foreground: Color,
    tool_background: Color,
}

impl CompactBarContent {
    pub fn new(
        menu_items: Vec<PositionedChromeItem<MenuBarItem>>,
        tool_items: Vec<PositionedChromeItem<ToolBarItem>>,
        menu_foreground: Color,
        menu_background: Color,
        tool_foreground: Color,
        tool_background: Color,
    ) -> Self {
        Self {
            menu_items,
            tool_items,
            menu_foreground,
            menu_background,
            tool_foreground,
            tool_background,
        }
    }

    pub fn empty() -> Self {
        Self::new(
            Vec::new(),
            Vec::new(),
            Color::WHITE,
            Color::BLACK,
            Color::WHITE,
            Color::BLACK,
        )
    }

    pub fn menu_items(&self) -> &[PositionedChromeItem<MenuBarItem>] {
        &self.menu_items
    }

    pub fn tool_items(&self) -> &[PositionedChromeItem<ToolBarItem>] {
        &self.tool_items
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ChromeMedia {
    Image {
        local_bounds: BandRect,
        image_id: ImageId,
        slot_id: Option<DisplaySlotId>,
    },
    Video {
        local_bounds: BandRect,
        video_id: VideoId,
        slot_id: Option<DisplaySlotId>,
        loop_count: i32,
        autoplay: bool,
    },
    Xwidget {
        local_bounds: BandRect,
        xwidget_id: XwidgetId,
        slot_id: Option<DisplaySlotId>,
    },
}

impl ChromeMedia {
    pub fn local_bounds(&self) -> BandRect {
        match self {
            Self::Image { local_bounds, .. }
            | Self::Video { local_bounds, .. }
            | Self::Xwidget { local_bounds, .. } => *local_bounds,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChromeDisplayRow {
    row: GlyphRow,
    media: Vec<ChromeMedia>,
}

impl ChromeDisplayRow {
    pub fn new(mut row: GlyphRow, media: Vec<ChromeMedia>) -> Self {
        row.pixel_y = 0.0;
        Self { row, media }
    }

    pub fn empty_tab_bar() -> Self {
        Self::new(GlyphRow::new(GlyphRowRole::TabBar), Vec::new())
    }

    pub fn row(&self) -> &GlyphRow {
        &self.row
    }

    pub fn media(&self) -> &[ChromeMedia] {
        &self.media
    }
}

impl ChromeHitRegion {
    pub fn new(local_bounds: BandRect, action: ChromeAction) -> Self {
        Self {
            local_bounds,
            action,
        }
    }

    pub fn local_bounds(&self) -> BandRect {
        self.local_bounds
    }

    pub fn action(&self) -> &ChromeAction {
        &self.action
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FrameChromeContent {
    DisplayRow(ChromeDisplayRow),
    MenuBar(MenuBarContent),
    ToolBar(ToolBarContent),
    CompactBar(CompactBarContent),
}

impl FrameChromeContent {
    pub fn kind(&self) -> FrameChromeKind {
        match self {
            Self::DisplayRow(_) => FrameChromeKind::TabBar,
            Self::MenuBar(_) => FrameChromeKind::MenuBar,
            Self::ToolBar(_) => FrameChromeKind::ToolBar,
            Self::CompactBar(_) => FrameChromeKind::CompactBar,
        }
    }

    fn validate_in(&self, bounds: FrameRect) -> Result<(), ChromeLayoutError> {
        match self {
            Self::DisplayRow(content) => {
                for medium in content.media() {
                    bounds.place(medium.local_bounds())?;
                }
            }
            Self::MenuBar(content) => {
                for item in content.items() {
                    bounds.place(item.local_bounds())?;
                }
            }
            Self::ToolBar(content) => {
                for item in content.items() {
                    bounds.place(item.local_bounds())?;
                }
            }
            Self::CompactBar(content) => {
                for item in content.menu_items() {
                    bounds.place(item.local_bounds())?;
                }
                for item in content.tool_items() {
                    bounds.place(item.local_bounds())?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChromeBandRequest {
    kind: FrameChromeKind,
    height: f32,
    content: FrameChromeContent,
    hit_regions: Vec<ChromeHitRegion>,
}

impl ChromeBandRequest {
    pub fn empty(kind: FrameChromeKind, height: f32) -> Self {
        let content = match kind {
            FrameChromeKind::MenuBar => FrameChromeContent::MenuBar(MenuBarContent::empty()),
            FrameChromeKind::ToolBar => FrameChromeContent::ToolBar(ToolBarContent::empty()),
            FrameChromeKind::CompactBar => {
                FrameChromeContent::CompactBar(CompactBarContent::empty())
            }
            FrameChromeKind::TabBar => {
                FrameChromeContent::DisplayRow(ChromeDisplayRow::empty_tab_bar())
            }
        };
        Self::new(kind, height, content)
    }

    pub fn new(kind: FrameChromeKind, height: f32, content: FrameChromeContent) -> Self {
        Self {
            kind,
            height,
            content,
            hit_regions: Vec::new(),
        }
    }

    pub fn with_hit_regions(mut self, hit_regions: Vec<ChromeHitRegion>) -> Self {
        self.hit_regions = hit_regions;
        self
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FrameChromeBand {
    id: ChromeBandId,
    kind: FrameChromeKind,
    bounds: FrameRect,
    content: FrameChromeContent,
    hit_regions: Vec<ChromeHitRegion>,
}

impl FrameChromeBand {
    pub fn id(&self) -> ChromeBandId {
        self.id
    }

    pub fn kind(&self) -> FrameChromeKind {
        self.kind
    }

    pub fn bounds(&self) -> FrameRect {
        self.bounds
    }

    pub fn content(&self) -> &FrameChromeContent {
        &self.content
    }

    pub fn hit_regions(&self) -> &[ChromeHitRegion] {
        &self.hit_regions
    }

    pub fn materialized_hit_regions(
        &self,
    ) -> Result<Vec<MaterializedChromeHitRegion>, ChromeLayoutError> {
        self.hit_regions
            .iter()
            .map(|region| {
                Ok(MaterializedChromeHitRegion {
                    bounds: self.bounds.place(region.local_bounds())?,
                    action: region.action().clone(),
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FrameChrome {
    bands: Vec<FrameChromeBand>,
}

impl FrameChrome {
    pub fn layout(
        frame: FrameSize,
        requests: Vec<ChromeBandRequest>,
    ) -> Result<Self, ChromeLayoutError> {
        validate_requests(&requests)?;

        let order = if requests
            .iter()
            .any(|request| request.kind == FrameChromeKind::CompactBar && request.height > 0.0)
        {
            [
                Some(FrameChromeKind::CompactBar),
                Some(FrameChromeKind::TabBar),
                None,
            ]
        } else {
            [
                Some(FrameChromeKind::MenuBar),
                Some(FrameChromeKind::ToolBar),
                Some(FrameChromeKind::TabBar),
            ]
        };

        let mut bands = Vec::new();
        let mut y = 0.0;
        for kind in order.into_iter().flatten() {
            let Some(request) = requests.iter().find(|request| request.kind == kind) else {
                continue;
            };
            if request.height == 0.0 {
                continue;
            }
            if y + request.height > frame.height() {
                return Err(ChromeLayoutError::ContentExceedsFrame { kind });
            }
            let bounds = FrameRect::new(0.0, y, frame.width(), request.height)?;
            request.content.validate_in(bounds)?;
            for region in &request.hit_regions {
                bounds.place(region.local_bounds())?;
            }
            bands.push(FrameChromeBand {
                id: ChromeBandId::from_position(bands.len()),
                kind,
                bounds,
                content: request.content.clone(),
                hit_regions: request.hit_regions.clone(),
            });
            y += request.height;
        }
        Ok(Self { bands })
    }

    pub fn bands(&self) -> &[FrameChromeBand] {
        &self.bands
    }

    pub fn band(&self, kind: FrameChromeKind) -> Option<&FrameChromeBand> {
        self.bands.iter().find(|band| band.kind == kind)
    }
}

fn validate_requests(requests: &[ChromeBandRequest]) -> Result<(), ChromeLayoutError> {
    let mut seen = Vec::new();
    for request in requests {
        if !request.height.is_finite() || request.height < 0.0 {
            return Err(ChromeLayoutError::InvalidMeasuredHeight { kind: request.kind });
        }
        if seen.contains(&request.kind) {
            return Err(ChromeLayoutError::DuplicateBand { kind: request.kind });
        }
        if request.content.kind() != request.kind {
            return Err(ChromeLayoutError::ContentKindMismatch { kind: request.kind });
        }
        seen.push(request.kind);
    }

    let compact = requests
        .iter()
        .any(|request| request.kind == FrameChromeKind::CompactBar && request.height > 0.0);
    let separate = requests.iter().any(|request| {
        matches!(
            request.kind,
            FrameChromeKind::MenuBar | FrameChromeKind::ToolBar
        ) && request.height > 0.0
    });
    if compact && separate {
        return Err(ChromeLayoutError::ConflictingPresentation);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChromeLayoutError {
    InvalidFrameSize,
    InvalidRect,
    InvalidMeasuredHeight { kind: FrameChromeKind },
    DuplicateBand { kind: FrameChromeKind },
    ConflictingPresentation,
    ContentExceedsFrame { kind: FrameChromeKind },
    ContentExceedsBand,
    ContentKindMismatch { kind: FrameChromeKind },
}
