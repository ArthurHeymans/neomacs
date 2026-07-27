use expect_test::expect;

use super::assert_ag_parity;

#[test]
fn ag_mode_initializes_real_compilation_navigation_filter_and_finish_state() {
    let elisp_form = r##"(with-temp-buffer
         (ag-mode)
         (list
          major-mode
          mode-name
          compilation-error-regexp-alist
          compilation-error-regexp-alist-alist
          compilation-error-face
          next-error-function
          compilation-finish-functions
          compilation-filter-hook
          (local-variable-p
           'compilation-error-regexp-alist)
          (local-variable-p
           'compilation-error-regexp-alist-alist)
          (local-variable-p 'next-error-function)
          (local-variable-p
           'compilation-finish-functions)))"##;
    let expect = expect![[
        r#"OK (ag-mode "Ag" (compilation-ag-nogroup compilation-ag-group) ((compilation-ag-nogroup "^\\(.+?\\):\\([1-9][0-9]*\\):\\([1-9][0-9]*\\):" 1 2 3) (compilation-ag-group "^\\([[:digit:]]+\\):\\([[:digit:]]+\\):" ag/compilation-match-grouped-filename 1 2)) ag-hit-face ag/next-error-function ag/run-finished-hook (ag-filter t) t t t t)"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_grouped_compilation_filename_lookup_tracks_nearest_real_file_header() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "File: src/first:file.el\n"
          "12:4:first match\n"
          "13:2:second match\n"
          "\n"
          "File: test/second.el\n"
          "7:9:third match\n")
         (list
          (progn
            (goto-char (point-min))
            (forward-line 1)
            (ag/compilation-match-grouped-filename))
          (progn
            (goto-char (point-min))
            (forward-line 2)
            (ag/compilation-match-grouped-filename))
          (progn
            (goto-char (point-max))
            (forward-line -1)
            (ag/compilation-match-grouped-filename))
          (progn
            (goto-char (point-min))
            (ag/compilation-match-grouped-filename))))"##;
    let expect =
        expect![[r#"OK (("src/first:file.el") ("src/first:file.el") ("test/second.el") nil)"#]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_filter_converts_grouped_real_ansi_output_into_navigation_text_and_faces() {
    let elisp_form = r##"(with-temp-buffer
         (setq ag-group-matches t
               ag-highlight-search t
               compilation-filter-start
               (copy-marker (point-min)))
         (insert
          "\e[1;32msrc/main.el\e[0m\e[K\n"
          "12:4:before \e[30;43mneedle\e[0m\e[K after\n"
          "13:2:\e[31mother\e[0m\e[K\n")
         (ag-filter)
         (let ((position (point-min))
               runs)
           (while (< position (point-max))
             (let ((next
                    (next-property-change
                     position nil (point-max))))
               (push
                (list
                 (buffer-substring-no-properties
                  position next)
                 (copy-tree
                  (text-properties-at position)))
                runs)
               (setq position next)))
           (list
            (buffer-substring-no-properties
             (point-min)
             (point-max))
            (nreverse runs)
            (marker-position
             compilation-filter-start))))"##;
    let expect = expect![[
        r#"OK ("File: src/main.el\n12:4:before needle after\n13:2:other\n" (("File: " nil) ("src/main.el" (font-lock-face compilation-info face nil)) ("\n12:4:before " nil) ("needle" (face nil font-lock-face ag-match-face)) (" after\n13:2:other\n" nil)) 1)"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_filter_leaves_partial_line_until_completion_then_strips_remaining_ansi() {
    let elisp_form = r##"(with-temp-buffer
         (setq ag-group-matches nil
               ag-highlight-search nil
               compilation-filter-start
               (copy-marker (point-min)))
         (insert
          "src/a.el:1:2:\e[31mcomplete\e[0m\e[K\n"
          "src/b.el:3:4:\e[32mpart")
         (ag-filter)
         (let ((first-pass
                (buffer-substring
                 (point-min)
                 (point-max))))
           (goto-char (point-max))
           (insert "ial\e[0m\e[K\n")
           (setq compilation-filter-start
                 (copy-marker
                  (string-match
                   "src/b"
                   (buffer-string))))
           (ag-filter)
           (list
            first-pass
            (buffer-substring
             (point-min)
             (point-max)))))"##;
    let expect = expect![[
        r#"OK ("src/a.el:1:2:complete\nsrc/b.el:3:4:\33[32mpart" "src/a.el:1:2:complete\nsrc/b.el:3:4:partial\n")"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_next_error_reuses_window_only_when_configured_and_restores_pop_to_buffer() {
    let elisp_form = r##"(let (events original-pop)
         (cl-letf (((symbol-function 'pop-to-buffer)
                    (lambda (&rest arguments)
                      (push (cons 'pop arguments) events)
                      'popped))
                   ((symbol-function 'switch-to-buffer)
                    (lambda (&rest arguments)
                      (push (cons 'switch arguments) events)
                      'switched))
                   ((symbol-function
                     'compilation-next-error-function)
                    (lambda (n reset)
                      (push
                       (list 'next n reset)
                       events)
                      (pop-to-buffer
                       "target"
                       'display-action))))
           (setq original-pop
                 (symbol-function 'pop-to-buffer))
           (let ((separate
                  (let ((ag-reuse-window nil))
                    (ag/next-error-function 2 t)))
                 (reused
                  (let ((ag-reuse-window t))
                    (ag/next-error-function -1 nil))))
             (list
              separate
              reused
              (eq
               original-pop
               (symbol-function 'pop-to-buffer))
              (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (popped switched t ((next 2 t) (pop "target" display-action) (next -1 nil) (switch "target")))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}

#[test]
fn ag_finished_hook_and_temporary_function_patch_run_in_real_buffer_and_unwind() {
    let elisp_form = r##"(let ((buffer
                (generate-new-buffer
                 " *ag-finished-parity*"))
               events)
         (fset 'ag-parity-target
               (lambda (value)
                 (list 'original value)))
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (add-hook
                  'ag-search-finished-hook
                  (lambda ()
                    (push
                     (list
                      'hook
                      (buffer-name)
                      major-mode)
                     events))
                  nil t))
               (ag/run-finished-hook
                buffer "finished\n")
               (let ((patched
                      (ag/with-patch-function
                       'ag-parity-target
                       (value)
                       (list 'patched value)
                       (ag-parity-target 10)))
                     errored)
                 (condition-case error-data
                     (ag/with-patch-function
                      'ag-parity-target
                      (value)
                      (error "patched failure %s" value)
                      (ag-parity-target 20))
                   (error
                    (setq errored error-data)))
                 (list
                  patched
                  errored
                  (ag-parity-target 30)
                  (nreverse events))))
           (when (buffer-live-p buffer)
             (kill-buffer buffer))
           (fmakunbound 'ag-parity-target)))"##;
    let expect = expect![[
        r#"OK ((patched 10) (error "patched failure 20") (original 30) ((hook " *ag-finished-parity*" fundamental-mode)))"#
    ]];
    assert_ag_parity(elisp_form, expect);
}
