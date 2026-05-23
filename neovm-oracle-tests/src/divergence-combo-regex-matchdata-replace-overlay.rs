//! Divergence tests: regex backreference + match-data + replace + overlay combos.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_regex_backref_replace_preserve_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "fooXXXbar fooYYYbar fooZZZbar")
  (let ((ov (make-overlay 4 7))
        (m (copy-marker 1 t)))
    (overlay-put ov 'tag 'middle)
    (put-text-property 1 10 'group 'first)
    (goto-char 1)
    (undo-boundary)
    (while (re-search-forward "foo\\([A-Z]+\\)bar" nil t)
      (replace-match "qux\\1baz"))
    (list (buffer-string)
          (overlay-start ov) (overlay-end ov)
          (overlay-get ov 'tag)
          (marker-position m)
          (> (marker-position m) 1)
          (get-text-property 1 'group)))) "#,
    );
}

#[test]
fn divergence_nested_match_data_save_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "alpha123beta456gamma789delta000")
  (goto-char 1)
  (re-search-forward "\\([a-z]+\\)\\([0-9]+\\)" nil t)
  (let ((outer-g1 (match-string 1))
        (outer-g2 (match-string 2))
        (outer-md (match-data t)))
    (re-search-forward "\\([a-z]+\\)\\([0-9]+\\)" nil t)
    (let ((inner-g1 (match-string 1))
          (inner-md (match-data t)))
      (set-match-data outer-md)
      (let ((restored-g1 (match-string 1))
            (restored-g2 (match-string 2)))
        (list outer-g1 outer-g2 inner-g1
              restored-g1 restored-g2
              (string= outer-g1 "alpha")
              (string= outer-g2 "123")
              (string= inner-g1 "gamma")
              (string= restored-g1 "alpha")
              (string= restored-g2 "123"))))) "#,
    );
}

#[test]
fn divergence_replace_with_overlay_at_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "AAAA-BBBB-CCCC-DDDD-EEEE")
  (let ((ov1 (make-overlay 4 5))
        (ov2 (make-overlay 9 10))
        (ov3 (make-overlay 14 15)))
    (overlay-put ov1 'edge 'a-b)
    (overlay-put ov2 'edge 'b-c)
    (overlay-put ov3 'edge 'c-d)
    (goto-char 1)
    (undo-boundary)
    (while (re-search-forward "BBBB\\|DDDD" nil t)
      (replace-match "XXXX"))
    (list (buffer-string)
          (overlay-start ov1) (overlay-end ov1)
          (overlay-start ov2) (overlay-end ov2)
          (overlay-start ov3) (overlay-end ov3)
          (overlay-get ov1 'edge)
          (overlay-get ov2 'edge)
          (overlay-get ov3 'edge)
          (buffer-size)))) "#,
    );
}

#[test]
fn divergence_regex_match_data_with_narrowing() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "MATCH1 nomatch MATCH2 nomatch MATCH3")
  (narrow-to-region 8 28)
  (goto-char (point-min))
  (let ((matches nil))
    (while (re-search-forward "MATCH[0-9]" nil t)
      (push (match-string 0) matches))
    (widen)
    (let ((md (match-data)))
      (list (nreverse matches)
            (equal (nreverse matches) '("MATCH2"))
            (= (length matches) 1)
            (buffer-string))))) "#,
    );
}

#[test]
fn divergence_replace_preserves_textprop_intervals() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "keep-REPLACE-keep-REPLACE-keep")
  (put-text-property 1 5 'zone 'start)
  (put-text-property 6 13 'zone 'replace1)
  (put-text-property 14 18 'zone 'middle)
  (put-text-property 19 26 'zone 'replace2)
  (put-text-property 27 31 'zone 'end)
  (goto-char 1)
  (undo-boundary)
  (while (re-search-forward "REPLACE" nil t)
    (replace-match "OK"))
  (list (buffer-string)
        (get-text-property 1 'zone)
        (eq (get-text-property 1 'zone) 'start)
        (get-text-property 6 'zone)
        (get-text-property 14 'zone)
        (eq (get-text-property 14 'zone) 'middle)
        (get-text-property 17 'zone)
        (get-text-property 21 'zone)
        (eq (get-text-property 21 'zone) 'end)
        (buffer-size))) "#,
    );
}

#[test]
fn divergence_regex_empty_match_advance() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "a,b,,c,,,d")
  (goto-char 1)
  (let ((parts nil)
        (last 0))
    (while (re-search-forward "\\(,\\)\\|$" nil t)
      (let ((sep-pos (match-beginning 0)))
        (push (buffer-substring last sep-pos) parts)
        (setq last (1+ sep-pos))))
    (list (nreverse parts)
          (= (length (nreverse parts)) 7)
          (buffer-string)))) "#,
    );
}

#[test]
fn divergence_match_data_after_failed_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "hello world")
  (goto-char 1)
  (re-search-forward "hello" nil t)
  (let ((md1 (match-data t))
        (ms1 (match-string 0)))
    (let ((found (re-search-forward "xyz" nil t)))
      (list found
            (null found)
            md1 ms1
            (string= ms1 "hello")
            (match-data)
            (equal (match-data) md1)))) "#,
    );
}

#[test]
fn divergence_replace_case_conversion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "Hello World Test")
  (goto-char 1)
  (undo-boundary)
  (while (re-search-forward "\\<[A-Z][a-z]+" nil t)
    (replace-match (downcase (match-string 0)) t))
  (list (buffer-string)
        (= (buffer-size) 16)
        (string= (buffer-string) "hello world test"))) "#,
    );
}

#[test]
fn divergence_regex_with_overlay_modification_hook() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defvar test-rmh-log-xxx nil)
  (insert "AAAA-BBBB-CCCC-DDDD-EEEE")
  (let ((ov (make-overlay 5 9)))
    (overlay-put ov 'modification-hooks
                 (list (lambda (ov after &rest _)
                         (when after (push 'modified test-rmh-log-xxx)))))
    (goto-char 1)
    (undo-boundary)
    (while (re-search-forward "BBBB" nil t)
      (replace-match "XXXX"))
    (list (buffer-string)
          test-rmh-log-xxx
          (>= (length test-rmh-log-xxx) 1)
          (overlay-start ov) (overlay-end ov)
          (buffer-size)))) "#,
    );
}

#[test]
fn divergence_multi_group_regex_replace_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "name:Alice age:30 name:Bob age:25")
  (goto-char 1)
  (let ((result nil))
    (while (re-search-forward "name:\\([A-Za-z]+\\) age:\\([0-9]+\\)" nil t)
      (push (list (match-string 1) (string-to-number (match-string 2))) result))
    (list (nreverse result)
          (= (length (nreverse result)) 2)
          (equal (car (nreverse result)) '("Alice" 30))
          (equal (cadr (nreverse result)) '("Bob" 25))
          (buffer-string)))) "#,
    );
}
