;;; neomacs-surface.el --- Shader surfaces: GPU textures from Lisp -*- lexical-binding: t -*-

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

;; NeoMacs extension (experimental): compositor-rendered GPU textures as
;; inline display objects.  See doc/display-engine/SHADER_SURFACES.md.
;;
;;   (neomacs-surface-create :width W :height H
;;                           :shader WGSL &optional :uniforms ALIST :animate BOOL)
;;   (neomacs-surface-create :width W :height H :pixels RGBA-UNIBYTE-STRING)
;;     => surface id (integer); signals on WGSL compile errors.
;;
;; Declarative form (no create call, like image/video specs — the spec
;; content is memoized into a surface; WGSL errors are logged, not signaled):
;;
;;   (insert (propertize " " 'display
;;            '(surface :shader "fn mainImage(...) ..."
;;                      :uniforms ((speed . 2.0))
;;                      :width 320 :height 120)))
;;
;; The shader defines `fn mainImage(fragCoord: vec2<f32>) -> vec4<f32>' and
;; reads Shadertoy-style uniforms `u.iTime', `u.iResolution', plus one
;; generated accessor per :uniforms entry ((speed . 2.0) => `u_speed()').
;;
;; Display a surface with `neomacs-surface-insert', or manually via the
;; display property (surface :id ID :width W :height H).
;;
;; Gate uses on (featurep 'neomacs-surface) and
;; (neomacs-surface-available-p).

;;; Code:

(defun neomacs-surface-insert (id width height)
  "Insert surface ID at point as a WIDTH x HEIGHT display object."
  (insert (propertize " " 'display (list 'surface :id id :width width :height height)
                      'neomacs-surface-id id))
  id)

(defun neomacs-surface-create-and-insert (&rest args)
  "Create a surface from ARGS (see `neomacs-surface-create') and insert it.
Returns the surface id."
  (let ((id (apply #'neomacs-surface-create args)))
    (neomacs-surface-insert id (plist-get args :width) (plist-get args :height))))

(defconst neomacs-surface--demo-shader
  "fn mainImage(fragCoord: vec2<f32>) -> vec4<f32> {
    let uv = fragCoord / u.iResolution.xy;
    let speed = u_speed();
    let col = 0.5 + 0.5 * cos(u.iTime * speed + uv.xyx * 6.2832 + vec3<f32>(0.0, 2.0, 4.0));
    return vec4<f32>(col, 1.0);
}"
  "Shadertoy-style plasma used by `neomacs-surface-demo'.")

(defun neomacs-surface--demo-checkerboard (size cell)
  "Return RGBA bytes for a SIZE x SIZE checkerboard with CELL-pixel squares."
  (let ((data (make-string (* size size 4) 0)))
    (dotimes (y size)
      (dotimes (x size)
        (let* ((i (* 4 (+ x (* y size))))
               (on (zerop (mod (+ (/ x cell) (/ y cell)) 2)))
               (v (if on 230 40)))
          (aset data i v)
          (aset data (+ i 1) (if on 120 40))
          (aset data (+ i 2) (if on 40 230))
          (aset data (+ i 3) 255))))
    data))

(defun neomacs-surface-demo ()
  "Pop a buffer showing an animated shader surface and a pixel surface."
  (interactive)
  (unless (neomacs-surface-available-p)
    (error "Shader surfaces need the NeoMacs GUI"))
  (with-current-buffer (get-buffer-create "*surface-demo*")
    (erase-buffer)
    (insert "Animated WGSL plasma (compositor clock, zero Lisp per frame):\n\n  ")
    (neomacs-surface-create-and-insert
     :width 320 :height 120
     :shader neomacs-surface--demo-shader
     :uniforms '((speed . 1.5))
     :animate t)
    (insert "\n\nStatic RGBA pixels uploaded from Lisp:\n\n  ")
    (neomacs-surface-create-and-insert
     :width 96 :height 96
     :pixels (neomacs-surface--demo-checkerboard 96 12))
    (insert "\n\nDeclarative display spec (no create call, spec is the identity):\n\n  ")
    (insert (propertize " " 'display
                        (list 'surface
                              :shader neomacs-surface--demo-shader
                              :uniforms '((speed . 4.0))
                              :width 200 :height 80)))
    (insert "\n")
    (goto-char (point-min))
    (pop-to-buffer (current-buffer))))

(provide 'neomacs-surface)
;;; neomacs-surface.el ends here
