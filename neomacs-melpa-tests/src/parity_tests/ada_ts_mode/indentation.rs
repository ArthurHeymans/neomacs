use expect_test::expect;

use super::assert_ada_ts_mode_parity;

#[test]
fn ada_ts_mode_indent_offset_watcher_recomputes_global_and_buffer_local_defaults() {
    let elisp_form = r##"(let ((symbols
                '(ada-ts-mode-indent-offset
                  ada-ts-mode-indent-when-offset
                  ada-ts-mode-indent-broken-offset
                  ada-ts-mode-indent-exp-item-offset
                  ada-ts-mode-indent-subprogram-is-offset
                  ada-ts-mode-indent-record-offset
                  ada-ts-mode-indent-label-offset)))
         (list
          (mapcar
           #'default-value
           symbols)
          (progn
            (set-default
             'ada-ts-mode-indent-offset
             5)
            (mapcar
             #'default-value
             symbols))
          (with-temp-buffer
            (setq-local
             ada-ts-mode-indent-offset
             8)
            (list
             (mapcar
              #'symbol-value
              symbols)
             (mapcar
              #'local-variable-p
              symbols)))))"##;
    let expect = expect!["OK ((3 3 2 0 2 3 3) (5 5 4 0 4 5 5) ((8 8 7 0 7 8 8) (t t t t t t t)))"];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_indent_offset_watcher_preserves_explicit_user_override() {
    let elisp_form = r##"(progn
         (set-default
          'ada-ts-mode-indent-broken-offset
          17)
         (set-default
          'ada-ts-mode-indent-offset
          6)
         (list
          (default-value
           'ada-ts-mode-indent-offset)
          (default-value
           'ada-ts-mode-indent-when-offset)
          (default-value
           'ada-ts-mode-indent-broken-offset)
          (default-value
           'ada-ts-mode-indent-subprogram-is-offset)
          (default-value
           'ada-ts-mode-indent-record-offset)
          (default-value
           'ada-ts-mode-indent-label-offset)))"##;
    let expect = expect!["OK (6 6 17 5 6 6)"];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_lsp_indentation_success_and_tree_sitter_fallback_dispatch_match() {
    let elisp_form = r##"(let (events
               line-success
               line-fallback
               region-success
               region-fallback)
         (cl-letf
             (((symbol-function
                'ada-ts-mode-indent)
               (lambda (strategy)
                 (push
                  (list
                   'tree-sitter
                   strategy)
                  events)
                 'tree-sitter-result)))
           (setq
            line-success
            (cl-letf
                (((symbol-function
                   'ada-ts-als-format-line)
                  (lambda (offset)
                    (push
                     (list
                      'lsp-line
                      offset
                      'success)
                     events)
                    'success)))
              (ada-ts-mode-indent-line
               'lsp)))
           (setq
            line-fallback
            (cl-letf
                (((symbol-function
                   'ada-ts-als-format-line)
                  (lambda (offset)
                    (push
                     (list
                      'lsp-line
                      offset
                      'failure)
                     events)
                    nil)))
              (let ((ada-ts-mode-indent-backend
                     'lsp))
                (ada-ts-mode-indent-line
                 'lsp))))
           (setq
            region-success
            (cl-letf
                (((symbol-function
                   'ada-ts-als-format-region)
                  (lambda (beg end offset)
                    (push
                     (list
                      'lsp-region
                      beg
                      end
                      offset
                      'success)
                     events)
                    'success)))
              (ada-ts-mode-indent-region
               'lsp
               3
               9)))
           (setq
            region-fallback
            (cl-letf
                (((symbol-function
                   'ada-ts-als-format-region)
                  (lambda (beg end offset)
                    (push
                     (list
                      'lsp-region
                      beg
                      end
                      offset
                      'failure)
                     events)
                    nil))
                 ((symbol-function
                   'treesit-indent-region)
                  (lambda (beg end)
                    (push
                     (list
                      'tree-sitter-region
                      beg
                      end)
                     events)
                    'tree-sitter-region-result)))
              (ada-ts-mode-indent-region
               'lsp
               4
               12))))
         (list
          line-success
          line-fallback
          region-success
          region-fallback
          (nreverse
           events)))"##;
    let expect = expect![
        "OK (success tree-sitter-result success tree-sitter-region-result ((lsp-line 3 success) (lsp-line 3 failure) (tree-sitter line) (lsp-region 3 9 3 success) (lsp-region 4 12 3 failure) (tree-sitter-region 4 12)))"
    ];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_unknown_indentation_backend_and_strategy_signal_exact_errors() {
    let elisp_form = r##"(mapcar
         (lambda (thunk)
           (condition-case error-data
               (funcall
                thunk)
             (error
              (list
               (car
                error-data)
               (error-message-string
                error-data)
               (cdr
                error-data)))))
         (list
          (lambda ()
            (ada-ts-mode-indent-line
             'unknown-backend))
          (lambda ()
            (ada-ts-mode-indent-region
             'unknown-backend
             2
             7))
          (lambda ()
            (ada-ts-mode-indent
             'unknown-strategy))))"##;
    let expect = expect![[
        r#"OK ((error "Unknown indentation backend: unknown-backend" ("Unknown indentation backend: unknown-backend")) (error "Unknown indentation backend: unknown-backend" ("Unknown indentation backend: unknown-backend")) (error "Unknown indentation strategy: unknown-strategy" ("Unknown indentation strategy: unknown-strategy")))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_after_change_electric_indent_state_matrix_matches() {
    let elisp_form = r##"(with-temp-buffer
         (let ((electric-indent-mode
                t)
               (electric-indent-inhibit
                nil)
               results)
           (dolist (case
                    '((1 2 0 t nil)
                      (1 1 2 t nil)
                      (1 2 2 t nil)
                      (1 2 0 nil nil)
                      (1 2 0 t t)))
             (setq
              ada-ts-indent--electric-indent-check-needed
              nil
              electric-indent-mode
              (nth
               3
               case)
              electric-indent-inhibit
              (nth
               4
               case))
             (ada-ts-indent--after-change
              (nth
               0
               case)
              (nth
               1
               case)
              (nth
               2
               case))
             (push
              (list
               case
               ada-ts-indent--electric-indent-check-needed)
              results))
           (nreverse
            results)))"##;
    let expect = expect![
        "OK (((1 2 0 t nil) t) ((1 1 2 t nil) nil) ((1 2 2 t nil) t) ((1 2 0 nil nil) nil) ((1 2 0 t t) nil))"
    ];
    assert_ada_ts_mode_parity(elisp_form, expect);
}

#[test]
fn ada_ts_mode_indent_verbosity_watcher_toggles_advice_global_state_and_buffer_rules() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "procedure Verbose is\n"
          "begin\n"
          "   null;\n"
          "end Verbose;\n")
         (let ((ada-ts-mode-grammar-install
                nil)
               messages)
           (ada-ts-mode)
           (cl-letf
               (((symbol-function
                  'message)
                 (lambda (format-string &rest arguments)
                   (push
                    (apply
                     #'format
                     format-string
                     arguments)
                    messages))))
             (let ((initial
                    (list
                     (default-value
                      'ada-ts-mode--indent-verbose)
                     treesit--indent-verbose
                     (and
                      (advice-member-p
                       #'ada-ts-mode--advice-treesit--indent-rules-optimize
                       'treesit--indent-rules-optimize)
                      t))))
               (set-default
                'ada-ts-mode--indent-verbose
                t)
               (let ((enabled
                      (list
                       (default-value
                        'ada-ts-mode--indent-verbose)
                       treesit--indent-verbose
                       (and
                        (advice-member-p
                         #'ada-ts-mode--advice-treesit--indent-rules-optimize
                         'treesit--indent-rules-optimize)
                        t)
                       (local-variable-p
                        'treesit-simple-indent-rules)
                       (length
                        treesit-simple-indent-rules))))
                 (set-default
                  'ada-ts-mode--indent-verbose
                  nil)
                 (list
                  initial
                  enabled
                  (list
                   (default-value
                    'ada-ts-mode--indent-verbose)
                   treesit--indent-verbose
                   (and
                    (advice-member-p
                     #'ada-ts-mode--advice-treesit--indent-rules-optimize
                     'treesit--indent-rules-optimize)
                    t)
                   (local-variable-p
                    'treesit-simple-indent-rules)
                   (length
                    treesit-simple-indent-rules))
                  (nreverse
                   messages)))))))"##;
    let expect = expect![[
        r#"OK ((nil nil nil) (t t t t 1) (nil nil nil t 1) ("Building uncompiled indent queries for  *temp*" "Building compiled indent queries for  *temp*"))"#
    ]];
    assert_ada_ts_mode_parity(elisp_form, expect);
}
