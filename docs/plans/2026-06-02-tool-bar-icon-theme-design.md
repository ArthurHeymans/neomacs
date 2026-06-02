# Tool Bar Icon Theme Design

Date: 2026-06-02
Status: design
Scope: GUI tool-bar, compact-bar tool icons, toolbar asset loading

## Goal

Provide modern tool-bar icon themes while preserving GNU Emacs tool-bar
semantics.

Users should be able to choose built-in icon themes such as:

- `gnu`
- `neomacs`
- `vscode-like`
- `jetbrains-like`
- `atom-like`
- `material`

Users should also be able to provide their own icon directories or per-command
icon overrides.

The design must not replace `tool-bar-map`, `tool-bar-setup`,
`tool-bar-make-keymap`, menu item parsing, enable predicates, selected state,
`:button`, `:wrap`, `:help`, `tool-bar-style`, `tool-bar-lines`, or click
dispatch. Icon themes are presentation only.

## GNU Model

GNU Emacs builds the tool bar from keymaps:

- `lisp/tool-bar.el` defines `tool-bar-map`.
- `[tool-bar]` uses a `:filter` that calls `tool-bar-make-keymap`.
- `tool-bar--image-expression` constructs image specs from an icon base name
  and the `tool-bar` face foreground/background.
- `tool-bar-add-item` and `tool-bar-add-item-from-menu` install `menu-item`
  entries with `:image`.
- `src/keyboard.c` parses those menu items into a flat frame tool-bar item
  vector with fixed slots: key, enabled, selected, caption, image, binding,
  type, help, RTL image, label, vertical-only, wrap.
- `src/xdisp.c` chooses the enabled/selected image variant, adds margins,
  relief, disabled conversion, and stores display/menu-item text properties in
  the frame tool-bar string.
- Tool-bar click handling maps a glyph back to the item vector slot and queues
  a `TOOL_BAR_EVENT` with the item key.

Neomacs should keep the same semantic shape. A theme must not change which
items exist, when they are enabled, which key is dispatched, or how buffer-local
tool bars override global ones.

## Current Neomacs Shape

Neomacs currently has the right broad split:

- `neomacs-bin/src/main.rs` ensures GNU `tool-bar-setup` runs when needed.
- `neomacs-layout-engine/src/gui_chrome.rs` collects GUI `ToolBarItem`s from
  Lisp keymaps.
- `neomacs-display-protocol/src/ui_types.rs` transports `ToolBarItem` and
  `ToolBarImageSource`.
- `neomacs-display-runtime/src/render_thread/ui_commands.rs` loads toolbar
  image textures.
- `neomacs-renderer-wgpu/src/renderer/ui_overlays.rs` renders the normal
  tool-bar and compact-bar icons.
- `neomacs-display-runtime/icons/toolbar/*.svg` contains one hardcoded icon
  set, and `backend/wgpu/toolbar_icons.rs` hardcodes name-to-asset lookup.

The weakness is that icon presentation is not a first-class typed layer. The
protocol carries only a GNU resolved image path, while the built-in SVG lookup
is frontend-private and flat. That makes multiple icon themes and custom icons
hard to express cleanly.

## Target Architecture

Split the toolbar into three layers:

1. Semantic item collection

   This remains GNU-compatible and keymap-driven. It produces item identity,
   state, labels, help, command binding, and the original GNU image source.

2. Icon identity and theme resolution

   This maps a semantic item to a theme icon asset without changing item
   semantics. It resolves built-in themes, user theme directories, and explicit
   overrides.

3. Rendering

   Rendering consumes resolved image assets plus item state. It draws toolbar
   geometry, hover/pressed/selected/disabled states, and icon textures.

This keeps GNU behavior in the Lisp/keymap path and keeps Neomacs visual
customization in the display path.

## Protocol Additions

Extend `ToolBarItem` instead of replacing `image`.

```rust
pub struct ToolBarItem {
    pub index: u32,
    pub key: String,
    pub command: Option<String>,
    pub icon_name: Option<String>,
    pub image: Option<ToolBarImageSource>,
    pub label: String,
    pub help: String,
    pub enabled: bool,
    pub selected: bool,
    pub item_type: ToolBarItemType,
    pub wrap: bool,
}
```

Meanings:

- `key`: GNU fake function key from the tool-bar keymap. This remains the click
  dispatch identity.
- `command`: the parsed command binding when it is a symbol. This is only for
  icon lookup and debugging; click dispatch still uses `key`.
