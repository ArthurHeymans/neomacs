;;; 07-string-borrow-across-change-hooks.el --- borrowed bytes vs. hooks -*- lexical-binding: t -*-
;;; expect: ("Zbcdefgh" "onetwo" "princ(sym)" (#("<arg>" 0 5 (face bold)) bold))

;; DIVERGENCES.md 163.  The `&'static LispString' seam has a second failure
;; mode that rooting does not cover: a live borrow whose BYTES are relocated.
;; `LispString::mutate_bytes' (heap_types.rs) rebuilds the payload `Vec' and
;; writes back a possibly-reallocated `data' pointer, so `aset' on a string can
;; invalidate an outstanding `&LispString' with no collection involved at all.
;;
;; GNU has the same hazard and is explicit about it: `compact_small_strings'
;; (src/alloc.c) RELOCATES small string data during every GC, which is why
;; `pin_string' exists at all.  A `char *' into string data held across a GC is
;; invalid in GNU by construction.
;;
;; The insert path is where the two meet.  `insert_lisp_string_with_change_
;; hooks_in_buffer' (editfns.rs) and `insert_print_lisp_string_with_hooks'
;; (builtins/misc_eval.rs) both take `text: &LispString', run
;; `signal_before_text_change' -- which calls `before-change-functions', i.e.
;; arbitrary Lisp and therefore a safe point -- and only THEN read `text'.
;;
;; Form 1 is also a behaviour pin, not only a safety one: GNU reads the string
;; AFTER the hook, so mutating the source string from `before-change-functions'
;; is observable, and the inserted text is "Zbcdefgh" rather than "abcdefgh".

(defvar gc-stress-sink nil)

(defun gc-stress-churn (&rest _)
  (setq gc-stress-sink (make-list 256 'churn))
  nil)

(prin1
 (list
  ;; 1. a before-change function mutates the very string being inserted
  (let ((s (copy-sequence "abcdefgh")))
    (with-temp-buffer
      (add-hook 'before-change-functions
                (lambda (&rest _) (gc-stress-churn) (aset s 0 ?Z))
                nil t)
      (insert s)
      (buffer-string)))
  ;; 2. an after-change function that conses, with a fresh source string
  (with-temp-buffer
    (add-hook 'after-change-functions (lambda (&rest _) (gc-stress-churn)) nil t)
    (insert (concat "one" "two"))
    (buffer-string))
  ;; 3. the print sinks, whose buffer has change hooks
  (with-temp-buffer
    (add-hook 'before-change-functions (lambda (&rest _) (gc-stress-churn)) nil t)
    (princ (concat "pri" "nc") (current-buffer))
    (prin1 (list (intern (concat "sy" "m"))) (current-buffer))
    (buffer-string))
  ;; 4. `format' carries the FORMAT string's text properties onto the result,
  ;; which `apply_format_prop_spans' (builtins/strings.rs) does through a
  ;; borrow of the freshly built result string.
  (let ((f (propertize (concat "<%s>") 'face 'bold)))
    (gc-stress-churn)
    (let ((r (format f (concat "ar" "g"))))
      (list r (get-text-property 0 'face r))))))
(terpri)
