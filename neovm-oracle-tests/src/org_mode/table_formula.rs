use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_table_named_formula_constants_recalc_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "#+CONSTANTS: tax=2 fee=3\n")
    (insert "| Item | Net | Tax | Total |\n")
    (insert "|------+-----+-----+-------|\n")
    (insert "| a | 10 |  |  |\n")
    (insert "| b | 20 |  |  |\n")
    (insert "#+TBLFM: $Tax=$Net*$tax::$Total=$Net+$Tax+$fee\n")
    (goto-char (point-min))
    (org-table-recalculate-buffer-tables)
    (goto-char (point-min))
    (search-forward "a")
    (list (org-table-formula-substitute-names "$Net*$tax+$fee")
          (buffer-substring-no-properties
           (point-min) (point-max)))))"##,
    );
}

#[test]
fn org_table_iterate_dependency_formula_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "| n | prev | next |\n")
    (insert "|---+------+------|\n")
    (insert "| 1 | 0 |  |\n")
    (insert "| 2 |  |  |\n")
    (insert "| 3 |  |  |\n")
    (insert "#+TBLFM: @3$2=@2$3::@4$2=@3$3::$3=$1+$2\n")
    (goto-char (point-min))
    (let ((org-table-iterate-max 5))
      (org-table-iterate))
    (list org-table-iterate-max
          (buffer-substring-no-properties
           (point-min) (point-max)))))"##,
    );
}

#[test]
fn org_table_field_replace_and_formula_rewrite_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "| A | B | C |\n")
    (insert "|---+---+---|\n")
    (insert "| 1 | 2 |  |\n")
    (insert "| 3 | 4 |  |\n")
    (insert "#+TBLFM: $3=$1+$2\n")
    (goto-char (point-min))
    (search-forward "2")
    (let ((old (org-table-get-field nil "5")))
      (org-table-align)
      (org-table-recalculate-buffer-tables)
      (let ((after-recalc
             (buffer-substring-no-properties
              (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "B")
        (org-table-delete-column)
        (list old
              after-recalc
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}
