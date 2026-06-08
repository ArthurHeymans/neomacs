//! Shelf-based atlas page allocator.
//!
//! A simple left-to-right, top-to-bottom allocator that packs glyph
//! rectangles onto a fixed-size atlas page. Each glyph gets padding
//! around all four sides. When a shelf fills up, a new shelf is started.
//! When the page fills up, the caller must create a new page.
//!
//! No behavior change — this is introduced alongside the existing code
//! and will be wired in during later steps.

use std::num::NonZeroU32;

use super::types::{AtlasAllocationRect, AtlasContentRect, PixelSize};

/// Result of a shelf allocation attempt.
pub struct Allocation {
    /// The full padded allocation rect (content + padding).
    pub allocation_rect: AtlasAllocationRect,
    /// The inner content rect (where glyph pixels go).
    pub content_rect: AtlasContentRect,
}

/// Shelf-based allocator for a single atlas page.
///
/// Allocates rectangles left-to-right on horizontal shelves. When the
/// current shelf cannot fit the requested width, a new shelf starts at
/// `cursor_y + shelf_height`. When the page cannot fit the requested
/// height, allocation fails and the caller should create a new page.
#[derive(Debug)]
pub struct ShelfAllocator {
    page_size: u32,
    padding: u32,
    cursor_x: u32,
    cursor_y: u32,
    shelf_height: u32,
}

impl ShelfAllocator {
    pub fn new(page_size: u32, padding: u32) -> Self {
        Self {
            page_size,
            padding,
            cursor_x: 0,
            cursor_y: 0,
            shelf_height: 0,
        }
    }

    /// Attempt to allocate space for a glyph of the given pixel size.
    ///
    /// Returns `None` if the glyph is too large for the page or the page
    /// is full.
    pub fn allocate(&mut self, glyph_size: PixelSize) -> Option<Allocation> {
        let max_content = self.page_size.saturating_sub(2 * self.padding);
        if glyph_size.width() > max_content || glyph_size.height() > max_content {
            return None;
        }

        let alloc_w = glyph_size.width() + 2 * self.padding;
        let alloc_h = glyph_size.height() + 2 * self.padding;

        let (x, y) = self.find_position(alloc_w, alloc_h)?;

        let alloc_rect = AtlasAllocationRect::new(
            x,
            y,
            NonZeroU32::new(alloc_w).unwrap(),
            NonZeroU32::new(alloc_h).unwrap(),
        );

        let content_x = x + self.padding;
        let content_y = y + self.padding;
        let content_rect = AtlasContentRect::new(
            content_x,
            content_y,
            NonZeroU32::new(glyph_size.width()).unwrap(),
            NonZeroU32::new(glyph_size.height()).unwrap(),
        );

        Some(Allocation {
            allocation_rect: alloc_rect,
            content_rect,
        })
    }

