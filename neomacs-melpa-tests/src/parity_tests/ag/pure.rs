use expect_test::expect;

use super::assert_ag_parity;

#[test]
fn ag_escape_pcre_ports_upstream_ert_and_covers_every_punctuation_class() {
    let elisp_form = r##"(mapcar
         (lambda (input)
           (list input (ag/escape-pcre input)))
         '("ab.*("
           ""
           "alphaNUM123"
           "a-b_c.d+e?f*g(h)i[j]{k}|l^m$n\\o"
           "path/with spaces"
           "café λ"
           "\t\n"))"##;
    let expect = expect![[
        r#"OK (("ab.*(" "ab\\.\\*\\(") ("" "") ("alphaNUM123" "alphaNUM123") ("a-b_c.d+e?f*g(h)i[j]{k}|l^m$n\\o" "a\\-b\\_c\\.d\\+e\\?f\\*g\\(h\\)i\\[j\\]\\{k\\}\\|l\\^m\\$n\\\\o") ("path/with spaces" "path\\/with\\ spaces") ("café λ" "caf\\é\\ \\λ") ("\11\n" "\\\11\\\n"))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_longest_string_ports_upstream_ert_and_preserves_tie_order() {
    let elisp_form = r##"(list
         (ag/longest-string
          nil "foo" nil "f" "foobarbaz" "z")
         (ag/longest-string)
         (ag/longest-string nil nil)
         (ag/longest-string "" "a" "bb")
         (ag/longest-string "first" "other" "third")
         (ag/longest-string "same-a" "same-b"))"##;
    let expect = expect![[r#"OK ("foobarbaz" nil nil "bb" "first" "same-a")"#]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_replace_first_handles_first_middle_last_missing_and_regexp_text() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (pcase-let ((`(,string ,before ,after) case))
             (list
              case
              (ag/replace-first string before after))))
         '(("one -- two -- three" " -- " "  -- ")
           ("prefix" "pre" "POST")
           ("suffix" "fix" "END")
           ("unchanged" "missing" "X")
           ("a.*b.*c" ".*" "[literal]")
           ("" "" "empty")))"##;
    let expect = expect![[
        r#"OK ((("one -- two -- three" " -- " "  -- ") "one  -- two -- three") (("prefix" "pre" "POST") "POSTfix") (("suffix" "fix" "END") "sufEND") (("unchanged" "missing" "X") "unchanged") (("a.*b.*c" ".*" "[literal]") "a[literal]b.*c") (("" "" "empty") ""))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_buffer_name_covers_reuse_literal_regexp_and_path_spelling() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (pcase-let ((`(,reuse ,regexp ,query ,directory) case))
             (let ((ag-reuse-buffers reuse))
               (list
                case
                (ag/buffer-name query directory regexp)))))
         '((nil nil "needle" "/work/project/")
           (nil t "n[e]+dle" "/work/project/")
           (t nil "needle" "/work/project/")
           (t t "regexp" "/space dir/")
           (nil nil "" "./relative")))"##;
    let expect = expect![[
        r#"OK (((nil nil "needle" "/work/project/") "*ag search text:needle dir:/work/project/*") ((nil t "n[e]+dle" "/work/project/") "*ag search regexp:n[e]+dle dir:/work/project/*") ((t nil "needle" "/work/project/") "*ag search*") ((t t "regexp" "/space dir/") "*ag search*") ((nil nil "" "./relative") "*ag search text: dir:./relative*"))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_format_ignore_expands_order_duplicates_spaces_and_empty_patterns() {
    let elisp_form = r##"(mapcar
         (lambda (patterns)
           (list patterns (ag/format-ignore patterns)))
         '(nil
           ("target")
           ("target" "*.min.js" "space dir" "target")
           ("")
           (".git" "!keep")))"##;
    let expect = expect![[
        r#"OK ((nil nil) (("target") ("--ignore" "target")) (("target" "*.min.js" "space dir" "target") ("--ignore" "target" "--ignore" "*.min.js" "--ignore" "space dir" "--ignore" "target")) (("") ("--ignore" "")) ((".git" "!keep") ("--ignore" ".git" "--ignore" "!keep")))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_dwim_at_point_reads_real_active_region_symbol_and_empty_context() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert "alpha beta")
           (goto-char 6)
           (set-mark 2)
           (setq mark-active t
                 transient-mark-mode t)
           (ag/dwim-at-point))
         (with-temp-buffer
           (emacs-lisp-mode)
           (insert "(alpha-beta gamma)")
           (goto-char 5)
           (ag/dwim-at-point))
         (with-temp-buffer
           (insert "   ")
           (goto-char 2)
           (ag/dwim-at-point))
         (with-temp-buffer
           (insert
            (propertize
             "chosen"
             'face 'bold
             'field 'decorated))
           (set-mark (point-min))
           (goto-char (point-max))
           (setq mark-active t
                 transient-mark-mode t)
           (list
            (ag/dwim-at-point)
            (text-properties-at
             0
             (ag/dwim-at-point)))))"##;
    let expect = expect![[r#"OK ("lpha" "alpha-beta" nil ("chosen" nil))"#]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_buffer_extension_regex_uses_real_buffer_file_names_and_pcre_escaping() {
    let elisp_form = r##"(mapcar
         (lambda (file-name)
           (with-temp-buffer
             (setq buffer-file-name file-name)
             (list
              file-name
              (and
               (stringp file-name)
               (file-name-extension file-name))
              (condition-case error-data
                  (ag/buffer-extension-regex)
                (error
                 (list
                  'error
                  (car error-data)
                  (cadr error-data)))))))
         '(nil
           "/work/source.el"
           "/work/archive.tar.gz"
           "/work/name.with+plus"
           "/work/Makefile"
           "/work/.env"
           "/work/trailing."))"##;
    let expect = expect![[
        r#"OK ((nil nil "") ("/work/source.el" "el" "\\.el$") ("/work/archive.tar.gz" "gz" "\\.gz$") ("/work/name.with+plus" "with+plus" "\\.with\\+plus$") ("/work/Makefile" nil "\\.$") ("/work/.env" nil "\\.$") ("/work/trailing." "" "\\.$"))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_project_root_uses_custom_resolver_or_longest_real_vcs_root() {
    let elisp_form = r##"(let (events)
         (cl-letf (((symbol-function 'vc-git-root)
                    (lambda (file)
                      (push (list 'git file) events)
                      "/repo/"))
                   ((symbol-function 'vc-svn-root)
                    (lambda (file)
                      (push (list 'svn file) events)
                      "/repo/deeper/"))
                   ((symbol-function 'vc-hg-root)
                    (lambda (file)
                      (push (list 'hg file) events)
                      nil))
                   ((symbol-function 'vc-bzr-root)
                    (lambda (file)
                      (push (list 'bzr file) events)
                      "/r/")))
           (let ((fallback
                  (let ((ag-project-root-function nil))
                    (ag/project-root "/repo/deeper/src/file.el")))
                 (custom
                  (let ((ag-project-root-function
                         (lambda (file)
                           (push (list 'custom file) events)
                           "/custom/root/")))
                    (ag/project-root "/elsewhere/file.el"))))
             (list fallback custom (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("/repo/deeper/" "/custom/root/" ((git "/repo/deeper/src/file.el") (svn "/repo/deeper/src/file.el") (hg "/repo/deeper/src/file.el") (bzr "/repo/deeper/src/file.el") (custom "/elsewhere/file.el")))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}
