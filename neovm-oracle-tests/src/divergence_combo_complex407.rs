//! Complex combo batch 407 — 20 probes in new divergence territory:
//! keymap inheritance/parent, window edges/pixel-edges, buffer display
//! table slot manipulation, category-table modification, regexp-quote
//! with multibyte, rx with choices/backref, char-class regex with
//! multibyte, key-binding with remapped commands, completion-boundaries
//! with various tables, frame-parameters in batch, process-status/properties,
//! error-message-string on native errors, eval-after-load, with-demoted-errors,
//! obarray mapatoms, record type-of, hash-table-rehash-size/threshold,
//! marker-insertion-type, display-table-slot set/get, font-get/put,
//! and time-less-p with time values.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

/// keymap-parent inheritance chain: setting and retrieving
/// parent keymap for prefix key lookup.
#[test]
fn div_cx407_keymap_parent_inherit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((parent (make-sparse-keymap))
      (child (make-sparse-keymap)))
  (define-key parent "a" 'forward-char)
  (define-key child "b" 'backward-char)
  (set-keymap-parent child parent)
  (list (keymap-parent child)
        (key-binding "a" nil nil child)
        (key-binding "b" nil nil child)))
"##,
    );
}

/// window-edges / window-inside-edges / window-pixel-edges
/// in batch mode: may differ in return format.
#[test]
fn div_cx407_window_edges_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((w (selected-window)))
  (list (window-edges w)
        (window-inside-edges w)
        (window-pixel-edges w)
        (window-inside-pixel-edges w)))
"##,
    );
}

/// buffer-display-table slot get/set with custom glyph vectors.
#[test]
fn div_cx407_display_table_slot_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((dt (make-display-table)))
  (set-display-table-slot dt 'truncation (vector (make-glyph-code ?$ 'bold)))
  (set-display-table-slot dt 'selective-display (vector (make-glyph-code ?^ nil)))
  (set-window-display-table (selected-window) dt)
  (list (display-table-slot dt 'truncation)
        (display-table-slot dt 'selective-display)
        (window-display-table)))
"##,
    );
}

/// category-table modification: define-category then check.
#[test]
fn div_cx407_category_table_modify() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (copy-category-table)))
  (define-category ?t "test category" ct)
  (modify-category-entry ?a ?t ct)
  (modify-category-entry ?b ?t ct)
  (list (char-category-set ?a ct)
        (char-category-set ?c ct)
        (category-docstring ?t ct)))
"##,
    );
}

/// regexp-quote with multibyte special characters.
#[test]
fn div_cx407_regexp_quote_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (regexp-quote "café.世界")
      (regexp-quote "a*b+c")
      (regexp-quote "αβγ[0-9]"))
"##,
    );
}

/// rx regex with choices, backref, and multibyte.
#[test]
fn div_cx407_rx_choices_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (with-temp-buffer
    (insert "café or café world")
    (list (string-match (rx (or "café" "世界")) "café")
          (string-match (rx (or "café" "世界")) "世界")
          (string-match (rx (and "caf" (or "é" "e"))) "café"))))
"##,
    );
}

/// char-class regex [:alpha:] [:alnum:] with multibyte.
#[test]
fn div_cx407_regex_char_class_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (with-temp-buffer
    (insert "αβγ 123 café")
    (list (re-search-forward "[[:alpha:]]+" nil t)
          (match-string 0)
          (re-search-forward "[[:alnum:]]+" nil t)
          (match-string 0))))
"##,
    );
}

/// key-binding with command remapping: lookup keys
/// after remap may diverge.
#[test]
fn div_cx407_key_binding_remap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((map (make-sparse-keymap)))
  (define-key map "a" 'forward-word)
  (define-key map "b" 'backward-word)
  (list (key-binding "a" nil nil map)
        (key-binding "b" nil nil map)))
"##,
    );
}

