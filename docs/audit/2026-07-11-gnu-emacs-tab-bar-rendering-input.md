# GNU Emacs Tab-Bar Rendering and Input

**Date:** 2026-07-11
**Reference revision:** GNU Emacs `0ee48ac4df205e0d915946b5db00e73a0cd21ae0`
(`emacs-31.0.90`)
**Scope:** close/new icons, image sizing, glyph hit-testing, mouse-event
construction, and the lifetime of the Lisp properties used by tab-bar commands

## Executive summary

GNU Emacs does not reduce a graphical tab-bar click to a tab index.  Its
non-toolkit tab bar keeps two related snapshots:

1. a Lisp string whose characters retain the item and close-button properties;
2. a glyph matrix whose glyphs retain the character position from which they
   were produced.

At click time, native code maps the pixel coordinate to a glyph, maps the
glyph's `charpos` back into the **currently displayed** tab-bar string, and
extracts both the tab-bar item and whether the clicked character has
`close-tab`.  It then creates a fresh propertized caption containing
`(KEY BINDING CLOSE-P)` and appends `(CAPTION . 0)` as the `posn-string` member
of the ordinary mouse position.  Lisp `tab-bar-mouse-1` reads that semantic
tuple and invokes either the item's binding (the plus button) or
`tab-bar-close-tab` (the close button).

Image dimensions are not discovered after layout.  Encountering an image
`display` property calls `lookup_image` during redisplay; a cache miss invokes
the format loader synchronously, and only then does glyph production use the
image's width, height, margins, and ascent.  GNU therefore has no initial
`1x1` tab-bar layout followed by an asynchronous dimension notification.

The important abstractions are consequently:

- a tab bar is a rendered, source-addressable Lisp string, not a list of
  rectangles that only know tab numbers;
- a published glyph matrix and its source-property string are one coherent
  interaction snapshot;
- a GUI mouse event carries the resolved tab item identity, binding, and
  close-button result into Lisp through `posn-string`;
- layout consumes final image metrics.  An asynchronous implementation must
  explicitly invalidate and republish layout when those metrics become known.

## 1. Lisp constructs the icons and their meanings

