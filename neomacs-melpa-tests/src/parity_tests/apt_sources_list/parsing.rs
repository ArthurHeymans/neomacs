use expect_test::expect;

use super::{assert_apt_sources_list_parity, assert_apt_sources_list_signal_parity};

#[test]
fn parser_extracts_every_field_from_realistic_binary_and_source_repositories() {
    let elisp_form = r##"(mapcar
 (lambda (line)
   (with-temp-buffer
     (insert line)
     (goto-char (point-min))
     (apt-sources-list-mode)
     (list
      (apt-sources-list-source-p)
      (progn
        (apt-sources-list-match-source)
        (mapcar
         (lambda (index)
           (match-string-no-properties index))
         '(0 1 2 3 4 5))))))
 '("deb http://deb.debian.org/debian stable main contrib"
   "deb-src [arch=amd64 signed-by=/usr/share/keyrings/vendor.gpg] https://packages.example/v1 bookworm-updates main non-free-firmware # production"
   "  deb file:/srv/apt-mirror testing main\t# local mirror"))"##;
    let expect = expect![[
        r#"OK ((0 ("deb http://deb.debian.org/debian stable main contrib" "deb" nil "http://deb.debian.org/debian" "stable main contrib" "main contrib")) (0 ("deb-src [arch=amd64 signed-by=/usr/share/keyrings/vendor.gpg] https://packages.example/v1 bookworm-updates main non-free-firmware #" "deb-src" "arch=amd64 signed-by=/usr/share/keyrings/vendor.gpg" "https://packages.example/v1" "bookworm-updates main non-free-firmware" "main non-free-firmware")) (0 ("  deb file:/srv/apt-mirror testing main\11#" "deb" nil "file:/srv/apt-mirror" "testing main" "main")))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn parser_handles_empty_and_nested_exact_suite_paths_without_components() {
    let elisp_form = r##"(mapcar
 (lambda (line)
   (with-temp-buffer
     (insert line)
     (goto-char (point-min))
     (apt-sources-list-match-source)
     (list
      (match-string-no-properties 1)
      (match-string-no-properties 3)
      (match-string-no-properties 4)
      (match-string-no-properties 5))))
 '("deb https://dl.bintray.com/sbt/debian /"
   "deb https://repo.example/releases stable/updates/"
   "deb-src [trusted=yes] ssh://mirror.example/debian dists/sid/main/binary-amd64/ # exact"))"##;
    let expect = expect![[
        r#"OK (("deb" "https://dl.bintray.com/sbt/debian" "/" nil) ("deb" "https://repo.example/releases" "stable/updates/" nil) ("deb-src" "ssh://mirror.example/debian" "dists/sid/main/binary-amd64/" nil))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn parser_preserves_complex_option_payloads_uri_schemes_and_component_sequences() {
    let elisp_form = r##"(mapcar
 (lambda (line)
   (when (string-match apt-sources-list-one-line line)
     (mapcar
      (lambda (index)
        (match-string-no-properties index line))
      '(1 2 3 4 5))))
 '("deb [arch=amd64,arm64 allow-insecure=yes] mirror.file:/srv/repo sid main contrib non-free"
   "deb-src [signed-by=/keys/acme.gpg] tor-https://apt.acme.example:8443/debian trixie main"
   "deb ftp://ftp.example.org/pub/debian oldstable main"
   "deb copy:/media/debian bookworm main"
   "deb https://packages.example stable/main component"))"##;
    let expect = expect![[
        r#"OK (("deb" "arch=amd64,arm64 allow-insecure=yes" "mirror.file:/srv/repo" "sid main contrib non-free" "main contrib non-free") ("deb-src" "signed-by=/keys/acme.gpg" "tor-https://apt.acme.example:8443/debian" "trixie main" "main") ("deb" nil "ftp://ftp.example.org/pub/debian" "oldstable main" "main") ("deb" nil "copy:/media/debian" "bookworm main" "main") ("deb" nil "https://packages.example" "stable/main component" "component"))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn parser_rejects_malformed_types_options_uris_suites_and_missing_components() {
    let elisp_form = r##"(mapcar
 (lambda (line)
   (list line
         (string-match-p
          apt-sources-list-one-line line)))
 '(""
   "# deb https://deb.debian.org/debian stable main"
   "rpm https://packages.example stable main"
   "deb [] https://packages.example stable main"
   "deb [arch=amd64 https://packages.example stable main"
   "deb packages.example/debian stable main"
   "deb mirror+file:/srv/repo stable main"
   "deb https://packages.example stable"
   "prefix deb https://packages.example stable main"))"##;
    let expect = expect![[
        r##"OK (("" nil) ("# deb https://deb.debian.org/debian stable main" nil) ("rpm https://packages.example stable main" nil) ("deb [] https://packages.example stable main" nil) ("deb [arch=amd64 https://packages.example stable main" nil) ("deb packages.example/debian stable main" nil) ("deb mirror+file:/srv/repo stable main" nil) ("deb https://packages.example stable" nil) ("prefix deb https://packages.example stable main" nil))"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn source_predicate_tracks_the_current_line_across_comments_blanks_and_invalid_entries() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "# production repositories\n"
   "deb https://deb.example/debian stable main\n"
   "\n"
   "deb malformed line\n"
   "deb-src [arch=arm64] https://deb.example/debian stable main\n"
   "deb https://exact.example/repo /\n")
  (goto-char (point-min))
  (let (results)
    (dotimes (_ 6)
      (push
       (list
        (line-number-at-pos)
        (apt-sources-list-source-p)
        (buffer-substring-no-properties
         (line-beginning-position)
         (line-end-position)))
       results)
      (forward-line))
    (nreverse results)))"##;
    let expect = expect![[
        r##"OK ((1 nil "# production repositories") (2 0 "deb https://deb.example/debian stable main") (3 nil "") (4 nil "deb malformed line") (5 0 "deb-src [arch=arm64] https://deb.example/debian stable main") (6 0 "deb https://exact.example/repo /"))"##
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn match_source_fills_all_match_groups_without_moving_point_or_mark() {
    let elisp_form = r##"(with-temp-buffer
  (insert
   "header\n"
   "  deb [arch=amd64] https://apt.example/debian stable main contrib # enabled\n"
   "footer")
  (goto-char (point-min))
  (forward-line)
  (move-to-column 18)
  (push-mark (point-max) t t)
  (let ((before-point (point))
        (before-mark (mark)))
    (apt-sources-list-match-source)
    (list
     before-point (point)
     before-mark (mark)
     (match-beginning 0)
     (match-end 0)
     (mapcar
      (lambda (index)
        (match-string-no-properties index))
      '(0 1 2 3 4 5)))))"##;
    let expect = expect![[
        r#"OK (26 26 90 90 8 75 ("  deb [arch=amd64] https://apt.example/debian stable main contrib #" "deb" "arch=amd64" "https://apt.example/debian" "stable main contrib" "main contrib"))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}

#[test]
fn match_source_signals_the_package_specific_error_on_a_non_source_line() {
    let elisp_form = r##"(with-temp-buffer
  (insert "deb missing-fields")
  (goto-char (point-min))
  (apt-sources-list-match-source))"##;
    let expect = expect!["ERR (apt-sources-list-not-found)"];
    assert_apt_sources_list_signal_parity(elisp_form, expect);
}

#[test]
fn parser_accepts_leading_and_trailing_space_and_stops_before_inline_comments() {
    let elisp_form = r##"(mapcar
 (lambda (line)
   (let ((matched
          (string-match apt-sources-list-one-line line)))
     (list
      line matched
      (and matched (match-string 0 line))
      (and matched (match-string 4 line))
      (and matched (match-string 5 line)))))
 '("\tdeb https://apt.example/debian stable main\t"
   " deb-src https://apt.example/debian testing main contrib#comment"
   "deb https://apt.example/debian stable main   # comment with spaces"
   "deb https://apt.example/debian /#exact comment"
   "x deb https://apt.example/debian stable main"))"##;
    let expect = expect![[
        r#"OK (("\11deb https://apt.example/debian stable main\11" 0 "\11deb https://apt.example/debian stable main\11" "stable main" "main") (" deb-src https://apt.example/debian testing main contrib#comment" 0 " deb-src https://apt.example/debian testing main contrib#" "testing main contrib" "main contrib") ("deb https://apt.example/debian stable main   # comment with spaces" 0 "deb https://apt.example/debian stable main   #" "stable main" "main") ("deb https://apt.example/debian /#exact comment" 0 "deb https://apt.example/debian /#" "/" nil) ("x deb https://apt.example/debian stable main" nil nil nil nil))"#
    ]];
    assert_apt_sources_list_parity(elisp_form, expect);
}