    fn find_position(&mut self, alloc_w: u32, alloc_h: u32) -> Option<(u32, u32)> {
        if self.cursor_x + alloc_w <= self.page_size {
            let x = self.cursor_x;
            let y = self.cursor_y;
            self.cursor_x += alloc_w;
            self.shelf_height = self.shelf_height.max(alloc_h);
            return Some((x, y));
        }

        let new_y = self.cursor_y + self.shelf_height;
        if new_y + alloc_h > self.page_size {
            return None;
        }

        // Place this glyph at the start of the new shelf and advance the cursor
        // past it, exactly like the same-shelf branch above. Leaving `cursor_x`
        // at 0 here would place the NEXT glyph on top of this one, overlapping
        // them in the atlas texture (they would render as each other).
        self.cursor_x = alloc_w;
        self.cursor_y = new_y;
        self.shelf_height = alloc_h;

        Some((0, new_y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_allocation_starts_at_origin() {
        let mut alloc = ShelfAllocator::new(256, 1);
        let result = alloc.allocate(PixelSize::new(10, 10).unwrap()).unwrap();
        assert_eq!(result.allocation_rect.x(), 0);
        assert_eq!(result.allocation_rect.y(), 0);
        assert_eq!(result.content_rect.x(), 1);
        assert_eq!(result.content_rect.y(), 1);
        assert_eq!(result.content_rect.width(), 10);
        assert_eq!(result.content_rect.height(), 10);
    }

    #[test]
    fn adjacent_allocations_dont_overlap() {
        let mut alloc = ShelfAllocator::new(256, 1);
        let a = alloc.allocate(PixelSize::new(10, 10).unwrap()).unwrap();
        let b = alloc.allocate(PixelSize::new(10, 10).unwrap()).unwrap();

        assert!(b.allocation_rect.x() >= a.allocation_rect.x() + a.allocation_rect.width());
    }

    #[test]
    fn first_two_glyphs_on_a_wrapped_shelf_do_not_overlap() {
        // Regression for the intermittent "wrong glyph" rendering bug: after a
        // shelf fills and allocation wraps to a new shelf, `cursor_x` must
        // advance past the glyph just placed at x=0. Otherwise the NEXT glyph is
        // placed at the same (x, y) and the two glyphs overlap in the atlas
        // texture, so one renders as the other.
        let page_size = 50u32;
        let padding = 1u32;
        let glyph_w = 10u32;
        let alloc_w = glyph_w + 2 * padding; // 12
        let mut alloc = ShelfAllocator::new(page_size, padding);

        // Fill the first shelf (x = 0, 12, 24, 36; the next would be 48+12 > 50).
        let fits = page_size / alloc_w;
        for _ in 0..fits {
            alloc.allocate(PixelSize::new(glyph_w, 10).unwrap()).unwrap();
        }

        // This allocation wraps to a new shelf at (0, new_y)...
        let wrapped = alloc.allocate(PixelSize::new(glyph_w, 10).unwrap()).unwrap();
        // ...and this one must land to its right on the same shelf, not on top.
        let next = alloc.allocate(PixelSize::new(glyph_w, 10).unwrap()).unwrap();

        assert_eq!(
            wrapped.allocation_rect.y(),
            next.allocation_rect.y(),
            "both glyphs belong to the freshly wrapped shelf"
        );
        assert!(
            next.allocation_rect.x()
                >= wrapped.allocation_rect.x() + wrapped.allocation_rect.width(),
            "next glyph (x={}) must start after the wrapped glyph (x={}, w={}); \
             overlapping allocations cause glyphs to render as each other",
            next.allocation_rect.x(),
            wrapped.allocation_rect.x(),
            wrapped.allocation_rect.width(),
        );
    }

    #[test]
    fn shelf_wraps_when_width_exceeded() {
        let page_size = 50u32;
        let padding = 1u32;
        let glyph_w = 10u32;
        let alloc_w = glyph_w + 2 * padding;

        let mut alloc = ShelfAllocator::new(page_size, padding);
        let fits_per_shelf = page_size / alloc_w;

        for i in 0..fits_per_shelf {
            let result = alloc.allocate(PixelSize::new(glyph_w, 10).unwrap());
            assert!(
                result.is_some(),
                "allocation {} should fit on first shelf",
                i
            );
            assert_eq!(result.unwrap().allocation_rect.y(), 0);
        }

        let result = alloc.allocate(PixelSize::new(glyph_w, 10).unwrap());
        assert!(result.is_some(), "should wrap to second shelf");
        assert!(
            result.unwrap().allocation_rect.y() > 0,
            "y must advance to new shelf"
        );
    }

    #[test]
    fn rejects_oversized_glyph() {
        let mut alloc = ShelfAllocator::new(64, 1);
        let result = alloc.allocate(PixelSize::new(64, 10).unwrap());
        assert!(
            result.is_none(),
            "glyph width 64 + 2 padding = 66 > 64 page"
        );
    }

    #[test]
    fn fills_multiple_shelves() {
        let mut alloc = ShelfAllocator::new(32, 0);
        let glyph = PixelSize::new(16, 4).unwrap();
        let mut shelves_used = std::collections::HashSet::new();

        for _ in 0..20 {
            if let Some(result) = alloc.allocate(glyph) {
                shelves_used.insert(result.allocation_rect.y());
            }
        }

        assert!(shelves_used.len() > 1, "should have used multiple shelves");
    }

    #[test]
    fn content_rect_is_inside_allocation_rect() {
        let mut alloc = ShelfAllocator::new(256, 2);
        let result = alloc.allocate(PixelSize::new(10, 10).unwrap()).unwrap();
        let a = result.allocation_rect;
        let c = result.content_rect;

        assert!(c.x() >= a.x());
        assert!(c.y() >= a.y());
        assert!(c.x() + c.width() <= a.x() + a.width());
        assert!(c.y() + c.height() <= a.y() + a.height());
    }

    #[test]
    fn padding_applied_exactly_once() {
        let padding = 3u32;
        let mut alloc = ShelfAllocator::new(256, padding);
        let result = alloc.allocate(PixelSize::new(10, 10).unwrap()).unwrap();

        assert_eq!(
            result.content_rect.x(),
            result.allocation_rect.x() + padding
        );
        assert_eq!(
            result.content_rect.y(),
            result.allocation_rect.y() + padding
        );
        assert_eq!(result.allocation_rect.width(), 10 + 2 * padding);
        assert_eq!(result.allocation_rect.height(), 10 + 2 * padding);
    }

    #[test]
    fn returns_none_when_page_full() {
        let mut alloc = ShelfAllocator::new(16, 0);
        loop {
            if alloc.allocate(PixelSize::new(8, 8).unwrap()).is_none() {
                break;
            }
        }
    }

    #[test]
    fn mixed_size_glyphs_fill_correctly() {
        let mut alloc = ShelfAllocator::new(64, 1);
        let tall = alloc.allocate(PixelSize::new(10, 30).unwrap());
        assert!(tall.is_some());
        assert_eq!(tall.unwrap().allocation_rect.y(), 0);

        let short = alloc.allocate(PixelSize::new(10, 5).unwrap());
        assert!(short.is_some());
        assert_eq!(short.unwrap().allocation_rect.y(), 0);

        let tall2 = alloc.allocate(PixelSize::new(10, 30).unwrap());
        assert!(tall2.is_some());
        assert_eq!(tall2.unwrap().allocation_rect.y(), 0);

        let big = alloc.allocate(PixelSize::new(60, 30).unwrap());
        assert!(big.is_some());
        assert!(
            big.unwrap().allocation_rect.y() > 0,
            "wide glyph should force a new shelf"
        );
    }

    #[test]
    fn zero_padding_works() {
        let mut alloc = ShelfAllocator::new(256, 0);
        let result = alloc.allocate(PixelSize::new(10, 10).unwrap()).unwrap();
        assert_eq!(result.content_rect.x(), 0);
        assert_eq!(result.content_rect.y(), 0);
        assert_eq!(result.content_rect.width(), 10);
        assert_eq!(result.content_rect.height(), 10);
    }
}
