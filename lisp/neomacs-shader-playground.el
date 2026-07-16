;;; neomacs-shader-playground.el --- Live WGSL editing with a GPU preview -*- lexical-binding: t -*-

;; Copyright (C) 2026 Free Software Foundation, Inc.

;; Author: Neomacs Contributors
;; Keywords: multimedia, graphics, tools

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

;; A Shadertoy-style playground on NeoMacs shader surfaces
;; (doc/display-engine/SHADER_SURFACES.md): edit WGSL on the left, see the
;; compositor-rendered result on the right.
;;
;;   M-x neomacs-shader-playground
;;
;;   C-c C-c   compile the buffer; on success the preview swaps to the new
;;             shader, on failure naga's diagnostics pop up and the preview
;;             keeps the last good shader
;;   C-c C-u   update one uniform live (no recompile)
;;   C-c C-l   toggle live recompile on idle after edits
;;   C-c C-k   close the preview and free its GPU surface
;;
;; Uniform initial values are declared in a `// uniforms:' comment line,
;; e.g. `// uniforms: (speed . 2.0) (tint . [1.0 0.5 0.2])'; each entry
;; generates a WGSL accessor (`u_speed()', `u_tint()').

;;; Code:

(require 'neomacs-surface)

(defgroup neomacs-shader-playground nil
  "Live WGSL editing with a GPU preview."
  :group 'multimedia
  :prefix "neomacs-shader-playground-")

(defcustom neomacs-shader-playground-size '(420 . 300)
  "Preview surface size in pixels as (WIDTH . HEIGHT)."
  :type '(cons natnum natnum))

(defcustom neomacs-shader-playground-idle-delay 0.4
  "Idle seconds after an edit before a live recompile."
  :type 'number)

(defconst neomacs-shader-playground--buffer "*shader-playground*")
(defconst neomacs-shader-playground--preview-buffer "*shader-preview*")
(defconst neomacs-shader-playground--error-buffer "*shader-playground-errors*")

(defvar neomacs-shader-playground--surface nil
  "Surface id currently shown in the preview, or nil.")
(defvar neomacs-shader-playground--idle-timer nil)

(defconst neomacs-shader-playground--template
  "// Shader playground — press C-c C-c to compile, C-c C-l for live mode.
// On a compile error the preview keeps the last good shader.
//
// Contract: define  fn mainImage(fragCoord: vec2<f32>) -> vec4<f32>
// Built-ins: u.iTime, u.iTimeDelta, u.iFrame,
//            u.iResolution (xy = pixels, z = scale), u.iMouse (reserved).
// fragCoord is y-up (Shadertoy convention); return linear RGBA.
// Each `uniforms' entry below becomes an accessor: (speed . 1.0) -> u_speed().
// uniforms: (speed . 1.0)

fn mainImage(fragCoord: vec2<f32>) -> vec4<f32> {
    let uv = (2.0 * fragCoord - u.iResolution.xy) / u.iResolution.y;
    let t = u.iTime * u_speed();
    var col = 0.5 + 0.5 * cos(t + uv.xyx * 3.0 + vec3<f32>(0.0, 2.0, 4.0));
    let d = abs(length(uv) - 0.6 - 0.15 * sin(t));
    col += vec3<f32>(0.02 / max(d, 0.02));
    return vec4<f32>(col, 1.0);
}
"
  "Starter shader inserted into an empty playground buffer.")

(defvar-keymap neomacs-shader-playground-mode-map
  :doc "Keymap for `neomacs-shader-playground-mode'."
  "C-c C-c" #'neomacs-shader-playground-compile
  "C-c C-u" #'neomacs-shader-playground-set-uniform
  "C-c C-l" #'neomacs-shader-playground-live-mode
  "C-c C-k" #'neomacs-shader-playground-close-preview)

(define-derived-mode neomacs-shader-playground-mode prog-mode "WGSL-Play"
  "Major mode for editing a WGSL shader with a live NeoMacs surface preview."
  (setq-local comment-start "// ")
  (setq-local comment-end ""))

;;;###autoload
(defun neomacs-shader-playground ()
  "Open the shader playground: WGSL buffer plus a live GPU preview."
  (interactive)
  (unless (neomacs-surface-available-p)
    (error "The shader playground needs the NeoMacs GUI"))
  (let ((buffer (get-buffer-create neomacs-shader-playground--buffer)))
    (with-current-buffer buffer
      (unless (derived-mode-p 'neomacs-shader-playground-mode)
        (neomacs-shader-playground-mode))
      (when (zerop (buffer-size))
        (insert neomacs-shader-playground--template)))
    (pop-to-buffer buffer '((display-buffer-reuse-window
                             display-buffer-same-window)))
    (neomacs-shader-playground-compile)))

(defun neomacs-shader-playground--uniforms ()
  "Parse the `// uniforms:' declaration line, if any.
Returns an alist suitable for `neomacs-surface-create' :uniforms."
  (save-excursion
    (goto-char (point-min))
    (when (re-search-forward "^// *uniforms: *\\(.+\\)$" nil t)
      (condition-case nil
          (car (read-from-string (concat "(" (match-string 1) ")")))
        (error
         (message "shader-playground: ignoring malformed uniforms line")
         nil)))))

(defun neomacs-shader-playground--show (id width height)
  "Show surface ID at WIDTH x HEIGHT in the preview window, freeing the old one."
  (let ((old neomacs-shader-playground--surface)
        (preview (get-buffer-create neomacs-shader-playground--preview-buffer)))
    (with-current-buffer preview
      (let ((inhibit-read-only t))
        (erase-buffer)
        (insert "\n ")
        (neomacs-surface-insert id width height)
        (insert "\n"))
      (setq buffer-read-only t)
      (add-hook 'kill-buffer-hook #'neomacs-shader-playground--release nil t))
    (setq neomacs-shader-playground--surface id)
    (when (and old (/= old id))
      (neomacs-surface-destroy old))
    (display-buffer preview
                    `((display-buffer-reuse-window
                       display-buffer-in-side-window)
                      (side . right)
                      (window-width . ,(+ 4 (ceiling (/ (float width)
                                                        (frame-char-width)))))))))

(defun neomacs-shader-playground--release ()
  "Free the preview's GPU surface."
  (when neomacs-shader-playground--surface
    (ignore-errors (neomacs-surface-destroy neomacs-shader-playground--surface))
    (setq neomacs-shader-playground--surface nil)))

(defun neomacs-shader-playground--show-errors (message)
  "Pop the error buffer with naga's MESSAGE."
  (with-current-buffer (get-buffer-create neomacs-shader-playground--error-buffer)
    (let ((inhibit-read-only t))
      (erase-buffer)
      (insert message "\n")
      (insert "\n(Line numbers refer to the composed module: the generated\n"
              " prelude precedes your code.  The annotated source lines above\n"
              " point at the offending WGSL.)\n")
      (goto-char (point-min)))
    (special-mode)
    (display-buffer (current-buffer)
                    '((display-buffer-reuse-window
                       display-buffer-at-bottom)
                      (window-height . 0.3)))))

