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
