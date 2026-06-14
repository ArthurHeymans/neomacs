# Overlay, text-property (propertize) & face divergences — Neomacs vs GNU Emacs

Oracle parity tests (`neovm-oracle-tests/src/divergence_overlay_props.rs`,
`divergence_faces.rs`, and the `divergence_face_*_matrix.rs` generated files)
probing overlay lifecycle, text properties, and faces/colors.

## Run them

```bash
cargo nextest run -p neovm-oracle-tests \
  -E 'test(/div_face_/)|test(/div_ov_/)|test(/div_prop_/)' --no-fail-fast
```

Authoritative result: **1699 tests, 1428 pass, 271 divergences.** (Target was 200.)

## Complex combo tests (+12 divergences, 82 tests)

Files `divergence_overlay_face_combo{,2,3,4}.rs` combine several features at
once (overlay + text-property precedence, before/after-string + editing,
invisible/intangible/field + navigation, modification-hooks + undo,
read-only enforcement across many modifying ops, combined overlay+text-property
char-property resolution, face merging, syntax-table override, point hooks).
82 combo tests, 12 divergences:

### Theme 6 — read-only text property NOT enforced (9 manifestations)
Neomacs blocks `delete-char`/`delete-region` and the `buffer-read-only`
variable, but does NOT block these in-place modifiers on a `read-only`
text-property region (GNU raises `text-read-only`, Neomacs mutates):
`insert` (combo/combo2), `upcase-region`, `downcase-region`,
`capitalize-region`, `replace-string`, `subst-char-in-region`,
`self-insert-command`, and the non-nil read-only value form.
=> `div_combo_ro_*` and `div_combo_read_only_*` tests.

### Theme 7 — text-property plist ordering differs
`propertize` / `put-text-property` store text properties in a different plist
order than GNU (e.g. NEO `(mouse-face highlight face bold)` vs GNU
`(face bold mouse-face highlight)`). Round-trip is `equal`, but the printed
plist order diverges. `div_combo_propertize_string_prin1_read_roundtrip`,
`div_combo_propertize_property_order_preserved`, `div_combo_put_text_property_appends_order`.

### Minor
`div_combo_field_property_line_beginning`: type-error message differs
(`integer-or-marker-p` vs `integerp`). `div_combo_compose_region_then_overlay_face`:
find-composition returns nil (Theme from UTF-8 index, composition).

Coverage (combo tests that PASS): combined overlay+text-property
get-char-property precedence, next/previous-char-property-change,
char-property-range-p, add-face-text-property merging, syntax-table override
of forward-sexp/forward-word, point-entered/point-left hooks, line-prefix/
wrap-prefix + fill, window-specific overlays, keymap text-property/overlay,
buffer-display-table, overlay survives undo, evaporate under delete, overlay
moves with insert, insert between adjacent overlays, substring/concat
property preservation, buffer-substring-with-properties, invisible
buffer-substring/kill-line/fill.

## Themes

### Theme 1 — Face IDs offset from GNU (~144 faces)
`face-id` of nearly every face differs in Neomacs — IDs are consistently a few
below GNU (e.g. `abbrev-table-name` NEO 65 / GNU 68; `border` NEO 44 / GNU 45).
The face-ID allocation base/order diverges. Matrix:
`face_id_matrix.rs` (per face).

### Theme 2 — face-all-attributes plist cell (~82 faces)
`face-all-attributes` diverges in plist construction (notably the `:inherit`
cell: Neomacs `(:inherit)` improper vs GNU `(:inherit . unspecified)`), plus
per-face attribute value differences. Matrix: `face_attributes_matrix.rs`.

### Theme 3 — Per-face foreground/background values (~37)
`face-attribute :foreground` (22) and `:background` (15) differ for colored
faces (region, mode-line, etc.) between the two tty default setups.
Matrices: `face_fg_matrix.rs`, `face_bg_matrix.rs`.

### Theme 4 — Overlay-lists point-relative categorization
`overlay-lists` splits overlays into before/after-point lists differently in
Neomacs — `(car (overlay-lists))` returns 0 vs GNU's correct count.
`divergence_overlay_props::div_ov_lists_and_count`.

### Smaller per-face attribute divergences
- `:weight` (2), `:underline` (3), `face-documentation` (1).
- Passing (coverage): `:height` (0), `:slant` (0), color-defined-p,
  color-values, color-rgb-to-hex, color-distance, defined-colors, defface,
  inheritance resolution, face-bold/italic/underline-p, face-unspecified,
  overlay make/move/delete/put/get, overlays-at/in priority ordering,
  before/after-string, evaporate, propertize, put/get/set/add/remove
  text-properties, property-change search, stickiness, insertion inheritance.

## Files
Hand-crafted: `divergence_overlay_props.rs`, `divergence_faces.rs`.
Generated matrices: `divergence_face_{attributes,documentation,fg,height,id,
bg,weight,slant,underline}_matrix.rs`.
Complex combos: `divergence_overlay_face_combo{,2,3,4}.rs`.
