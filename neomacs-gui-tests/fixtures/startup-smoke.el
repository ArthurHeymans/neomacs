;;; startup-smoke.el --- GUI smoke fixture for neomacs-gui-tests -*- lexical-binding: t -*-

(switch-to-buffer (get-buffer-create "*neomacs-gui-smoke*"))
(erase-buffer)
(dotimes (i 80)
  (insert (format "NeoMacs GUI smoke line %02d\n" i)))
(goto-char (point-min))

(defun neomacs-gui-smoke-json-escape (value)
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

(defun neomacs-gui-smoke-write-state ()
  (let ((path (getenv "NEOMACS_GUI_STATE_JSON")))
    (when path
      (let* ((visible-text (buffer-substring-no-properties
                            (window-start)
                            (window-end nil t)))
             (payload
              (format
               "{\"buffer_name\":\"%s\",\"point\":%d,\"window_start\":%d,\"window_end\":%d,\"visible_text\":\"%s\"}\n"
               (neomacs-gui-smoke-json-escape (buffer-name))
               (point)
               (window-start)
               (window-end nil t)
               (neomacs-gui-smoke-json-escape visible-text))))
        (make-directory (file-name-directory path) t)
        (with-temp-file path
          (insert payload))))))

(neomacs-gui-smoke-write-state)
(run-at-time 2 nil (lambda () (kill-emacs 0)))
