# Inline Image Baseline Design

GNU Emacs image specifications make vertical placement part of the image glyph's
layout semantics.  `:ascent` is either a percentage of image height or `center`,
where centering uses the active text face's ascent and descent.  The resulting
image ascent and descent participate in row metrics, and drawing is positioned
from the row baseline.

Neomacs currently lowers inline media to two disconnected representations: a
stretch glyph used for layout and a media rectangle used for drawing.  The
image's `:ascent` is discarded, the stretch ascent is hard-coded to image
height, and the media rectangle is placed at the row top.  This makes the tab
bar's 16-pixel close icon top-aligned in its 18-pixel row even though Lisp asks
for `:ascent center`.

The inline-media module will own this behavior behind its existing small
interface.  Image parsing produces a typed `DisplayImageAscent` policy.  Image
resolution converts that policy to a concrete pixel ascent using the image
height and text-face metrics.  `DisplayMediaReplacement` carries the concrete
ascent so the replacement stretch and drawable medium use one geometry model.
Non-image media retain their existing full-height ascent.

Row rendering will first collect baseline-relative media placements.  Once all
glyphs have contributed to final row ascent and height, it resolves each medium
to an absolute `y = row_y + row_ascent - media_ascent`.  FrameChrome, window
output, and WGPU continue receiving final rectangles and remain ignorant of
alignment policy.

Tests cover GNU-compatible parsing and the full Lisp-string-to-rendered-media
seam.  The regression uses a 16-pixel image in an 18-pixel row with
`:ascent center` and asserts that both the stretch and image rectangle share the
baseline-derived position.
