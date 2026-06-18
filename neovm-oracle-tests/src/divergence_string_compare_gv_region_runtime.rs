//! String compare/collate (compare-strings, string-version-lessp,
//! string-greaterp/collate/equalp, string-search start, string-distance),
//! generalized variables (setf gethash/get/cl-letf/nested places/push-pop),
//! and region transforms (case ops, subst-char, indent-rigidly,
//! replace-buffer-contents, fill-region) parity.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn compare_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (compare-strings "abc" nil nil "abd" nil nil)
        (compare-strings "ABC" nil nil "abc" nil nil t)
        (compare-strings "abc" 0 2 "abXYZ" 0 2)
        (compare-strings "abc" nil nil "abc" nil nil))"##,
    );
}

#[test]
fn string_collate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((s1 "apple") (s2 "banana"))
  (list (and (string-collate-lessp s1 s2 "C") t)
        (and (string-collate-equalp "abc" "abc" "C") t)))"##,
    );
}

#[test]
fn string_distance_bytes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-distance "abc" "abc") (string-distance "abc" "abd")
        (string-distance "kitten" "sitting") (string-distance "" "abc")
        (string-distance "café" "cafe" t))"##,
    );
}

#[test]
fn string_greaterp_lessp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-greaterp "b" "a") (string-lessp "a" "b")
        (string-lessp "" "a") (string-lessp "abc" "abc")
        (string< "Z" "a") (string-equal-ignore-case "ABC" "abc"))"##,
    );
}

#[test]
fn string_search_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-search "a" "banana" 2) (string-search "na" "banana" 3)
        (string-search "x" "banana") (string-search "" "abc" 1))"##,
    );
}

#[test]
fn string_version_lessp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(list (string-version-lessp "foo2" "foo10")
        (string-version-lessp "foo10" "foo2")
        (string-version-lessp "a9b" "a10b")
        (string-version-lessp "1.2" "1.10"))"##,
    );
}

#[test]
fn cl_letf_dynamic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(defvar neo-cl-letf-var-xyz 'orig)
(let ((during (cl-letf (((symbol-value 'neo-cl-letf-var-xyz) 'temp)) neo-cl-letf-var-xyz)))
  (list during neo-cl-letf-var-xyz))"##,
    );
}

#[test]
fn push_pop_places() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(let ((al (list (cons 'k (list 1 2)))))
  (push 0 (cdr (assq 'k al)))
  (let ((p (pop (cdr (assq 'k al))))) (list p al)))"##,
    );
}

#[test]
fn setf_get_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(let ((sym 'neo-gv-sym-xyz))
  (setf (get sym 'p) 1) (cl-incf (get sym 'p) 9)
  (list (get sym 'p) (symbol-plist sym)))"##,
    );
}

#[test]
fn setf_gethash_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(let ((h (make-hash-table :test 'eq)))
  (setf (gethash 'k h) 10) (cl-incf (gethash 'k h) 5)
  (fset 'neo-tmp-fn-xyz (lambda () 1))
  (list (gethash 'k h) (functionp (symbol-function 'neo-tmp-fn-xyz))))"##,
    );
}

#[test]
fn setf_nested_places() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(require 'cl-lib)
(let ((data (list (list 1 2) (list 3 4))))
  (setf (caar data) 99) (setf (nth 1 (nth 1 data)) 88)
  (cl-rotatef (car (nth 0 data)) (car (nth 1 data)))
  data)"##,
    );
}

#[test]
fn delete_dups_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "  multiple   spaces  here  ")
  (goto-char (point-min))
  (while (re-search-forward " +" nil t) (replace-match " "))
  (string-trim (buffer-string)))"##,
    );
}

#[test]
fn fill_region_cols() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (setq fill-column 15)
  (insert "one two three four five six seven")
  (fill-region (point-min) (point-max))
  (buffer-string))"##,
    );
}

#[test]
fn indent_rigidly() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "line1\nline2\nline3")
  (indent-rigidly (point-min) (point-max) 4)
  (buffer-string))"##,
    );
}

#[test]
fn region_case_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "hello world foo")
  (upcase-region 1 6) (capitalize-region 7 12)
  (buffer-string))"##,
    );
}

#[test]
fn replace_buffer_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(let ((src (generate-new-buffer " neo-src-xxx")))
  (with-current-buffer src (insert "new content here"))
  (prog1 (with-temp-buffer (insert "old stuff")
           (replace-buffer-contents src) (buffer-string))
    (kill-buffer src)))"##,
    );
}

#[test]
fn subst_char_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "a-b-c-d")
  (subst-char-in-region 1 (point-max) ?- ?_)
  (buffer-string))"##,
    );
}
