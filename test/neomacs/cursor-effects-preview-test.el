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

(defcustom cursor-effects-preview-move-seconds 1.0
  "Seconds between preview cursor moves."
  :type 'number
  :group 'cursor-effects-preview)

(defcustom cursor-effects-preview-min-frame-width 3200
  "Minimum frame pixel width requested for the cursor effect gallery."
  :type 'integer
  :group 'cursor-effects-preview)

(defcustom cursor-effects-preview-min-frame-height 2000
  "Minimum frame pixel height requested for the cursor effect gallery."
  :type 'integer
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

(defconst cursor-effects-preview--cursor-types
  '((bar . 8))
  "Cursor shapes to combine with preview effects.")

(defvar cursor-effects-preview--gallery-timer nil)
(defvar-local cursor-effects-preview--visual-lines nil)
(defvar-local neomacs-visual-cursors nil)

(defvar cursor-effects-preview-mode-map
  (let ((map (make-sparse-keymap)))
    (define-key map (kbd "q") #'cursor-effects-preview-quit)
    map))

(define-derived-mode cursor-effects-preview-mode special-mode "Cursor-Effects"
  "Major mode for the Neomacs cursor effects preview."
  (setq-local cursor-type nil)
  (setq-local truncate-lines t)
  (setq-local mode-line-format nil)
  (setq-local header-line-format nil)
  (setq-local tab-line-format nil))

(defun cursor-effects-preview--maximize-frame ()
  (setq frame-resize-pixelwise t)
  (setq window-resize-pixelwise t)
  (setq window-combination-resize nil)
  (setq window-min-height 1)
  (setq window-min-width 1)
  (set-frame-parameter nil 'fullscreen 'maximized)
  (when (fboundp 'set-frame-size)
    (let ((width (max (frame-pixel-width)
                      cursor-effects-preview-min-frame-width))
          (height (max (frame-pixel-height)
                       cursor-effects-preview-min-frame-height)))
      (set-frame-size (selected-frame) width height t))))

(defun cursor-effects-preview--line-offset (line tick)
  (let* ((width (plist-get line :width))
         (index (plist-get line :index))
         (period (+ 28 (mod (* index 7) 23)))
         (phase (+ (* tick (+ 1 (mod index 4)))
                   (* index 11)))
         (step (mod phase (* 2 period))))
    (min (1- width)
         (if (< step period)
             step
           (- (* 2 period) step)))))

(defun cursor-effects-preview--refresh-visual-cursors (&optional buffer)
  (let ((buffer (or buffer (current-buffer))))
    (when (buffer-live-p buffer)
      (with-current-buffer buffer
        (let ((tick (floor (* 10 (float-time)))))
          (setq-local
           neomacs-visual-cursors
           (mapcar
            (lambda (line)
              (let* ((offset (cursor-effects-preview--line-offset line tick))
                     (position (+ (plist-get line :start) offset)))
                (list :position position
                      :cursor-type (plist-get line :cursor-type)
                      :effect (plist-get line :effect)
                      :color (plist-get line :color))))
            cursor-effects-preview--visual-lines)))
        (when (fboundp 'force-window-update)
          (force-window-update buffer))
        (redisplay t)))))

(defun cursor-effects-preview--insert-lines ()
  (let ((lines nil)
        (track "Make Emacs Great Again! Make Emacs Great Again!"))
    (cl-loop for effect in cursor-effects-preview--effects
             for index from 1
             for shape = (car cursor-effects-preview--cursor-types)
             for color = (if (= (mod index 2) 0) "#66CCFF" "#FF9966")
             do
             (insert (format "%02d  %-28s " index (plist-get effect :name)))
             (let ((start (point)))
               (insert track)
               (insert "\n")
               (push (list :index index
                           :start start
                           :width (length track)
                           :cursor-type shape
                           :effect (plist-get effect :forms)
                           :color color)
                     lines)))
    (setq-local cursor-effects-preview--visual-lines (nreverse lines))))

(defun cursor-effects-preview--make-buffer ()
  (let ((buffer (get-buffer-create "*Cursor Effects Preview*")))
    (with-current-buffer buffer
      (cursor-effects-preview-mode)
      (let ((inhibit-read-only t))
        (erase-buffer)
        (cursor-effects-preview--insert-lines)
        (goto-char (point-min)))
      (cursor-effects-preview--refresh-visual-cursors buffer))
    buffer))

(defun cursor-effects-preview--show-buffer (buffer)
  (delete-other-windows)
  (switch-to-buffer buffer)
  (goto-char (point-min)))

(defun cursor-effects-preview-stop ()
  "Stop timers and disable all cursor effects used by the preview."
  (interactive)
  (when (timerp cursor-effects-preview--gallery-timer)
    (cancel-timer cursor-effects-preview--gallery-timer))
  (setq cursor-effects-preview--gallery-timer nil))

(defun cursor-effects-preview-quit ()
  "Quit the cursor effects preview."
  (interactive)
  (cursor-effects-preview-stop)
  (quit-window t))

(defun cursor-effects-preview-start ()
  "Start the Neomacs cursor effects preview."
  (interactive)
  (cursor-effects-preview--maximize-frame)
  (cursor-effects-preview-stop)
  (when (fboundp 'blink-cursor-mode)
    (blink-cursor-mode -1))
  (let ((buffer (cursor-effects-preview--make-buffer)))
    (cursor-effects-preview--show-buffer buffer)
    (setq cursor-effects-preview--gallery-timer
          (run-at-time 0 cursor-effects-preview-move-seconds
                       #'cursor-effects-preview--refresh-visual-cursors
                       buffer))))

(cursor-effects-preview-start)

;;; cursor-effects-preview-test.el ends here
