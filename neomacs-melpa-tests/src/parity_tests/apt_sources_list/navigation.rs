use expect_test::expect;

use super::{assert_apt_sources_list_parity, assert_apt_sources_list_signal_parity};

#[test]
fn forward_and_backward_navigation_skip_comments_blanks_and_malformed_repository_lines() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "deb [arch=armel] http://deb.test/debian stable main\n"
   "# comment\n"
   "\n"
   "deb invalid line\n"
   "deb http://deb.test/debian testing main\n"
   "deb-src https://sources.test/debian unstable main\n")
  (goto-char (point-min))
  (let (visits)
    (apt-sources-list-forward-source)
    (push
     (list (line-number-at-pos) (point)
           (thing-at-point 'line t))
     visits)
    (apt-sources-list-forward-source)
    (push
     (list (line-number-at-pos) (point)
           (thing-at-point 'line t))
     visits)
    (apt-sources-list-backward-source)
    (push
     (list (line-number-at-pos) (point)
           (thing-at-point 'line t))
     visits)
    (apt-sources-list-backward-source)
    (push
     (list (line-number-at-pos) (point)
           (thing-at-point 'line t))
     visits)
    (nreverse visits)))"##;
    let expect = expect![[
        r#"OK ((5 81 "deb http://deb.test/debian testing main\n") (6 121 "deb-src https://sources.test/debian unstable main\n") (5 81 "deb http://deb.test/debian testing main\n") (1 1 "deb [arch=armel] http://deb.test/debian stable main\n"))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn navigation_counts_multiple_sources_and_treats_negative_counts_as_the_opposite_direction() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "deb https://one.example/debian stable main\n"
   "deb https://two.example/debian stable main\n"
   "deb https://three.example/debian stable main\n"
   "deb https://four.example/debian stable main\n")
  (goto-char (point-min))
  (let (results)
    (apt-sources-list-forward-source 2)
    (push
     (list 'forward-2 (line-number-at-pos) (point))
     results)
    (apt-sources-list-forward-source -1)
    (push
     (list 'forward-minus-1
           (line-number-at-pos) (point))
     results)
    (apt-sources-list-backward-source -2)
    (push
     (list 'backward-minus-2
           (line-number-at-pos) (point))
     results)
    (apt-sources-list-backward-source 2)
    (push
     (list 'backward-2
           (line-number-at-pos) (point))
     results)
    (nreverse results)))"##;
    let expect = expect![
        "OK ((forward-2 3 87) (forward-minus-1 2 44) (backward-minus-2 4 132) (backward-2 2 44))"
    ];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn navigation_lands_at_match_start_and_retains_target_match_groups() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "  deb https://one.example/debian stable main\n"
   "# ignored\n"
   "\tdeb-src [arch=arm64] https://two.example/debian testing main contrib # source\n")
  (goto-char (point-min))
  (apt-sources-list-forward-source)
  (list
   (point)
   (line-number-at-pos)
   (current-column)
   (match-beginning 0)
   (match-end 0)
   (mapcar
    (lambda (index)
      (match-string-no-properties index))
    '(1 2 3 4 5))))"##;
    let expect = expect![[
        r#"OK (56 3 0 56 127 ("deb-src" "arch=arm64" "https://two.example/debian" "testing main contrib" "main contrib"))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn forward_navigation_signals_the_exact_boundary_error_when_count_exceeds_sources() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "deb https://one.example/debian stable main\n"
   "# no more repositories\n")
  (goto-char (point-min))
  (apt-sources-list-forward-source 2))"##;
    let expect = expect![[r#"ERR (error "No further repositories found buffer")"#]];
    assert_apt_sources_list_signal_parity(elisp_form, expect);
}

#[test]
fn backward_navigation_signals_the_exact_boundary_error_before_the_first_source() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "deb https://one.example/debian stable main\n"
   "deb https://two.example/debian testing main\n")
  (goto-char (point-min))
  (apt-sources-list-backward-source))"##;
    let expect = expect![[r#"ERR (error "No further repositories found buffer")"#]];
    assert_apt_sources_list_signal_parity(elisp_form, expect);
}
