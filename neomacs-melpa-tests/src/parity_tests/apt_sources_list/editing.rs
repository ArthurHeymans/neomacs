use expect_test::expect;

use super::{assert_apt_sources_list_parity, assert_apt_sources_list_signal_parity};

#[test]
fn insert_uses_default_type_suite_component_and_inserts_at_the_exact_point() {
    let elisp_form = r##"(with-temp-buffer
  (insert "before\n\nafter")
  (goto-char (point-min))
  (forward-line)
  (let ((start (point)))
    (list
     (apt-sources-list-insert
      "https://deb.debian.org/debian")
     start
     (point)
     (buffer-string))))"##;
    let expect = expect![[
        r#"OK (nil 8 53 "before\ndeb https://deb.debian.org/debian stable main\nafter")"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn insert_renders_all_keyword_arguments_and_a_custom_named_source_comment() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apt-sources-list-name-format
         "# Repository %s"))
    (apt-sources-list-insert
     "https://packages.acme.example/debian"
     :name "Acme Production"
     :type "deb-src"
     :options "arch=amd64,arm64 signed-by=/keys/acme.gpg"
     :suite "bookworm-updates"
     :components "main contrib non-free-firmware")
    (list
     (buffer-string)
     (point)
     (apt-sources-list-source-p))))"##;
    let expect = expect![[
        r##"OK ("# Repository Acme Production\ndeb-src [arch=amd64,arm64 signed-by=/keys/acme.gpg] https://packages.acme.example/debian bookworm-updates main contrib non-free-firmware" 166 0)"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn insert_omits_components_for_root_and_nested_exact_suite_paths() {
    let elisp_form = r##"(mapcar
 (lambda (suite)
   (with-temp-buffer
     (apt-sources-list-insert
      "https://apt.example/repository"
      :suite suite
      :components "must-not-appear")
     (buffer-string)))
 '("/" "stable/updates/" "dists/sid/main/binary-amd64/"))"##;
    let expect = expect![[
        r#"OK ("deb https://apt.example/repository /" "deb https://apt.example/repository stable/updates/" "deb https://apt.example/repository dists/sid/main/binary-amd64/")"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn insert_uses_runtime_customized_first_suite_and_component_defaults() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apt-sources-list-suites
         '("trixie" "bookworm"))
        (apt-sources-list-components
         '("main non-free-firmware" "contrib")))
    (apt-sources-list-insert
     "https://mirror.example/debian"
     :name "Customized defaults")
    (buffer-string)))"##;
    let expect = expect![[
        r##"OK "# Customized defaults\ndeb https://mirror.example/debian trixie main non-free-firmware""##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn insert_preserves_explicit_empty_and_nil_keyword_values_as_lisp_formatting_does() {
    let elisp_form = r##"(mapcar
 (lambda (arguments)
   (with-temp-buffer
     (apply #'apt-sources-list-insert
            "https://mirror.example/debian"
            arguments)
     (buffer-string)))
 '((:name "" :options "" :suite "stable" :components "")
   (:name nil :options nil :suite "stable" :components nil)
   (:name nil :options nil :suite "/" :components nil)))"##;
    let expect = expect![[
        r##"OK ("# \ndeb [] https://mirror.example/debian stable " "deb https://mirror.example/debian stable nil" "deb https://mirror.example/debian /")"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn change_type_toggles_repeatedly_and_accepts_an_explicit_replacement_type() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "deb [arch=amd64] https://apt.example/debian stable main # enabled")
  (goto-char (point-min))
  (let ((start (point)))
    (apt-sources-list-change-type)
    (let ((first (buffer-string)))
      (apt-sources-list-change-type)
      (let ((second (buffer-string)))
        (apt-sources-list-change-type "deb-src")
        (list
         first second (buffer-string)
         start (point))))))"##;
    let expect = expect![[
        r#"OK ("deb-src [arch=amd64] https://apt.example/debian stable main # enabled" "deb [arch=amd64] https://apt.example/debian stable main # enabled" "deb-src [arch=amd64] https://apt.example/debian stable main # enabled" 1 1)"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn change_options_adds_replaces_and_removes_multi_option_blocks_without_touching_comments() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "deb https://apt.example/debian stable main # production")
  (goto-char (point-min))
  (apt-sources-list-change-options
   "arch=amd64 signed-by=/keys/acme.gpg")
  (let ((added (buffer-string)))
    (apt-sources-list-change-options
     "arch=arm64 trusted=yes")
    (let ((replaced (buffer-string)))
      (apt-sources-list-change-options "")
      (list added replaced (buffer-string)
            (point)))))"##;
    let expect = expect![[
        r#"OK ("deb [arch=amd64 signed-by=/keys/acme.gpg] https://apt.example/debian stable main # production" "deb [arch=arm64 trusted=yes] https://apt.example/debian stable main # production" "deb https://apt.example/debian stable main # production" 1)"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn change_uri_replaces_only_the_uri_across_options_components_and_comments() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "deb-src [arch=arm64] https://old.example/debian bookworm main contrib # source")
  (goto-char (point-min))
  (move-to-column 45)
  (let ((before (point)))
    (apt-sources-list-change-uri
     "ssh://mirror.internal/srv/debian")
    (list before (point) (buffer-string))))"##;
    let expect = expect![[
        r#"OK (46 22 "deb-src [arch=arm64] ssh://mirror.internal/srv/debian bookworm main contrib # source")"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn change_suite_handles_regular_exact_empty_and_default_component_transitions() {
    let elisp_form = r##"(list
 (with-temp-buffer
   (insert
    "deb https://apt.example/debian stable main contrib # a")
   (goto-char (point-min))
   (apt-sources-list-change-suite "path/")
   (buffer-string))
 (with-temp-buffer
   (insert "deb https://apt.example/debian / # b")
   (goto-char (point-min))
   (apt-sources-list-change-suite
    "unstable" "main non-free")
   (buffer-string))
 (with-temp-buffer
   (let ((apt-sources-list-components
          '("main contrib")))
     (insert "deb https://apt.example/debian /")
     (goto-char (point-min))
     (apt-sources-list-change-suite "testing")
     (buffer-string)))
 (with-temp-buffer
   (insert
    "deb https://apt.example/debian stable main contrib # d")
   (goto-char (point-min))
   (apt-sources-list-change-suite
    "bookworm" "ignored-default")
   (buffer-string)))"##;
    let expect = expect![[
        r#"OK ("deb https://apt.example/debian path/ # a" "deb https://apt.example/debian unstable main non-free # b" "deb https://apt.example/debian testing main contrib" "deb https://apt.example/debian bookworm main contrib # d")"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn change_components_replaces_complete_multi_component_sequences_and_preserves_suffixes() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "deb [arch=amd64] https://apt.example/debian stable main contrib # production")
  (goto-char (point-min))
  (apt-sources-list-change-components
   "main non-free non-free-firmware")
  (list
   (buffer-string)
   (apt-sources-list-source-p)
   (progn
     (apt-sources-list-match-source)
     (match-string-no-properties 5))))"##;
    let expect = expect![[
        r#"OK ("deb [arch=amd64] https://apt.example/debian stable main non-free non-free-firmware # production" 0 "main non-free non-free-firmware")"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn change_components_rejects_exact_suite_paths_with_the_package_specific_error() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "deb https://apt.example/repository dists/stable/main/binary-amd64/")
  (goto-char (point-min))
  (apt-sources-list-change-components "main"))"##;
    let expect = expect!["ERR (apt-sources-list-suite-component-mismatch)"];
    assert_apt_sources_list_signal_parity(elisp_form, expect);
}

#[test]
fn read_components_passes_custom_candidates_initial_value_and_a_space_enabled_minibuffer() {
    let elisp_form = r##"(let
    ((apt-sources-list-components
      '("main" "contrib" "non-free-firmware"))
     calls)
  (cl-letf
      (((symbol-function 'completing-read)
        (lambda (prompt collection predicate require-match
                 initial &rest arguments)
          (push
           (list prompt collection predicate require-match
                 initial arguments
                 (lookup-key
                  minibuffer-local-completion-map
                  (kbd "SPC")))
           calls)
          "main contrib")))
    (list
     (apt-sources-list--read-components
      "main non-free-firmware")
     (apt-sources-list--read-components)
     (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("main contrib" "main contrib" (("Components: " #1=("main" "contrib" "non-free-firmware") nil nil "main non-free-firmware" nil nil) ("Components: " #1# nil nil "main" nil nil)))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn interactive_insert_without_prefix_collects_name_uri_suite_and_multiple_components() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (let ((strings
         '("Corporate mirror"
           "https://apt.corp.example/debian"))
        (completions
         '("bookworm" "main contrib"))
        events
        current-prefix-arg)
    (cl-letf
        (((symbol-function 'read-string)
          (lambda (prompt &rest arguments)
            (push (list 'read-string prompt arguments) events)
            (pop strings)))
         ((symbol-function 'completing-read)
          (lambda (prompt collection &rest arguments)
            (push
             (list 'completing-read prompt collection arguments)
             events)
            (pop completions))))
      (list
       (call-interactively #'apt-sources-list-insert)
       (buffer-string)
       (nreverse events)))))"##;
    let expect = expect![[
        r##"OK (nil "# Corporate mirror\ndeb https://apt.corp.example/debian bookworm main contrib" ((read-string "Source name: " nil) (read-string "URI: " ("https://")) (completing-read "Suite: " ("stable" "testing" "unstable" "oldstable" "jessie" "stretch" "sid") (nil nil "stable")) (completing-read "Components: " ("main" "contrib" "non-free") (nil nil "main"))))"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn interactive_insert_with_prefix_collects_type_options_and_an_exact_suite() {
    let elisp_form = r##"(with-temp-buffer
  (apt-sources-list-mode)
  (let ((strings
         '("Source package mirror"
           "arch=amd64"
           "https://sources.example/debian"))
        (completions
         '("deb-src" "/"))
        events
        (current-prefix-arg '(4)))
    (cl-letf
        (((symbol-function 'read-string)
          (lambda (prompt &rest arguments)
            (push (list 'read-string prompt arguments) events)
            (pop strings)))
         ((symbol-function 'completing-read)
          (lambda (prompt collection &rest arguments)
            (push
             (list 'completing-read prompt collection arguments)
             events)
            (pop completions))))
      (list
       (call-interactively #'apt-sources-list-insert)
       (buffer-string)
       (nreverse events)))))"##;
    let expect = expect![[
        r##"OK (nil "# Source package mirror\ndeb-src [arch=amd64] https://sources.example/debian /" ((read-string "Source name: " nil) (completing-read "Type: " ("deb" "deb-src") (nil t "deb")) (read-string "Options: " nil) (read-string "URI: " ("https://")) (completing-read "Suite: " ("stable" "testing" "unstable" "oldstable" "jessie" "stretch" "sid") (nil nil "stable"))))"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}