`tab-bar--load-buttons` defines the plus icon using `symbols/plus_16.svg` and
the close icon using `symbols/cross_16.svg`.  Both request `:height (1 . em)`,
the configured button margin, and `:ascent center`.  Only the close icon string
is additionally propertized with `close-tab t`.
([`lisp/tab-bar.el`, `tab-bar--load-buttons`, lines 216-250](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/lisp/tab-bar.el#L216-L250))

`icon-string` selects the best supported representation.  For an image it
returns a one-character string (`" "`) whose `display` property is an image
spec made by `create-image`; height, width, scale, rotation, margin, and ascent
remain data in that spec.
([`lisp/emacs-lisp/icons.el`, `icon-string` and `icons--create`, lines 133-218](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/lisp/emacs-lisp/icons.el#L133-L218))

The plus button is not recognized by inspecting its picture.  It is a regular
tab-bar keymap item whose key is `add-tab` and whose binding is
`tab-bar-new-tab`.
([`lisp/tab-bar.el`, `tab-bar-format-add-tab`, lines 1204-1208](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/lisp/tab-bar.el#L1204-L1208))

Therefore the two actions have different sources of identity:

| Action | Identity used by native/Lisp code |
|---|---|
| New tab | Item key `add-tab` plus binding `tab-bar-new-tab` |
| Close tab | Tab item identity plus `close-tab` on the exact clicked character |

## 2. Keymap items become a property-bearing display string

During redisplay, `update_tab_bar` asks `tab_bar_items` to parse the active
`[tab-bar]` keymaps.  It republishes the item vector when either its values or
its text properties have changed; the comparison deliberately uses
`equal-including-properties`.
([`src/xdisp.c`, `update_tab_bar`, lines 14544-14632](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/xdisp.c#L14544-L14632))

`tab_bar_items` stores each parsed `menu-item` in a native vector with slots
for the key, caption, binding, enabled state, selection state, and help.  In
particular, `parse_tab_bar_item` retains the Lisp caption string and binding
rather than replacing them with a native tab number.
([`src/keyboard.c`, `tab_bar_items` and `parse_tab_bar_item`, lines 9177-9516](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/keyboard.c#L9177-L9516))

For a graphical, internally drawn tab bar,
`build_desired_tab_bar_string` copies every item caption, adds a `menu-item`
property whose value is the item's slot offset in the item vector, and
concatenates the captions.  Existing properties such as the close icon's
localized `close-tab` survive the copy and concatenation.
([`src/xdisp.c`, `build_desired_tab_bar_string`, lines 14753-14792](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/xdisp.c#L14753-L14792))

This combined string is the source used to build the desired glyph matrix.
`redisplay_tab_bar` seats the normal display iterator on the string, and
`display_tab_bar_line` repeatedly calls `get_next_display_element` and
`PRODUCE_GLYPHS`.  Tab-bar text, spaces, and image display properties therefore
use the ordinary display iterator and ordinary glyph production.
([`src/xdisp.c`, `redisplay_tab_bar`, lines 15034-15123](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/xdisp.c#L15034-L15123),
[`display_tab_bar_line`, lines 14808-14918](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/xdisp.c#L14808-L14918))

## 3. Image dimensions participate in the same layout pass

When the display iterator encounters an image `display` property,
`handle_display_prop` calls `lookup_image` before switching the iterator to
`GET_FROM_IMAGE`.
([`src/xdisp.c`, image branch of `handle_display_prop`, lines 6616-6636](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/xdisp.c#L6616-L6636))

On a cache miss, `lookup_image` creates and caches the image and immediately
calls the image type's `load_img` function.  A successful loader has supplied
the real dimensions before `lookup_image` returns.  On failure, GNU uses the
explicit numeric `:width`/`:height` when present, otherwise fixed fallback
dimensions.  Image-independent values such as `:ascent` and `:margin` are also
resolved in this call.
([`src/image.c`, `lookup_image`, lines 3517-3648](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/image.c#L3517-L3648))

`produce_image_glyph` then reads `img->width` and `img->height`, computes ascent,
descent, margins, and pixel width, and emits one `IMAGE_GLYPH`.  Crucially, the
glyph stores both `CHARPOS (it->position)` and `it->object`, so an image glyph
remains addressable back to its source character.
([`src/xdisp.c`, `produce_image_glyph`, lines 32344-32519](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/xdisp.c#L32344-L32519))

For `:ascent center`, `image_ascent` centers the image using the active face's
font base and descent.  The resulting ascent is used both for line metrics and
for drawing.
([`src/image.c`, `image_ascent`, lines 1883-1924](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/image.c#L1883-L1924))

The GNU arrangement is synchronous from the perspective of redisplay:

```text
display property
  -> lookup/load image and establish dimensions
  -> produce image glyph with final metrics and source charpos
  -> compute row metrics
  -> publish desired matrix
```

It does not need an `ImageDimensionsReady` input event.  If another renderer
loads assets asynchronously, the equivalent invariant is not “redraw when the
texture is ready”; it is “rerun layout and publish a new interaction snapshot
when layout-affecting dimensions are ready.”

## 4. The current matrix and current property string are swapped together

After `update_window` publishes a bar window, `update_bar_window` swaps the
desired and current Lisp strings.  `update_tab_bar` performs this for
`current_tab_bar_string` and `desired_tab_bar_string` alongside the tab-bar
window update.
([`src/dispnew.c`, `update_bar_window` and `update_tab_bar`, lines 3846-3874](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/dispnew.c#L3846-L3874))

That pairing is significant: hit-testing the current glyph matrix consults
the properties of `current_tab_bar_string`, not a newly evaluated keymap or the
next desired string.  Geometry and semantics describe the same displayed
frame.

The generic glyph representation makes this possible.  Every glyph records a
`charpos`, and when its source `object` is a Lisp string, that position is an
index into the string.
([`src/dispextern.h`, `struct glyph`, lines 452-483](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/dispextern.h#L452-L483))

## 5. Pixel hit-testing recovers item and close-button semantics

GUI backends such as PGTK first determine whether the native mouse coordinate
is in the tab-bar window.  If so, they call the backend-independent
`handle_tab_bar_click` and attach its returned value to the ordinary mouse
input event.
([`src/pgtkterm.c`, button handling, lines 6107-6144](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/pgtkterm.c#L6107-L6144))

The backend-independent path is:

1. `get_tab_bar_item` calls `x_y_to_hpos_vpos` to obtain the exact glyph under
   the coordinate.
2. `tab_bar_item_info` clamps that glyph's `charpos` into
   `current_tab_bar_string`.
3. It reads `menu-item` at that source character to find the item-vector
   offset.
4. It reads `close-tab` at that same source character to distinguish the close
   icon from the rest of the tab caption.

([`src/xdisp.c`, `tab_bar_item_info` and `get_tab_bar_item`, lines 15184-15259](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/xdisp.c#L15184-L15259))

`handle_tab_bar_click` then copies the selected item's caption and adds this
property across the copy:

```elisp
(menu-item (KEY BINDING CLOSE-P))
```

It returns `(tab-bar CAPTION . 0)`.  Thus the native-to-keyboard boundary
already carries resolved semantic information; the Lisp command does not have
to repeat pixel hit-testing or look up a tab by numeric index.
([`src/xdisp.c`, `handle_tab_bar_click`, lines 15294-15349](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/xdisp.c#L15294-L15349))

This also explains why an item-wide hit rectangle is insufficient for the
close button: selection/new-tab identity is item-wide, but `close-tab` is a
property of the particular source character rendered as the close image.

## 6. Keyboard event construction deliberately fills `posn-string`

`make_lispy_position` initially recognizes the special tab-bar window and
builds the ordinary mouse position with `tab-bar` as its area.
([`src/keyboard.c`, `make_lispy_position`, lines 5778-5842](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/keyboard.c#L5778-L5842))

When converting a `MOUSE_CLICK_EVENT`, keyboard code notices that the backend
argument begins with `tab-bar` and appends its `(CAPTION . 0)` tail to the
position.  It explicitly describes this as adding the propertized string with
button information as the position's object member.
([`src/keyboard.c`, mouse click conversion, lines 6542-6548](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/keyboard.c#L6542-L6548))

That appended fifth member is exactly `POSN_STRING` in native code and exactly
what Lisp `posn-string` returns when it is a cons.
([`src/keyboard.h`, position accessors, lines 429-445](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/keyboard.h#L429-L445),
[`lisp/subr.el`, `posn-string`, lines 2125-2133](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/lisp/subr.el#L2125-L2133))

The command loop also expands a mouse event whose position area is `tab-bar`
into the `tab-bar` prefix plus the mouse event, which is why the normal binding
resolves to `tab-bar-mouse-1`.
([`src/keyboard.c`, prefix expansion, lines 11475-11504](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/src/keyboard.c#L11475-L11504))

## 7. Lisp consumes the semantic tuple

On a GUI frame, `tab-bar--event-to-item` takes the caption from
`(car (posn-string posn))` and reads its `menu-item` property at character zero.
The result is the native-generated `(KEY BINDING CLOSE-P)` tuple.
([`lisp/tab-bar.el`, `tab-bar--event-to-item`, lines 352-367](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/lisp/tab-bar.el#L352-L367))

Mouse press and release divide the behavior:

- `tab-bar-mouse-down-1` selects an ordinary tab, but deliberately does not
  close a tab or invoke the add button;
- `tab-bar-mouse-1` invokes the binding for `add-tab` (and related action
  items), or closes the identified tab when `CLOSE-P` is non-nil.

([`lisp/tab-bar.el`, `tab-bar-mouse-down-1` and `tab-bar-mouse-1`, lines 388-419](https://github.com/emacs-mirror/emacs/blob/0ee48ac4df205e0d915946b5db00e73a0cd21ae0/lisp/tab-bar.el#L388-L419))

GNU therefore preserves Lisp policy at the final step, but native display/input
code supplies the semantic facts that only the rendered glyph snapshot can
answer.

## 8. Design implications for Neomacs

The closest GNU-compatible seam is not `SelectTab { index }`, nor is it merely
`{ item_index, character_offset }` sent to the evaluator to be interpreted
against whatever keymap happens to be current.  GNU resolves the click against
the **published display snapshot** and transports an event object equivalent
to:

```text
TabBarMouseTarget {
    key,
    binding,
    close: bool,
    caption,
}
```

The concrete representation can differ in Neomacs, but it must preserve these
invariants:

1. Every rendered tab-bar glyph or hit fragment maps back to its caption
   character/source slot.
2. The interaction map is published atomically with the frame geometry it
   describes.
3. Hit-testing distinguishes a close-image source slot from the rest of the
   same tab item.
4. The resulting mouse position contains a non-nil `posn-string` whose caption
   has a `menu-item` value shaped as `(KEY BINDING CLOSE-P)`.
5. New-tab and close-tab behavior remains in the existing GNU Lisp commands;
   the renderer does not directly mutate tab state.
6. A layout-affecting asset transition from placeholder metrics to final
   metrics invalidates layout and republishes both geometry and interaction
   metadata.  A texture-only redraw cannot repair stale glyph geometry.

An implementation may choose to resolve `(key, binding, close)` before crossing
the display-runtime boundary, as GNU does, or carry a snapshot-scoped source
identifier to the evaluator.  It should not carry only a current tab ordinal:
that value cannot represent `add-tab`, cannot distinguish the close character,
and loses the binding Lisp intended for the rendered item.
