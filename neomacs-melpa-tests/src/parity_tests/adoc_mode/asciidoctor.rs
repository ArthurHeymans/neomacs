use expect_test::expect;

use super::assert_adoc_mode_parity;

#[test]
fn adoc_mode_asciidoctor_guards_source_and_export_command_builders_match() {
    let elisp_form = r##"(let (commands)
         (cl-letf
             (((symbol-function 'executable-find)
               (lambda (command)
                 (and (equal command "asciidoctor")
                      "/tools/asciidoctor")))
              ((symbol-function 'compilation-start)
               (lambda (command &rest args)
                 (push (list command args default-directory) commands)
                 'compilation-buffer)))
           (let ((adoc-asciidoctor-extra-args
                  '("-a" "sectnums")))
             (with-temp-buffer
               (setq buffer-file-name
                     (expand-file-name "document with space.adoc"
                                       default-directory))
               (mapcar
                (lambda (function)
                  (funcall function))
                '(adoc-export-html adoc-export-docbook
                  adoc-export-pdf adoc-export-epub)))
             (list
              (nreverse
               (mapcar
                (lambda (entry)
                  (list (car entry)
                        (mapcar
                         (lambda (arg)
                           (if (functionp arg) 'function arg))
                         (cadr entry))))
                commands))
              (with-temp-buffer
                (condition-case error
                    (adoc--asciidoctor-source-file)
                  (error (list (car error) (cadr error)))))
              (let ((adoc-asciidoctor-command "missing"))
                (condition-case error
                    (adoc--asciidoctor-ensure)
                  (error (list (car error) (cadr error)))))))))"##;
    let expect = expect![[
        r#"OK ((("asciidoctor -a sectnums -b html5 document\\ with\\ space.adoc" (nil function)) ("asciidoctor -a sectnums -b docbook5 document\\ with\\ space.adoc" (nil function)) ("asciidoctor -a sectnums -r asciidoctor-pdf -b pdf document\\ with\\ space.adoc" (nil function)) ("asciidoctor -a sectnums -r asciidoctor-epub3 -b epub3 document\\ with\\ space.adoc" (nil function))) (user-error "Current buffer is not visiting a file") (user-error "Cannot find the Asciidoctor executable \"missing\"; customize ‘adoc-asciidoctor-command’"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_preview_backend_update_display_and_cleanup_contract_match() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function 'display-graphic-p) (lambda () nil))
              ((symbol-function 'adoc--asciidoctor-render-preview)
               (lambda () "preview.html"))
              ((symbol-function 'adoc--preview-display)
               (lambda (file) (push (list 'display file) calls)))
              ((symbol-function 'delete-file)
               (lambda (file) (push (list 'delete file) calls)))
              ((symbol-function 'file-exists-p)
               (lambda (_file) t)))
           (list
            (mapcar
             (lambda (backend)
               (let ((adoc-preview-backend backend))
                 (adoc--preview-resolve-backend)))
             '(auto eww browser xwidget))
            (progn
              (adoc--preview-update)
              (nreverse calls))
            (with-temp-buffer
              (setq adoc--preview-file "preview.html")
              (adoc--preview-cleanup)
              (list adoc--preview-file (nreverse calls)))
            (with-temp-buffer
              (cl-letf
                  (((symbol-function 'adoc--preview-update)
                    (lambda () (push 'update calls))))
                (adoc-live-preview-mode 1)
                (let ((enabled
                       (list
                        adoc-live-preview-mode
                        (memq #'adoc--preview-update after-save-hook)
                        (memq #'adoc--preview-cleanup kill-buffer-hook))))
                  (adoc-live-preview-mode -1)
                  (list enabled
                        adoc-live-preview-mode
                        (memq #'adoc--preview-update after-save-hook)
                        (memq #'adoc--preview-cleanup kill-buffer-hook)
                        (nreverse calls))))))))"##;
    let expect = expect![[
        r#"OK ((eww eww browser xwidget) #1=((display "preview.html") . #2=((delete "preview.html") update)) (nil #1#) ((t (adoc--preview-update t) (adoc--preview-cleanup t)) nil nil nil #2#))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_compilation_regexp_matches_modern_warning_error_and_legacy_formats() {
    let elisp_form = r##"(with-temp-buffer
         (adoc-mode)
         (let ((regexp
                (car (alist-get
                      'asciidoc
                      compilation-error-regexp-alist-alist))))
           (mapcar
            (lambda (line)
              (and (string-match regexp line)
                   (list
                    (match-string 1 line)
                    (match-string 2 line)
                    (match-string 3 line))))
            '("asciidoctor: ERROR: doc.adoc: line 5: include missing"
              "asciidoctor: WARNING: doc.adoc: line 1: table open"
              "asciidoctor: DEPRECATED: old.adoc: line 8: old syntax"
              "asciidoc: WARNING: doc.txt: line 9: missing"
              "unrelated output"))))"##;
    let expect = expect![[
        r#"OK ((nil "doc.adoc" "5") ("WARNING" "doc.adoc" "1") ("DEPRECATED" "old.adoc" "8") ("WARNING" "doc.txt" "9") nil)"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_flymake_parser_maps_severity_positions_messages_and_fatal_fallback() {
    let elisp_form = r##"(with-temp-buffer
         (insert "one\ntwo\nthree\nfour\nfive\nsix\nseven\n")
         (let ((source (current-buffer)))
           (mapcar
            (lambda (case)
              (mapcar
               (lambda (diagnostic)
                 (list
                  (flymake-diagnostic-beg diagnostic)
                  (flymake-diagnostic-end diagnostic)
                  (flymake-diagnostic-type diagnostic)
                  (flymake-diagnostic-text diagnostic)))
               (adoc--flymake-parse-output
                (car case) source (cadr case))))
            '(("asciidoctor: ERROR: <stdin>: line 5: include missing\nasciidoctor: WARNING: <stdin>: line 7: table open\n" 0)
              ("asciidoctor: WARNING: <stdin>: Line 3: capital line\n" 0)
              ("asciidoctor: DEPRECATED: <stdin>: line 2: old syntax\n" 0)
              ("asciidoctor: FAILED: converter missing\n" 1)
              ("" 0)))))"##;
    let expect = expect![[
        r#"OK (((20 24 :error "include missing") (29 34 :warning "table open")) ((9 14 :warning "capital line")) ((5 8 :note "old syntax")) ((1 4 :error "asciidoctor: FAILED: converter missing")) nil)"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_preview_renderer_pipes_unsaved_text_builds_arguments_reports_errors_and_reuses_output()
{
    let elisp_form = r##"(with-temp-buffer
         (insert "= Unsaved Document\n")
         (let ((adoc-asciidoctor-extra-args
                '("-a" "sectnums"))
               (statuses '(0 1))
               calls messages)
           (cl-letf
               (((symbol-function 'executable-find)
                 (lambda (command)
                   (and (equal command "asciidoctor")
                        "/tools/asciidoctor")))
                ((symbol-function 'call-process-region)
                 (lambda
                   (start end command delete destination display
                          &rest arguments)
                   (with-temp-file (cadr destination)
                     (insert "deterministic diagnostic"))
                   (push
                    (list
                     (buffer-substring-no-properties start end)
                     command delete display
                     (append
                      (seq-take arguments 3)
                      (list
                       (equal
                        (nth 3 arguments)
                        (expand-file-name default-directory)))
                      (seq-subseq arguments 4 7)
                      (list
                       (equal
                        (nth 7 arguments)
                        adoc--preview-file))
                      (seq-drop arguments 8)))
                    calls)
                   (pop statuses)))
                ((symbol-function 'message)
                 (lambda (format-string &rest arguments)
                   (push
                    (apply #'format format-string arguments)
                    messages))))
             (unwind-protect
                 (let* ((first
                         (adoc--asciidoctor-render-preview))
                        (second
                         (adoc--asciidoctor-render-preview)))
                   (list
                    (equal first adoc--preview-file)
                    (file-exists-p first)
                    second
                    (nreverse calls)
                    (nreverse messages)))
               (when (and adoc--preview-file
                          (file-exists-p adoc--preview-file))
                 (delete-file adoc--preview-file))))))"##;
    let expect = expect![[
        r#"OK (t t nil (("= Unsaved Document\n" "asciidoctor" nil nil ("-a" "sectnums" "-B" t "-b" "html5" "-o" t "-")) ("= Unsaved Document\n" "asciidoctor" nil nil ("-a" "sectnums" "-B" t "-b" "html5" "-o" t "-"))) ("Asciidoctor: deterministic diagnostic" "Asciidoctor: deterministic diagnostic"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_preview_actual_backend_dispatch_auto_graphical_choice_and_failed_update_match() {
    let elisp_form = r##"(progn
         (require 'eww)
         (let (calls)
           (cl-letf
               (((symbol-function 'display-graphic-p)
                 (lambda () t))
                ((symbol-function 'browse-url)
                 (lambda (url)
                   (push (list 'browser url) calls)))
                ((symbol-function 'xwidget-webkit-browse-url)
                 (lambda (url)
                   (push (list 'xwidget url) calls)))
                ((symbol-function 'eww-open-file)
                 (lambda (file)
                   (push (list 'eww file) calls)))
                ((symbol-function 'get-buffer)
                 (lambda (name)
                   (cond
                    ((equal name "*xwidget-webkit*")
                     'xwidget-buffer)
                    ((equal name "*eww*") 'eww-buffer))))
                ((symbol-function 'display-buffer-in-side-window)
                 (lambda (buffer alist)
                   (push
                    (list 'side buffer (copy-tree alist))
                    calls)
                   'window)))
             (dolist (backend '(browser xwidget eww auto))
               (let ((adoc-preview-backend backend))
                 (adoc--preview-display "preview.html")))
             (let ((renders '("fresh.html" nil))
                   displayed)
               (cl-letf
                   (((symbol-function
                      'adoc--asciidoctor-render-preview)
                     (lambda () (pop renders)))
                    ((symbol-function 'adoc--preview-display)
                     (lambda (file) (push file displayed))))
                 (adoc--preview-update)
                 (adoc--preview-update))
               (list
                (nreverse calls)
                (nreverse displayed)
                (let ((adoc-preview-backend 'unknown))
                  (condition-case error
                      (adoc--preview-display "preview.html")
                    (error
                     (list (car error) (cadr error))))))))))"##;
    let expect = expect![[
        r#"OK (((browser "file://preview.html") (xwidget "file://preview.html") (side xwidget-buffer ((side . right) (window-width . 0.5))) (eww "preview.html") (side eww-buffer ((side . right) (window-width . 0.5))) (xwidget "file://preview.html") (side xwidget-buffer ((side . right) (window-width . 0.5)))) ("fresh.html") (error "Unknown ‘adoc-preview-backend’: unknown"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_flymake_process_lifecycle_reports_kills_obsolete_and_cancels_dead_source() {
    let elisp_form = r##"(let ((real-make-process
                (symbol-function 'make-process))
               (real-kill-process
                (symbol-function 'kill-process))
               (scripts
                '("cat >/dev/null; printf 'asciidoctor: ERROR: <stdin>: line 2: deterministic failure\n'; exit 1"
                  "cat >/dev/null; sleep 0.2; exit 0"
                  "cat >/dev/null; exit 0"
                  "cat >/dev/null; sleep 0.1; exit 0"))
               requested
               (kill-count 0))
         (cl-labels
             ((await
               (process)
               (let ((tries 0))
                 (while (and (process-live-p process)
                             (< tries 100))
                   (accept-process-output process 0.05)
                   (setq tries (1+ tries)))
                 (accept-process-output process 0.05))))
           (cl-letf
               (((symbol-function 'executable-find)
                 (lambda (command)
                   (and (equal command "asciidoctor")
                        "/tools/asciidoctor")))
                ((symbol-function 'make-process)
                 (lambda (&rest arguments)
                   (push
                    (copy-sequence
                     (plist-get arguments :command))
                    requested)
                   (apply
                    real-make-process
                    (plist-put
                     (copy-sequence arguments)
                     :command
                     (list "sh" "-c" (pop scripts))))))
                ((symbol-function 'kill-process)
                 (lambda (process &optional current-group)
                   (setq kill-count (1+ kill-count))
                   (funcall
                    real-kill-process process current-group))))
             (let ((adoc-asciidoctor-extra-args
                    '("-a" "strict"))
                   first-report
                   replacement-reports
                   killed-source-called
                   killed-source-process)
               (with-temp-buffer
                 (insert "one\ntwo\n")
                 (adoc-mode)
                 (adoc-flymake
                  (lambda (diagnostics &rest _)
                    (setq first-report diagnostics)))
                 (await adoc--flymake-proc))
               (with-temp-buffer
                 (insert "= Doc\n")
                 (adoc-mode)
                 (adoc-flymake
                  (lambda (diagnostics &rest _)
                    (push (list 'obsolete diagnostics)
                          replacement-reports)))
                 (adoc-flymake
                  (lambda (diagnostics &rest _)
                    (push (list 'current diagnostics)
                          replacement-reports)))
                 (await adoc--flymake-proc))
               (let ((source
                      (generate-new-buffer
                       " *adoc-killed-source*")))
                 (with-current-buffer source
                   (insert "= Doc\n")
                   (adoc-mode)
                   (adoc-flymake
                    (lambda (&rest _)
                      (setq killed-source-called t)))
                   (setq killed-source-process
                         adoc--flymake-proc))
                 (kill-buffer source)
                 (await killed-source-process))
               (list
                (mapcar
                 (lambda (command)
                   (list
                    (seq-take command 4)
                    (equal
                     (nth 4 command)
                     (expand-file-name default-directory))
                    (seq-drop command 5)))
                 (nreverse requested))
                (mapcar
                 (lambda (diagnostic)
                   (list
                    (flymake-diagnostic-beg diagnostic)
                    (flymake-diagnostic-end diagnostic)
                    (flymake-diagnostic-type diagnostic)
                    (flymake-diagnostic-text diagnostic)))
                 first-report)
                kill-count
                (nreverse replacement-reports)
                killed-source-called
                (process-live-p killed-source-process)
                (buffer-live-p
                 (process-buffer killed-source-process)))))))"##;
    let expect = expect![[
        r#"OK (((("asciidoctor" "-a" "strict" "-B") t ("-o" "/dev/null" "-")) (("asciidoctor" "-a" "strict" "-B") t ("-o" "/dev/null" "-")) (("asciidoctor" "-a" "strict" "-B") t ("-o" "/dev/null" "-")) (("asciidoctor" "-a" "strict" "-B") t ("-o" "/dev/null" "-"))) ((5 8 :error "deterministic failure")) 1 ((current nil)) nil nil nil)"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}
