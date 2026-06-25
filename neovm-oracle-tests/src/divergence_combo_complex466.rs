/// Batch 466: stress tests - many buffers, many overlays, large structures.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx466_many_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let (bufs)
  (dotimes (i 20) (push (get-buffer-create (format " *cx466-buf-%d*" i)) bufs))
  (mapc #'kill-buffer bufs)
  (length bufs))"##,
    );
}

#[test]
fn div_cx466_many_overlays() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert (make-string 1000 ?x))
  (let ((overs ()))
    (dotimes (i 100) (push (make-overlay (1+ i) (+ 2 i)) overs))
    (list (length (overlays-in 1 1000))
          (length overs)
          (length (car (overlay-lists)))
          (length (cdr (overlay-lists))))))"##,
    );
}

#[test]
fn div_cx466_many_markers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert (make-string 100 ?x))
  (let ((marks ()))
    (dotimes (i 50) (push (set-marker (make-marker) (1+ i)) marks))
    (length marks)))"##,
    );
}

#[test]
fn div_cx466_large_hash_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((ht (make-hash-table :test 'equal :size 500)))
  (dotimes (i 100) (puthash (format "key-%d" i) i ht))
  (hash-table-count ht))"##,
    );
}

#[test]
fn div_cx466_large_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((v (make-vector 1000 0)))
  (dotimes (i 1000) (aset v i (* i 2)))
  (aref v 999))"##,
    );
}

#[test]
fn div_cx466_deeply_nested_lists() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((l '(1 2 3 4 5 6 7 8 9 10)))
  (dotimes (_ 5) (setq l (list l l l)))
  (condition-case e (length l) (error (car e))))"##,
    );
}

#[test]
fn div_cx466_large_string_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((s (make-string 10000 ?x)))
  (list (length s) (string-bytes s) (aref s 5000)))"##,
    );
}

#[test]
fn div_cx466_many_processes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((procs ()))
  (dotimes (i 5)
    (push (make-process :name (format "cx466-p-%d" i)
                        :command '("echo" "hi") :connection-type 'pipe :buffer nil) procs))
  (mapc (lambda (p) (accept-process-output p 1)) procs)
  (mapc #'delete-process procs)
  (length procs))"##,
    );
}

#[test]
fn div_cx466_long_regex_backtracking() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-match "a*b*c*" "aaaaabbbbbccccc")
      (string-match "a+b+c+" "aaaabbbccc")
      (string-match "a.?b.?c.?" "abc"))"##,
    );
}

#[test]
fn div_cx466_many_text_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert (make-string 100 ?x))
  (dotimes (i 100) (put-text-property (1+ i) (+ 2 i) 'face 'bold))
  (length (text-properties-at 50)))"##,
    );
}

#[test]
fn div_cx466_many_font_lock_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'font-lock)
  (with-temp-buffer
    (emacs-lisp-mode)
    (insert "(defun a (x) x) (defun b (y) y) (defun c (z) z)")
    (font-lock-fontify-buffer)
    (count-lines (point-min) (point-max))))"##,
    );
}

#[test]
fn div_cx466_deep_recursion_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((max-lisp-eval-depth 100))
      (defun neo-cx466-recur (n) (if (<= n 0) 0 (1+ (neo-cx466-recur (1- n)))))
      (neo-cx466-recur 50))
  (error (car e)))"##,
    );
}

#[test]
fn div_cx466_large_process_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((buf (get-buffer-create " *cx466-large*")))
  (let ((proc (make-process :name "cx466-large"
                            :command '("sh" "-c" "printf '%%s' {1..1000}")
                            :connection-type 'pipe :buffer buf)))
    ;; Drain to completion (no-op the incidental sentinel) so buffer-size is
    ;; read after all output has arrived and the process has exited, instead of
    ;; racing a fixed 2s window where the process may still be live.
    (set-process-sentinel proc #'ignore)
    (while (process-live-p proc) (accept-process-output proc 1))
    (while (accept-process-output proc 0))
    (prog1 (with-current-buffer buf (buffer-size))
      (kill-buffer buf))))"##,
    );
}

#[test]
fn div_cx466_many_windows_created() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (split-window w nil 'right)
  (split-window w nil 'below)
  (count-windows))"##,
    );
}

#[test]
fn div_cx466_list_all_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((count 0))
  (mapatoms (lambda (_) (setq count (1+ count))))
  count)"##,
    );
}
