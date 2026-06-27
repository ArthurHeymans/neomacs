//! Strict combo oracle probes, batch 12: cross-type append/vconcat, charset
//! encode/decode + char-charset, coding EOL conversion, terminal/frame
//! listing, buffer-list ordering after bury, cl-loop hash-key/value
//! iteration, and catch/throw tag semantics (nil tag, nesting, uncaught).
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_e7_cross_type_append_vconcat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (append [1 2 3] nil)
      (append [1 2] 3)
      (append '(1 2) [3 4])
      (vconcat [1] '(2 3))
      (vconcat '(1 2) [3])
      (append "ab" nil)
      (append nil 1 2)
      (vconcat nil 1 2)
      (append [1 2 3] [4 5]))
"##,
    );
}

#[test]
fn div_e7_charset_encode_decode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (decode-char 'ascii 65)
      (decode-char 'latin-iso8859-1 233)
      (encode-char ?é 'iso-8859-1)
      (encode-char 233 'unicode)
      (char-charset ?a)
      (char-charset ?é)
      (char-charset ?あ)
      (charsetp 'ascii)
      (charsetp 'nonexistent-probe-charset))
"##,
    );
}

#[test]
fn div_e7_coding_eol_conversion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (coding-system-change-eol-conversion 'utf-8 'unix)
      (coding-system-change-eol-conversion 'utf-8 'dos)
      (coding-system-change-eol-conversion 'utf-8 'mac)
      (coding-system-p (coding-system-change-eol-conversion 'utf-8 'dos))
      (coding-system-eol-type (coding-system-change-eol-conversion 'latin-1 'mac)))
"##,
    );
}

#[test]
fn div_e7_terminal_and_frame_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (length (terminal-list))
      (terminalp (car (terminal-list)))
      (framep (selected-frame))
      (length (frame-list))
      (eq (car (frame-list)) (selected-frame))
      (terminal-name (car (terminal-list)))
      (frame-visible-p (selected-frame)))
"##,
    );
}

#[test]
fn div_e7_buffer_list_ordering() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((a (get-buffer-create " *probe-blo-a*"))
      (b (get-buffer-create " *probe-blo-b*"))
      (c (get-buffer-create " *probe-blo-c*")))
  (unwind-protect
      (progn
        (switch-to-buffer a)
        (switch-to-buffer b)
        (switch-to-buffer c)
        (bury-buffer b)
        (let ((names (delq nil (mapcar (lambda (buf)
                                         (let ((n (buffer-name buf)))
                                           (and (string-prefix-p " *probe-blo-" n) n)))
                                       (buffer-list)))))
          (set-buffer a)
          (list names (buffer-name (current-buffer)))))
    (mapc (lambda (x) (when (buffer-live-p x) (kill-buffer x))) (list a b c))))
"##,
    );
}

#[test]
fn div_e7_cl_loop_hash_iteration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((h (make-hash-table :test 'equal)))
  (puthash 'a 1 h)
  (puthash 'b 2 h)
  (puthash 'c 3 h)
  (list (sort (cl-loop for k being the hash-keys of h collect k) #'string<)
        (sort (cl-loop for v being the hash-values of h collect v) #'<)
        (sort (cl-loop for k being the hash-keys of h using (hash-values v)
                       collect (cons k v))
              (lambda (x y) (string< (car x) (car y))))))
"##,
    );
}

#[test]
fn div_e7_catch_throw_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (catch 'tag (throw 'tag 42))
      (catch 'tag (dotimes (i 5) (when (= i 3) (throw 'tag i))) 'not-thrown)
      (catch nil (throw nil 'caught-nil))
      (catch 'outer (catch 'inner (throw 'outer 'escaped)))
      (catch 'tag2 (mapc (lambda (x) (when (> x 2) (throw 'tag2 x))) '(1 2 3 4)))
      (list (catch 'tag3 (throw 'tag3 (list 'a 'b)))))
"##,
    );
}
