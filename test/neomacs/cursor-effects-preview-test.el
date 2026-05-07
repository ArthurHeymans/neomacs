;;; cursor-effects-preview-test.el --- Preview Neomacs cursor effects -*- lexical-binding: t -*-

;; Run with:
;;   ./target/release/neomacs -Q -l test/neomacs/cursor-effects-preview-test.el

(require 'cl-lib)
(require 'term/neo-win nil t)

(defgroup cursor-effects-preview nil
  "Preview Neomacs cursor animation effects."
  :group 'neomacs)

(defcustom cursor-effects-preview-effect-seconds 2.2
  "Seconds to display each cursor effect before advancing."
  :type 'number
  :group 'cursor-effects-preview)

(defcustom cursor-effects-preview-move-seconds 0.085
  "Seconds between preview cursor moves."
  :type 'number
  :group 'cursor-effects-preview)

(defvar cursor-effects-preview--effect-timer nil)
(defvar cursor-effects-preview--move-timer nil)
(defvar cursor-effects-preview--index 0)
(defvar cursor-effects-preview--positions nil)
(defvar cursor-effects-preview--position-index 0)
(defvar cursor-effects-preview--effect-overlays nil)
(defvar cursor-effects-preview--paused nil)

(defconst cursor-effects-preview--all-setters
  '(neomacs-set-cursor-blink
    neomacs-set-cursor-animation
    neomacs-set-cursor-glow
    neomacs-set-cursor-pulse
    neomacs-set-cursor-color-cycle
    neomacs-set-cursor-shadow
    neomacs-set-cursor-wake
    neomacs-set-cursor-error-pulse
    neomacs-set-cursor-crosshair
    neomacs-set-cursor-magnetism
    neomacs-set-cursor-comet
    neomacs-set-cursor-spotlight
    neomacs-set-cursor-particles
    neomacs-set-cursor-trail-fade
    neomacs-set-cursor-size-transition
    neomacs-set-cursor-elastic-snap
    neomacs-set-cursor-ghost
    neomacs-set-cursor-ripple-wave
    neomacs-set-cursor-lighthouse
    neomacs-set-cursor-sonar-ping
    neomacs-set-cursor-orbit-particles
    neomacs-set-cursor-heartbeat
    neomacs-set-cursor-metronome
    neomacs-set-cursor-radar
    neomacs-set-cursor-ripple-ring
    neomacs-set-cursor-scope
    neomacs-set-cursor-shockwave
    neomacs-set-cursor-gravity-well
    neomacs-set-cursor-water-drop
    neomacs-set-cursor-pixel-dust
    neomacs-set-cursor-candle-flame
    neomacs-set-cursor-moth-flame
    neomacs-set-cursor-sparkler
    neomacs-set-cursor-plasma-ball
    neomacs-set-cursor-quill-pen
    neomacs-set-cursor-aurora-borealis
    neomacs-set-cursor-feather
    neomacs-set-cursor-stardust
    neomacs-set-cursor-compass-needle
    neomacs-set-cursor-galaxy
    neomacs-set-cursor-prism
    neomacs-set-cursor-moth
    neomacs-set-cursor-flame
    neomacs-set-cursor-crystal
    neomacs-set-cursor-lightning
    neomacs-set-cursor-snowflake
    neomacs-set-cursor-firework
    neomacs-set-cursor-tornado
    neomacs-set-cursor-portal
    neomacs-set-cursor-bubble
    neomacs-set-cursor-sparkle-burst
    neomacs-set-cursor-compass
    neomacs-set-cursor-dna-helix
    neomacs-set-cursor-pendulum)
  "Every cursor effect setter that the preview disables between effects.")

(defconst cursor-effects-preview--effects
  '((:name "Smooth animation"
     :forms ((neomacs-set-cursor-animation t 240)))
    (:name "Blink"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-blink t 0.45)))
    (:name "Glow"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-glow t "#66CCFF" 48)))
    (:name "Pulse"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-glow t "#66CCFF" 48)
             (neomacs-set-cursor-pulse t 180)))
    (:name "Color cycle"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-glow t "#66CCFF" 44)
             (neomacs-set-cursor-color-cycle t 90 90 60)))
    (:name "Shadow"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-shadow t 5 5 45)))
    (:name "Wake"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-wake t 180 145)))
    (:name "Error pulse"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-error-pulse t "#FF3333" 450)))
    (:name "Crosshair"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-crosshair t "#808080" 24)))
    (:name "Magnetism"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-magnetism t "#66B3FF" 4 360 60)))
    (:name "Comet"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-comet t 12 700 "#80B3FF" 80)))
    (:name "Spotlight"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-spotlight t 240 26 "#FFFFE6")))
    (:name "Particles"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-particles t "#FF9933" 12 950 110)))
    (:name "Trail fade"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-trail-fade t 12 520)))
    (:name "Size transition"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-size-transition t 260)))
    (:name "Elastic snap"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-elastic-snap t 22 320)))
    (:name "Ghost"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-ghost t "#8080FF" 750 50)))
    (:name "Ripple wave"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-ripple-wave t "#6699FF" 95 650 45)))
    (:name "Lighthouse"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-lighthouse t)))
    (:name "Sonar ping"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-sonar-ping t)))
    (:name "Orbit particles"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-orbit-particles t)))
    (:name "Heartbeat"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-heartbeat t)))
    (:name "Metronome"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-metronome t)))
    (:name "Radar"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-radar t)))
    (:name "Ripple ring"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-ripple-ring t)))
    (:name "Scope"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-scope t)))
    (:name "Shockwave"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-shockwave t)))
    (:name "Gravity well"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-gravity-well t)))
    (:name "Water drop"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-water-drop t)))
    (:name "Pixel dust"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-pixel-dust t)))
    (:name "Candle flame"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-candle-flame t)))
    (:name "Moth flame"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-moth-flame t)))
    (:name "Sparkler"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-sparkler t)))
    (:name "Plasma ball"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-plasma-ball t)))
    (:name "Quill pen"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-quill-pen t)))
    (:name "Aurora borealis"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-aurora-borealis t)))
    (:name "Feather"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-feather t)))
    (:name "Stardust"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-stardust t)))
    (:name "Compass needle"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-compass-needle t)))
    (:name "Galaxy"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-galaxy t)))
    (:name "Prism"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-prism t)))
    (:name "Moth"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-moth t)))
    (:name "Flame"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-flame t)))
    (:name "Crystal"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-crystal t)))
    (:name "Lightning"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-lightning t)))
    (:name "Snowflake"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-snowflake t)))
    (:name "Firework"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-firework t)))
    (:name "Tornado"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-tornado t)))
    (:name "Portal"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-portal t)))
    (:name "Bubble"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-bubble t)))
    (:name "Sparkle burst"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-sparkle-burst t)))
    (:name "Compass"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-compass t)))
    (:name "DNA helix"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-dna-helix t)))
    (:name "Pendulum"
     :forms ((neomacs-set-cursor-animation t 240)
             (neomacs-set-cursor-pendulum t))))
  "Cursor effects to preview.")