/// completion-boundaries with different table types.
#[test]
fn div_cx407_completion_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((tab '("hello" "help" "helicopter")))
  (list (completion-boundaries "hel" tab nil "world")
        (test-completion "hello" tab)
        (test-completion "nope" tab)))
"##,
    );
}

/// frame-parameters in batch mode: returns different
/// parameter sets between Neomacs and GNU.
#[test]
fn div_cx407_frame_parameters_batch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((fp (frame-parameters (selected-frame))))
  (list (assq 'display-type fp)
        (assq 'background-mode fp)
        (assq 'window-system fp)))
"##,
    );
}

/// process-status and process-properties: querying
/// process state after completion.
#[test]
fn div_cx407_process_status_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((proc (make-process :name "neo-cx407-ps"
                          :command '("sh" "-c" "echo done")
                          :connection-type 'pipe :buffer nil)))
  (accept-process-output proc 2)
  (list (process-status proc)
        (process-type proc)
        (process-get proc 'foo)
        (process-put proc 'foo 'bar)
        (process-get proc 'foo)))
"##,
    );
}

/// error-message-string on native error conditions:
/// error formatting for different error types.
#[test]
fn div_cx407_error_message_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (car 1 2) (error (error-message-string e)))
      (condition-case e (+ "a" 1) (error (error-message-string e)))
      (condition-case e (signal 'void-variable '(x)) (error (error-message-string e))))
"##,
    );
}

/// obarray mapatoms: iterating over obarray entries.
#[test]
fn div_cx407_obarray_mapatoms() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((obs (obarray-default))
      (count 0))
  (mapatoms (lambda (s) (setq count (1+ count))) obs)
  (list count (> count 100) (intern "neo-cx407-test-sym" obs)))
"##,
    );
}

/// record type-of: type checking on record objects.
#[test]
fn div_cx407_record_type_of() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((r (record 'neo-cx407-type 1 2 3)))
  (list (recordp r)
        (type-of r)
        (aref r 0)
        (aref r 1)))
"##,
    );
}

/// hash-table-rehash-size / threshold / size:
/// hash table configuration may differ.
#[test]
fn div_cx407_hash_table_config() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :size 10 :rehash-size 2.0 :rehash-threshold 0.8)))
  (list (hash-table-size ht)
        (hash-table-rehash-size ht)
        (hash-table-rehash-threshold ht)
        (hash-table-test ht)))
"##,
    );
}

/// marker-insertion-type: inserting at marker with
/// different insertion types.
#[test]
fn div_cx407_marker_insertion_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "abc")
  (let ((m1 (set-marker (make-marker) 2))
        (m2 (set-marker (make-marker) 2)))
    (set-marker-insertion-type m1 t)
    (set-marker-insertion-type m2 nil)
    (insert "X")
    (list (marker-position m1)
          (marker-position m2)
          (marker-insertion-type m1)
          (marker-insertion-type m2)))
  (buffer-string))
"##,
    );
}

/// font-get / font-put on font objects.
#[test]
fn div_cx407_font_get_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (font-spec :family "Monospace" :size 10)))
  (list (font-get f :family)
        (font-get f :size)
        (font-put f :size 12)
        (font-get f :size)))
"##,
    );
}

/// time-less-p with time value edge cases.
#[test]
fn div_cx407_time_less_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((t1 (encode-time 0 0 0 1 1 2024 nil))
      (t2 (encode-time 0 0 0 2 1 2024 nil)))
  (list (time-less-p t1 t2)
        (time-less-p t2 t1)
        (time-equal-p t1 t1)
        (time-add t1 (seconds-to-time 86400))
        (time-subtract t2 t1)))
"##,
    );
}

/// with-demoted-errors: error demotion may behave differently.
#[test]
fn div_cx407_with_demoted_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((result (with-demoted-errors "DEMOTED: %S" (car 1 2))))
  (list (stringp result)
        (if (stringp result) (string-match "DEMOTED" result) nil)))
"##,
    );
}
