//! Oracle parity tests for GNU `subr.el` `field-at-pos`.

use super::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn oracle_prop_gnu_field_at_pos_boundary_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU subr.el:field-at-pos reads the field at `field-beginning`; when that
    // raw field is `boundary`, it returns the field before `field-end`.
    let form = r#"
(with-temp-buffer
  (insert "aaXbb")
  (put-text-property 1 3 'field 'left)
  (put-text-property 3 4 'field 'boundary)
  (put-text-property 4 6 'field 'right)
  (list
   (mapcar #'field-at-pos (number-sequence 1 5))
   (mapcar (lambda (p)
             (list p
                   (field-beginning p)
                   (field-end p)
                   (get-char-property p 'field)))
           (number-sequence 1 5))))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_field_bounds_escape_and_limit_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "aaXbbZcc")
  (put-text-property 1 3 'field 'left)
  (put-text-property 3 4 'field 'boundary)
  (put-text-property 4 6 'field 'right)
  (put-text-property 7 9 'field 'tail)
  (list
   (mapcar (lambda (p)
             (list p
                   (field-beginning p)
                   (field-beginning p t)
                   (field-end p)
                   (field-end p t)))
           (number-sequence 1 9))
   (list
    (field-beginning 5 nil 4)
    (field-beginning 5 t 2)
    (field-end 2 nil 4)
    (field-end 3 t 5)
    (field-end 3 t 8))))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_field_string_and_delete_field_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "aaXbbZcc")
  (put-text-property 1 3 'field 'left)
  (put-text-property 1 3 'face 'bold)
  (put-text-property 3 4 'field 'boundary)
  (put-text-property 4 6 'field 'right)
  (put-text-property 7 9 'field 'tail)
  (let ((field-with-props (field-string 1))
        (field-no-props (field-string-no-properties 1))
        (delete-result (delete-field 3)))
    (list
     field-with-props
     (text-properties-at 0 field-with-props)
     field-no-props
     (text-properties-at 0 field-no-props)
     delete-result
     (buffer-string)
     (condition-case err
         (field-string 99)
       (error (list (car err) (cdr err)))))))
"#;
    assert_oracle_parity(form);
}

#[test]
fn oracle_constrain_to_field_boundary_motion_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(with-temp-buffer
  (insert "aaXbb\ncc")
  (put-text-property 1 3 'field 'left)
  (put-text-property 3 4 'field 'boundary)
  (put-text-property 4 6 'field 'right)
  (put-text-property 7 9 'field 'tail)
  (put-text-property 2 3 'capture t)
  (let ((inhibit-field-text-motion nil))
    (goto-char 8)
    (list
     (constrain-to-field 5 2)
     (constrain-to-field 5 2 t)
     (constrain-to-field 8 2 nil t)
     (constrain-to-field 8 2 nil nil)
     (constrain-to-field 5 2 nil nil 'capture)
     (let ((inhibit-field-text-motion t))
       (constrain-to-field 8 2))
     (list (constrain-to-field nil 2) (point)))))
"#;
    assert_oracle_parity(form);
}
