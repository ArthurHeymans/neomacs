;;; font-selection-noto-bold.el --- Font selection oracle fixture -*- lexical-binding: t -*-

(switch-to-buffer (get-buffer-create "*neomacs-font-selection-noto-bold*"))
(erase-buffer)

(insert "Font selection oracle: Noto Sans bold\n\n")
(let ((start (point)))
  (insert "neomacs\n")
  (put-text-property start (point) 'face
                     '(:family "Noto Sans"
                       :weight bold
                       :slant normal
                       :height 150)))
(insert "\nThe line above is the diagnostic target.\n")
(goto-char (point-min))

(defconst neomacs-font-selection-request
  '(:family "Noto Sans"
    :weight bold
    :slant normal
    :size 12
    :height 150
    :text "neomacs"))

(defun neomacs-font-selection-info-list (font)
  (let ((info (and font (font-info font))))
    (and info (append info nil))))

(defun neomacs-font-selection-font-fields (font)
  (let ((info (neomacs-font-selection-info-list font)))
    (list :type (and font (type-of font))
          :family (and font (font-get font :family))
          :weight (and font (font-get font :weight))
          :slant (and font (font-get font :slant))
          :xlfd (and font (font-xlfd-name font nil t))
          :font-info info
          :font-info-file (and info (nth 12 info)))))

(defun neomacs-font-selection-result ()
  (let* ((target (propertize
                  (plist-get neomacs-font-selection-request :text)
                  'face
                  '(:family "Noto Sans"
                    :weight bold
                    :slant normal
                    :height 150)))
         (spec (font-spec :family "Noto Sans"
                          :weight 'bold
                          :slant 'normal
                          :size 12))
         (entity (find-font spec))
         (object (font-at 0 nil target)))
    (list :request neomacs-font-selection-request
          :find-font (neomacs-font-selection-font-fields entity)
          :font-at (neomacs-font-selection-font-fields object))))

(defun neomacs-font-selection-write-oracle-result ()
  (let ((path (getenv "NEOMACS_GUI_FONT_SELECTION_RESULT")))
    (when path
      (make-directory (file-name-directory path) t)
      (with-temp-file path
        (prin1 (neomacs-font-selection-result) (current-buffer))
        (insert "\n")))))

(defun neomacs-font-selection-json-escape (value)
  (let ((start 0)
        (out ""))
    (while (string-match "[\\\"\n\r\t]" value start)
      (setq out (concat out (substring value start (match-beginning 0))
                        (pcase (match-string 0 value)
                          ("\"" "\\\"")
                          ("\\" "\\\\")
                          ("\n" "\\n")
                          ("\r" "\\r")
                          ("\t" "\\t"))))
      (setq start (match-end 0)))
    (concat out (substring value start))))

(defun neomacs-font-selection-write-state ()
  (let ((path (getenv "NEOMACS_GUI_STATE_JSON")))
    (when path
      (let* ((visible-text (buffer-substring-no-properties
                            (window-start)
                            (window-end nil t)))
             (payload
              (format
               "{\"buffer_name\":\"%s\",\"point\":%d,\"window_start\":%d,\"window_end\":%d,\"visible_text\":\"%s\"}\n"
               (neomacs-font-selection-json-escape (buffer-name))
               (point)
               (window-start)
               (window-end nil t)
               (neomacs-font-selection-json-escape visible-text))))
        (make-directory (file-name-directory path) t)
        (with-temp-file path
          (insert payload))))))

(neomacs-font-selection-write-oracle-result)
(neomacs-font-selection-write-state)

(let ((snap-json (getenv "NEOMACS_GUI_FRAME_SNAPSHOT_JSON"))
      (snap-txt (getenv "NEOMACS_GUI_FRAME_SNAPSHOT_TXT")))
  (when (and snap-json (fboundp 'neomacs--write-frame-snapshot))
    (make-directory (file-name-directory snap-json) t)
    (neomacs--write-frame-snapshot snap-json t 'json))
  (when (and snap-txt (fboundp 'neomacs--write-frame-snapshot))
    (make-directory (file-name-directory snap-txt) t)
    (neomacs--write-frame-snapshot snap-txt t 'text-faces)))

(run-at-time 2 nil (lambda () (kill-emacs 0)))

;;; font-selection-noto-bold.el ends here
