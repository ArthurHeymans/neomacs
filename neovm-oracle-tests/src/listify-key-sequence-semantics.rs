//! Oracle parity tests for GNU `subr.el' `listify-key-sequence' semantics.

use super::common::assert_oracle_parity_with_bootstrap;

#[test]
fn oracle_listify_key_sequence_vector_appends_elements_verbatim() {
    let form = r#"
(let* ((event (list 'mouse-1 '(posn-placeholder)))
       (nested [nested])
       (listed (listify-key-sequence (vector ?a 'f1 event nested))))
  (list
   listed
   (eq (nth 2 listed) event)
   (eq (nth 3 listed) nested)))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_listify_key_sequence_string_decodes_high_bit_events() {
    let form = r#"
(list
 (listify-key-sequence "Az")
 (listify-key-sequence (unibyte-string 225 129))
 (mapcar #'single-key-description
         (listify-key-sequence (unibyte-string 225 129))))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_string_rejects_modified_character_events() {
    let form = r#"
(list
 (condition-case err (string ?\M-a) (error (car err)))
 (condition-case err (string ?\M-\C-a) (error (car err)))
 (condition-case err (string (event-convert-list '(meta ?a))) (error (car err))))"#;
    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_listify_key_sequence_rejects_non_sequences_like_gnu() {
    let form = r#"
(list
 (condition-case err (listify-key-sequence nil) (error (car err)))
 (condition-case err (listify-key-sequence 'mouse-1) (error (car err)))
 (condition-case err (listify-key-sequence 123) (error (car err))))"#;
    assert_oracle_parity_with_bootstrap(form);
}
