use expect_test::expect;

use super::assert_ag_parity;

/// The plain search: `M-x ag` over a real tree.  Pins the exact argument vector
/// the silver searcher was given, the results buffer's generated name and mode,
/// and the fully rendered buffer -- ag.el's filter has turned the colour escapes
/// into `File:` headers and stripped the rest, and the echoed command line shows
/// how the arguments were shell-quoted.
#[test]
fn a_plain_search_builds_the_argv_and_renders_grouped_results() {
    let elisp_form = r##"(ag-test-with-project
 (ag "Grüße" ag-test-project)
 (ag-test-wait-for-search)
 (list :calls (ag-test-calls) :rendered (ag-test-rendered)))"##;

    let expect = expect![[
        r#"OK (:calls (("--literal" "--group" "--line-number" "--column" "--color" "--color-match" "30;43" "--color-path" "1;32" "--smart-case" "--stats" "--" "Grüße" ".")) :rendered (:name "*ag search text:Grüße dir:[ORACLE-SANDBOX]/project/*" :mode ag-mode :text "-*- mode: ag; default-directory: \"<ROOT>/project/\" -*-\n<STATUS>\n\n<ROOT>/bin/ag --literal --group --line-number --column --color --color-match 30\\;43 --color-path 1\\;32 --smart-case --stats -- Gr\\ü\\ße .\nFile: src/greeting.el\n1:4:;; Grüße an alle\n2:24:(defun greet () \"Grüße\")\n\nFile: docs/design notes.md\n3:9:We say Grüße in the greeting module.\n\nFile: README.md\n1:1:Grüße everyone.\n\n<STATUS>\n"))"#
    ]];

    assert_ag_parity(elisp_form, expect);
}

/// Walking the results.  Four successive `compilation-next-error` /
/// `compile-goto-error` pairs land on the exact file, line, column and line text
/// of every match, including the one in `docs/design notes.md` -- a path with a
/// space in it, which only resolves if the command line was quoted correctly and
/// the grouped-output filename is matched backwards from the line number.
#[test]
fn visiting_each_match_lands_on_the_exact_file_line_and_column() {
    let elisp_form = r##"(ag-test-with-project
 (ag "Grüße" ag-test-project)
 (ag-test-wait-for-search)
 (let ((results (ag-test-results-buffer)) hops)
   (with-current-buffer results (goto-char (point-min)))
   (dotimes (_ 4)
     (push (with-current-buffer results
             (compilation-next-error 1)
             (compile-goto-error)
             (with-current-buffer (window-buffer (selected-window))
               (list (if (buffer-file-name)
                         (file-relative-name (buffer-file-name) ag-test-project)
                       (buffer-name))
                     (line-number-at-pos) (current-column)
                     (buffer-substring-no-properties
                      (line-beginning-position) (line-end-position)))))
           hops))
   (list :hops (reverse hops)
         :results-mode (with-current-buffer results major-mode)
         :next-error-fn (with-current-buffer results next-error-function)
         :error-alist (with-current-buffer results compilation-error-regexp-alist))))"##;

    let expect = expect![[
        r#"OK (:hops (("src/greeting.el" 1 3 ";; Grüße an alle") ("src/greeting.el" 2 23 "(defun greet () \"Grüße\")") ("docs/design notes.md" 3 8 "We say Grüße in the greeting module.") ("README.md" 1 0 "Grüße everyone.")) :results-mode ag-mode :next-error-fn ag/next-error-function :error-alist (compilation-ag-nogroup compilation-ag-group))"#
    ]];

    assert_ag_parity(elisp_form, expect);
}

