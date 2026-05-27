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

#[test]
fn org_table_remote_named_range_hline_total_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (with-temp-buffer
    (org-mode)
    (insert "#+NAME: rates\n")
    (insert "| Kind | Rate |\n")
    (insert "|------+------|\n")
    (insert "| base | 2 |\n")
    (insert "| rush | 5 |\n\n")
    (insert "#+NAME: jobs\n")
    (insert "| Job | Hours | Fee | Total |\n")
    (insert "|-----+-------+-----+-------|\n")
    (insert "| A | 3 | 1 | |\n")
    (insert "| B | 4 | 2 | |\n")
    (insert "| Sum | | | |\n")
    (insert "#+TBLFM: @2$4=$2*remote(rates,@2$2)+$3::@3$4=$2*remote(rates,@3$2)+$3::@4$4=vsum(@2$4..@3$4)\n")
    (goto-char (point-min))
    (search-forward "#+NAME: jobs")
    (org-table-recalculate-buffer-tables)
    (goto-char (point-min))
    (search-forward "#+NAME: jobs")
    (list (org-table-get-remote-range "rates" "@2$2")
          (org-table-get-remote-range "jobs" "@2$2..@3$4")
          (buffer-substring-no-properties
           (point-min) (point-max)))))"##,
    );
}

#[test]
fn org_table_formula_reference_conversion_and_current_field_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (with-temp-buffer
    (org-mode)
    (insert "| Item | Qty | Price | Line |\n")
    (insert "|------+-----+-------+------|\n")
    (insert "| one | 2 | 10 | |\n")
    (insert "| two | 3 | 7 | |\n")
    (insert "| ! | | | Total |\n")
    (insert "#+TBLFM: $Line=$Qty*$Price;%.1f::@>$Line=vsum(@2..@-1);%.2f\n")
    (goto-char (point-min))
    (org-table-recalculate-buffer-tables)
    (goto-char (point-min))
    (search-forward "10")
    (org-table-next-field)
    (org-table-analyze)
    (let ((field-formula (org-table-current-field-formula 'key 'noerror))
          (current-line (org-table-current-line))
          (current-column (org-table-current-column)))
      (list field-formula
            current-line
            current-column
            (org-table-convert-refs-to-rc "B3..D4")
            (org-table-convert-refs-to-an "@3$2..@4$4")
            (org-table-formula-substitute-names "$Line=$Qty*$Price")
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_table_rectangle_cut_paste_sum_wrap_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (with-temp-buffer
    (org-mode)
    (insert "| Name | Q1 | Q2 | Q3 | Note |\n")
    (insert "|------+----+----+----+------|\n")
    (insert "| a | 1 | 2 | 3 | alpha beta gamma |\n")
    (insert "| b | 4 | 5 | 6 | delta epsilon |\n")
    (insert "| c | 7 | 8 | 9 | zeta eta |\n")
    (goto-char (point-min))
    (search-forward "2")
    (let ((beg (point)))
      (search-forward "8")
      (let* ((clip (org-table-copy-region beg (point) t))
             (after-cut (buffer-substring-no-properties
                         (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "delta")
        (org-table-paste-rectangle)
        (let ((after-paste (buffer-substring-no-properties
                            (point-min) (point-max))))
          (goto-char (point-min))
          (search-forward "Q2")
          (let ((sum (org-table-sum nil nil 3)))
            (goto-char (point-min))
            (search-forward "alpha")
            (search-forward "beta")
            (org-table-wrap-region nil)
            (list clip
                  after-cut
                  after-paste
                  sum
                  (current-kill 0 t)
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}
