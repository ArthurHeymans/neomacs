use expect_test::expect;

use super::{assert_ac_php_core_parity, assert_ac_php_core_signal_parity};

#[test]
fn ac_php_core_pure_scalar_path_document_and_tag_helpers_cover_boundaries() {
    let elisp_form = r##"(let ((ac-php-tags-path
                    "/cache/ac-php"))
               (list
                (mapcar
                 #'ac-php--get-timestamp
                 '((0 0)
                   (0 65535)
                   (1 0)
                   (7 42)))
                (mapcar
                 (lambda (limit)
                   (ac-php--reduce-path
                    "/alpha/beta/gamma/delta/file.php"
                    limit))
                 '(100 24 12 1))
                (ac-php-g--project-root-dir
                 '(classes functions inherits files
                           "/project/root/"))
                (mapcar
                 (lambda (progress)
                   (let ((ac-php-phptags-index-progress
                          progress))
                     (ac-php-mode-line-project-status)))
                 '(0 7 42 100))
                (ac-php--get-common-json-file)
                (mapcar
                 #'ac-php-clean-document
                 '(nil
                   "plain"
                   "<#first#>"
                   "[#second#]"
                   "<#a#> [#b#]"))
                (mapcar
                 #'ac-php--tag-name-is-function
                 '("plain"
                   "call("
                   "(anonymous"
                   "close)"))
                (mapcar
                 #'ac-php--clean-return-type
                 '(nil
                   " Result "
                   "Foo|Bar|nil"
                   "  \\Acme\\Type | null "))
                (mapcar
                 #'ac-php-gen-el-func
                 '("name($one, $two = 2)"
                   "empty()"
                   "not a function"
                   "spaced(  $one ,\t&$two  ) tail"))
                (mapcar
                 #'ac-php--check-global-name
                 '("\\Acme"
                   "Acme"
                   ""
                   "\\"))
                (mapcar
                 #'ac-php--as-global-name
                 '("\\Acme"
                   "Acme"
                   ""
                   "\\"))
                (mapcar
                 #'ac-php--get-item-info
                 '("method("
                   "method"
                   "("
                   ""
                   "$property"))
                (let ((array
                       ["value" nil ""]))
                  (mapcar
                   (lambda (index)
                     (ac-php--get-array-string
                      array
                      (length array)
                      index))
                   '(0 1 2 3)))))"##;
    let expect = expect![[
        r#"OK ((0 65535 65536 458794) ("/alpha/beta/gamma/delta/file.php" "/a/b/g/delta/file.php" "/a/b/g/d/file.php" "/a/b/g/d/file.php") "/project/root/" (":00%%" ":07%%" ":42%%" ":100%%") "/cache/ac-php/common.el" (nil "plain" "first" "second " "a b ") (nil t t nil) (nil "Result" "Foo" "\\Acme\\Type") ("$one,$two = 2" "" "" "$one,&$two") (t nil nil t) ("\\Acme" "\\Acme" "\\" "\\") (("method(" "m") ("method" "p") ("(" "p") ("" "p") ("$property" "p")) ("value" "" "" ""))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_case_insensitive_string_helper_handles_case_unicode_empty_and_unequal_values() {
    let elisp_form = r##"(mapcar
               (lambda (pair)
                 (ac-php--string=-ignore-care
                  (car pair)
                  (cdr pair)))
               '(("Service"
                  . "service")
                 (""
                  . "")
                 ("Straße"
                  . "STRASSE")
                 ("İ"
                  . "i")
                 ("alpha"
                  . "beta")))"##;
    let expect = expect!["OK (t t t nil nil)"];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_parent_definition_helper_exposes_upstream_list_aref_signal() {
    let elisp_form = r##"(ac-php--get-class-name-from-parent-define
               "\\Class1, interface1")"##;
    let expect = expect![[r#"ERR (wrong-type-argument arrayp ("\\Class1" " interface1"))"#]];

    assert_ac_php_core_signal_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_debug_macro_and_toggle_preserve_exact_calls_and_global_state() {
    let elisp_form = r##"(let ((original-debug-on-error
                    debug-on-error)
                   (original-flag
                    ac-php-debug-flag)
                   calls)
               (unwind-protect
                   (cl-letf
                       (((symbol-function
                          'message)
                         (lambda (&rest arguments)
                           (push arguments calls)
                           (apply
                            #'format
                            arguments))))
                     (setq
                     ac-php-debug-flag
                      nil
                      debug-on-error
                      nil)
                     (let* ((disabled
                             (ac-php--debug
                              "item=%s"
                              "one"))
                            (enable-debug
                             (setq
                              ac-php-debug-flag
                              t))
                            (enabled
                             (ac-php--debug
                              "item=%s/%d"
                              "two"
                              2))
                            (disable-debug
                             (setq
                              ac-php-debug-flag
                              nil))
                            (first-toggle
                             (ac-php-toggle-debug))
                            (after-first
                             (list
                              ac-php-debug-flag
                              debug-on-error))
                            (second-toggle
                             (ac-php-toggle-debug)))
                       (list
                        disabled
                        enable-debug
                        enabled
                        disable-debug
                        first-toggle
                        after-first
                        second-toggle
                        ac-php-debug-flag
                        debug-on-error
                        (nreverse calls)
                        (macroexpand-1
                         '(ac-php--debug
                           "x=%s"
                           value)))))
                 (setq
                  debug-on-error
                  original-debug-on-error
                  ac-php-debug-flag
                  original-flag)))"##;
    let expect = expect![[
        r#"OK (nil t "[DEBUG]: item=two/2" nil "Debug mode was enabled in ac-php" (t t) "Debug mode was disabled in ac-php" nil nil (("[DEBUG]: item=%s/%d" "two" 2) ("Debug mode was %s in ac-php" "enabled") ("Debug mode was %s in ac-php" "disabled")) (when ac-php-debug-flag (message (concat "[DEBUG]: " "x=%s") value)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_word_extractors_cover_identifiers_functions_dollars_and_point_positions() {
    let elisp_form = r##"(mapcar
               (lambda (fixture)
                 (with-temp-buffer
                   (insert
                    (car fixture))
                   (goto-char
                    (or
                     (cdr fixture)
                     (point-max)))
                   (let ((before
                          (point)))
                     (list
                      (car fixture)
                      before
                      (ac-php--get-cur-word)
                      (ac-php--get-cur-word-with-function-flag)
                      (ac-php-get-cur-word-with-dollar)
                      (ac-php-get-cur-word-without-clean)
                      (point)))))
               '(("$someVariable")
                 ("\\Acme\\Service\\Foo")
                 ("foo()->bar")
                 ("foo()?->bar")
                 ("foo()")
                 ("call  (")
                 ("$object->method ();")
                 ("before $target after"
                  . 15)
                 ("'some string'")))"##;
    let expect = expect![[
        r#"OK (("$someVariable" 14 "someVariable" "someVariable" "$someVariable" "$someVariable" 14) ("\\Acme\\Service\\Foo" 18 "\\Acme\\Service\\Foo" "\\Acme\\Service\\Foo" "Foo" "\\Acme\\Service\\Foo" 18) ("foo()->bar" 11 "bar" "bar" "bar" "bar" 11) ("foo()?->bar" 12 "bar" "bar" "bar" "bar" 12) ("foo()" 6 "" "" "" "" 6) ("call  (" 8 "" "" "" "" 8) ("$object->method ();" 20 "" "" "" "" 20) ("before $target after" 15 "target" "target" "$target" "$target" 15) ("'some string'" 14 "" "" "" "" 14))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_location_format_line_column_and_dispatch_forms_match() {
    let elisp_form = r##"(with-temp-buffer
               (rename-buffer
                "ac-php-location-fixture"
                t)
               (insert
                "alpha\nbeta\ngamma\n")
               (goto-char
                9)
               (let ((initial
                      (list
                       (ac-php-current-location)
                       (ac-php-current-location
                        2)))
                     calls)
                 (cl-letf
                     (((symbol-function
                        'ac-php-find-file-or-buffer)
                       (lambda
                           (target
                            &optional
                            other-window)
                         (push
                          (list
                           target
                           other-window)
                          calls)
                         'opened)))
                   (let (results)
                     (dolist
                         (location
                          '("fixture.php:3:2"
                            "fixture.php:2"
                            "fixture.php,4"
                            "  named-buffer"
                            ""))
                       (goto-char
                        (point-min))
                       (push
                        (list
                         location
                         (ac-php-goto-location
                          location t)
                         (point))
                        results))
                     (goto-char
                      (point-min))
                     (let ((line-column-return
                            (ac-php-goto-line-col
                             2 3)))
                       (list
                        initial
                        (nreverse results)
                        line-column-return
                        (point)
                        (nreverse calls)))))))"##;
    let expect = expect![[
        r#"OK (("ac-php-location-fixture:2:3" "ac-php-location-fixture:1:-4") (("fixture.php:3:2" t 13) ("fixture.php:2" t 7) ("fixture.php,4" t 5) ("  named-buffer" opened 1) ("" nil 1)) nil 9 (("fixture.php" t) ("fixture.php" t) ("fixture.php" t) ("named-buffer" t)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_find_file_or_buffer_routes_files_buffers_windows_and_missing_targets() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'file-exists-p)
                     (lambda (target)
                       (string-prefix-p
                        "file:"
                        target)))
                    ((symbol-function
                      'find-file)
                     (lambda (target)
                       (push
                        (list
                         'find target)
                        calls)
                       'found))
                    ((symbol-function
                      'find-file-other-window)
                     (lambda (target)
                       (push
                        (list
                         'find-other target)
                        calls)
                       'found-other))
                    ((symbol-function
                      'get-buffer)
                     (lambda (target)
                       (and
                        (string-prefix-p
                         "buffer:"
                         target)
                        'buffer-object)))
                    ((symbol-function
                      'switch-to-buffer)
                     (lambda (target)
                       (push
                        (list
                         'switch target)
                        calls)
                       'switched))
                    ((symbol-function
                      'switch-to-buffer-other-window)
                     (lambda (target)
                       (push
                        (list
                         'switch-other
                         target)
                        calls)
                       'switched-other))
                    ((symbol-function
                      'message)
                     (lambda (&rest arguments)
                       (push
                        (cons
                         'message
                         arguments)
                        calls)
                       'messaged)))
                 (list
                  (ac-php-find-file-or-buffer
                   "file:one")
                  (ac-php-find-file-or-buffer
                   "file:two" t)
                  (ac-php-find-file-or-buffer
                   "buffer:one")
                  (ac-php-find-file-or-buffer
                   "buffer:two" t)
                  (ac-php-find-file-or-buffer
                   "missing")
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (found found-other switched switched-other messaged ((find "file:one") (find-other "file:two") (switch "buffer:one") (switch-other "buffer:two") (message "No buffer named %s; you can M-x: ac-php-remake-tags-all fix it" "missing")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_location_stack_push_truncates_forward_history_deduplicates_and_caps() {
    let elisp_form = r##"(let ((ac-php-location-stack
                    '("current"
                      "older"
                      "oldest"))
                   (ac-php-location-stack-index
                    2)
                   (ac-php-max-bookmark-count
                    2)
                   (locations
                    '("fresh"
                      "fresh"))
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php-current-location)
                     (lambda (&optional _offset)
                       (pop locations)))
                    ((symbol-function
                      'xref-push-marker-stack)
                     (lambda ()
                       (push
                        'xref
                        calls)
                       'pushed)))
                 (let ((first
                        (ac-php-location-stack-push)))
                   (let ((after-first
                          (list
                           first
                           ac-php-location-stack-index
                           (copy-sequence
                            ac-php-location-stack))))
                     (let ((second
                            (ac-php-location-stack-push)))
                       (list
                        after-first
                        second
                        ac-php-location-stack-index
                        ac-php-location-stack
                        (nreverse calls)))))))"##;
    let expect =
        expect![[r#"OK ((nil 0 ("fresh" "oldest")) nil 0 ("fresh" "oldest") (xref xref))"#]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_location_stack_jump_covers_current_mismatch_bounds_and_wrappers() {
    let elisp_form = r##"(let ((ac-php-location-stack
                    '("newest"
                      "middle"
                      "oldest"))
                   (ac-php-location-stack-index
                    1)
                   (current
                    "different")
                   calls)
               (cl-letf
                   (((symbol-function
                      'ac-php-current-location)
                     (lambda (&optional _offset)
                       current))
                    ((symbol-function
                      'ac-php-goto-location)
                     (lambda
                         (location
                          &optional
                          other-window)
                       (push
                        (list
                         location
                         other-window)
                        calls)
                       'jumped)))
                 (let ((mismatch
                        (ac-php-location-stack-jump
                         1)))
                   (setq
                    current
                    "middle")
                   (let ((back
                          (ac-php-location-stack-back))
                         (after-back
                          ac-php-location-stack-index))
                     (setq
                      current
                      "oldest")
                     (let ((out-of-bounds
                            (ac-php-location-stack-back)))
                       (setq
                        current
                        "oldest")
                       (let ((forward
                              (ac-php-location-stack-forward)))
                         (list
                          mismatch
                          back
                          after-back
                          out-of-bounds
                          forward
                          ac-php-location-stack-index
                          (nreverse calls))))))))"##;
    let expect = expect![[
        r#"OK (jumped jumped 2 nil jumped 1 (("middle" nil) ("oldest" nil) ("middle" nil)))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_current_function_vars_extracts_deduplicates_and_case_folds_names() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "<?php\nfunction demo($Arg, $second) {\n  $local = $Arg;\n  $UPPER = 1;\n  \"quoted\";\n  'single';\n  $lo")
               (php-mode)
               (goto-char
                (point-max))
               (let ((table
                      (ac-php--get-cur-function-vars))
                     keys)
                 (maphash
                  (lambda (key _value)
                    (push key keys))
                  table)
                 (list
                  (sort keys
                        #'string-lessp)
                  (hash-table-test
                   table)
                  (gethash
                   "$arg"
                   table
                   'missing)
                  (gethash
                   "$upper"
                   table
                   'missing))))"##;
    let expect = expect![[
        r#"OK (("" "$Arg" "$UPPER" "$l" "$local" "$second" "quoted" "single") case-fold nil nil)"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_case_fold_hash_test_matches_equality_hashing_and_table_replacement() {
    let elisp_form = r##"(let ((table
                    (make-hash-table
                     :test
                     'case-fold)))
               (puthash
                "Alpha"
                1
                table)
               (puthash
                "ALPHA"
                2
                table)
               (puthash
                "Beta"
                3
                table)
               (list
                (mapcar
                 (lambda (pair)
                   (apply
                    #'case-fold-string=
                    pair))
                 '(("Alpha" "alpha")
                   ("Alpha" "ALPHA")
                   ("Alpha" "Beta")
                   ("" "")))
                (=
                 (case-fold-string-hash
                  "Mixed")
                 (case-fold-string-hash
                  "mIXED"))
                (hash-table-count
                 table)
                (gethash
                 "alpha"
                 table)
                (gethash
                 "BETA"
                 table)
                (hash-table-test
                 table)))"##;
    let expect = expect!["OK ((t t nil t) t 2 2 3 case-fold)"];

    assert_ac_php_core_parity(elisp_form, expect);
}
