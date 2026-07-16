;;; neomacs-shaders.el --- Shader showcase: gallery and frame effects -*- lexical-binding: t -*-

;; Copyright (C) 2026 Free Software Foundation, Inc.

;; Author: Neomacs Contributors
;; Keywords: multimedia, graphics

;; This file is part of GNU Emacs.

;; GNU Emacs is free software: you can redistribute it and/or modify
;; it under the terms of the GNU General Public License as published by
;; the Free Software Foundation, either version 3 of the License, or
;; (at your option) any later version.

;; GNU Emacs is distributed in the hope that it will be useful,
;; but WITHOUT ANY WARRANTY; without even the implied warranty of
;; MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
;; GNU General Public License for more details.

;; You should have received a copy of the GNU General Public License
;; along with GNU Emacs.  If not, see <https://www.gnu.org/licenses/>.

;;; Commentary:

;; A small collection built on shader surfaces
;; (doc/display-engine/SHADER_SURFACES.md):
;;
;;   M-x neomacs-shaders-gallery   inline surfaces: raymarched 3D, an
;;                                 iMouse toy, Shadertoy GLSL pasted verbatim
;;   M-x neomacs-shaders-crt       CRT frame shader over the whole editor
;;                                 (barrel distortion, scanlines, vignette;
;;                                 Shadertoy-dialect GLSL)
;;   M-x neomacs-shaders-glow      soft bloom frame shader (WGSL)
;;   M-x neomacs-shaders-matrix    digital-rain frame shader (GLSL)
;;   M-x neomacs-shaders-off       remove the frame shader

;;; Code:

(require 'neomacs-surface)

;;;; Gallery surfaces

(defconst neomacs-shaders--raymarch-wgsl
  "fn mainImage(fragCoord: vec2<f32>) -> vec4<f32> {
    let res = u.iResolution.xy;
    let p = (2.0 * fragCoord - res) / res.y;
    let t = u.iTime * 0.7;
    let ro = vec3<f32>(0.0, 0.0, -2.6);
    let rd = normalize(vec3<f32>(p, 1.6));
    var d = 0.0;
    var glow = 0.0;
    var it = 0;
    for (var i = 0; i < 72; i++) {
        it = i;
        let pos = ro + rd * d;
        let c = cos(t);
        let s = sin(t);
        let q = vec3<f32>(c * pos.x - s * pos.z, pos.y, s * pos.x + c * pos.z);
        let dist = length(vec2<f32>(length(q.xz) - 0.9, q.y)) - 0.32;
        if (dist < 0.001 || d > 8.0) { break; }
        glow += 0.02 / (0.1 + dist);
        d += dist;
    }
    if (d > 8.0) {
        return vec4<f32>(0.02 + glow * 0.02, 0.02 + glow * 0.03, 0.06 + glow * 0.05, 1.0);
    }
    let g = 1.0 - f32(it) / 72.0;
    return vec4<f32>(0.2 + 0.8 * g, 0.15 + 0.5 * g, 0.35 + 0.55 * g, 1.0);
}"
  "Raymarched spinning torus — 3D from a fragment shader, no mesh API.")

(defconst neomacs-shaders--ripple-wgsl
  "fn mainImage(fragCoord: vec2<f32>) -> vec4<f32> {
    let d = distance(fragCoord, u.iMouse.xy) / u.iResolution.y;
    let ring = sin(42.0 * d - u.iTime * 5.0);
    let glow = smoothstep(0.45, 0.0, d);
    let c = 0.5 + 0.5 * ring;
    return vec4<f32>(0.08 + 0.55 * c * glow,
                     0.15 + 0.55 * glow,
                     0.45 + 0.45 * c,
                     1.0);
}"
  "Rings radiating from the pointer (u.iMouse) — hover to play.")

(defconst neomacs-shaders--shadertoy-glsl
  "void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    vec3 col = 0.5 + 0.5 * cos(iTime + uv.xyx + vec3(0, 2, 4));
    fragColor = vec4(col, 1.0);
}"
  "Shadertoy's default new-shader template, pasted verbatim.")

