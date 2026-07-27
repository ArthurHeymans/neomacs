use expect_test::expect;

use super::assert_anyins_parity;

#[test]
fn recorded_position_insertion_orders_lines_offsets_and_content_independently_of_recording_order() {
    let elisp_form = r##"(with-temp-buffer
  (insert "one three five\nseven nine eleven\nthirteen fifteen\n")
  (let ((rows '(" two" " four" " six" " eight" " ten" " twelve" " fourteen"))
        (positions '((2 5) (1 3) (3 8) (1 9) (2 10) (1 14) (2 16))))
    (list
     (anyins-insert-at-recorded-positions rows positions)
     (buffer-string)
     (anyins-get-current-position))))"##;
    let expect = expect![[
        r#"OK (nil "one four three eight five twelve\nseven two nine ten eleve fourteenn\nthirteen six fifteen\n" (3 12))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn current_position_insertion_applies_one_row_to_each_remaining_line_at_the_same_offset() {
    let elisp_form = r##"(with-temp-buffer
  (insert "fruit\nfruit\nfruit\nfruit\n")
  (goto-char (point-min))
  (forward-char 5)
  (list
   (anyins-insert-from-current-position
    '(" is sweet" " is tart" " is ripe" " is seasonal"))
   (buffer-string)
   (anyins-get-current-position)))"##;
    let expect = expect![[
        r#"OK (nil "fruit is sweet\nfruit is tart\nfruit is ripe\nfruit is seasonal\n     " (5 5))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn current_position_insertion_pads_irregular_short_lines_before_inserting_columnar_data() {
    let elisp_form = r##"(with-temp-buffer
  (insert "category\nname\ncolor\nweight\n")
  (goto-char (point-min))
  (forward-char 8)
  (list
   (anyins-insert-from-current-position
    '(" : fruit" " : strawberry" " : red" " : 8"))
   (buffer-string)
   (mapcar
    (lambda (line)
      (goto-char (point-min))
      (forward-line (1- line))
      (anyins-get-current-position))
    '(1 2 3 4))))"##;
    let expect = expect![[
        r#"OK (nil "category : fruit\nname     : strawberry\ncolor    : red\nweight   : 8\n        " ((1 0) (2 0) (3 0) (4 0)))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn current_position_insertion_with_fewer_rows_leaves_remaining_lines_unchanged() {
    let elisp_form = r##"(with-temp-buffer
  (insert "fruit\nfruit\nfruit\nfruit\nfruit\nfruit\n")
  (goto-char (point-min))
  (list
   (anyins-insert-from-current-position '("1." "2." "3."))
   (buffer-string)
   (anyins-get-current-position)))"##;
    let expect = expect![[r#"OK (nil "1.fruit\n2.fruit\n3.fruit\nfruit\nfruit\nfruit\n" (7 0))"#]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn current_position_insertion_with_extra_rows_stops_at_the_existing_buffer_lines() {
    let elisp_form = r##"(with-temp-buffer
  (insert "fruit\nfruit\n")
  (goto-char (point-min))
  (list
   (anyins-insert-from-current-position
    '("1." "2." "3." "4." "5."))
   (buffer-string)
   (anyins-get-current-position)))"##;
    let expect = expect![[r#"OK (nil "1.fruit\n2.fruit\n3." (3 2))"#]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn top_level_insert_uses_recorded_positions_then_consumes_them_after_a_practical_edit() {
    let elisp_form = r##"(with-temp-buffer
  (insert "apple is a fruit\ncarrot is a vegetable\nstrawberry is a fruit\n")
  (let ((anyins-buffers-positions '((3 16) (1 10))))
    (list
     (anyins-insert " very good\n red and tasty\n unused")
     (buffer-string)
     anyins-buffers-positions
     (anyins-get-current-position))))"##;
    let expect = expect![[
        r#"OK (nil "apple is a red and tasty fruit\ncarrot is a vegetable\nstrawberry is a  very goodfruit\n" nil (3 26))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn top_level_insert_without_marks_uses_the_current_column_and_preserves_empty_rows() {
    let elisp_form = r##"(with-temp-buffer
  (insert "A\nB\nC\nD\n")
  (goto-char (point-min))
  (forward-char 1)
  (let ((anyins-buffers-positions nil))
    (list
     (anyins-insert " one\n\n three\n")
     (buffer-string)
     anyins-buffers-positions
     (anyins-get-current-position))))"##;
    let expect = expect![[r#"OK (nil "A one\nB\nC three\nD\n " nil (5 1))"#]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn nil_content_walks_remaining_lines_and_pads_the_final_empty_line_to_the_starting_column() {
    let elisp_form = r##"(with-temp-buffer
  (insert "alpha\nbeta\ngamma\n")
  (goto-char (point-min))
  (forward-char 2)
  (let ((anyins-buffers-positions nil))
    (list
     (anyins-insert nil)
     (buffer-string)
     (anyins-get-current-position))))"##;
    let expect = expect![[r#"OK (nil "alpha\nbeta\ngamma\n  " (4 2))"#]];
    assert_anyins_parity(elisp_form, expect);
}
