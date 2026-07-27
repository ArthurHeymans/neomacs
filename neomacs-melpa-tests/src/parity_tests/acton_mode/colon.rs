use expect_test::expect;

use super::assert_acton_mode_parity;

#[test]
fn acton_mode_colon_handler_ignores_non_colons_and_non_clause_lines() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (acton-mode)
           (insert
            "header\n    if ready:\n        value")
           (goto-char
            (point-max))
           (let ((before
                  (buffer-string)))
             (list
              (acton-handle-colon)
              (equal before
                     (buffer-string))
              (current-indentation))))
         (with-temp-buffer
           (acton-mode)
           (insert
            "header\n    if ready:\n        value:")
           (goto-char
            (point-max))
           (let ((before
                  (buffer-string)))
             (list
              (acton-handle-colon)
              (equal before
                     (buffer-string))
              (current-indentation)))))"##;
    let expect = expect!["OK ((nil t 8) (nil t 8))"];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_colon_handler_reindents_each_clause_to_if_or_try_parent() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (let ((parent
                  (car case))
                 (clause
                  (cadr case)))
             (with-temp-buffer
               (acton-mode)
               (insert
                "header\n"
                "    "
                parent
                " condition:\n"
                "        value\n"
                "            "
                clause
                ":")
               (goto-char
                (point-max))
               (let ((before
                      (current-indentation))
                     (result
                      (acton-handle-colon)))
                 (list
                  parent
                  clause
                  before
                  result
                  (current-indentation)
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position)))))))
         '(("if" "else")
           ("if" "elif")
           ("try" "except")
           ("try" "finally")))"##;
    let expect = expect![[
        r#"OK (("if" "else" 12 nil 4 "    else:") ("if" "elif" 12 nil 4 "    elif:") ("try" "except" 12 nil 4 "    except:") ("try" "finally" 12 nil 4 "    finally:"))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_colon_handler_covers_equal_and_zero_parent_indentation() {
    let elisp_form = r##"(mapcar
         (lambda (source)
           (with-temp-buffer
             (acton-mode)
             (insert source)
             (goto-char
              (point-max))
             (let (indent-targets)
               (cl-letf
                   (((symbol-function
                     'indent-line-to)
                     (lambda (target)
                       (push
                        target
                        indent-targets)
                       nil)))
                 (let ((result
                        (acton-handle-colon)))
                   (list
                    source
                    result
                    (nreverse
                     indent-targets)
                    (current-indentation)
                    (buffer-substring-no-properties
                     (line-beginning-position)
                     (line-end-position))))))))
         '("header\n    if ready:\n        value\n    else:"
           "header\nif ready:\n    value\n            else:"))"##;
    let expect = expect![[
        r#"OK (("header\n    if ready:\n        value\n    else:" nil nil 4 "    else:") ("header\nif ready:\n    value\n            else:" nil (0) 12 "            else:"))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_colon_handler_chooses_the_nearest_matching_nested_parent() {
    let elisp_form = r##"(with-temp-buffer
         (acton-mode)
         (insert
          "header\n"
          "    if outer:\n"
          "        value\n"
          "        if inner:\n"
          "            nested\n"
          "                else:")
         (goto-char
          (point-max))
         (let ((before
                (current-indentation))
               (result
                (acton-handle-colon)))
           (list
            before
            result
            (current-indentation)
            (buffer-substring-no-properties
             (line-beginning-position)
             (line-end-position)))))"##;
    let expect = expect![[r#"OK (16 nil 8 "        else:")"#]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_colon_handler_leaves_clauses_without_a_searchable_parent_unchanged() {
    let elisp_form = r##"(mapcar
         (lambda (source)
           (with-temp-buffer
             (acton-mode)
             (insert source)
             (goto-char
              (point-max))
             (let ((before
                    (buffer-string))
                   (indent
                    (current-indentation))
                   (result
                    (acton-handle-colon)))
               (list
                source
                indent
                result
                (current-indentation)
                (equal before
                       (buffer-string))))))
         '("if root:\n            else:"
           "header\n    value\n            elif:"
           "header\n    while ready:\n            finally:"
           "header\n        if deeper:\n    else:"))"##;
    let expect = expect![[
        r#"OK (("if root:\n            else:" 12 nil 12 t) ("header\n    value\n            elif:" 12 nil 12 t) ("header\n    while ready:\n            finally:" 12 nil 12 t) ("header\n        if deeper:\n    else:" 4 nil 4 t))"#
    ]];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_colon_handler_honors_iteration_and_distance_search_bounds() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (intervening-lines)
            (with-temp-buffer
              (acton-mode)
              (insert
               "header\n    if root:\n")
              (dotimes (_ intervening-lines)
                (insert
                 "        value\n"))
              (insert
               "            else:")
              (goto-char
               (point-max))
              (let ((before
                     (current-indentation))
                    (result
                     (acton-handle-colon)))
                (list
                 intervening-lines
                 (buffer-size)
                 before
                 result
                 (current-indentation)))))
          '(998 999 1001))
         (mapcar
          (lambda (payload-size)
            (with-temp-buffer
              (acton-mode)
              (insert
               "header\n    try root:\n        "
               (make-string
                payload-size
                ?x)
               "\n            finally:")
              (goto-char
               (point-max))
              (let ((before
                     (current-indentation))
                    (result
                     (acton-handle-colon)))
                (list
                 payload-size
                 (buffer-size)
                 before
                 result
                 (current-indentation)))))
          '(49900 50001)))"##;
    let expect = expect![
        "OK (((998 14001 12 nil 4) (999 14023 12 nil 12) (1001 14051 12 nil 12)) ((49900 49942 12 nil 4) (50001 50051 12 nil 12)))"
    ];
    assert_acton_mode_parity(elisp_form, expect);
}

#[test]
fn acton_mode_post_self_insert_hook_runs_colon_reindent_in_mode_buffers_only() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (acton-mode)
           (insert
            "header\n    if ready:\n        value\n            else")
           (goto-char
            (point-max))
           (insert
            ":")
           (let ((before
                  (current-indentation)))
             (run-hooks
              'post-self-insert-hook)
             (list
              before
              (current-indentation)
              (buffer-substring-no-properties
               (line-beginning-position)
               (line-end-position))
              (local-variable-p
               'post-self-insert-hook))))
         (with-temp-buffer
           (prog-mode)
           (insert
            "header\n    if ready:\n        value\n            else:")
           (goto-char
            (point-max))
           (let ((before
                  (buffer-string)))
             (run-hooks
              'post-self-insert-hook)
             (list
              (equal before
                     (buffer-string))
              (current-indentation)
              (memq
               'acton-handle-colon
               post-self-insert-hook)))))"##;
    let expect = expect![[r#"OK ((12 4 "    else:" t) (t 12 nil))"#]];
    assert_acton_mode_parity(elisp_form, expect);
}