(defvar cursor-effects-preview-mode-map
  (let ((map (make-sparse-keymap)))
    (define-key map (kbd "q") #'cursor-effects-preview-quit)
    (define-key map (kbd "n") #'cursor-effects-preview-next)
    (define-key map (kbd "p") #'cursor-effects-preview-previous)
    (define-key map (kbd "SPC") #'cursor-effects-preview-toggle-pause)
    map))

(define-derived-mode cursor-effects-preview-mode special-mode "Cursor-Effects"
  "Major mode for the Neomacs cursor effects preview."
  (setq-local cursor-type 'box)
  (setq-local truncate-lines t))

(defun cursor-effects-preview--call (form)
  (let ((fn (car form))
        (args (cdr form)))
    (if (fboundp fn)
        (condition-case err
            (apply fn args)
          (error
           (message "Cursor effect call failed: %S -> %S" form err)))
      (message "Cursor effect setter is not available: %S" fn))))

(defun cursor-effects-preview--disable-all ()
  (dolist (fn cursor-effects-preview--all-setters)
    (when (fboundp fn)
      (ignore-errors (funcall fn nil)))))

(defun cursor-effects-preview--set-overlays ()
  (dolist (overlay cursor-effects-preview--effect-overlays)
    (delete-overlay overlay))
  (setq cursor-effects-preview--effect-overlays nil)
  (save-excursion
    (goto-char (point-min))
    (dotimes (i (length cursor-effects-preview--effects))
      (when (search-forward (format "%2d. " (1+ i)) nil t)
        (let ((overlay (make-overlay (line-beginning-position) (line-end-position))))
          (overlay-put overlay 'face
                       (if (= i cursor-effects-preview--index)
                           'highlight
                         'default))
          (push overlay cursor-effects-preview--effect-overlays))))
    (setq cursor-effects-preview--effect-overlays
          (nreverse cursor-effects-preview--effect-overlays))))

(defun cursor-effects-preview--apply-current ()
  (let* ((effect (nth cursor-effects-preview--index cursor-effects-preview--effects))
         (name (plist-get effect :name)))
    (cursor-effects-preview--disable-all)
    (dolist (form (plist-get effect :forms))
      (cursor-effects-preview--call form))
    (cursor-effects-preview--set-overlays)
    (message "Cursor effect preview: %s (%d/%d)"
             name
             (1+ cursor-effects-preview--index)
             (length cursor-effects-preview--effects))))

(defun cursor-effects-preview-next ()
  "Advance to the next cursor effect."
  (interactive)
  (setq cursor-effects-preview--index
        (mod (1+ cursor-effects-preview--index)
             (length cursor-effects-preview--effects)))
  (cursor-effects-preview--apply-current))

(defun cursor-effects-preview-previous ()
  "Go back to the previous cursor effect."
  (interactive)
  (setq cursor-effects-preview--index
        (mod (1- cursor-effects-preview--index)
             (length cursor-effects-preview--effects)))
  (cursor-effects-preview--apply-current))

(defun cursor-effects-preview-toggle-pause ()
  "Toggle automatic effect cycling."
  (interactive)
  (setq cursor-effects-preview--paused (not cursor-effects-preview--paused))
  (message "Cursor effect preview %s"
           (if cursor-effects-preview--paused "paused" "running")))

(defun cursor-effects-preview--advance-timer ()
  (unless cursor-effects-preview--paused
    (cursor-effects-preview-next)))

(defun cursor-effects-preview--move-timer ()
  (when (and cursor-effects-preview--positions
             (eq major-mode 'cursor-effects-preview-mode))
    (let ((pos (nth cursor-effects-preview--position-index
                    cursor-effects-preview--positions)))
      (when (integer-or-marker-p pos)
        (goto-char pos)
        (setq cursor-effects-preview--position-index
              (mod (1+ cursor-effects-preview--position-index)
                   (length cursor-effects-preview--positions)))))))

(defun cursor-effects-preview--collect-positions (start end)
  (let (positions)
    (save-excursion
      (goto-char start)
      (while (< (point) end)
        (unless (or (eolp) (looking-at-p "[ \t]"))
          (push (point) positions))
        (forward-char 1)))
    (nreverse positions)))

(defun cursor-effects-preview--insert-buffer ()
  (let ((inhibit-read-only t)
        preview-start
        preview-end)
    (erase-buffer)
    (insert "Neomacs Cursor Effects Preview\n")
    (insert "\n")
    (insert "Keys: n next, p previous, SPC pause, q quit.\n")
    (insert "The point moves continuously so motion-triggered effects are visible.\n")
    (insert "\n")
    (insert "Preview path:\n")
    (setq preview-start (point))
    (dotimes (i 12)
      (insert
       (format "  row_%02d: abcdefghijklmnopqrstuvwxyz 0123456789 () [] {} <> +-*/ = %s\n"
               i
               (make-string (+ 8 (mod i 9)) ?x))))
    (setq preview-end (point))
    (insert "\n")
    (insert "Effects:\n")
    (cl-loop for effect in cursor-effects-preview--effects
             for i from 1
             do (insert (format "%2d. %s\n" i (plist-get effect :name))))
    (setq cursor-effects-preview--positions
          (cursor-effects-preview--collect-positions preview-start preview-end))
    (setq cursor-effects-preview--position-index 0)
    (goto-char (or (car cursor-effects-preview--positions) preview-start))))

(defun cursor-effects-preview-stop ()
  "Stop timers and disable all cursor effects used by the preview."
  (interactive)
  (when (timerp cursor-effects-preview--effect-timer)
    (cancel-timer cursor-effects-preview--effect-timer))
  (when (timerp cursor-effects-preview--move-timer)
    (cancel-timer cursor-effects-preview--move-timer))
  (setq cursor-effects-preview--effect-timer nil)
  (setq cursor-effects-preview--move-timer nil)
  (cursor-effects-preview--disable-all))

(defun cursor-effects-preview-quit ()
  "Quit the cursor effects preview."
  (interactive)
  (cursor-effects-preview-stop)
  (quit-window t))

(defun cursor-effects-preview-start ()
  "Start the Neomacs cursor effects preview."
  (interactive)
  (cursor-effects-preview-stop)
  (switch-to-buffer (get-buffer-create "*Cursor Effects Preview*"))
  (cursor-effects-preview-mode)
  (cursor-effects-preview--insert-buffer)
  (add-hook 'kill-buffer-hook #'cursor-effects-preview-stop nil t)
  (when (fboundp 'blink-cursor-mode)
    (blink-cursor-mode -1))
  (setq cursor-effects-preview--index 0)
  (setq cursor-effects-preview--paused nil)
  (cursor-effects-preview--apply-current)
  (setq cursor-effects-preview--move-timer
        (run-at-time 0 cursor-effects-preview-move-seconds
                     #'cursor-effects-preview--move-timer))
  (setq cursor-effects-preview--effect-timer
        (run-at-time cursor-effects-preview-effect-seconds
                     cursor-effects-preview-effect-seconds
                     #'cursor-effects-preview--advance-timer)))

(cursor-effects-preview-start)

;;; cursor-effects-preview-test.el ends here
