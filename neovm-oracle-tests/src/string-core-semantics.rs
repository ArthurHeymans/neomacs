//! Oracle parity tests for GNU core string and sequence construction semantics.
//!
//! GNU implements `string-equal`, `string-lessp`, `concat`, `vconcat`,
//! `copy-sequence`, `substring`, and `substring-no-properties` in `src/fns.c`.
//! These tests focus on symbol coercion, text-property behavior, negative
//! subarray validation, vector substrings, and character-sequence validation.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_string_comparison_symbol_coercion_and_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (string-equal 'alpha "alpha")
 (string= 'alpha 'alpha)
 (string-equal (propertize "alpha" 'face 'bold) "alpha")
 (string-lessp 'alpha "beta")
 (string-lessp "beta" 'alpha)
 (string< 'alpha 'beta)
 (condition-case err
     (string-equal 42 "42")
   (error (list (car err) (cdr err))))
 (condition-case err
     (string-lessp "x" 42)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_substring_properties_and_no_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((s (propertize "abcdef" 'face 'bold 'tag 'source))
       (sub (substring s 1 5))
       (plain (substring-no-properties s 1 5)))
  (list
   sub
   (get-text-property 0 'face sub)
   (get-text-property 3 'tag sub)
   (text-properties-at 0 sub)
   plain
   (text-properties-at 0 plain)
   (equal-including-properties sub plain)
   (string= sub plain)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_substring_vector_negative_and_error_payloads() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (substring [a b c d e] -4 -1)
 (substring [a b c d e] 2 nil)
 (substring "aébcd" -4 -1)
 (condition-case err
     (substring [a b c] 'bad 2)
   (error (list (car err) (cdr err))))
 (condition-case err
     (substring [a b c] 0 4)
   (error (list (car err) (cdr err))))
 (condition-case err
     (substring-no-properties [a b c] 0 1)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_substring_rejects_record_without_crashing_like_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU fns.c:Fsubstring starts with CHECK_VECTOR_OR_STRING, so records are
    // rejected with `arrayp` and must not be treated as vector storage.
    let form = r#"
(condition-case err
    (substring (record 'a 1 2) 0 1)
  (error (list (car err) (cdr err))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_substring_rejects_bool_vector_like_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU Emacs src/fns.c:Fsubstring calls CHECK_VECTOR_OR_STRING, whose
    // src/lisp.h definition accepts only VECTORP and STRINGP.  Bool-vectors
    // are arrays for `aref`, but not valid `substring` inputs.
    let form = r#"
(condition-case err
    (substring (bool-vector t nil t) 0 2)
  (error (list (car err) (cdr err))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_substring_rejects_char_table_like_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU fns.c:Fsubstring uses CHECK_VECTOR_OR_STRING; char-tables are
    // rejected here even though `copy-sequence` has a char-table-specific path.
    let form = r#"
(condition-case err
    (substring (make-char-table 'generic 65) 0 1)
  (error (list (car err) (cdr err))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_substring_no_properties_rejects_vectorlike_objects_like_gnu() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU fns.c:Fsubstring_no_properties uses CHECK_STRING, unlike
    // Fsubstring's CHECK_VECTOR_OR_STRING gate.  Vectorlike objects must signal
    // `stringp` here rather than being treated as arrays.
    let form = r#"
(list
 (condition-case err
     (substring-no-properties (make-char-table 'generic 65) 0 1)
   (error (list (car err) (cdr err))))
 (condition-case err
     (substring-no-properties (make-bool-vector 3 t) 0 1)
   (error (list (car err) (cdr err))))
 (condition-case err
     (substring-no-properties (record 'tag 1 2) 0 1)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_concat_and_vconcat_character_sequence_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((s (propertize "ab" 'face 'bold))
       (joined (concat s '(?c ?d) [?e ?f]))
       (vec (vconcat "ab" '(c d) [e f])))
  (list
   joined
   (get-text-property 0 'face joined)
   (get-text-property 1 'face joined)
   (get-text-property 2 'face joined)
   vec
   (condition-case err
       (concat '(?a bad ?c))
     (error (list (car err) (cdr err))))
   (condition-case err
       (concat [65 4194304])
     (error (list (car err) (cdr err))))
   (condition-case err
       (vconcat 42)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_copy_sequence_text_properties_and_shallow_copy() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((cell (list 'shared))
       (lst (list cell))
       (lst-copy (copy-sequence lst))
       (str (propertize "abc" 'face 'bold))
       (str-copy (copy-sequence str))
       (vec (vector cell))
       (vec-copy (copy-sequence vec)))
  (setcar cell 'changed)
  (list
   (eq lst lst-copy)
   (eq (car lst) (car lst-copy))
   lst-copy
   (eq str str-copy)
   str-copy
   (text-properties-at 0 str-copy)
   (eq vec vec-copy)
   (eq (aref vec 0) (aref vec-copy 0))
   vec-copy
   (copy-sequence nil)
   (condition-case err
       (copy-sequence 42)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_copy_sequence_vectorlike_type_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU fns.c:Fcopy_sequence has explicit record, char-table, and
    // bool-vector branches, but no closure branch.
    let form = r#"
(let ((bv (make-bool-vector 3 t))
      (rec (record 'tag 1 2))
      (table (make-char-table 'generic 65)))
  (list
   (bool-vector-p (copy-sequence bv))
   (equal bv (copy-sequence bv))
   (recordp (copy-sequence rec))
   (equal rec (copy-sequence rec))
   (char-table-p (copy-sequence table))
   (char-table-range (copy-sequence table) ?A)
   (condition-case err
       (copy-sequence (lambda (x) x))
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_copy_sequence_circular_and_improper_list_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU src/fns.c:Fcopy_sequence copies conses with FOR_EACH_TAIL and
    // then CHECK_LIST_END.  Circular data is normalized here by probing the
    // signaled cycle tail instead of printing the circular object directly.
    let form = r#"
(list
 (condition-case err
     (copy-sequence '(a b . c))
   (wrong-type-argument (list (car err) (cdr err))))
 (let ((c (list 1 2 3)))
   (setcdr (last c) c)
   (condition-case err
       (copy-sequence c)
     (circular-list
      (list (car err)
            (consp (cadr err))
            (safe-length (cadr err))
            (car (cadr err))))))
 (let ((l (list 'p0 'p1 'c0 'c1)))
   (setcdr (last l) (nthcdr 2 l))
   (condition-case err
       (copy-sequence l)
     (circular-list
      (list (car err)
            (consp (cadr err))
            (safe-length (cadr err))
            (car (cadr err)))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
