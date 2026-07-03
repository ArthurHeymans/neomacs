;;; font-selection-noto-bold.el --- Font selection oracle fixture -*- lexical-binding: t -*-

(defconst neomacs-font-selection-cases
  '((:id noto-sans-normal-normal-h150-s12
     :family "Noto Sans"
     :weight normal
     :slant normal
     :height 150
     :size 12
     :text "neomacs")
    (:id noto-sans-bold-normal-h150-s12
     :family "Noto Sans"
     :weight bold
     :slant normal
     :height 150
     :size 12
     :text "neomacs")
    (:id noto-sans-normal-italic-h150-s12
     :family "Noto Sans"
     :weight normal
     :slant italic
     :height 150
     :size 12
     :text "neomacs")
    (:id noto-sans-bold-italic-h150-s12
     :family "Noto Sans"
     :weight bold
     :slant italic
     :height 150
     :size 12
     :text "neomacs")
    (:id noto-sans-light-normal-h150-s12
     :family "Noto Sans"
     :weight light
     :slant normal
     :height 150
     :size 12
     :text "neomacs")
    (:id noto-sans-semibold-normal-h150-s12
     :family "Noto Sans"
     :weight semibold
     :slant normal
     :height 150
     :size 12
     :text "neomacs")
    (:id noto-sans-bold-oblique-h150-s12
     :family "Noto Sans"
     :weight bold
     :slant oblique
     :height 150
     :size 12
     :text "neomacs")
    (:id noto-sans-bold-normal-h100-s10
     :family "Noto Sans"
     :weight bold
     :slant normal
     :height 100
     :size 10
     :text "neomacs")
    (:id noto-sans-bold-normal-h220-s18
     :family "Noto Sans"
     :weight bold
     :slant normal
     :height 220
     :size 18
     :text "neomacs"))
  "Font-selection requests compared between GNU Emacs and NEO Emacs.")

(defun neomacs-font-selection-label (case)
  (format "family=%s weight=%s slant=%s height=%s size=%s"
          (plist-get case :family)
          (plist-get case :weight)
          (plist-get case :slant)
          (plist-get case :height)
          (plist-get case :size)))

(defun neomacs-font-selection-case-request (case)
  (let ((request (copy-sequence case)))
    (plist-put request :label (neomacs-font-selection-label case))
    request))

(defun neomacs-font-selection-face-plist (case)
  (list :family (plist-get case :family)
        :weight (plist-get case :weight)
        :slant (plist-get case :slant)
        :height (plist-get case :height)))

(switch-to-buffer (get-buffer-create "*neomacs-font-selection-noto-bold*"))
(erase-buffer)

(insert "Font selection oracle matrix\n\n")
(dolist (case neomacs-font-selection-cases)
  (let* ((label (neomacs-font-selection-label case))
         (text (plist-get case :text))
         (start nil))
    (insert label "\n")
    (setq start (point))
    (insert text "\n")
    (put-text-property start (point) 'face
                       (neomacs-font-selection-face-plist case))
    (insert "\n")))
(goto-char (point-min))

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

(defun neomacs-font-selection-font-spec (case)
  (font-spec :family (plist-get case :family)
             :weight (plist-get case :weight)
             :slant (plist-get case :slant)
             :size (plist-get case :size)))

(defun neomacs-font-selection-case-result (case)
  (let* ((target (propertize
                  (plist-get case :text)
                  'face
                  (neomacs-font-selection-face-plist case)))
         (entity (find-font (neomacs-font-selection-font-spec case)))
         (object (font-at 0 nil target)))
    (list :case (plist-get case :id)
          :label (neomacs-font-selection-label case)
          :request (neomacs-font-selection-case-request case)
          :find-font (neomacs-font-selection-font-fields entity)
          :font-at (neomacs-font-selection-font-fields object))))

(defun neomacs-font-selection-result ()
  (list :cases
        (mapcar #'neomacs-font-selection-case-result
                neomacs-font-selection-cases)))

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