/// The four documented entry points produce four different command lines:
/// `ag-regexp` drops `--literal`, `ag-files` with a known type adds `--elisp`,
/// `ag-files` with a PCRE adds `--file-search-regex`, and `ag-project` locates
/// the repository root from a subdirectory and searches there.  The generated
/// buffer names distinguish a literal search from a regexp one.
#[test]
fn each_search_command_produces_its_own_argv_and_buffer_name() {
    let elisp_form = r##"(ag-test-with-project
 (ag-regexp "Gr[uü][ßs]" ag-test-project)
 (ag-test-wait-for-search)
 (ag-files "Grüße" (list :file-type "elisp") ag-test-project)
 (ag-test-wait-for-search)
 (ag-files "Grüße" (list :file-regex "\\.md$") ag-test-project)
 (ag-test-wait-for-search)
 (let ((default-directory (file-name-as-directory
                           (expand-file-name "src" ag-test-project))))
   (ag-project "Grüße")
   (ag-test-wait-for-search)
   (list :calls (ag-test-calls)
         :project-root (ag/project-root default-directory)
         :buffers (sort (delq nil (mapcar (lambda (b)
                                            (and (string-prefix-p "*ag search" (buffer-name b))
                                                 (buffer-name b)))
                                          (buffer-list)))
                        #'string<))))"##;

    let expect = expect![[
        r#"OK (:calls (("--group" "--line-number" "--column" "--color" "--color-match" "30;43" "--color-path" "1;32" "--smart-case" "--stats" "--" "Gr[uü][ßs]" ".") ("--elisp" "--literal" "--group" "--line-number" "--column" "--color" "--color-match" "30;43" "--color-path" "1;32" "--smart-case" "--stats" "--" "Grüße" ".") ("--file-search-regex" "\\.md$" "--literal" "--group" "--line-number" "--column" "--color" "--color-match" "30;43" "--color-path" "1;32" "--smart-case" "--stats" "--" "Grüße" ".") ("--literal" "--group" "--line-number" "--column" "--color" "--color-match" "30;43" "--color-path" "1;32" "--smart-case" "--stats" "--" "Grüße" ".")) :project-root "[ORACLE-SANDBOX]/project/" :buffers ("*ag search regexp:Gr[uü][ßs] dir:[ORACLE-SANDBOX]/project/*" "*ag search text:Grüße dir:[ORACLE-SANDBOX]/project/*"))"#
    ]];

    assert_ag_parity(elisp_form, expect);
}

/// The customizations a user actually sets: `ag-ignore-list` and
/// `ag-context-lines` add arguments, `ag-group-matches` nil switches ag to
/// `--nogroup` and changes the rendering from `File:` headers to per-line paths,
/// and `ag-highlight-search` makes the filter propertize the matched text with
/// `ag-match-face` instead of merely deleting the escape sequence.
#[test]
fn customizations_change_both_the_argv_and_the_rendering() {
    let elisp_form = r##"(ag-test-with-project
 (let ((ag-group-matches nil)
       (ag-context-lines 2)
       (ag-ignore-list '("vendor" "node_modules"))
       (ag-highlight-search t))
   (ag "Grüße" ag-test-project)
   (ag-test-wait-for-search)
   (let ((rendered (ag-test-rendered))
         (faces (with-current-buffer (ag-test-results-buffer)
                  (goto-char (point-min))
                  (when (search-forward "Grüße" nil t)
                    (list (get-text-property (match-beginning 0) 'font-lock-face)
                          (buffer-substring-no-properties
                           (line-beginning-position) (line-end-position)))))))
     (list :calls (ag-test-calls) :rendered rendered :match-face faces))))"##;

    let expect = expect![[
        r#"OK (:calls (("--ignore" "vendor" "--ignore" "node_modules" "--context=2" "--literal" "--nogroup" "--line-number" "--column" "--color" "--color-match" "30;43" "--color-path" "1;32" "--smart-case" "--stats" "--" "Grüße" ".")) :rendered (:name "*ag search text:Grüße dir:[ORACLE-SANDBOX]/project/*" :mode ag-mode :text "-*- mode: ag; default-directory: \"<ROOT>/project/\" -*-\n<STATUS>\n\n<ROOT>/bin/ag --ignore vendor --ignore node_modules --context\\=2 --literal --nogroup --line-number --column --color --color-match 30\\;43 --color-path 1\\;32 --smart-case --stats -- Gr\\ü\\ße .\nsrc/greeting.el\n1:4:;; Grüße an alle\n2:24:(defun greet () \"Grüße\")\n\ndocs/design notes.md\n3:9:We say Grüße in the greeting module.\n\nREADME.md\n1:1:Grüße everyone.\n\n<STATUS>\n") :match-face (ag-match-face "1:4:;; Grüße an alle"))"#
    ]];

    assert_ag_parity(elisp_form, expect);
}

/// The two ways a search comes back empty-handed.  ag exits 1 with no output
/// when nothing matched, and non-zero with a message on stderr when it rejects
/// the command line; both still produce a rendered `ag-mode` buffer, and the
/// second shows the tool's complaint.
#[test]
fn a_search_with_no_matches_and_a_failing_search_are_both_rendered() {
    let elisp_form = r##"(ag-test-with-project
 (let ((none (progn (ag "NOTHING" ag-test-project)
                    (ag-test-wait-for-search)
                    (ag-test-rendered))))
   (dolist (b (buffer-list))
     (when (string-prefix-p "*ag search" (buffer-name b))
       (let ((kill-buffer-query-functions nil)) (kill-buffer b))))
   (ag "EXPLODE" ag-test-project)
   (ag-test-wait-for-search)
   (list :none none :failed (ag-test-rendered) :calls (ag-test-calls))))"##;

    let expect = expect![[
        r#"OK (:none (:name "*ag search text:NOTHING dir:[ORACLE-SANDBOX]/project/*" :mode ag-mode :text "-*- mode: ag; default-directory: \"<ROOT>/project/\" -*-\n<STATUS>\n\n<ROOT>/bin/ag --literal --group --line-number --column --color --color-match 30\\;43 --color-path 1\\;32 --smart-case --stats -- NOTHING .\n\n<STATUS>\n") :failed (:name "*ag search text:EXPLODE dir:[ORACLE-SANDBOX]/project/*" :mode ag-mode :text "-*- mode: ag; default-directory: \"<ROOT>/project/\" -*-\n<STATUS>\n\n<ROOT>/bin/ag --literal --group --line-number --column --color --color-match 30\\;43 --color-path 1\\;32 --smart-case --stats -- EXPLODE .\nag: unknown option\n\n<STATUS>\n") :calls (("--literal" "--group" "--line-number" "--column" "--color" "--color-match" "30;43" "--color-path" "1;32" "--smart-case" "--stats" "--" "NOTHING" ".") ("--literal" "--group" "--line-number" "--column" "--color" "--color-match" "30;43" "--color-path" "1;32" "--smart-case" "--stats" "--" "EXPLODE" ".")))"#
    ]];

    assert_ag_parity(elisp_form, expect);
}
