;;; 05-thread-yield-handoff.el --- in-flight thread-blocked payload -*- lexical-binding: t -*-
;;; expect: (thread-probe-done (blocked datum))

;; DIVERGENCES.md 162. `Flow::ThreadBlocked' carries the object being waited on
;; and the forms to re-dispatch on resume. `sf_condition_case_value_named''s
;; ThreadBlocked arm rebuilds a continuation from both while they live only in
;; Rust locals, and one frame out the same `unwind-protect' cleanups run.

(defvar gc-stress-sink nil)
(defvar gc-stress-result nil)

(let ((thread
       (make-thread
        (lambda ()
          (condition-case _err
              (unwind-protect
                  (progn
                    (thread-yield)
                    (setq gc-stress-sink (make-list 256 'churn))
                    (thread-yield)
                    (setq gc-stress-result
                          (list (intern (concat "blocked" ""))
                                (intern (concat "datum" "")))))
                (setq gc-stress-sink (make-list 256 'cleanup)))
            (error nil)))
        "gc-stress-probe")))
  (while (thread-live-p thread)
    (setq gc-stress-sink (make-list 64 'main))
    (thread-yield))
  (thread-join thread))

(prin1 (list 'thread-probe-done gc-stress-result))
(terpri)
