;;; sustained-native-video.el --- physical-GPU video benchmark -*- lexical-binding: t; -*-

(require 'json)
(require 'neomacs-video)
(require 'seq)

(defvar neomacs-perf-native-video--handle nil)
(defvar neomacs-perf-native-video--started-at nil)
(defvar neomacs-perf-native-video--cpu-started-at nil)
(defvar neomacs-perf-native-video--warmup-started-at nil)
(defvar neomacs-perf-native-video--viewport-started-at nil)
(defvar neomacs-perf-native-video--presentation-width nil)
(defvar neomacs-perf-native-video--presentation-height nil)
(defvar neomacs-perf-native-video--baseline nil)
(defvar neomacs-perf-native-video--ticks 0)
(defvar neomacs-perf-native-video--done nil)
(defvar neomacs-perf-native-video--gate-process nil)
(defvar neomacs-perf-native-video--gate-response "")
(defvar neomacs-perf-native-video--sampling-enabled nil)
(defvar neomacs-perf-native-video--viewport-timer nil)
(defvar neomacs-perf-native-video--warmup-timer nil)
(defvar neomacs-perf-native-video--sample-timer nil)

(defun neomacs-perf-native-video--required-environment (name)
  (or (getenv name)
      (error "required performance environment variable %s is absent" name)))

(defun neomacs-perf-native-video--positive-environment-integer (name)
  (let* ((text (neomacs-perf-native-video--required-environment name))
         (value (string-to-number text)))
    (unless (> value 0)
      (error "required performance environment variable %s is not positive: %S"
             name text))
    value))

(defun neomacs-perf-native-video--cancel-timers ()
  (dolist (timer (list neomacs-perf-native-video--viewport-timer
                       neomacs-perf-native-video--warmup-timer
                       neomacs-perf-native-video--sample-timer))
    (when (timerp timer)
      (cancel-timer timer)))
  (setq neomacs-perf-native-video--viewport-timer nil
        neomacs-perf-native-video--warmup-timer nil
        neomacs-perf-native-video--sample-timer nil))

(defun neomacs-perf-native-video--gate-filter (_process output)
  (setq neomacs-perf-native-video--gate-response
        (concat neomacs-perf-native-video--gate-response output)))

(defun neomacs-perf-native-video--sampling-command (command)
  (let ((port-text (getenv "NEOMACS_PERF_GATE_PORT")))
    (when port-text
      (unless (process-live-p neomacs-perf-native-video--gate-process)
        (setq neomacs-perf-native-video--gate-process
              (make-network-process
               :name "neomacs-perf-native-video-gate"
               :family 'ipv4 :host "127.0.0.1"
               :service (string-to-number port-text)
               :coding 'binary :noquery t
               :filter #'neomacs-perf-native-video--gate-filter)))
      (setq neomacs-perf-native-video--gate-response "")
      (process-send-string neomacs-perf-native-video--gate-process
                           (concat command "\n"))
      (let ((deadline (+ (float-time) 30.0)))
        (while (and (not (string-suffix-p
                          "\n" neomacs-perf-native-video--gate-response))
                    (< (float-time) deadline))
          (unless (process-live-p neomacs-perf-native-video--gate-process)
            (error "performance gate disconnected during %s" command))
          (accept-process-output neomacs-perf-native-video--gate-process 0.05)))
      (unless (equal neomacs-perf-native-video--gate-response "ack\n")
        (error "performance gate rejected %s: %S"
               command neomacs-perf-native-video--gate-response)))))

(defun neomacs-perf-native-video--close-gate ()
  (when (processp neomacs-perf-native-video--gate-process)
    (delete-process neomacs-perf-native-video--gate-process)
    (setq neomacs-perf-native-video--gate-process nil)))

(defun neomacs-perf-native-video--session (snapshot)
  (car (plist-get snapshot :sessions)))

(defun neomacs-perf-native-video--pool (snapshot)
  (seq-find (lambda (pool)
              (eq (plist-get pool :role) 'compositor-import))
            (plist-get snapshot :surface-pools)))

(defun neomacs-perf-native-video--nested (plist outer inner)
  (plist-get (plist-get plist outer) inner))

(defun neomacs-perf-native-video--delta (final baseline key &optional outer)
  (max 0 (- (or (if outer
                    (neomacs-perf-native-video--nested final outer key)
                  (plist-get final key))
                0)
            (or (if outer
                    (neomacs-perf-native-video--nested baseline outer key)
                  (plist-get baseline key))
                0))))

(defun neomacs-perf-native-video--write-result
    (status iterations elapsed-cpu-us elapsed-wall-us snapshot error-message)
  (let* ((session (neomacs-perf-native-video--session snapshot))
         (baseline-session
          (neomacs-perf-native-video--session
           neomacs-perf-native-video--baseline))
         (path (plist-get session :frame-path))
         (decoder (plist-get session :decoder))
         (renderer (plist-get snapshot :renderer))
         (counts (plist-get session :import-counts))
         (presentation (plist-get session :presentation-counts))
         (timing (plist-get session :presentation-timing))
         (gpu (plist-get session :gpu-timing))
         (baseline-counts (plist-get baseline-session :import-counts))
         (baseline-presentation
          (plist-get baseline-session :presentation-counts))
         (baseline-timing
          (plist-get baseline-session :presentation-timing))
         (baseline-gpu (plist-get baseline-session :gpu-timing))
         (pool (neomacs-perf-native-video--pool snapshot))
         (baseline-pool
          (neomacs-perf-native-video--pool
           neomacs-perf-native-video--baseline)))
    (with-temp-file
        (neomacs-perf-native-video--required-environment "NEOMACS_PERF_RESULT")
      (insert
       (json-serialize
        `((schema_version . 4)
          (scenario . "sustained-native-video")
          (status . ,status)
          (iterations . ,iterations)
          (elapsed_cpu_us . ,elapsed-cpu-us)
          (elapsed_wall_us . ,elapsed-wall-us)
          (presentation_width_pixels
           . ,neomacs-perf-native-video--presentation-width)
          (presentation_height_pixels
           . ,neomacs-perf-native-video--presentation-height)
          (viewport_width_pixels . ,(window-body-width nil t))
          (viewport_height_pixels . ,(window-body-height nil t))
          (backend . ,(symbol-name (plist-get session :backend)))
          (decode_residency
           . ,(symbol-name (plist-get path :decode-residency)))
          (decoder_factory . ,(plist-get decoder :factory))
          (decoder_plugin . ,(plist-get decoder :plugin))
          (decoder_kind . ,(symbol-name (plist-get decoder :kind)))
          (gpu_adapter_name . ,(plist-get renderer :adapter-name))
          (gpu_vendor . ,(plist-get renderer :vendor))
          (gpu_device . ,(plist-get renderer :device))
          (gpu_device_type . ,(plist-get renderer :device-type))
          (graphics_backend
           . ,(symbol-name (plist-get renderer :graphics-backend)))
          (gpu_driver . ,(plist-get renderer :driver))
          (gpu_driver_info . ,(plist-get renderer :driver-info))
          (drm_render_node . ,(plist-get renderer :drm-render-node))
          (display_refresh_hz . ,(plist-get renderer :display-refresh-hz))
          (frame_format . ,(symbol-name (plist-get session :frame-format)))
          (compositor_import
           . ,(symbol-name (plist-get path :compositor-import)))
          (presentation . ,(symbol-name (plist-get path :presentation)))
          (decoded_frames
           . ,(neomacs-perf-native-video--delta
               session baseline-session :decoded-frames))
          (replaced_frames
           . ,(neomacs-perf-native-video--delta
               session baseline-session :replaced-frames))
          (late_dropped_frames
           . ,(neomacs-perf-native-video--delta
               session baseline-session :late-dropped-frames))
          (imported_frames
           . ,(neomacs-perf-native-video--delta
               session baseline-session :imported-frames))
          (backpressured_frames
           . ,(neomacs-perf-native-video--delta
               session baseline-session :backpressured-frames))
          (borrowed_native_frames
           . ,(max 0 (- (or (plist-get counts :borrowed-native-frames) 0)
                         (or (plist-get baseline-counts
                                        :borrowed-native-frames)
                             0))))
          (gpu_blit_frames
           . ,(max 0 (- (or (plist-get counts :gpu-blit-frames) 0)
                         (or (plist-get baseline-counts :gpu-blit-frames) 0))))
          (cpu_upload_frames
           . ,(max 0 (- (or (plist-get counts :cpu-upload-frames) 0)
                         (or (plist-get baseline-counts :cpu-upload-frames) 0))))
          (submitted_frames
           . ,(max 0 (- (or (plist-get presentation :submitted-frames) 0)
                         (or (plist-get baseline-presentation :submitted-frames)
                             0))))
          (presented_frames
           . ,(max 0 (- (or (plist-get presentation :presented-frames) 0)
                         (or (plist-get baseline-presentation :presented-frames)
                             0))))
          (interval_samples
           . ,(max 0 (- (or (plist-get timing :interval-samples) 0)
                         (or (plist-get baseline-timing :interval-samples) 0))))
          (interval_p50_us . ,(or (plist-get timing :interval-p50-us) 0))
          (interval_p95_us . ,(or (plist-get timing :interval-p95-us) 0))
          (interval_p99_us . ,(or (plist-get timing :interval-p99-us) 0))
          (interval_max_us . ,(or (plist-get timing :interval-max-us) 0))
          (gpu_timing_status . ,(symbol-name (plist-get gpu :status)))
          (gpu_pass_samples
           . ,(max 0 (- (or (plist-get gpu :pass-samples) 0)
                         (or (plist-get baseline-gpu :pass-samples) 0))))
          (gpu_pass_total_us
           . ,(max 0 (- (or (plist-get gpu :pass-total-us) 0)
                         (or (plist-get baseline-gpu :pass-total-us) 0))))
          (gpu_pass_min_us . ,(plist-get gpu :pass-min-us))
          (gpu_pass_max_us . ,(plist-get gpu :pass-max-us))
          (gpu_memory_bytes . ,(plist-get snapshot :gpu-memory-bytes))
          (pool_capacity . ,(or (plist-get pool :capacity) 0))
          (pool_allocations
           . ,(max 0 (- (or (plist-get pool :allocations) 0)
                         (or (plist-get baseline-pool :allocations) 0))))
          (pool_reuses
           . ,(max 0 (- (or (plist-get pool :reuses) 0)
                         (or (plist-get baseline-pool :reuses) 0))))
          (pool_backpressured_acquires
           . ,(max 0 (- (or (plist-get pool :backpressured-acquires) 0)
                         (or (plist-get baseline-pool
                                        :backpressured-acquires)
                             0))))
          (pool_in_flight_high_water
           . ,(or (plist-get pool :in-flight-high-water) 0))
          (error . ,error-message))
        :null-object nil :false-object :json-false)))))

(defun neomacs-perf-native-video--finish
    (status &optional snapshot error-message)
  (unless neomacs-perf-native-video--done
    (setq neomacs-perf-native-video--done t)
    (neomacs-perf-native-video--cancel-timers)
    (let* ((snapshot
            (or snapshot
                (ignore-errors
                  (neomacs-video-diagnostics
                   neomacs-perf-native-video--handle))))
           (elapsed-cpu-us
            (if neomacs-perf-native-video--cpu-started-at
                (max 1 (- (car (current-cpu-time))
                          neomacs-perf-native-video--cpu-started-at))
              0))
           (elapsed-wall-us
            (if neomacs-perf-native-video--started-at
                (max 1 (round
                        (* 1000000
                           (- (float-time)
                              neomacs-perf-native-video--started-at))))
              0)))
      (when neomacs-perf-native-video--sampling-enabled
        (setq neomacs-perf-native-video--sampling-enabled nil)
        (condition-case gate-error
            (neomacs-perf-native-video--sampling-command "disable")
          (error
           (setq status "error"
                 error-message
                 (format "failed to disable performance sampling: %S"
                         gate-error)))))
      (neomacs-perf-native-video--close-gate)
      (condition-case write-error
          (neomacs-perf-native-video--write-result
           status neomacs-perf-native-video--ticks elapsed-cpu-us
           elapsed-wall-us snapshot error-message)
        (error
         (message "failed to publish native-video result: %S" write-error)))
      (with-temp-file
          (neomacs-perf-native-video--required-environment "SENTINEL")
        (insert status "\n"))
      (when (and neomacs-perf-native-video--handle
                 (neomacs-video-p neomacs-perf-native-video--handle))
        (ignore-errors
          (neomacs-video-destroy neomacs-perf-native-video--handle)))
      (kill-emacs (if (equal status "ok") 0 2)))))

(defun neomacs-perf-native-video--sample ()
  (condition-case error-data
      (progn
        (setq neomacs-perf-native-video--ticks
              (1+ neomacs-perf-native-video--ticks))
        (when (>= neomacs-perf-native-video--ticks
                  (string-to-number
                   (neomacs-perf-native-video--required-environment
                    "NEOMACS_PERF_ITERATIONS")))
          ;; One final redisplay lets the asynchronous GPU timer retire its
          ;; last completed slots before the diagnostic snapshot.
          (redisplay t)
          (neomacs-perf-native-video--finish
           "ok"
           (neomacs-video-diagnostics neomacs-perf-native-video--handle))))
    (error
     (neomacs-perf-native-video--finish
      "error" nil (prin1-to-string error-data)))))

(defun neomacs-perf-native-video--warmup ()
  (condition-case error-data
      (let* ((snapshot
              (neomacs-video-diagnostics
               neomacs-perf-native-video--handle))
             (session (neomacs-perf-native-video--session snapshot))
             (path (plist-get session :frame-path))
             (gpu (plist-get session :gpu-timing))
             (presented
              (neomacs-perf-native-video--nested
               session :presentation-counts :presented-frames))
             (timing-samples
              (neomacs-perf-native-video--nested
               session :presentation-timing :interval-samples))
             (gpu-samples (plist-get gpu :pass-samples)))
        (cond
         ((eq (plist-get session :state) 'failed)
          (error "video backend failed during warmup"))
         ((> (- (float-time) neomacs-perf-native-video--warmup-started-at) 20)
          (error "timed out waiting for direct video and timing samples"))
         ((and (eq (plist-get path :compositor-import)
                   'borrowed-native-surface)
               (eq (plist-get path :presentation) 'wgpu-composited)
               (>= (or presented 0) 30)
               (>= (or timing-samples 0) 29)
               (or (eq (plist-get gpu :status) 'unsupported)
                   (and (eq (plist-get gpu :status) 'enabled)
                        (> (or gpu-samples 0) 0))))
          (when (timerp neomacs-perf-native-video--warmup-timer)
            (cancel-timer neomacs-perf-native-video--warmup-timer))
          (setq neomacs-perf-native-video--warmup-timer nil)
          ;; Enable external sampling and start clocks before crossing the
          ;; acknowledged renderer/native boundary. The boundary returns its
          ;; zero-point snapshot atomically; consequently every post-reset
          ;; frame is inside the elapsed-time and external-sampling windows.
          (neomacs-perf-native-video--sampling-command "enable")
          (setq neomacs-perf-native-video--sampling-enabled t
                neomacs-perf-native-video--cpu-started-at
                (car (current-cpu-time))
                neomacs-perf-native-video--started-at (float-time))
          (setq neomacs-perf-native-video--baseline
                (neomacs-video-begin-measurement-epoch))
          (setq neomacs-perf-native-video--sample-timer
                (run-at-time
                 0.1 0.1 #'neomacs-perf-native-video--sample)))))
    (error
     (neomacs-perf-native-video--finish
      "error" nil (prin1-to-string error-data)))))

(defun neomacs-perf-native-video--open-video (video-file)
  (setq neomacs-perf-native-video--handle
        (neomacs-video-insert
         video-file
         neomacs-perf-native-video--presentation-width
         neomacs-perf-native-video--presentation-height
         -1 t)
        neomacs-perf-native-video--warmup-started-at (float-time))
  (goto-char (point-min))
  (redisplay t)
  (setq neomacs-perf-native-video--warmup-timer
        (run-at-time 0.05 0.05 #'neomacs-perf-native-video--warmup)))

(defun neomacs-perf-native-video--await-viewport ()
  (condition-case error-data
      (let ((width (window-body-width nil t))
            (height (window-body-height nil t)))
        (cond
         ((and (>= width neomacs-perf-native-video--presentation-width)
               (>= height neomacs-perf-native-video--presentation-height))
          (when (timerp neomacs-perf-native-video--viewport-timer)
            (cancel-timer neomacs-perf-native-video--viewport-timer))
          (setq neomacs-perf-native-video--viewport-timer nil)
          (neomacs-perf-native-video--open-video
           (neomacs-perf-native-video--required-environment
            "NEOMACS_PERF_VIDEO_FILE")))
         ((> (- (float-time) neomacs-perf-native-video--viewport-started-at)
             5.0)
          (error
           "GUI viewport %dx%d cannot contain requested %dx%d video"
           width height
           neomacs-perf-native-video--presentation-width
           neomacs-perf-native-video--presentation-height))))
    (error
     (neomacs-perf-native-video--finish
      "error" nil (prin1-to-string error-data)))))

(defun neomacs-perf-native-video--start ()
  (condition-case error-data
      (let ((video-file
             (neomacs-perf-native-video--required-environment
              "NEOMACS_PERF_VIDEO_FILE")))
        (unless (file-readable-p video-file)
          (error "native-video input is unreadable: %s" video-file))
        (setq neomacs-perf-native-video--presentation-width
              (neomacs-perf-native-video--positive-environment-integer
               "NEOMACS_PERF_VIDEO_WIDTH")
              neomacs-perf-native-video--presentation-height
              (neomacs-perf-native-video--positive-environment-integer
               "NEOMACS_PERF_VIDEO_HEIGHT")
              neomacs-perf-native-video--viewport-started-at (float-time))
        (switch-to-buffer (get-buffer-create "*Sustained Native Video*"))
        (erase-buffer)
        (setq neomacs-perf-native-video--viewport-timer
              (run-at-time
               0 0.05 #'neomacs-perf-native-video--await-viewport)))
    (error
     (neomacs-perf-native-video--finish
      "error" nil (prin1-to-string error-data)))))

(add-hook 'emacs-startup-hook #'neomacs-perf-native-video--start)

;;; sustained-native-video.el ends here