(defun neomacs-shader-playground--hide-errors ()
  "Bury the error buffer if it is showing."
  (when-let* ((buffer (get-buffer neomacs-shader-playground--error-buffer))
              (window (get-buffer-window buffer)))
    (quit-window nil window)))

(defun neomacs-shader-playground-compile (&optional quiet)
  "Compile the playground buffer into the preview surface.
With QUIET non-nil (live mode), report errors in the echo area instead of
popping the error buffer."
  (interactive)
  (let* ((source (buffer-substring-no-properties (point-min) (point-max)))
         (uniforms (neomacs-shader-playground--uniforms))
         (width (car neomacs-shader-playground-size))
         (height (cdr neomacs-shader-playground-size)))
    (condition-case err
        (let ((id (apply #'neomacs-surface-create
                         :shader source :width width :height height :animate t
                         (and uniforms (list :uniforms uniforms)))))
          (neomacs-shader-playground--show id width height)
          (neomacs-shader-playground--hide-errors)
          (message "Shader compiled (surface %d)" id)
          t)
      (error
       (if quiet
           (message "shader error: %s"
                    (car (split-string (error-message-string err) "\n")))
         (neomacs-shader-playground--show-errors (error-message-string err)))
       nil))))

(defun neomacs-shader-playground-set-uniform (name value)
  "Set uniform NAME to VALUE on the current preview surface, live.
VALUE is a number or a vector of up to four numbers."
  (interactive
   (list (intern (read-string "Uniform name: "))
         (car (read-from-string (read-string "Value (number or [x y z]): ")))))
  (unless neomacs-shader-playground--surface
    (error "No live preview surface; compile first (C-c C-c)"))
  (neomacs-surface-set-uniform neomacs-shader-playground--surface name value)
  (message "%s = %s" name value))

(defun neomacs-shader-playground-close-preview ()
  "Close the preview window and free its GPU surface."
  (interactive)
  (when-let* ((buffer (get-buffer neomacs-shader-playground--preview-buffer)))
    (kill-buffer buffer))
  (neomacs-shader-playground--hide-errors))

(define-minor-mode neomacs-shader-playground-live-mode
  "Recompile the playground shader after every pause in editing."
  :lighter " Live"
  (if neomacs-shader-playground-live-mode
      (add-hook 'after-change-functions
                #'neomacs-shader-playground--schedule nil t)
    (remove-hook 'after-change-functions
                 #'neomacs-shader-playground--schedule t)
    (when neomacs-shader-playground--idle-timer
      (cancel-timer neomacs-shader-playground--idle-timer)
      (setq neomacs-shader-playground--idle-timer nil))))

(defun neomacs-shader-playground--schedule (&rest _)
  "Debounce a live recompile of the playground buffer."
  (when neomacs-shader-playground--idle-timer
    (cancel-timer neomacs-shader-playground--idle-timer))
  (setq neomacs-shader-playground--idle-timer
        (run-with-idle-timer
         neomacs-shader-playground-idle-delay nil
         (lambda ()
           (setq neomacs-shader-playground--idle-timer nil)
           (when-let* ((buffer (get-buffer neomacs-shader-playground--buffer)))
             (with-current-buffer buffer
               (neomacs-shader-playground-compile t)))))))

(provide 'neomacs-shader-playground)
;;; neomacs-shader-playground.el ends here