;;;###autoload
(defun neomacs-shaders-gallery ()
  "Pop a buffer showing the shader showcase; killing it frees the surfaces."
  (interactive)
  (unless (neomacs-surface-available-p)
    (error "Shader surfaces need the NeoMacs GUI"))
  (with-current-buffer (get-buffer-create "*shader-gallery*")
    (erase-buffer)
    (insert "Raymarched 3D (WGSL fragment shader; no mesh API):\n\n  ")
    (neomacs-surface-create-and-insert
     :width 320 :height 180 :animate t
     :shader neomacs-shaders--raymarch-wgsl)
    (insert "\n\nPointer ripple (u.iMouse — move the mouse over it):\n\n  ")
    (neomacs-surface-create-and-insert
     :width 320 :height 140 :animate t
     :shader neomacs-shaders--ripple-wgsl)
    (insert "\n\nShadertoy GLSL, pasted verbatim (:glsl):\n\n  ")
    (neomacs-surface-create-and-insert
     :width 320 :height 140 :animate t
     :glsl neomacs-shaders--shadertoy-glsl)
    (insert "\n\nFrame effects: M-x neomacs-shaders-crt / -glow / -matrix / -off\n")
    (goto-char (point-min))
    (pop-to-buffer (current-buffer))))

;;;; Frame shaders

(defconst neomacs-shaders--crt-glsl
  "void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = vec2(fragCoord.x, iResolution.y - fragCoord.y) / iResolution.xy;
    vec2 c = uv - 0.5;
    float r2 = dot(c, c);
    uv = 0.5 + c * (1.0 + 0.10 * r2);
    vec3 col;
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        col = vec3(0.0);
    } else {
        col = texture(iChannel0, uv).rgb;
    }
    float scan = 0.88 + 0.12 * sin(fragCoord.y * 2.8);
    col *= scan;
    col *= vec3(1.03, 0.99, 0.93);
    col *= 1.0 - 0.4 * r2;
    fragColor = vec4(col, 1.0);
}"
  "CRT: barrel distortion, scanlines, warm tint, vignette (GLSL).")

(defconst neomacs-shaders--glow-wgsl
  "fn mainImage(fragCoord: vec2<f32>) -> vec4<f32> {
    let res = u.iResolution.xy;
    let uv = vec2<f32>(fragCoord.x, res.y - fragCoord.y) / res;
    var col = textureSample(iChannel0, iChannel0Sampler, uv).rgb;
    var glow = vec3<f32>(0.0);
    let px = 2.5 / res;
    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            let offs = vec2<f32>(f32(i), f32(j)) * px;
            let s = textureSample(iChannel0, iChannel0Sampler, uv + offs).rgb;
            glow += max(s - vec3<f32>(0.65), vec3<f32>(0.0));
        }
    }
    col += glow * 0.10;
    return vec4<f32>(col, 1.0);
}"
  "Soft bloom: bright pixels bleed into their neighborhood (WGSL).")

(defconst neomacs-shaders--matrix-glsl
  "float mhash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = vec2(fragCoord.x, iResolution.y - fragCoord.y) / iResolution.xy;
    vec3 col = texture(iChannel0, uv).rgb * 0.6;
    float colw = 14.0;
    float x = floor(fragCoord.x / colw);
    float speed = 90.0 * (0.4 + 0.6 * mhash(vec2(x, 1.0)));
    float y = fragCoord.y + iTime * speed + mhash(vec2(x, 7.0)) * 900.0;
    float cell = floor(y / colw);
    float ch = step(0.6, mhash(vec2(x, cell)));
    float fade = pow(fract(y / 320.0), 3.0);
    col += vec3(0.15, 1.0, 0.35) * ch * fade * 0.55;
    fragColor = vec4(col, 1.0);
}"
  "Digital rain falling over the editor content (GLSL).")

;;;###autoload
(defun neomacs-shaders-crt ()
  "Apply the CRT frame shader to the whole editor."
  (interactive)
  (neomacs-frame-shader neomacs-shaders--crt-glsl 'glsl)
  (message "CRT on — M-x neomacs-shaders-off to remove"))

;;;###autoload
(defun neomacs-shaders-glow ()
  "Apply the bloom frame shader to the whole editor."
  (interactive)
  (neomacs-frame-shader neomacs-shaders--glow-wgsl)
  (message "Glow on — M-x neomacs-shaders-off to remove"))

;;;###autoload
(defun neomacs-shaders-matrix ()
  "Apply the digital-rain frame shader to the whole editor."
  (interactive)
  (neomacs-frame-shader neomacs-shaders--matrix-glsl 'glsl)
  (message "Rain on — M-x neomacs-shaders-off to remove"))

;;;###autoload
(defun neomacs-shaders-off ()
  "Remove the frame shader."
  (interactive)
  (neomacs-frame-shader nil)
  (message "Frame shader removed"))

(provide 'neomacs-shaders)
;;; neomacs-shaders.el ends here
