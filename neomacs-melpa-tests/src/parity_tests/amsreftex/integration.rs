use expect_test::expect;

use super::assert_amsreftex_parity;

#[test]
fn amsreftex_parse_from_file_marks_real_documents_and_records_ltb_databases() {
    let elisp_form = r##"(let* ((buffer
                          (generate-new-buffer
                           " *amsreftex-document*"))
               (file
                (expand-file-name
                 "tmp/melpa-parity/amsreftex/article.tex"
                 (getenv
                  "CARGO_WORKSPACE_DIR")))
               killed)
         (unwind-protect
             (progn
               (with-current-buffer buffer
                 (latex-mode)
                 (reftex-mode 1)
                 (setq buffer-file-name file)
                 (insert
                  "\\documentclass{article}\n"
                  "\\begin{document}\n"
                  "\\bibselect{primary,secondary}\n"
                  "\\end{document}\n"))
               (cl-letf
                   (((symbol-function
                      'reftex-locate-file)
                     (lambda (&rest _)
                       file))
                    ((symbol-function
                      'reftex-get-file-buffer-force)
                     (lambda (&rest _)
                       buffer))
                    ((symbol-function
                      'reftex-everything-regexp)
                     (lambda ()
                       "\\`NEVER-MATCH\\'"))
                    ((symbol-function
                      'reftex-kill-temporary-buffers)
                     (lambda (candidate)
                       (setq killed candidate)))
                    ((symbol-function
                      'amsreftex-locate-bibliography-files)
                     (lambda (master &optional files)
                       (list
                        master files
                        "primary.ltb"
                        "secondary.ltb"))))
                 (let ((reftex-keep-temporary-buffers
                        t))
                   (list
                    (amsreftex-parse-from-file
                     file nil
                     "/project/")
                    killed
                    (buffer-live-p buffer)))))
           (when
               (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK (((eof "[ORACLE-WORKSPACE]/tmp/melpa-parity/amsreftex/article.tex") (bib "/project/" nil "primary.ltb" "secondary.ltb") (database . "amsrefs") (bof "[ORACLE-WORKSPACE]/tmp/melpa-parity/amsreftex/article.tex")) (:buffer nil) t)"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}

#[test]
fn amsreftex_database_selection_dispatches_real_amsrefs_and_bibtex_entries() {
    let elisp_form = r##"(let ((reftex-call-back-to-this-buffer
                        (current-buffer))
                       calls)
         (cl-letf
             (((symbol-function
                'reftex-bib-or-thebib)
               (lambda ()
                 'bib))
              ((symbol-function
                'reftex-get-bibfile-list)
               (lambda ()
                 '("one.ltb"
                   "two.ltb")))
              ((symbol-function
                'reftex-visited-files)
               (lambda (files)
                 (list
                  'visited
                  files)))
              ((symbol-function
                'amsreftex-pop-to-database-entry)
               (lambda (&rest arguments)
                 (push
                  (cons
                   'amsrefs arguments)
                  calls)))
              ((symbol-function
                'reftex-pop-to-bibtex-entry)
               (lambda (&rest arguments)
                 (push
                  (cons
                   'bibtex arguments)
                  calls))))
           (amsreftex-database-selection-callback
            '(("&key" . "ams-key")
              ("&entry" .
               "\\bib{ams-key}{article}{title={A}}"))
            nil nil)
           (amsreftex-database-selection-callback
            '(("&key" . "bib-key")
              ("&entry" .
               "@article{bib-key,title={B}}"))
            nil t)
           (nreverse calls)))"##;
    let expect = expect![[
        r#"OK ((amsrefs "ams-key" #1=("one.ltb" "two.ltb") nil t nil) (bibtex "bib-key" (visited #1#) nil t nil))"#
    ]];
    assert_amsreftex_parity(elisp_form, expect);
}
