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

(defvar cursor-effects-preview--windows nil)
(defvar cursor-effects-preview--gallery-timer nil)
(defvar cursor-effects-preview--placeholder-buffers nil)

(defvar cursor-effects-preview-mode-map
  (let ((map (make-sparse-keymap)))
    (define-key map (kbd "q") #'cursor-effects-preview-quit)
    map))

(define-derived-mode cursor-effects-preview-mode special-mode "Cursor-Effects"
  "Major mode for the Neomacs cursor effects preview."
  (setq-local cursor-type '(bar . 8))
  (setq-local truncate-lines t)
  (setq-local mode-line-format nil)
  (setq-local header-line-format nil)
  (setq-local tab-line-format nil))

(defun cursor-effects-preview--buffer-name (index effect)
  (format "*Cursor Effect %02d %s*" index (plist-get effect :name)))

(defun cursor-effects-preview--grid-shape (count)
  (cons 7 8))

(defun cursor-effects-preview--insert-gallery-buffer (index effect shape)
  (let ((inhibit-read-only t))
    (erase-buffer)
    (insert (format "%02d  %s\n" index (plist-get effect :name)))
    (insert (format "%S\n\n" shape))
    (insert "||||||||||||||||||||||||\n")
    (goto-char (point-min))
    (forward-line 3)
    (forward-char (mod index 20))))

(defun cursor-effects-preview--make-buffer (index effect shape)
  (let ((buffer (get-buffer-create
                 (cursor-effects-preview--buffer-name index effect))))
    (with-current-buffer buffer
      (cursor-effects-preview-mode)
      (setq-local cursor-type shape)
      (setq-local cursor-in-non-selected-windows shape)
      (setq-local neomacs-cursor-effect (plist-get effect :forms))
      (cursor-effects-preview--insert-gallery-buffer index effect shape))
    buffer))

(defun cursor-effects-preview--make-placeholder-buffer (index)
  (let ((buffer (get-buffer-create
                 (format "*Cursor Effect Placeholder %02d*" index))))
    (with-current-buffer buffer
      (cursor-effects-preview-mode)
      (setq-local cursor-type nil)
      (let ((inhibit-read-only t))
        (erase-buffer)
        (insert "\n")))
    buffer))

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

(defun cursor-effects-preview--sort-windows (windows)
  (sort windows
        (lambda (a b)
          (let ((ea (window-pixel-edges a))
                (eb (window-pixel-edges b)))
            (or (< (cadr ea) (cadr eb))
                (and (= (cadr ea) (cadr eb))
                     (< (car ea) (car eb))))))))

(defun cursor-effects-preview--split-one (window side target-count)
  (condition-case err
      (split-window window nil side)
    (error
     (error "Cursor effects preview needs %d windows; split %S failed for window %S (%dx%d px) in frame %dx%d: %S"
            target-count
            side
            window
            (window-size window t t)
            (window-size window nil t)
            (frame-pixel-width)
            (frame-pixel-height)
            err))))

(defun cursor-effects-preview--largest-window (windows side)
  (let ((horizontal (eq side 'right)))
    (car (sort (copy-sequence windows)
               (lambda (a b)
                 (> (window-size a horizontal t)
                    (window-size b horizontal t)))))))

(defun cursor-effects-preview--split-evenly (window count side target-count)
  (let ((windows (list window))
        (tail window))
    (dotimes (_ (1- count))
      (push (cursor-effects-preview--split-one tail side target-count)
            windows)
      (balance-windows)
      (setq windows (cursor-effects-preview--sort-windows windows))
      (setq tail (cursor-effects-preview--largest-window windows side)))
    windows))

(defun cursor-effects-preview--split-grid (buffers)
  (delete-other-windows)
  (let* ((shape (cursor-effects-preview--grid-shape (length buffers)))
         (rows (car shape))
         (columns (cdr shape))
         (target-count (* rows columns))
         (placeholders nil)
         (all-buffers buffers)
         (column-windows nil)
         (windows nil))
    (dotimes (index (- target-count (length buffers)))
      (push (cursor-effects-preview--make-placeholder-buffer (1+ index))
            placeholders))
    (setq placeholders (nreverse placeholders))
    (setq cursor-effects-preview--placeholder-buffers placeholders)
    (setq all-buffers (append buffers placeholders))
    (setq column-windows
          (cursor-effects-preview--split-evenly
           (selected-window) columns 'right target-count))
    (setq windows
          (apply #'append
                 (mapcar (lambda (column-window)
                           (cursor-effects-preview--split-evenly
                            column-window rows 'below target-count))
                         column-windows)))
    (balance-windows)
    (setq windows
          (cursor-effects-preview--sort-windows
           (window-list nil 'no-minibuf)))
    (cl-loop for window in windows
             for buffer in all-buffers
             do (set-window-buffer window buffer)
             do (with-current-buffer buffer
                  (set-window-point window (point))
                  (set-window-parameter
                   window 'neomacs-cursor-effect neomacs-cursor-effect)))
    (balance-windows)
    (setq windows
          (cursor-effects-preview--sort-windows
           (window-list nil 'no-minibuf)))
    (cl-subseq windows 0 (length buffers))))

(defun cursor-effects-preview--pulse-selection ()
  (when cursor-effects-preview--windows
    (setq cursor-effects-preview--index
          (mod (1+ cursor-effects-preview--index)
               (length cursor-effects-preview--windows)))
    (select-window (nth cursor-effects-preview--index
                        cursor-effects-preview--windows))))

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
  (let ((buffers nil))
    (cl-loop for effect in cursor-effects-preview--effects
             for index from 1
             for shape = (nth (mod (1- index)
                                    (length cursor-effects-preview--cursor-types))
                              cursor-effects-preview--cursor-types)
             do (push (cursor-effects-preview--make-buffer index effect shape)
                      buffers))
    (setq cursor-effects-preview--windows
          (cursor-effects-preview--split-grid (nreverse buffers))))
  (setq cursor-effects-preview--index 0)
  (setq cursor-effects-preview--gallery-timer
        (run-at-time cursor-effects-preview-effect-seconds
                     cursor-effects-preview-effect-seconds
                     #'cursor-effects-preview--pulse-selection)))

(cursor-effects-preview-start)

;;; cursor-effects-preview-test.el ends here
