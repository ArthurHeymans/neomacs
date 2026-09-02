;;; native-video-path-probe.el --- Verify the observed native video path -*- lexical-binding: t -*-

;;; Commentary:

;; Hardware-dependent release acceptance probe.  Run it through
;; scripts/probe-linux-native-video.sh, which guarantees a release Neomacs
;; binary and supplies NEOMACS_VIDEO_PROBE_FILE.

;;; Code:

(require 'neomacs-video)
(require 'seq)

(defvar neomacs-video-path-probe--handle nil)
(defvar neomacs-video-path-probe--started-at nil)
(defvar neomacs-video-path-probe--done nil)

(defun neomacs-video-path-probe--positive-integer-p (value)
  "Return non-nil when VALUE is an integer greater than zero."
  (and (integerp value) (> value 0)))

(defun neomacs-video-path-probe--zero-p (value)
  "Return non-nil when VALUE is the integer zero."
  (and (integerp value) (= value 0)))

(defun neomacs-video-path-probe--coherent-pool-p (pool)
  "Return non-nil when compositor-import POOL reports coherent pressure data."
  (let ((capacity (plist-get pool :capacity))
        (allocated (plist-get pool :allocated))
        (idle (plist-get pool :idle))
        (in-flight (plist-get pool :in-flight))
        (allocations (plist-get pool :allocations))
        (reuses (plist-get pool :reuses))
        (backpressure (plist-get pool :backpressured-acquires))
        (high-water (plist-get pool :in-flight-high-water)))
    (and (neomacs-video-path-probe--positive-integer-p capacity)
         (neomacs-video-path-probe--positive-integer-p allocated)
         (natnump idle)
         (natnump in-flight)
         (neomacs-video-path-probe--positive-integer-p allocations)
         (natnump reuses)
         (neomacs-video-path-probe--zero-p backpressure)
         (neomacs-video-path-probe--positive-integer-p high-water)
         (= allocated (+ idle in-flight))
         (<= allocated capacity)
         (<= allocated allocations)
         (<= in-flight high-water)
         (<= high-water capacity))))

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
               (sessions (plist-get snapshot :sessions))
               (session (car sessions))
               (state (and session (plist-get session :state)))
               (path (and session (plist-get session :frame-path)))
               (import (and path (plist-get path :compositor-import)))
               (presentation (and path (plist-get path :presentation)))
               (format (and session (plist-get session :frame-format)))
               (decoded (and session (plist-get session :decoded-frames)))
               (imported (and session (plist-get session :imported-frames)))
               (backpressured
                (and session (plist-get session :backpressured-frames)))
               (counts (and session (plist-get session :import-counts)))
               (presentation-counts
                (and session (plist-get session :presentation-counts)))
               (borrowed
                (and counts (plist-get counts :borrowed-native-frames)))
               (gpu-blits (and counts (plist-get counts :gpu-blit-frames)))
               (cpu-uploads (and counts (plist-get counts :cpu-upload-frames)))
               (submitted
                (and presentation-counts
                     (plist-get presentation-counts :submitted-frames)))
               (presented
                (and presentation-counts
                     (plist-get presentation-counts :presented-frames)))
               (pools
                (seq-filter
                 (lambda (pool)
                   (eq (plist-get pool :role) 'compositor-import))
                 (plist-get snapshot :surface-pools)))
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
            (cond
             ((/= (length sessions) 1)
              (neomacs-video-path-probe--finish
               1 (format "FAIL session-count=%s" (length sessions)) snapshot))
             ((not (memq format '(nv12 p010)))
              (neomacs-video-path-probe--finish
               1 (format "FAIL direct-frame-format=%s" format) snapshot))
             ((not (and
                    (neomacs-video-path-probe--positive-integer-p decoded)
                    (neomacs-video-path-probe--positive-integer-p imported)
                    (neomacs-video-path-probe--positive-integer-p borrowed)
                    (= borrowed imported)
                    (neomacs-video-path-probe--zero-p gpu-blits)
                    (neomacs-video-path-probe--zero-p cpu-uploads)
                    (neomacs-video-path-probe--zero-p backpressured)
                    (neomacs-video-path-probe--positive-integer-p submitted)
                    (neomacs-video-path-probe--positive-integer-p presented)
                    (<= presented submitted)))
              (neomacs-video-path-probe--finish
               1 "FAIL incoherent-import-or-presentation-counts" snapshot))
             ((/= (length pools) 1)
              (neomacs-video-path-probe--finish
               1 (format "FAIL compositor-pool-count=%s" (length pools))
               snapshot))
             ((not (neomacs-video-path-probe--coherent-pool-p (car pools)))
              (neomacs-video-path-probe--finish
               1 "FAIL incoherent-compositor-pool" snapshot))
             (t
              (neomacs-video-path-probe--finish
               0 "PASS direct-yuv-dma-buf-zero-copy" snapshot))))
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
