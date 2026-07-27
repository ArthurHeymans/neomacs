use expect_test::expect;

use super::assert_anyins_parity;

#[test]
fn upstream_kill_ring_feature_inserts_six_rows_vertically_from_the_current_column() {
    let elisp_form = r##"(with-temp-buffer
  (insert "a fruit\na fruit\na fruit\na fruit\na fruit\na fruit")
  (goto-char (point-min))
  (search-forward "a fruit")
  (let ((kill-ring
         '(" could be very good\n could be red and tasty\n could be spiky\n could be yellow\n could be tiny\n could be round and orange"))
        (anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil)))
      (anyins-mode 1)
      (list
       (anyins-yank)
       (buffer-string)
       anyins-mode
       buffer-read-only
       anyins-buffers-positions
       anyins-buffers-overlays))))"##;
    let expect = expect![[
        r#"OK (nil "a fruit could be very good\na fruit could be red and tasty\na fruit could be spiky\na fruit could be yellow\na fruit could be tiny\na fruit could be round and orange" nil nil nil nil)"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_kill_ring_feature_pads_irregular_lines_to_the_starting_column() {
    let elisp_form = r##"(with-temp-buffer
  (insert "category\nname\ncolor\nweight")
  (goto-char (point-min))
  (search-forward "category")
  (let ((kill-ring '(" : fruit\n : strawberry\n : red\n : 8"))
        (anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil)))
      (anyins-mode 1)
      (list
       (anyins-yank)
       (buffer-string)
       anyins-mode
       buffer-read-only))))"##;
    let expect = expect![[
        r#"OK (nil "category : fruit\nname     : strawberry\ncolor    : red\nweight   : 8" nil nil)"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_kill_ring_feature_inserts_rows_only_at_three_explicit_marks() {
    let elisp_form = r##"(with-temp-buffer
  (insert "apple is a fruit\ncarrot is a vegetable\nstrawberry is a fruit\ncauliflower is a vegetable\npineapple is a fruit")
  (let ((kill-ring '(" very good\n red and tasty\n spiky"))
        (anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil)))
      (anyins-mode 1)
      (goto-char (point-min))
      (search-forward "apple is a")
      (anyins-record-current-position)
      (search-forward "strawberry is a")
      (anyins-record-current-position)
      (search-forward "pineapple is a")
      (anyins-record-current-position)
      (list
       (anyins-yank)
       (buffer-string)
       anyins-mode
       buffer-read-only
       anyins-buffers-positions
       anyins-buffers-overlays
       (overlays-in (point-min) (point-max))))))"##;
    let expect = expect![[
        r#"OK (nil "apple is a very good fruit\ncarrot is a vegetable\nstrawberry is a red and tasty fruit\ncauliflower is a vegetable\npineapple is a spiky fruit" nil nil nil nil nil)"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_kill_ring_feature_handles_multiple_marks_on_each_line_with_cumulative_offsets() {
    let elisp_form = r##"(with-temp-buffer
  (insert "one three five\nseven nine eleven\nthirteen fifteen")
  (let ((kill-ring '(" two\n four\n six\n eight\n ten\n twelve\n fourteen"))
        (anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil)))
      (anyins-mode 1)
      (goto-char (point-min))
      (dolist (needle '("one" "three" "five" "seven" "nine" "eleven" "thirteen"))
        (search-forward needle)
        (anyins-record-current-position))
      (list
       (anyins-yank)
       (buffer-string)
       anyins-mode
       buffer-read-only
       anyins-buffers-positions
       anyins-buffers-overlays))))"##;
    let expect = expect![[
        r#"OK (nil "one two three four five six\nseven eight nine ten eleven twelve\nthirteen fourteen fifteen" nil nil nil nil)"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_shell_feature_inserts_the_first_six_command_rows_into_six_buffer_lines() {
    let elisp_form = r##"(with-temp-buffer
  (insert "fruit\nfruit\nfruit\nfruit\nfruit\nfruit")
  (goto-char (point-min))
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil)
        captured-command)
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil))
              ((symbol-function 'shell-command-to-string)
               (lambda (command)
                 (setq captured-command command)
                 "1.\n2.\n3.\n4.\n5.\n6.\n7.\n8.\n9.\n10.\n")))
      (anyins-mode 1)
      (list
       (anyins-insert-command "seq 1 10|xargs -I {} echo '{}.'")
       captured-command
       (buffer-string)
       anyins-mode
       buffer-read-only))))"##;
    let expect = expect![[
        r#"OK (nil "seq 1 10|xargs -I {} echo '{}.'" "1.fruit\n2.fruit\n3.fruit\n4.fruit\n5.fruit\n6.fruit" nil nil)"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_shell_feature_with_three_rows_leaves_the_last_three_lines_untouched() {
    let elisp_form = r##"(with-temp-buffer
  (insert "fruit\nfruit\nfruit\nfruit\nfruit\nfruit")
  (goto-char (point-min))
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil))
              ((symbol-function 'shell-command-to-string)
               (lambda (_command) "1.\n2.\n3.\n")))
      (anyins-mode 1)
      (list
       (anyins-insert-command "three rows")
       (buffer-string)
       anyins-mode
       buffer-read-only))))"##;
    let expect = expect![[r#"OK (nil "1.fruit\n2.fruit\n3.fruit\nfruit\nfruit\nfruit" nil nil)"#]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_shell_feature_with_extra_rows_stops_after_the_two_existing_lines() {
    let elisp_form = r##"(with-temp-buffer
  (insert "fruit\nfruit")
  (goto-char (point-min))
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil))
              ((symbol-function 'shell-command-to-string)
               (lambda (_command) "1.\n2.\n3.\n4.\n5.\n")))
      (anyins-mode 1)
      (list
       (anyins-insert-command "five rows")
       (buffer-string)
       anyins-mode
       buffer-read-only))))"##;
    let expect = expect![[r#"OK (nil "1.fruit\n2.fruit" nil nil)"#]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_shell_feature_inserts_country_labels_at_five_explicit_address_marks() {
    let elisp_form = r##"(with-temp-buffer
  (insert "498-1686 Maecenas St.Gabon\nAp #252-3643 Odio Av. Cook Islands\nAp #666-7930 Risus. Street Niue\n6998 Accumsan Avenue Zambia\n205-1886 Eu Rd. United States Minor Outlying Islands")
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil))
              ((symbol-function 'shell-command-to-string)
               (lambda (_command)
                 "| Country : \n| Country : \n| Country : \n| Country : \n| Country : \n")))
      (anyins-mode 1)
      (goto-char (point-min))
      (dolist (needle '("St.Gabon" "Cook Islands" "Niue" "Zambia" "United States Minor Outlying Islands"))
        (search-forward needle)
        (goto-char (match-beginning 0))
        (anyins-record-current-position)
        (goto-char (match-end 0)))
      (list
       (anyins-insert-command "yes '| Country : '|head -n 10")
       (buffer-string)
       anyins-mode
       buffer-read-only
       anyins-buffers-positions
       anyins-buffers-overlays))))"##;
    let expect = expect![[
        r#"OK (nil "498-1686 Maecenas | Country : St.Gabon\nAp #252-3643 Odio Av. | Country : Cook Islands\nAp #666-7930 Risus. Street | Country : Niue\n6998 Accumsan Avenue | Country : Zambia\n205-1886 Eu Rd. | Country : United States Minor Outlying Islands" nil nil nil nil)"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}

#[test]
fn upstream_shell_feature_numbers_nine_words_across_three_lines_at_explicit_marks() {
    let elisp_form = r##"(with-temp-buffer
  (insert "one two three\nfour five six\nseven eight nine")
  (let ((anyins-buffers-positions nil)
        (anyins-buffers-overlays nil))
    (cl-letf (((symbol-function 'message) (lambda (&rest _arguments) nil))
              ((symbol-function 'shell-command-to-string)
               (lambda (_command)
                 "1.\n2.\n3.\n4.\n5.\n6.\n7.\n8.\n9.\n")))
      (anyins-mode 1)
      (goto-char (point-min))
      (dolist (needle '("one" "two" "three" "four" "five" "six" "seven" "eight" "nine"))
        (search-forward needle)
        (goto-char (match-beginning 0))
        (anyins-record-current-position)
        (goto-char (match-end 0)))
      (list
       (anyins-insert-command "seq 1 9")
       (buffer-string)
       anyins-mode
       buffer-read-only
       anyins-buffers-positions
       anyins-buffers-overlays))))"##;
    let expect = expect![[
        r#"OK (nil "1.one 2.two 3.three\n4.four 5.five 6.six\n7.seven 8.eight 9.nine" nil nil nil nil)"#
    ]];
    assert_anyins_parity(elisp_form, expect);
}
