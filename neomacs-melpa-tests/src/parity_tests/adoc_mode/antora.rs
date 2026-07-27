use expect_test::expect;

use super::assert_adoc_mode_parity;

#[test]
fn adoc_mode_antora_layout_detection_page_resolution_and_target_forms_match() {
    let elisp_form = r##"(let* ((root (make-temp-file "adoc-antora-" t))
                (pages (expand-file-name "modules/ROOT/pages" root))
                (source (expand-file-name "sub/source.adoc" pages)))
         (unwind-protect
             (progn
               (make-directory (file-name-directory source) t)
               (with-temp-file (expand-file-name "antora.yml" root)
                 (insert "name: demo\nversion: ~\n"))
               (with-temp-file source (insert "= Source\n"))
               (with-current-buffer (find-file-noselect source)
                 (unwind-protect
                     (list
                      (file-relative-name (adoc--antora-root) root)
                      (adoc--antora-p)
                      (adoc--antora-current-module root source)
                      (mapcar
                       (lambda (target)
                         (let ((resolved
                                (adoc--antora-resolve-page target)))
                           (and resolved
                                (list
                                 (file-relative-name (car resolved) root)
                                 (cdr resolved)))))
                       '("basics/install.adoc"
                         "p.adoc#frag"
                         "other:p.adoc"
                         "2.0@other:p.adoc"
                         "pa@ge.adoc"
                         "p.adoc#"
                         "comp:mod:p.adoc"))
                      (adoc--antora-current-page-targets)
                      (adoc--section-id-params))
                   (kill-buffer))))
           (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK ("./" t "ROOT" (("modules/ROOT/pages/basics/install.adoc" nil) ("modules/ROOT/pages/p.adoc" "frag") ("modules/other/pages/p.adoc" nil) ("modules/other/pages/p.adoc" nil) ("modules/ROOT/pages/pa@ge.adoc" nil) ("modules/ROOT/pages/p.adoc" nil) nil) ("sub/source.adoc" "ROOT:sub/source.adoc") ("" . "-"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_antora_page_and_fragment_completion_filters_hidden_and_lock_files() {
    let elisp_form = r##"(let* ((root (make-temp-file "adoc-antora-" t))
                (root-pages
                 (expand-file-name "modules/ROOT/pages" root))
                (extra-pages
                 (expand-file-name "modules/extra/pages" root))
                (source (expand-file-name "source.adoc" root-pages)))
         (unwind-protect
             (progn
               (make-directory (expand-file-name "sub" root-pages) t)
               (make-directory (expand-file-name ".hidden" root-pages) t)
               (make-directory extra-pages t)
               (with-temp-file (expand-file-name "antora.yml" root)
                 (insert "name: demo\n"))
               (with-temp-file source
                 (insert "= Source\n\n== Local Section\n"))
               (with-temp-file
                   (expand-file-name "sub/target.adoc" root-pages)
                 (insert "= Target\n\n== Deep Section\n\n[[explicit]]\n"))
               (with-temp-file
                   (expand-file-name ".hidden/buried.adoc" root-pages)
                 (insert "= Hidden\n"))
               (with-temp-file
                   (expand-file-name ".#busy.adoc" root-pages)
                 (insert "= Lock\n"))
               (with-temp-file
                   (expand-file-name "other.adoc" extra-pages)
                 (insert "= Other\n"))
               (with-current-buffer (find-file-noselect source)
                 (unwind-protect
                     (progn
                       (adoc-mode)
                       (cl-labels
                           ((complete
                             (text)
                             (save-excursion
                               (goto-char (point-max))
                               (insert "\n" text)
                               (let* ((capf (adoc-completion-at-point))
                                      (start (nth 0 capf))
                                      (end (nth 1 capf))
                                      (table (nth 2 capf)))
                                 (sort
                                  (all-completions
                                   (buffer-substring-no-properties
                                    start end)
                                   table)
                                  #'string<)))))
                         (list
                          (sort (adoc--antora-page-targets)
                                #'string<)
                          (sort
                           (adoc--antora-page-fragments
                            "sub/target.adoc")
                           #'string<)
                          (sort
                           (adoc--antora-page-fragments
                            "source.adoc")
                           #'string<)
                          (complete "xref:sub/target.adoc#")
                          (complete "xref:#"))))
                   (kill-buffer))))
           (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("extra:other.adoc" "source.adoc" "sub/target.adoc") ("deep-section" "explicit") ("local-section") ("deep-section" "explicit") ("local-section"))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_antora_page_xref_detection_and_project_reference_search_match() {
    let elisp_form = r##"(let* ((root (make-temp-file "adoc-antora-" t))
                (root-pages
                 (expand-file-name "modules/ROOT/pages" root))
                (extra-pages
                 (expand-file-name "modules/extra/pages" root))
                (guide (expand-file-name "guide.adoc" root-pages)))
         (unwind-protect
             (progn
               (make-directory root-pages t)
               (make-directory extra-pages t)
               (with-temp-file (expand-file-name "antora.yml" root)
                 (insert "name: demo\n"))
               (with-temp-file guide
                 (insert
                  "= Guide\n\n== Deep Section\n\n"
                  "Self <<deep-section>>.\n"))
               (with-temp-file
                   (expand-file-name "intro.adoc" root-pages)
                 (insert
                  "= Intro\n\n"
                  "xref:guide.adoc#deep-section[x].\n"))
               (with-temp-file
                   (expand-file-name "other.adoc" extra-pages)
                 (insert
                  "= Other\n\n"
                  "xref:ROOT:guide.adoc#deep-section[y].\n"))
               (with-temp-file
                   (expand-file-name "noise.adoc" root-pages)
                 (insert
                  "xref:guide.adoc#other[a] "
                  "xref:elsewhere.adoc#deep-section[b]\n"))
               (with-current-buffer (find-file-noselect guide)
                 (unwind-protect
                     (let (detected)
                       (dolist
                           (sample
                            '("xref:sub/target.adoc#sec[label]"
                              "xref:some-section-id[label]"
                              "plain prose"))
                         (with-temp-buffer
                           (insert sample)
                           (adoc-mode)
                           (goto-char (point-min))
                           (search-forward
                            (if (string-prefix-p "xref:" sample)
                                "xref:" "plain"))
                           (push
                            (adoc--antora-page-xref-at-point)
                            detected)))
                       (list
                        (nreverse detected)
                        (mapcar
                         #'xref-item-summary
                         (adoc--antora-references
                          "deep-section"))))
                   (kill-buffer))))
           (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("sub/target.adoc#sec" nil nil) (#("Self <<deep-section>>." 5 20 (face xref-match)) #("xref:guide.adoc#deep-section[x]." 0 29 (face xref-match)) #("xref:ROOT:guide.adoc#deep-section[y]." 0 34 (face xref-match))))"#
    ]];
    assert_adoc_mode_parity(elisp_form, expect);
}

#[test]
fn adoc_mode_follow_antora_page_xrefs_opens_fragment_rejects_missing_and_preserves_plain_fallback()
{
    let elisp_form = r##"(let* ((root
                 (make-temp-file "adoc-antora-follow-" t))
                (plain-root
                 (make-temp-file "adoc-plain-follow-" t))
                (pages
                 (expand-file-name
                  "modules/ROOT/pages" root))
                (source
                 (expand-file-name "src.adoc" pages))
                (target
                 (expand-file-name
                  "sub/target.adoc" pages))
                (plain
                 (expand-file-name "plain.adoc" plain-root)))
         (unwind-protect
             (progn
               (make-directory
                (file-name-directory target) t)
               (with-temp-file
                   (expand-file-name "antora.yml" root)
                 (insert "name: demo\n"))
               (with-temp-file target
                 (insert
                  "= Target\n\n"
                  "== Deep Section\n\n"
                  "body\n"))
               (with-temp-file source
                 (insert
                  "= Source\n\n"
                  "See xref:sub/target.adoc#deep-section[x].\n"
                  "See xref:nope.adoc[x].\n"))
               (with-temp-file plain
                 (insert "See xref:foo.adoc[x].\n"))
               (cl-labels
                   ((follow
                     (file needle)
                     (with-current-buffer
                         (find-file-noselect file)
                       (adoc-mode)
                       (goto-char (point-min))
                       (search-forward needle)
                       (condition-case error
                           (progn
                             (adoc-follow-thing-at-point)
                             (list
                              'ok
                              (file-name-nondirectory
                               buffer-file-name)
                              (buffer-substring-no-properties
                               (line-beginning-position)
                               (line-end-position))))
                         (user-error
                          (list
                           'user-error
                           (and
                            (string-match-p
                             "Antora"
                             (error-message-string error))
                            t)))))))
                 (list
                  (follow source "target.adoc")
                  (follow source "nope.adoc")
                  (follow plain "foo.adoc"))))
           (dolist (buffer (buffer-list))
             (when-let* ((file
                          (buffer-local-value
                           'buffer-file-name buffer)))
               (when (or (string-prefix-p root file)
                         (string-prefix-p plain-root file))
                 (with-current-buffer buffer
                   (set-buffer-modified-p nil))
                 (kill-buffer buffer))))
           (delete-directory root t)
           (delete-directory plain-root t)))"##;
    let expect =
        expect![[r#"OK ((ok "target.adoc" "== Deep Section") (user-error t) (user-error nil))"#]];
    assert_adoc_mode_parity(elisp_form, expect);
}
