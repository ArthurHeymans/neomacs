;;; task02-profile-session.el --- JIT intrinsics round-2 profiling session  -*- lexical-binding: t; -*-

;; Batch-scripted interactive editing session for task 02 (JIT intrinsics
;; round 2, STAGE 1: profile the REAL interactive builtin population).
;;
;; Run against a `vm-profile'-instrumented binary (see task 02 §3(a)):
;;
;;   cargo xtask fresh-build --release --features vm-profile
;;   NEOVM_JIT=0 target/release/neomacs --batch -l scripts/task02-profile-session.el
;;
;; NEOVM_JIT=0 is REQUIRED: a tiered-to-native body bypasses the interpreter's
;; `vm_profile::bump*` sites and silently drops counts. The session opens a
;; large real .el buffer, font-locks it, then runs several thousand mixed
;; interactive operations (motion, search, edit, indent, replace, re-fontify).
;; `neovm--vm-profile-reset' clears loadup/startup traffic first;
;; `neovm--vm-profile-dump' prints the OP-MIX + SUBR-MIX (with the
;; Op::Call-vs-CallBuiltinSym entry split) to stderr at the end.
;;
;; The driver defuns are byte-compiled so their OWN calls traverse `run_loop'
;; and land in the entry split (a tree-walked driver would still count SUBR-MIX
;; totals but attribute its calls to the "other" bucket).

(require 'font-lock)

(defvar t2-source-files '("subr.el" "simple.el" "files.el" "cl-lib.el")
  "Real elisp libraries concatenated into the edit buffer (sizeable, varied).")

(defun t2-load-big-buffer ()
  "Return a buffer holding several large real .el files in `emacs-lisp-mode'."
  (let ((buf (get-buffer-create "*t2-edit*")))
    (with-current-buffer buf
      (fundamental-mode)
      (erase-buffer)
      (dolist (f t2-source-files)
        (let ((path (locate-library f t)))
          (when (and path (file-readable-p path))
            (goto-char (point-max))
            (insert-file-contents path)
            (goto-char (point-max)))))
      (goto-char (point-min))
      (emacs-lisp-mode)
      (font-lock-set-defaults)
      (setq buffer-undo-list t))       ; don't grow undo during churn
    buf))

(defun t2-fontlock-chunked (chunk)
  "Fontify the whole buffer CHUNK chars at a time, as jit-lock does."
  (let ((pos (point-min)) (end (point-max)))
    (while (< pos end)
      (let ((to (min end (+ pos chunk))))
        (font-lock-fontify-region pos to)
        (setq pos to)))))

(defun t2-motion-sweep (limit)
  "Character + line motion with position/edge inspection (buffer-op opcodes)."
  (goto-char (point-min))
  (let ((k 0))
    (while (and (< (point) (point-max)) (< k limit))
      (forward-char 1)
      (when (bolp) (setq k (1+ k)))
      (when (eolp) (setq k (1+ k)))
      (following-char) (preceding-char) (current-column) (point)
      (setq k (1+ k))))
  (goto-char (point-min))
  (let ((k 0))
    (while (and (zerop (forward-line 1)) (< k limit))
      (end-of-line) (beginning-of-line) (current-column)
      (setq k (1+ k)))))

(defun t2-search-sweep (limit)
  "search-forward + re-search-forward with match-data accessors."
  (goto-char (point-min))
  (let ((k 0))
    (while (and (< k limit) (search-forward "def" nil t))
      (match-beginning 0) (match-end 0)
      (setq k (1+ k))))
  (goto-char (point-min))
  (let ((k 0))
    (while (and (< k limit)
                (re-search-forward "(\\(defun\\|defvar\\|defmacro\\|defconst\\)\\_>" nil t))
      (match-beginning 1) (match-end 1) (match-string 1)
      (setq k (1+ k)))))

(defun t2-edit-churn (rounds)
  "Insert/delete churn — drives after-change hooks + re-fontification."
  (goto-char (point-min))
  (let ((n 0))
    (while (and (< n rounds) (zerop (forward-line 7)))
      (let ((p (point)))
        (insert "; t2 touch line\n")
        (delete-region p (line-end-position))
        (delete-char 1))
      (setq n (1+ n)))))

(defun t2-indent-sweep (limit)
  "Re-indent lines — heavy syntax parsing (parse-partial-sexp/forward-sexp)."
  (goto-char (point-min))
  (let ((k 0))
    (while (and (< k limit) (zerop (forward-line 1)))
      (indent-according-to-mode)
      (setq k (1+ k)))))

(defun t2-replace-sweep (limit)
  "query-replace's inner engine: re-search-forward + replace-match loop."
  (goto-char (point-min))
  (let ((k 0))
    (while (and (< k limit) (re-search-forward "\\_<nil\\_>" nil t))
      (replace-match "nil")            ; identity replacement: buffer stable
      (setq k (1+ k)))))

(defmacro t2-phase (name &rest body)
  "Run BODY, logging failures without aborting the session (so the dump runs)."
  `(condition-case e (progn ,@body)
     (error (message "t2: phase %s FAILED: %s" ,name (error-message-string e)))))

(defun t2-run ()
  (let ((buf (t2-load-big-buffer)))
    (with-current-buffer buf
      (message "t2: buffer %d chars, mode %s" (buffer-size) major-mode)
      (t2-phase "fontlock-1"  (t2-fontlock-chunked 1500))
      (t2-phase "font-lock-ensure" (font-lock-ensure (point-min) (point-max)))
      (t2-phase "motion"      (t2-motion-sweep 40000))
      (t2-phase "search"      (t2-search-sweep 40000))
      (t2-phase "edit-churn"  (t2-edit-churn 3000))
      (t2-phase "indent"      (t2-indent-sweep 6000))
      (t2-phase "replace"     (t2-replace-sweep 8000))
      (t2-phase "fontlock-2"  (t2-fontlock-chunked 1500)) ; re-fontify after edits
      (buffer-size))))

;; Byte-compile the driver so its own calls traverse run_loop (entry split).
(dolist (fn '(t2-load-big-buffer t2-fontlock-chunked t2-motion-sweep
              t2-search-sweep t2-edit-churn t2-indent-sweep t2-replace-sweep
              t2-run))
  (byte-compile fn))

(if (not (fboundp 'neovm--vm-profile-reset))
    (progn
      (message "ERROR: binary lacks neovm--vm-profile-* subrs; \
build with --features vm-profile")
      (kill-emacs 2))
  (garbage-collect)
  (neovm--vm-profile-reset)
  (let ((t0 (current-time)))
    (condition-case e (t2-run)
      (error (message "t2: t2-run aborted: %s" (error-message-string e))))
    (message "t2: session ran in %.2fs" (float-time (time-subtract (current-time) t0))))
  ;; Always dump, even if a phase aborted — partial data still ranks the mix.
  (neovm--vm-profile-dump "task02 batch interactive editing session"))

(kill-emacs 0)

;;; task02-profile-session.el ends here
