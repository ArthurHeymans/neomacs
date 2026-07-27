use expect_test::expect;

use super::assert_apt_sources_list_parity;

#[test]
fn upstream_invalid_workflow_reports_package_errors_for_type_and_uri_edits() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert "invalid")
  (goto-char (point-min))
  (mapcar
   (lambda (operation)
     (condition-case error
         (progn
           (funcall operation)
           'unexpected-success)
       (error
        (list
         (car error)
         (cdr error)
         (get (car error) 'error-conditions)
         (get (car error) 'error-message)))))
   (list
    #'apt-sources-list-change-type
    (lambda ()
      (apt-sources-list-change-uri
       "http://foo")))))"##;
    let expect = expect![[
        r#"OK ((apt-sources-list-not-found nil #1=(apt-sources-list-not-found error) "The point is not on an APT source line") (apt-sources-list-not-found nil #1# "The point is not on an APT source line"))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn upstream_insert_workflow_covers_default_named_exact_and_source_option_variants() {
    let elisp_form = r##"(list
 (with-temp-buffer
   (apt-sources-list-mode)
   (apt-sources-list-insert
    "https://deb.test")
   (buffer-string))
 (with-temp-buffer
   (apt-sources-list-mode)
   (apt-sources-list-insert
    "https://deb.test"
    :name "example")
   (buffer-string))
 (with-temp-buffer
   (apt-sources-list-mode)
   (apt-sources-list-insert
    "https://deb.test"
    :suite "path/")
   (buffer-string))
 (with-temp-buffer
   (apt-sources-list-mode)
   (apt-sources-list-insert
    "https://deb.test"
    :type "deb-src"
    :options "arch=amd64")
   (buffer-string)))"##;
    let expect = expect![[
        r##"OK ("deb https://deb.test stable main" "# example\ndeb https://deb.test stable main" "deb https://deb.test path/" "deb-src [arch=amd64] https://deb.test stable main")"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn upstream_replicate_workflow_copies_the_line_and_toggles_only_the_new_type() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb http://deb.test/debian stable main")
  (goto-char (point-min))
  (list
   (apt-sources-list-replicate)
   (buffer-string)
   (line-number-at-pos)
   (point)))"##;
    let expect = expect![[
        r#"OK (nil "deb http://deb.test/debian stable main\ndeb-src http://deb.test/debian stable main" 1 1)"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn upstream_change_type_workflow_round_trips_binary_and_source_types() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb http://deb.test/debian stable main")
  (goto-char (point-min))
  (apt-sources-list-change-type)
  (let ((source (buffer-string)))
    (apt-sources-list-change-type)
    (list source (buffer-string))))"##;
    let expect = expect![[
        r#"OK ("deb-src http://deb.test/debian stable main" "deb http://deb.test/debian stable main")"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn upstream_change_options_workflow_adds_extends_and_removes_the_option_block() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb http://deb.test/ stable main")
  (goto-char (point-min))
  (apt-sources-list-change-options
   "arch=amd64")
  (let ((added (buffer-string)))
    (apt-sources-list-change-options
     "arch=amd64 lang=en")
    (let ((extended (buffer-string)))
      (apt-sources-list-change-options "")
      (list
       added extended (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ("deb [arch=amd64] http://deb.test/ stable main" "deb [arch=amd64 lang=en] http://deb.test/ stable main" "deb http://deb.test/ stable main")"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn upstream_change_url_workflow_replaces_the_repository_location() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb http://deb.test/debian stable main")
  (goto-char (point-min))
  (apt-sources-list-change-uri
   "ftp://deb2.test/debian2")
  (buffer-string))"##;
    let expect = expect![[r#"OK "deb ftp://deb2.test/debian2 stable main""#]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn upstream_change_suite_workflow_moves_between_exact_and_regular_suites() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb http://deb.test/debian stable main # foo")
  (goto-char (point-min))
  (apt-sources-list-change-suite "path/")
  (let ((exact (buffer-string)))
    (apt-sources-list-change-suite
     "unstable" "xxx")
    (let ((unstable (buffer-string)))
      (apt-sources-list-change-suite "stable")
      (list exact unstable (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ("deb http://deb.test/debian path/ # foo" "deb http://deb.test/debian unstable xxx # foo" "deb http://deb.test/debian stable xxx # foo")"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn upstream_empty_suite_workflow_accepts_root_and_restores_components() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb http://deb.test/debian stable main # foo")
  (goto-char (point-min))
  (apt-sources-list-change-suite "/")
  (let ((root (buffer-string)))
    (apt-sources-list-change-suite
     "unstable" "xxx")
    (list root (buffer-string))))"##;
    let expect = expect![[
        r#"OK ("deb http://deb.test/debian / # foo" "deb http://deb.test/debian unstable xxx # foo")"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn upstream_change_components_workflow_accepts_space_separated_component_sets() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb http://deb.test/debian stable main # foo")
  (goto-char (point-min))
  (apt-sources-list-change-components "a b")
  (buffer-string))"##;
    let expect = expect![[r#"OK "deb http://deb.test/debian stable a b # foo""#]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn upstream_motion_workflow_moves_both_directions_and_reports_both_boundaries() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb [arch=armel] http://deb.test/debian stable main\n"
   "# comment\n"
   "deb invalid line\n"
   "deb http://deb.test/debian stable main")
  (goto-char (point-min))
  (let (events)
    (apt-sources-list-forward-source)
    (push (list 'forward (line-number-at-pos)) events)
    (apt-sources-list-forward-source -1)
    (push (list 'negative-forward
                (line-number-at-pos))
          events)
    (apt-sources-list-backward-source -1)
    (push (list 'negative-backward
                (line-number-at-pos))
          events)
    (apt-sources-list-backward-source)
    (push (list 'backward
                (line-number-at-pos))
          events)
    (dolist
        (operation
         (list
          (lambda ()
            (apt-sources-list-backward-source))
          (lambda ()
            (apt-sources-list-forward-source 2))))
      (push
       (condition-case error
           (progn
             (funcall operation)
             '(unexpected-success))
         (error
          (list 'boundary-error
                (car error)
                (cdr error))))
       events))
    (nreverse events)))"##;
    let expect = expect![[
        r#"OK ((forward 4) (negative-forward 1) (negative-backward 4) (backward 1) (boundary-error error ("No further repositories found buffer")) (boundary-error error ("No further repositories found buffer")))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn upstream_font_lock_workflow_assigns_fields_and_comment_faces_in_sequence() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (insert
   "deb [arch=armel] http://deb.test/debian stable main # bar")
  (goto-char (point-min))
  (font-lock-ensure)
  (mapcar
   (lambda (needle)
     (goto-char (point-min))
     (search-forward needle)
     (list
      needle
      (get-text-property
       (- (point) (length needle))
       'face)))
   '("deb" "arch" "http" "stable"
     "main" "#" "bar")))"##;
    let expect = expect![[
        r##"OK (("deb" apt-sources-list-type) ("arch" apt-sources-list-options) ("http" apt-sources-list-uri) ("stable" apt-sources-list-suite) ("main" apt-sources-list-components) ("#" font-lock-comment-delimiter-face) ("bar" font-lock-comment-face))"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}
