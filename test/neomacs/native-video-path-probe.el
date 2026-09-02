;;; native-video-path-probe.el --- Verify the observed native video path -*- lexical-binding: t -*-

;;; Commentary:

;; Hardware-dependent release acceptance probe.  Run it through
;; scripts/probe-linux-native-video.sh, which guarantees a release Neomacs
;; binary and supplies NEOMACS_VIDEO_PROBE_FILE.

;;; Code:

(require 'neomacs-video)

(defvar neomacs-video-path-probe--handle nil)
(defvar neomacs-video-path-probe--started-at nil)
(defvar neomacs-video-path-probe--done nil)

(defun neomacs-video-path-probe--record (text)
  "Record TEXT on the probe's explicit result channel, when configured."
  (when-let ((result-file (getenv "NEOMACS_VIDEO_PROBE_RESULT_FILE")))
    (write-region text nil result-file 'append 'silent)))

(defun neomacs-video-path-probe--finish (status result &optional snapshot)
  "Exit with STATUS after printing RESULT and optional diagnostic SNAPSHOT."
  (unless neomacs-video-path-probe--done
    (setq neomacs-video-path-probe--done t)
    (neomacs-video-path-probe--record
     (concat
      (format "NEOMACS_VIDEO_PROBE_RESULT %s\n" result)
      (when snapshot
        (format "NEOMACS_VIDEO_DIAGNOSTICS %S\n" snapshot))))
    (message "NEOMACS_VIDEO_PROBE_RESULT %s" result)
    (princ (format "NEOMACS_VIDEO_PROBE_RESULT %s\n" result))
    (when snapshot
      (message "NEOMACS_VIDEO_DIAGNOSTICS %S" snapshot)
      (princ "NEOMACS_VIDEO_DIAGNOSTICS ")
      (prin1 snapshot)
      (princ "\n"))
    (when (and neomacs-video-path-probe--handle
               (neomacs-video-p neomacs-video-path-probe--handle))
      (ignore-errors
        (neomacs-video-destroy neomacs-video-path-probe--handle)))
    (kill-emacs status)))

(defun neomacs-video-path-probe--poll ()
  "Wait for a compositor-visible frame and verify its observed path."
  (condition-case err
      (progn
        ;; Keep the display property in the submitted scene while polling.
        (redisplay t)
        (let* ((snapshot
                (neomacs-video-diagnostics
                 neomacs-video-path-probe--handle))
               (session (car (plist-get snapshot :sessions)))
               (state (and session (plist-get session :state)))
               (path (and session (plist-get session :frame-path)))
               (import (and path (plist-get path :compositor-import)))
               (presentation (and path (plist-get path :presentation)))
               (elapsed (- (float-time)
                           neomacs-video-path-probe--started-at))
               (timeout (string-to-number
                         (or (getenv "NEOMACS_VIDEO_PROBE_TIMEOUT") "15"))))
          (cond
           ((eq state 'failed)
            (neomacs-video-path-probe--finish
             1 "FAIL session-failed" snapshot))
           ((and path (not (eq presentation 'wgpu-composited)))
            (neomacs-video-path-probe--finish
             1 (format "FAIL presentation=%s" presentation) snapshot))
           ((eq import 'borrowed-native-surface)
            (neomacs-video-path-probe--finish
             0 "PASS dma-buf-zero-copy" snapshot))
           ((memq import '(gpu-blit cpu-upload))
            (neomacs-video-path-probe--finish
             1 (format "FAIL compositor-import=%s" import) snapshot))
           ((>= elapsed timeout)
            (neomacs-video-path-probe--finish
             1 "FAIL timed-out-without-compositor-frame" snapshot)))))
    (error
     (neomacs-video-path-probe--finish
      1 (format "FAIL probe-error=%S" err)))))

(defun neomacs-video-path-probe--start ()
  "Create one visible playing session and start observing its frame path."
  (let ((file (getenv "NEOMACS_VIDEO_PROBE_FILE")))
    (unless (and file (file-readable-p file))
      (neomacs-video-path-probe--finish
       1 (format "FAIL unreadable-input=%S" file)))
    (unless (fboundp 'neomacs-video-diagnostics)
      (neomacs-video-path-probe--finish
       1 "FAIL binary-does-not-expose-video-diagnostics"))
    (switch-to-buffer (get-buffer-create "*Native Video Path Probe*"))
    (erase-buffer)
    (insert "Neomacs native video path acceptance probe\n\n")
    (setq neomacs-video-path-probe--handle
          (neomacs-video-insert file 640 360 0 t))
    (goto-char (point-min))
    (setq neomacs-video-path-probe--started-at (float-time))
    (redisplay t)
    (run-at-time 0.1 0.1 #'neomacs-video-path-probe--poll)))

(add-hook 'emacs-startup-hook #'neomacs-video-path-probe--start)

;;; native-video-path-probe.el ends here
