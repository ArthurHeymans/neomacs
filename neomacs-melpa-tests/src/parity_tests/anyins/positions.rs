use expect_test::expect;

use super::assert_anyins_parity;

#[test]
fn upstream_ert_record_position_keeps_first_occurrences_and_reports_duplicates() {
    let elisp_form = r##"(let ((anyins-buffers-positions nil))
  (list
   (mapcar
    #'anyins-record-position
    '((3 42) (2 32) (1 22) (1 45) (2 32) (2 26) (3 42) (1 45)))
   anyins-buffers-positions))"##;
    let expect = expect!["OK ((t t t t nil t nil nil) ((3 42) (2 32) (1 22) (1 45) (2 26)))"];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_ert_remove_positions_is_repeatable_and_keeps_buffer_local_state_empty() {
    let elisp_form = r##"(let ((anyins-buffers-positions '((3 3) (4 4) (5 5))))
  (list
   (copy-tree anyins-buffers-positions)
   (anyins-remove-positions)
   (copy-tree anyins-buffers-positions)
   (anyins-remove-positions)
   (copy-tree anyins-buffers-positions)))"##;
    let expect = expect!["OK (((3 3) (4 4) (5 5)) nil nil nil nil)"];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_ert_prepare_content_splits_multiline_single_line_empty_and_nil_inputs() {
    let elisp_form = r##"(mapcar
 #'anyins-prepare-content-to-insert
 '("hello world\nhello world\nhello world"
   "hello world"
   ""
   "\n"
   "alpha\n\nomega"
   nil))"##;
    let expect = expect![[
        r#"OK (("hello world" "hello world" "hello world") ("hello world") ("") ("" "") ("alpha" "" "omega") nil)"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_ert_compute_offsets_orders_unsorted_lines_and_accumulates_same_line_insertions() {
    let elisp_form = r##"(anyins-compute-position-offset
 '("Lorem ipsum dolor sit"
   "amet, consectetur adipiscing elit."
   "Vivamus non erat laoreet,"
   "tincidunt neque"
   "et, tempus"
   "nulla. Fusce"
   "iaculis eros.")
 '((1 2) (3 6) (2 3) (1 1) (2 1) (3 1) (2 2)))"##;
    let expect = expect![[
        r#"OK ((1 ((1 "tincidunt neque") (17 "Lorem ipsum dolor sit"))) (2 ((1 "et, tempus") (12 "iaculis eros.") (26 "Vivamus non erat laoreet,"))) (3 ((1 "nulla. Fusce") (18 "amet, consectetur adipiscing elit."))))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_ert_compute_offsets_ignores_rows_without_corresponding_positions() {
    let elisp_form = r##"(anyins-compute-position-offset
 '("Lorem ipsum dolor sit"
   "amet, consectetur adipiscing elit."
   "Vivamus non erat laoreet,"
   "tincidunt neque"
   "et, tempus"
   "nulla. Fusce"
   "iaculis eros.")
 '((1 2) (3 6) (2 3)))"##;
    let expect = expect![[
        r#"OK ((1 ((2 "Lorem ipsum dolor sit"))) (2 ((3 "Vivamus non erat laoreet,"))) (3 ((6 "amet, consectetur adipiscing elit."))))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_ert_compute_offsets_ignores_positions_without_corresponding_rows() {
    let elisp_form = r##"(anyins-compute-position-offset
 '("Lorem ipsum dolor sit"
   "amet, consectetur adipiscing elit."
   "Vivamus non erat laoreet,")
 '((1 2) (3 6) (2 3) (3 5) (10 8) (2 7)))"##;
    let expect = expect![[
        r#"OK ((1 ((2 "Lorem ipsum dolor sit"))) (2 ((3 "Vivamus non erat laoreet,"))) (3 ((6 "amet, consectetur adipiscing elit."))))"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn cursor_position_round_trip_uses_one_based_lines_and_character_offsets() {
    let elisp_form = r##"(with-temp-buffer
  (insert "zero\nαβγ delta\nlast")
  (goto-char (point-min))
  (forward-line 1)
  (forward-char 3)
  (let ((first (anyins-get-current-position)))
    (goto-char (point-max))
    (anyins-goto-position first)
    (list
     first
     (point)
     (char-after)
     (buffer-substring-no-properties
      (line-beginning-position)
      (line-end-position))
     (anyins-get-current-position))))"##;
    let expect = expect![[r#"OK ((2 3) 9 32 "αβγ delta" (2 3))"#]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn goto_or_create_position_reuses_existing_text_or_pads_short_lines_with_spaces() {
    let elisp_form = r##"(with-temp-buffer
  (insert "alphabet\nxy\n")
  (goto-char (point-min))
  (let ((existing-result (anyins-goto-or-create-position '(1 5)))
        existing-position
        padded-result
        padded-position)
    (setq existing-position
          (list (anyins-get-current-position) (char-after)))
    (setq padded-result (anyins-goto-or-create-position '(2 7)))
    (setq padded-position
          (list (anyins-get-current-position) (char-before) (char-after)))
    (list
     existing-result
     existing-position
     padded-result
     padded-position
     (buffer-string))))"##;
    let expect = expect![[r#"OK (6 ((1 5) 98) nil ((2 7) 32 10) "alphabet\nxy     \n")"#]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn recording_current_positions_creates_one_face_overlay_per_unique_location_then_deletes_all() {
    let elisp_form = r##"(with-temp-buffer
  (insert "alpha beta\ngamma delta\n")
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (goto-char (point-min))
    (search-forward "alpha")
    (let ((first-result (anyins-record-current-position))
          (duplicate-result (anyins-record-current-position)))
      (search-forward "gamma")
      (let ((second-result (anyins-record-current-position))
            before-delete)
        (setq before-delete
              (mapcar
               (lambda (overlay)
                 (list
                  (overlay-start overlay)
                  (overlay-end overlay)
                  (overlay-get overlay 'face)
                  (eq (overlay-buffer overlay) (current-buffer))))
               (reverse anyins-buffers-overlays)))
        (let ((delete-result (anyins-delete-overlays)))
          (list
           first-result
           duplicate-result
           second-result
           (copy-tree anyins-buffers-positions)
           before-delete
           delete-result
           anyins-buffers-overlays
           (overlays-in (point-min) (point-max))))))))"##;
    let expect = expect![
        "OK (#1=(#<overlay in no buffer>) nil (#<overlay in no buffer> . #1#) ((1 5) (2 5)) ((6 7 anyins-recorded-positions t) (17 18 anyins-recorded-positions t)) nil nil nil)"
    ];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn recorded_positions_and_overlays_are_isolated_between_live_buffers() {
    let elisp_form = r##"(let ((first (generate-new-buffer " *anyins-first*"))
      (second (generate-new-buffer " *anyins-second*")))
  (unwind-protect
      (progn
        (with-current-buffer first
          (insert "first")
          (goto-char 3)
          (anyins-record-current-position))
        (with-current-buffer second
          (insert "second")
          (goto-char 5)
          (anyins-record-current-position))
        (list
         (with-current-buffer first
           (list
            (copy-tree anyins-buffers-positions)
            (length anyins-buffers-overlays)
            (overlay-start (car anyins-buffers-overlays))))
         (with-current-buffer second
           (list
            (copy-tree anyins-buffers-positions)
            (length anyins-buffers-overlays)
            (overlay-start (car anyins-buffers-overlays))))
         (default-value 'anyins-buffers-positions)
         (default-value 'anyins-buffers-overlays)))
    (kill-buffer first)
    (kill-buffer second)))"##;
    let expect = expect!["OK ((((1 2)) 1 3) (((1 4)) 1 5) nil nil)"];
    assert_anyins_parity(elisp_form, expect);
}
