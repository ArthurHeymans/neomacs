# Animations

All animations run on the GPU render thread at display refresh rate,
independent of the Emacs redisplay. Everything is configurable from Elisp.

## Cursor

**8 particle/visual modes** (Neovide-inspired):

| Mode | Description |
|------|-------------|
| `none` | No animation, instant movement |
| `smooth` | Smooth interpolated movement (default) |
| `railgun` | Particles shoot backward from cursor |
| `torpedo` | Comet-like trail follows cursor |
| `pixiedust` | Sparkly particles scatter around cursor |
| `sonicboom` | Shockwave ring expands from cursor |
| `ripple` | Concentric rings emanate outward |
| `wireframe` | Animated outline glow |

**7 movement styles** controlling how the cursor interpolates between positions:

| Style | Description |
|-------|-------------|
| `exponential` | Smooth deceleration, no fixed duration (uses speed param) |
| `spring` | Critically-damped spring, Neovide-like feel (default) |
| `ease-out-quad` | Gentle deceleration curve |
| `ease-out-cubic` | Stronger deceleration curve |
| `ease-out-expo` | Sharp deceleration curve |
| `ease-in-out-cubic` | Smooth S-curve |
| `linear` | Constant speed |

The spring style also supports a **4-corner trail effect** where leading corners snap ahead and trailing corners stretch behind, controlled by a `trail-size` parameter (0.0-1.0).

## Buffer switch (crossfade/transition)

**10 buffer-switch effects** triggered when the visible buffer changes:

| Effect | Description |
|--------|-------------|
| `none` | Instant switch |
| `crossfade` | Alpha blend between old and new (default) |
| `slide-left/right/up/down` | Directional slide transitions |
| `scale-fade` | Scale and fade |
| `push` | New buffer pushes old buffer out |
| `blur` | Blur transition |
| `page-curl` | 3D page-turning effect |

## Scroll

**21 scroll animation effects** organized into categories:

| # | Effect | Category | Description |
|---|--------|----------|-------------|
| 0 | `slide` | 2D | Content slides in scroll direction (default) |
| 1 | `crossfade` | 2D | Alpha blend between old and new positions |
| 2 | `scale-zoom` | 2D | Destination zooms from 95% to 100% |
| 3 | `fade-edges` | 2D | Lines fade at viewport edges |
| 4 | `cascade` | 2D | Lines drop in with stagger delay |
| 5 | `parallax` | 2D | Layers scroll at different speeds |
| 6 | `tilt` | 3D | Subtle 3D perspective tilt |
| 7 | `page-curl` | 3D | Page turning effect |
| 8 | `card-flip` | 3D | Card flips around X-axis |
| 9 | `cylinder-roll` | 3D | Content wraps around cylinder |
| 10 | `wobbly` | Deformation | Jelly-like deformation |
| 11 | `wave` | Deformation | Sine-wave distortion |
| 12 | `per-line-spring` | Deformation | Each line springs independently |
| 13 | `liquid` | Deformation | Noise-based fluid distortion |
| 14 | `motion-blur` | Post-process | Vertical blur during scroll |
| 15 | `chromatic-aberration` | Post-process | RGB channel separation |
| 16 | `ghost-trails` | Post-process | Semi-transparent afterimages |
| 17 | `color-temperature` | Post-process | Warm/cool tint by direction |
| 18 | `crt-scanlines` | Post-process | Retro scanline overlay |
| 19 | `depth-of-field` | Post-process | Center sharp, edges dim |
| 20 | `typewriter-reveal` | Creative | Lines appear left-to-right |

**5 scroll easing functions:**

| # | Easing | Description |
|---|--------|-------------|
| 0 | `ease-out-quad` | Standard deceleration (default) |
| 1 | `ease-out-cubic` | Stronger deceleration |
| 2 | `spring` | Critically damped spring with overshoot |
| 3 | `linear` | Constant speed |
| 4 | `ease-in-out-cubic` | Smooth S-curve |

## Configuration

```elisp
;; All-in-one configuration:
;; (neomacs-set-animation-config
;;   CURSOR-ENABLED CURSOR-SPEED CURSOR-STYLE CURSOR-DURATION
;;   CROSSFADE-ENABLED CROSSFADE-DURATION
;;   SCROLL-ENABLED SCROLL-DURATION
;;   &optional SCROLL-EFFECT SCROLL-EASING TRAIL-SIZE)

;; Example: spring cursor, crossfade buffer switch, page-curl scroll with spring easing
(neomacs-set-animation-config t 15.0 'spring 150 t 200 t 150 7 2 0.7)

;; Example: fast linear cursor, no crossfade, wobbly scroll
(neomacs-set-animation-config t 20.0 'linear 100 nil 200 t 200 10 0 0.0)
```