- `icon_name`: icon base name recovered from the GNU `:image` expression or
  image file path, such as `open`, `save`, `undo`, `search`. This is the most
  stable bridge between GNU toolbar definitions and themed assets.
- `image`: the original GNU image source fallback. It remains in the protocol so
  `gnu` theme and missing themed icons can still use GNU image specs.

Do not encode theme names in `ToolBarItem`. Theme selection is global/frame
presentation state, not part of GNU item semantics.

## Typed Rust Model

Add typed domains rather than passing raw strings through all layers.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum ToolBarIconThemeKind {
    Gnu,
    Neomacs,
    VscodeLike,
    JetbrainsLike,
    AtomLike,
    Material,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ToolBarIconTheme {
    BuiltIn(ToolBarIconThemeKind),
    CustomDirectory(PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ToolBarIconKey {
    Known(KnownToolBarIcon),
    Custom(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum KnownToolBarIcon {
    New,
    Open,
    DiredOpen,
    Close,
    Save,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Search,
    SearchReplace,
    Help,
    Bookmark,
    Spell,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ToolBarIconState {
    EnabledDeselected,
    EnabledSelected,
    DisabledDeselected,
    DisabledSelected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ToolBarIconFormat {
    Svg,
    Png,
    Xpm,
    Pbm,
    Xbm,
}
```

`ToolBarIconKey` should be derived from, in order:

1. explicit user override key;
2. `icon_name` from GNU image expression/file base name;
3. known command mapping, for example `save-buffer -> save`;
4. GNU fake key name;
5. no themed icon.

The command mapping is a fallback only. GNU image names are preferred because
that is the established toolbar customization API.

## User-Facing Lisp Variables

Add Neomacs-specific customization variables. These should not alter GNU
variables or GNU keymap behavior.

```elisp
(defcustom neomacs-tool-bar-icon-theme 'neomacs
  "Icon theme used for graphical tool-bar and compact-bar icons."
  :type '(choice (const :tag "GNU Emacs" gnu)
                 (const :tag "Neomacs" neomacs)
                 (const :tag "VSCode-like" vscode-like)
                 (const :tag "JetBrains-like" jetbrains-like)
                 (const :tag "Atom-like" atom-like)
                 (const :tag "Material" material)
                 (directory :tag "Custom directory")))

(defcustom neomacs-tool-bar-icon-overrides nil
  "Per-icon or per-command toolbar icon overrides.
Each entry is (KEY . FILE), where KEY is an icon name, command symbol,
or toolbar key symbol."
  :type '(alist :key-type symbol :value-type file))

(defcustom neomacs-tool-bar-icon-theme-path nil
  "Additional directories searched for toolbar icon themes."
  :type '(repeat directory))
```

Changing any of these should invalidate toolbar icon texture caches and request
GUI chrome resync.

## Resolution Order

For each non-separator item:

1. If `neomacs-tool-bar-icon-overrides` has an entry for command symbol, use it.
2. If it has an entry for GNU key symbol, use it.
3. If it has an entry for icon base name, use it.
4. If selected theme is a custom directory, look for that icon there.
5. If selected theme is built-in, look for the embedded built-in asset.
6. If theme is not `gnu`, try the `neomacs` built-in theme as a neutral
   fallback.
7. Use the original GNU `ToolBarImageSource`.
8. If no image exists and `tool-bar-style` allows text, render the label.

This makes custom user intent strongest while preserving GNU fallback behavior.

## Asset Layout

Use one directory per theme:

```text
neomacs-display-runtime/icons/toolbar/
  gnu/
  neomacs/
  vscode-like/
  jetbrains-like/
  atom-like/
  material/
```

The current flat SVG files should move to one built-in theme, probably
`jetbrains-like` if their style stays outline-only, or `neomacs` if they become
the default house style.

Custom directories use the same file names:

```text
~/.config/neomacs/toolbar-icons/my-theme/open.svg
~/.config/neomacs/toolbar-icons/my-theme/save.svg
~/.config/neomacs/toolbar-icons/my-theme/search.svg
```

Supported search extensions, in order:

1. `.svg`
2. `.png`
3. `.xpm`
4. `.pbm`
5. `.xbm`

SVG should be preferred for built-in themes because it scales cleanly on HiDPI
displays. GNU bitmap/XPM assets remain valid fallback inputs.

Do not copy proprietary VSCode, JetBrains, or Atom artwork. The built-in themes
should be original icons that borrow broad visual language:

- `vscode-like`: compact, geometric, mostly single-stroke symbols.
- `jetbrains-like`: sharp balanced glyphs, dense 20x20 optical grid.
- `atom-like`: softer, rounder, slightly heavier outline.
- `material`: Material-symbol-like geometry from compatible open assets or
  original equivalents with clear license tracking.

## Rendering Rules

Toolbar faces still control bar foreground/background. For symbolic SVG icons,
use the toolbar foreground as `currentColor`.

States:

- enabled: full foreground alpha;
- disabled: lower alpha, matching GNU disabled conversion intent;
- hovered: subtle foreground-derived background;
- pressed: stronger foreground-derived background;
- selected/toggle/radio: selected indicator plus state-specific icon variant if
  the theme provides one.

The existing renderer already has normal and compact toolbar draw paths. Both
must use the same resolver and texture cache so compact-bar does not become a
separate icon system.

## Texture Cache

The current cache key is `ToolBarImageSource`. Introduce a resolved asset key:

```rust
pub enum ResolvedToolBarIconSource {
    BuiltIn {
        theme: ToolBarIconThemeKind,
        key: ToolBarIconKey,
        state: ToolBarIconState,
        format: ToolBarIconFormat,
    },
    CustomFile {
        path: PathBuf,
        state: ToolBarIconState,
    },
    GnuImage(ToolBarImageSource),
}
```

Cache by `ResolvedToolBarIconSource` plus pixel size. Theme changes, face color
changes for symbolic SVG, and icon size changes must invalidate the relevant
entries.

The renderer should load embedded SVG data through a data path, not by writing
temporary files. Existing `load_image_data` and SVG decoding support can be
reused for built-in SVG assets.

## Implementation Phases

1. Preserve more GNU slots in `ToolBarItem`.

   Add `command` and `icon_name`. Update GUI chrome parsing tests to verify GNU
   key, command, label, help, image, type, selected, enabled, and wrap remain
   independent.

2. Add typed icon theme model and resolver.

   Add unit tests for theme parsing, known icon parsing, resolution priority,
   custom directory lookup, and GNU fallback.

3. Replace hardcoded flat SVG lookup.

   Move current SVGs into a theme directory, replace `get_icon_svg(name)` with
   `resolve_builtin_toolbar_icon(theme, key, state)`, and keep compatibility
   aliases such as `search-replace -> search` in typed code.

4. Add Lisp customization variables and GUI sync.

   Variables should trigger chrome resync and icon cache invalidation without
   changing `tool-bar-map`.

5. Add the first complete built-in theme.

   Start with `neomacs` or `jetbrains-like` using the current SVGs after
   cleanup. Verify `neomacs -Q` screenshots at 1x and HiDPI scale.

6. Add theme families.

   Add `vscode-like`, `atom-like`, and `material` as separate assets. Keep all
   assets license-clean and package them in Linux, macOS, and Windows builds.

7. Add custom theme documentation.

   Document directory structure, override alist examples, supported formats,
   and fallback behavior.

## Tests

Minimum tests:

- `gui_chrome` extracts `icon_name` from GNU `tool-bar--image-expression`.
- `gui_chrome` keeps GNU image fallback even when a theme icon would exist.
- resolver chooses override before theme before GNU fallback.
- resolver rejects path traversal in theme-relative icon names.
- resolver invalidates cache on theme and size changes.
- compact-bar and normal toolbar share the same resolved icon source.
- click tests continue to dispatch GNU toolbar key, not icon name.

Visual checks:

- `neomacs -Q` default tool-bar screenshot.
- `neomacs -Q` with each built-in theme.
- disabled item state.
- selected toggle/radio state.
- compact-bar mode with menu and toolbar icons on one line.
- user custom SVG directory.

## Compatibility Decision

Default theme should be `neomacs` for Neomacs GUI builds, with `gnu` available
for users who want GNU assets. This changes presentation, not Emacs Lisp
semantics. If a strict visual-compatibility mode is introduced later, it can set
`neomacs-tool-bar-icon-theme` to `gnu` while leaving every other behavior
unchanged.

## First Refactor Target

The first code refactor should be small and structural:

- add `ToolBarItem::command`;
- add `ToolBarItem::icon_name`;
- add `ToolBarIconThemeKind`, `KnownToolBarIcon`, and resolver tests;
- keep rendering output unchanged by resolving to the existing image path until
  theme assets are wired.

That creates the right long-term shape without mixing feature work into GNU
toolbar semantics.
