;;; cjk-cursor-repro.el --- Repro fixture for CJK cursor alignment -*- lexical-binding: t -*-

;; This fixture prepares a buffer with mixed ASCII/CJK content and places point
;; in known positions so renderer cursor placement can be visually inspected.

(defvar cjk-cursor-repro-font-candidates
  '("Sarasa Mono SC"
    "Noto Sans Mono CJK SC"
    "Noto Sans CJK SC"
    "PingFang SC"
    "Hiragino Sans GB"
    "STHeiti"
    "WenQuanYi Zen Hei")
  "Font candidates to improve CJK repro consistency across platforms.")

(defun cjk-cursor-repro--first-available-font ()
  "Return first available font from `cjk-cursor-repro-font-candidates'."
  (catch 'found
    (dolist (name cjk-cursor-repro-font-candidates)
      (when (find-font (font-spec :name name))
        (throw 'found name)))
    nil))

(defun cjk-cursor-repro--setup-font ()
  "Apply a CJK-capable font when available."
  (let ((font (cjk-cursor-repro--first-available-font)))
    (when font
      (set-frame-font (format "%s-22" font) t t)
      (message "CJK repro font: %s" font))))

(defun cjk-cursor-repro--insert-content ()
  "Insert mixed-width text designed to expose cursor/glyph misalignment."
  (insert "CJK Cursor Repro\n")
  (insert "Use arrow keys to move point across mixed-width text.\n\n")
  (insert "RULER: 1234567890123456789012345678901234567890\n")
  (insert "ASCII: ........................................\n")
  (insert "CJK  : 你好世界你好世界你好世界你好世界\n")
  (insert "MIX1 : A汉B字C测D试E中F文G混H排I\n")
  (insert "MIX2 : 123汉字abcかなカナ한글XYZ\n")
  (insert "FULL : ＡＢＣ１２３，。、；：？！\n")
  (insert "MIX3 : []{}()<>|!@# 汉字 / kana かな / hangul 한글\n")
  (insert "\nTarget line for screenshot:\n")
  (insert "TARGET: A汉B字C测D试E中F文G混H排I\n"))

(defun cjk-cursor-repro--goto-target ()
  "Move point to a deterministic position on the TARGET line."
  (goto-char (point-min))
  (search-forward "TARGET: ")
  ;; Move onto the first CJK char on target line ("汉").
  (forward-char 1)
  (message "Point prepared on TARGET CJK char for screenshot"))

(defun cjk-cursor-repro-start ()
  "Create and display the CJK cursor repro buffer."
  (switch-to-buffer (get-buffer-create "*CJK Cursor Repro*"))
  (setq-local cursor-type 'box)
  (setq-local truncate-lines t)
  (erase-buffer)
  (cjk-cursor-repro--setup-font)
  (cjk-cursor-repro--insert-content)
  (goto-char (point-min))
  (cjk-cursor-repro--goto-target)
  (blink-cursor-mode -1)
  (message "CJK cursor repro ready"))

(cjk-cursor-repro-start)

;;; cjk-cursor-repro.el ends here
