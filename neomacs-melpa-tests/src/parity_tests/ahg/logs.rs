use expect_test::expect;

use super::assert_ahg_parity;

#[test]
fn ahg_revision_argument_builder_handles_ranges_disjoint_revisions_numbers_and_empty_values() {
    let elisp_form = r##"(mapcar
                      (lambda (case)
                        (apply #'ahg-args-add-revs case))
                      '((nil nil)
                        ("" "")
                        ("tip" nil)
                        (nil "0")
                        ("tip" "0")
                        ("tip" "0" t)
                        (17 3)
                        (17 3 t)
                        ("branch(default)" nil)
                        ("ancestors(.)" "descendants(0)")))"##;
    let expect = expect![[
        r#"OK (nil nil ("-r" "tip") ("-r" "0") ("-r" "tip:0") ("-r" "tip" "-r" "0") ("-r" "17:3") ("-r" "17" "-r" "3") ("-r" "branch(default)") ("-r" "ancestors(.):descendants(0)"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_log_argument_reader_builds_practical_revsets_limits_reverse_and_extra_flags() {
    let elisp_form = r##"(let (answers prompts)
                      (cl-letf (((symbol-function 'read-string)
                                 (lambda (prompt &optional initial _history default)
                                   (push (list prompt initial default) prompts)
                                   (or (pop answers) default initial ""))))
                        (let ((ahg-log-revrange-size 100)
                              (ahg-log-default-revisions '("tip" . "0"))
                              (ahg-log-default-extra-flags ""))
                          (list
                           (progn
                             (setq answers '("tip" "-25" "--follow --removed"))
                             (ahg-log-read-args t t))
                           (progn
                             (setq answers '("branch(default)" "10"))
                             (ahg-log-read-args nil nil))
                           (progn
                             (setq answers '("17" "3"))
                             (ahg-log-read-args nil nil t))
                           ahg-log-default-revisions
                           ahg-log-default-extra-flags
                           (nreverse prompts)))))"##;
    let expect = expect![[
        r#"OK (("reverse(last(ancestors(tip),25))" nil "--follow --removed") ("reverse(last(branch(default),10))" nil) ("descendants(3) & ancestors(17)" nil) ("17") "--follow --removed" (("hg log (on selected files), R1: " "tip" nil) ("hg log (on selected files), R2: " "-100" "-100") ("hg log (on selected files), extra switches: " "" nil) ("hg log, R1: " "tip" nil) ("hg log, limit (default 100): " nil "100") ("hg log, R1: " "branch(default)" nil) ("hg log, R2: " "-100" "-100")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_short_log_parser_ewoc_navigation_and_revision_lookup_form_a_real_table_session() {
    let elisp_form = r##"(with-temp-buffer
                      (let ((ew (ewoc-create #'ahg-short-log-pp))
                            (contents
                             "12 2026-07-10 alice Add parser support\n11 2026-07-09 verylongauthor Fix λ rendering\n10 2026-07-08 bob Merge branch\n"))
                        (setq ewoc ew)
                        (ahg-short-log-insert-contents ew contents)
                        (goto-char (ewoc-location (ewoc-nth ew 0)))
                        (let ((first (ahg-short-log-revision-at-point)))
                          (ahg-short-log-next 2)
                          (let ((third (ahg-short-log-revision-at-point)))
                            (ahg-short-log-previous 1)
                            (let ((second (ahg-short-log-revision-at-point)))
                              (ahg-short-log-goto-revision "12")
                              (list
                               first second third
                               (ahg-short-log-revision-at-point)
                               (mapcar #'identity
                                       (ewoc-collect ew #'identity))
                               (buffer-substring-no-properties
                                (point-min) (point-max))))))))"##;
    let expect = expect![[
        r#"OK ("12" "11" "10" "12" (("12" "2026-07-10" "alice" "Add parser support") ("11" "2026-07-09" "verylongauthor" "Fix λ rendering") ("10" "2026-07-08" "bob" "Merge branch")) "\n     12 | 2026-07-10 |    alice | Add parser support                            \n     11 | 2026-07-09 | verylong | Fix λ rendering                               \n     10 | 2026-07-08 |      bob | Merge branch                                  \n\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_short_log_implementation_combines_templates_ranges_flags_and_selected_files() {
    let elisp_form = r##"(let (calls)
                      (cl-letf
                          (((symbol-function 'ahg-root)
                            (lambda (&optional _noerror) "/repo/"))
                           ((symbol-function 'ahg-do-short-log)
                            (lambda (&rest arguments)
                              (push arguments calls))))
                        (let ((ahg-file-list-for-log-command
                               '("src/main.el" "notes λ.txt")))
                          (ahg-short-log-impl
                           "custom log"
                           "{desc|firstline}"
                           "Summary"
                           "tip"
                           "0"
                           "--follow --removed"))
                        (let ((ahg-file-list-for-log-command nil))
                          (ahg-short-log "branch(default)" nil nil))
                        (nreverse calls)))"##;
    let expect = expect![[
        r#"OK (("/repo/" "custom log" "Summary" ("-r" "tip:0" "--template" "{rev} {date|shortdate} {author|user} {desc|firstline}\\n" "--follow" "--removed" "src/main.el" "notes λ.txt")) ("/repo/" "hg log (summary)" "Summary" ("-r" "branch(default)" "--template" "{rev} {date|shortdate} {author|user} {desc|firstline}\\n")))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_detailed_log_formatter_transforms_raw_style_output_into_navigable_records() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "17:abc123\n"
                       "r991\n"
                       "deadbeef\n"
                       "draft\n"
                       "feature\n"
                       "tip release\n"
                       "active other\n"
                       "16:def456 15:fed321\n"
                       "Alice Example <alice@example.test>\n"
                       "1783703220 0\n"
                       "src/main.el\nREADME.md\n\n"
                       "\tImplement parser\n\twith multiline details\n\n"
                       "16:def456\n\n\npublic\n\n\n\n15:fed321\n"
                       "Bob\n1783616820 0\nREADME.md\n\n\tInitial docs\n")
                      (ahg-format-log-buffer)
                      (goto-char (point-min))
                      (let ((first-long (ahg-log-revision-at-point))
                            (first-short (ahg-log-revision-at-point t)))
                        (ahg-log-next 1)
                        (list
                         first-long
                         first-short
                         (ahg-log-revision-at-point)
                         (buffer-substring-no-properties
                          (point-min) (point-max)))))"##;
    let expect = expect![[
        r#"OK ("abc123" "17" "def456" "changeset:   17:abc123\nsvn:         r991\ngit:         deadbeef\nphase:       draft\nbranch:      feature\ntag:         tip\ntag:         release\nbookmark:    active\nbookmark:    other\nparent:      16:def456\nparent:      15:fed321\nuser:        Alice Example <alice@example.test>\ndate:        1783703220 0\nfiles:       src/main.el\n             README.md\ndescription:\nImplement parser\nwith multiline details\n\n\n\nchangeset:   16:def456\nparent:      15:fed321\nuser:        Bob\ndate:        1783616820 0\nfiles:       README.md\ndescription:\nInitial docs\n\n\n")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_log_navigation_revision_and_filename_helpers_resolve_real_buffer_context() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox))))
                      (make-directory (expand-file-name ".hg" root) t)
                      (with-temp-buffer
                        (setq default-directory root)
                        (insert
                         "changeset:   7:abc123\n"
                         "files:       src/one.el\n"
                         "             docs/two file.md\n"
                         "description:\nfirst\n\n"
                         "changeset:   6:def456\n"
                         "files:       old.txt\n"
                         "description:\nsecond\n")
                        (goto-char (point-min))
                        (forward-line 2)
                        (let ((relative
                               (ahg-log-filename-at-point (point) t))
                              (absolute
                               (ahg-log-filename-at-point (point))))
                          (ahg-log-next 1)
                          (let ((second (ahg-log-revision-at-point)))
                            (ahg-log-previous 1)
                            (ahg-log-goto-revision "6:def")
                            (list
                             relative
                             absolute
                             second
                             (line-number-at-pos)
                             (ahg-log-revision-at-point t))))))"##;
    let expect = expect![[
        r#"OK ("docs/two file.md" "[ORACLE-SANDBOX]/repo/docs/two file.md" "def456" 7 "6")"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_glog_navigation_extracts_revisions_across_branching_graph_lines() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "@  19  abc  2026-07-10  alice  {work}.\n"
                       "|  newest\n"
                       "|\\\n"
                       "| o  18  def  2026-07-09  bob  (feature).\n"
                       "| |  branch\n"
                       "o |  17  fed  2026-07-08  carol  [v1].\n"
                       "  |  older\n")
                      (goto-char (point-min))
                      (let ((first (ahg-glog-revision-at-point)))
                        (ahg-glog-next 1)
                        (let ((second (ahg-glog-revision-at-point)))
                          (ahg-glog-next 1)
                          (let ((third (ahg-glog-revision-at-point)))
                            (ahg-glog-previous 2)
                            (ahg-glog-goto-revision-line)
                            (list first second third
                                  (line-number-at-pos)
                                  (ahg-glog-revision-at-point))))))"##;
    let expect = expect![[r#"OK ("19" "18" "17" 1 "19")"#]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_log_style_map_is_written_inside_hg_metadata_with_exact_template() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (hg-dir (expand-file-name ".hg" root)))
                      (make-directory hg-dir t)
                      (let ((relative (ahg-log-prepare-style-map root)))
                        (list
                         relative
                         (file-exists-p (expand-file-name relative root))
                         (with-temp-buffer
                           (insert-file-contents
                            (expand-file-name relative root))
                           (buffer-string))
                         (equal
                          (with-temp-buffer
                            (insert-file-contents
                             (expand-file-name relative root))
                            (buffer-string))
                          ahg-log-style-map))))"##;
    let expect = expect![[
        r#"OK (".hg/ahg-log-style-map" t "changeset = \"{rev}:{node|short}\\n{svnrev}\\n{gitnode}\\n{phase}\\n{branches}\\n{tags}\\n{bookmarks}\\n{parents}\\n{author}\\n{date|date}\\n{files}\\n\\t{desc|tabindent}\\n\"\nfile = \"{file}\\n\"\n" t)"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_update_to_revision_resolves_bookmarks_force_flags_and_refresh_callback() {
    let elisp_form = r##"(let* ((sandbox (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
                          (root (file-name-as-directory
                                 (expand-file-name "repo" sandbox)))
                          (callback-buffer
                           (generate-new-buffer " *ahg-update-callback*"))
                          commands messages refreshes killed)
                      (make-directory root t)
                      (cl-letf
                          (((symbol-function 'ahg-get-bookmarks)
                            (lambda (revision)
                              (cond
                               ((equal revision "17") '("release"))
                               ((equal revision "18") '("one" "two"))
                               (t nil))))
                           ((symbol-function 'ahg-generic-command)
                            (lambda (command arguments sentinel &rest _rest)
                              (push (list command arguments) commands)
                              (funcall sentinel 'fake-process "finished\n")
                              'fake-process))
                           ((symbol-function 'process-buffer)
                            (lambda (_process) callback-buffer))
                           ((symbol-function 'kill-buffer)
                            (lambda (&optional buffer)
                              (push (buffer-name (or buffer (current-buffer)))
                                    killed)
                              t))
                           ((symbol-function 'message)
                            (lambda (format-string &rest arguments)
                              (push (apply #'format format-string arguments)
                                    messages))))
                        (let ((default-directory root)
                              (ahg-update-to-rev-check-bookmarks t)
                              (ahg-update-to-rev-refresh-function
                               (lambda (root) (push root refreshes))))
                          (ahg-update-to-rev "17" nil)
                          (ahg-update-to-rev "18" t)
                          (ahg-update-to-rev "19" nil)
                          (ahg-update-to-rev nil t))
                        (list (nreverse commands)
                              (nreverse refreshes)
                              (nreverse messages)
                              (nreverse killed))))"##;
    let expect = expect![[
        r#"OK ((("update" ("release")) ("update" ("-C" "-r" "18")) ("update" ("-r" "19"))) ("[ORACLE-SANDBOX]/repo/" "[ORACLE-SANDBOX]/repo/" "[ORACLE-SANDBOX]/repo/") ("Activating bookmark release" "Updated to revision 17" "Warning: more than 1 bookmark at rev 18, not making any of them active" "Updated to revision 18" "Updated to revision 19") (" *ahg-update-callback*" " *ahg-update-callback*" " *ahg-update-callback*"))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_heads_tags_and_bookmarks_use_real_revsets_and_columns() {
    let elisp_form = r##"(let (calls)
                      (cl-letf
                          (((symbol-function 'ahg-short-log-impl)
                            (lambda (&rest arguments)
                              (push arguments calls))))
                        (ahg-heads)
                        (ahg-tags)
                        (ahg-bookmarks)
                        (nreverse calls)))"##;
    let expect = expect![[
        r#"OK (("hg heads" "{if(tags, '[{tags}]  ')}{if(bookmarks, '\\{{bookmarks}}  ')}{ifeq(branch, 'default', '', '({branch})')}" "[Tags] {Bookmarks} (Branch)" "reverse(head() and tip:0)" nil nil) ("hg tags" "{tags}" "Tag Name" "reverse(tag(\"re:.*\") and tip:0)" nil nil) ("hg bookmarks" "{bookmarks}" "Bookmark Name" "reverse(bookmark() and tip:0)" nil nil))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}

#[test]
fn ahg_revision_process_helpers_parse_tip_range_ids_parents_and_bookmarks() {
    let elisp_form = r##"(let (calls)
                      (cl-letf
                          (((symbol-function 'ahg-call-process)
                            (lambda (command &optional arguments _global)
                              (push (list command arguments) calls)
                              (cond
                               ((equal command "id")
                                (insert "245+\n")
                                0)
                               ((and (equal command "log")
                                     (member "{bookmarks}" arguments))
                                (insert "work release\n")
                                0)
                               ((equal command "log")
                                (insert "abc123 ")
                                0)
                               (t 1)))))
                        (let ((ahg-log-revrange-size 100))
                          (list
                           (ahg-log-revrange-end)
                           (ahg-rev-id "tip")
                           (ahg-rev-id 17 "{rev}")
                           (ahg-first-parent-of-rev "abc123")
                           (ahg-get-bookmarks "tip")
                           (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK ("-100" "abc123" "abc123" "p1(abc123)" ("work" "release") (("id" ("-n")) ("log" ("-r" "tip" "--template" "{node|short} ")) ("log" ("-r" 17 "--template" "{rev} ")) ("log" ("-r" "tip" "--template" "{bookmarks}"))))"#
    ]];
    assert_ahg_parity(elisp_form, expect);
}
