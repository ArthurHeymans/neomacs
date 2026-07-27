use expect_test::expect;

use super::{
    assert_arxiv_mode_autoload_parity, assert_arxiv_mode_parity, assert_arxiv_mode_vars_parity,
};

#[test]
fn descriptor_records_exact_pin_dependency_and_installed_payload() {
    let elisp_form = r##"(let* ((desc (cadr (assq 'arxiv-mode package-alist)))
              (dir (package-desc-dir desc)))
         (list
          (package-version-join (package-desc-version desc))
          (package-desc-reqs desc)
          (package-desc-kind desc)
          (sort
           (mapcar #'file-name-nondirectory
                   (directory-files dir t "^[^.].*"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK ("20240111.2203" ((emacs (27 1)) (hydra (0))) nil ("README-elpa" "arxiv-mode-autoloads.el" "arxiv-mode-pkg.el" "arxiv-mode.el" "arxiv-mode.elc" "arxiv-query.el" "arxiv-query.elc" "arxiv-vars.el" "arxiv-vars.elc"))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn all_three_source_files_have_exact_content_hashes_and_features() {
    let elisp_form = r##"(mapcar
         (lambda (library)
           (let ((source (locate-library library)))
             (list library
                   (file-name-nondirectory source)
                   (with-temp-buffer
                     (set-buffer-multibyte nil)
                     (insert-file-contents-literally source)
                     (secure-hash 'sha256 (current-buffer)))
                   (featurep (intern library)))))
         '("arxiv-mode" "arxiv-query" "arxiv-vars"))"##;
    let expect = expect![[
        r#"OK (("arxiv-mode" "arxiv-mode.el" "dcf1173d69914804f24f738985125ccdf8f78a47d3a39f7af8b12252bcc6a028" t) ("arxiv-query" "arxiv-query.el" "27fae6dd2b24d6f1315dd610cd238f344eb2862a313e6b4e57377a898f12c776" t) ("arxiv-vars" "arxiv-vars.el" "4152f167b8e1457b8f3b0d185d48f5a23b16b6f2fec4c05d5412962f57972dc5" t))"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn complete_declared_callable_surface_has_exact_arities_and_command_status() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (help-function-arglist symbol t)
                 (commandp symbol)
                 (macrop symbol)))
         '(arxiv-extract-pdf arxiv-parse-query-data arxiv-get-api-url
           arxiv-geturl-date arxiv-getxml-context arxiv-parse-api
           arxiv-query arxiv-query-sort-cat arxiv-query-general
           arxiv-mode arxiv-abstract-mode arxiv-insert-with-face
           arxiv-next-entry arxiv-prev-entry arxiv-select-entry
           arxiv-click-select-entry arxiv-open-current-url
           arxiv-download-pdf arxiv-customize arxiv-show-abstract
           arxiv-toggle-abstract arxiv-SPC arxiv-exit
           arxiv-headerline-format arxiv-fill-page arxiv-populate-page
           arxiv-show-next-page arxiv-format-abstract-page
           arxiv-export-bibtex-to-string arxiv-export-bibtex
           arxiv-export-bibtex-to-buffer arxiv-download-pdf-export-bibtex
           arxiv-read-new arxiv-read-recent arxiv-read-author
           arxiv-search arxiv-complex-search arxiv-refine-search
           arxiv-query-data-update arxiv-query-order-update
           arxiv-hydra-perform-search arxiv-search-menu/body
           arxiv-search-menu-ex/body arxiv-help-menu/body))"##;
    let expect = expect![
        "OK ((arxiv-extract-pdf (my-list) nil nil) (arxiv-parse-query-data (query-string) nil nil) (arxiv-get-api-url (&optional start) nil nil) (arxiv-geturl-date (date-start date-end category &optional start ascending) nil nil) (arxiv-getxml-context (node child-name) nil nil) (arxiv-parse-api (url) nil nil) (arxiv-query (cat date-start date-end &optional start ascending) nil nil) (arxiv-query-sort-cat (cat) nil nil) (arxiv-query-general (&optional start) nil nil) (arxiv-mode nil t nil) (arxiv-abstract-mode nil t nil) (arxiv-insert-with-face (string face-property) nil nil) (arxiv-next-entry (&optional arg) t nil) (arxiv-prev-entry (&optional arg) t nil) (arxiv-select-entry nil t nil) (arxiv-click-select-entry (ev) t nil) (arxiv-open-current-url nil t nil) (arxiv-download-pdf (&optional confirm) t nil) (arxiv-customize nil t nil) (arxiv-show-abstract nil nil nil) (arxiv-toggle-abstract nil t nil) (arxiv-SPC nil t nil) (arxiv-exit nil t nil) (arxiv-headerline-format nil nil nil) (arxiv-fill-page (&optional min-entry max-entry) nil nil) (arxiv-populate-page nil nil nil) (arxiv-show-next-page nil nil nil) (arxiv-format-abstract-page (entry) nil nil) (arxiv-export-bibtex-to-string (&optional pdfpath) nil nil) (arxiv-export-bibtex (&optional pdfpath) t nil) (arxiv-export-bibtex-to-buffer (&optional pdfpath) t nil) (arxiv-download-pdf-export-bibtex nil t nil) (arxiv-read-new (&optional cat res-time) t nil) (arxiv-read-recent nil t nil) (arxiv-read-author (&optional author) t nil) (arxiv-search nil t nil) (arxiv-complex-search nil t nil) (arxiv-refine-search nil t nil) (arxiv-query-data-update (field condition) nil nil) (arxiv-query-order-update nil nil nil) (arxiv-hydra-perform-search nil t nil) (arxiv-search-menu/body nil t nil) (arxiv-search-menu-ex/body nil t nil) (arxiv-help-menu/body nil t nil))"
    ];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn customization_surface_has_exact_defaults_types_groups_and_docs() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (symbol-value symbol)
                 (get symbol 'custom-type)
                 (get symbol 'custom-group)
                 (documentation-property symbol
                                         'variable-documentation)))
         '(arxiv-pop-up-new-frame arxiv-frame-alist
           arxiv-startup-with-abstract-window arxiv-use-variable-pitch
           arxiv-entries-per-fetch arxiv-author-list-maximum
           arxiv-default-category arxiv-default-download-folder
           arxiv-default-bibliography arxiv-pdf-open-function))"##;
    let expect = expect![[
        r#"OK ((arxiv-pop-up-new-frame t boolean nil "Whether to start ‘arxiv-mode’ with a new pop-up frame.") (arxiv-frame-alist ((name . "*arXiv*") (width . 240) (height . 80)) sexp nil "The alist containing the property of arXiv pop-up frame.") (arxiv-startup-with-abstract-window nil boolean nil "Whether to start ‘arxiv-mode’ with an abstract window.") (arxiv-use-variable-pitch nil boolean nil "Whether to use variable pitch fonts in ‘arxiv-mode’ buffers.") (arxiv-entries-per-fetch 100 integer nil "Number of entries per page in the article list.") (arxiv-author-list-maximum 10 integer nil "Maximum number of authors shown per entry on the article list.\n0 means no maximum limit.") (arxiv-default-category "hep-th" string nil "Default search category when using ‘arxiv-read’.") (arxiv-default-download-folder "~/Downloads/" string nil "Default download folder to save PDF file.") (arxiv-default-bibliography "" string nil "Default master bibliography file to append for ‘arxiv-mode’.") (arxiv-pdf-open-function find-file-other-window function nil "Default function to open PDF file."))"#
    ]];
    assert_arxiv_mode_vars_parity(elisp_form, expect);
}

#[test]
fn state_variables_start_with_exact_values_and_documentation() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (symbol-value symbol)
                 (documentation-property symbol
                                         'variable-documentation)))
         '(arxiv-frame arxiv-buffer arxiv-abstract-window
           arxiv-abstract-buffer arxiv-highlight-overlay arxiv-entry-list
           arxiv-current-entry arxiv-query-results-min
           arxiv-query-results-max arxiv-query-data-list
           arxiv-query-sorting arxiv-order-info arxiv-query-info
           arxiv-mode-entry-function))"##;
    let expect = expect![[
        r#"OK ((arxiv-frame nil "Current frame accommodating ‘arxiv-mode’.\nOnly used when ‘arxiv-pop-up-new-frame’ is set to t.") (arxiv-buffer nil "Current buffer for viewing arXiv updates.") (arxiv-abstract-window nil "Current window for viewing the arXiv abstract.") (arxiv-abstract-buffer nil "Current buffer for viewing the arXiv abstract.") (arxiv-highlight-overlay nil "Overlay for displaying the selected article in arXiv article list.") (arxiv-entry-list nil "Entries for arXiv articles.") (arxiv-current-entry nil "Current entry in the arXiv article list.") (arxiv-query-results-min nil "Current minimun entry of query result.") (arxiv-query-results-max nil "Current maxmum entry of query result.") (arxiv-query-data-list nil "List of current query data.\nElements of this list must have the form (field condition context)\nAvailable fields are ’all, ’id, ’time, ’title, ’author, ’abstract,\n’comment, ’journal and ’category.\nIf condition is nil then the the search excludes the context and vice versa.\ncontext is a string seperated by quotes and spaces.") (arxiv-query-sorting nil "A plist indicating how to sort arxiv query results.\n:sortby is one of ‘relevance’, ‘lastUpdatedDate’ or ‘submittedDate’;\n:sortorder either ‘ascending’ or ‘descending’.") (arxiv-order-info "Default" "The string giving the current sorting for arxiv query.") (arxiv-query-info "" "A string containing the information of query data displayed in the header line.") (arxiv-mode-entry-function nil "Variables showing the entry function used to enter ‘arxiv-mode’."))"#
    ]];
    assert_arxiv_mode_vars_parity(elisp_form, expect);
}

#[test]
fn face_surface_has_exact_specs_and_documentation() {
    let elisp_form = r##"(mapcar
         (lambda (face)
           (list face
                 (get face 'face-defface-spec)
                 (face-documentation face)))
         '(arxiv-title-face arxiv-keyword-face arxiv-author-face
           arxiv-date-face arxiv-abstract-face arxiv-abstract-title-face
           arxiv-abstract-author-face arxiv-subfield-face
           arxiv-abstract-math-face))"##;
    let expect = expect![[
        r#"OK ((arxiv-title-face ((t (:inherit font-lock-keyword-face :height 1.2))) "Face name for article titles in the arXiv article list.") (arxiv-keyword-face ((t (:inherit font-lock-constant-face))) "Face name for keywords in the arXiv article list.") (arxiv-author-face ((t (:inherit font-lock-type-face))) "Face name for authors in the arXiv article list.") (arxiv-date-face ((t (:inherit shadow))) "Face name for date in the arXiv article list.") (arxiv-abstract-face ((t (:inherit font-lock-doc-face))) "Face name for abstract in the arXiv abstract viewing window.") (arxiv-abstract-title-face ((t (:inherit font-lock-keyword-face :height 1.5 :weight semi-bold :underline t))) "Face name for title in the arXiv abstract viewing window.") (arxiv-abstract-author-face ((t (:inherit font-lock-type-face :height 1.2 :underline t))) "Face name for authors in the arXiv abstract viewing window.") (arxiv-subfield-face ((t (:inherit default))) "Face name for subfields (comments, subjects, etc.) in the arXiv abstract viewing window.") (arxiv-abstract-math-face ((t (:inherit font-lock-reference-face :family "Monospace"))) "Face name for the latex content in abstract in the arXiv abstract viewing window."))"#
    ]];
    assert_arxiv_mode_vars_parity(elisp_form, expect);
}

#[test]
fn categories_classifications_and_prettify_tables_have_stable_real_entries() {
    let elisp_form = r##"(list
         (length arxiv-categories)
         (seq-take arxiv-categories 8)
         (seq-take (reverse arxiv-categories) 8)
         (mapcar (lambda (category)
                   (cons category
                         (alist-get category
                                    arxiv-subject-classifications)))
                 '(cs.AI cs.LG math.NT astro-ph.CO quant-ph stat.ML))
         (length arxiv-subject-classifications)
         (length arxiv-abstract-prettify-symbols-alist)
         (mapcar (lambda (source)
                   (cons source
                         (alist-get source
                                    arxiv-abstract-prettify-symbols-alist
                                    nil nil #'equal)))
                 '("\\alpha" "\\Longrightarrow" "\\mathbb{R}"
                   "\\Bbb{R}" "---")))"##;
    let expect = expect![[
        r#"OK (166 (CoRR cs.AI cs.AR cs.CC cs.CE cs.CG cs.CL cs.CR) (stat.TH stat.OT stat.ML stat.ME stat.CO stat.AP stat q-fin.TR) ((cs.AI . "Artificial Intelligence") (cs.LG . "Machine Learning") (math.NT . "Number Theory") (astro-ph.CO . "Cosmology and Nongalactic Astrophysics") (quant-ph . "Quantum Physics") (stat.ML . "Machine Learning")) 155 448 (("\\alpha" . 945) ("\\Longrightarrow" . 8658) ("\\mathbb{R}") ("\\Bbb{R}" . 8477) ("---" . 8212)))"#
    ]];
    assert_arxiv_mode_vars_parity(elisp_form, expect);
}

#[test]
fn keymaps_modes_and_syntax_table_expose_the_documented_interaction_contract() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (key)
            (cons key (lookup-key arxiv-mode-map (kbd key))))
          '("p" "n" "RET" "SPC" "d" "e" "b" "B" "r" "q" "?"
            "<mouse-1>"))
         (mapcar
          (lambda (key)
            (cons key (lookup-key arxiv-abstract-mode-map (kbd key))))
          '("RET" "SPC" "d" "e" "b" "B" "q"))
         (with-temp-buffer
           (arxiv-mode)
           (list major-mode mode-name buffer-read-only
                 (overlayp arxiv-highlight-overlay)
                 header-line-format))
         (with-temp-buffer
           (arxiv-abstract-mode)
           (list major-mode mode-name buffer-read-only))
         (with-syntax-table arxiv-abstract-syntax-table
           (char-syntax ?$)))"##;
    let expect = expect![[
        r#"OK ((("p" . arxiv-prev-entry) ("n" . arxiv-next-entry) ("RET" . arxiv-open-current-url) ("SPC" . arxiv-SPC) ("d" . arxiv-download-pdf) ("e" . arxiv-download-pdf-export-bibtex) ("b" . arxiv-export-bibtex) ("B" . arxiv-export-bibtex-to-buffer) ("r" . arxiv-refine-search) ("q" . arxiv-exit) ("?" . arxiv-help-menu/body) ("<mouse-1>" . arxiv-click-select-entry)) (("RET" . arxiv-open-current-url) ("SPC" . arxiv-toggle-abstract) ("d" . arxiv-download-pdf) ("e" . arxiv-download-pdf-export-bibtex) ("b" . arxiv-export-bibtex) ("B" . arxiv-export-bibtex-to-buffer) ("q" . arxiv-exit)) (arxiv-mode "arXiv" t t (:eval (arxiv-headerline-format))) (arxiv-abstract-mode "arXiv-abstract" t) 41)"#
    ]];
    assert_arxiv_mode_parity(elisp_form, expect);
}

#[test]
fn autoload_file_exposes_only_the_documented_entry_commands() {
    let elisp_form = r##"(list
         (featurep 'arxiv-mode)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (autoloadp (symbol-function symbol))
                  (commandp symbol)))
          '(arxiv-read-new arxiv-read-recent arxiv-read-author
            arxiv-search arxiv-complex-search arxiv-query
            arxiv-export-bibtex-to-string)))"##;
    let expect = expect![
        "OK (nil ((arxiv-read-new t t t) (arxiv-read-recent t t t) (arxiv-read-author t t t) (arxiv-search t t t) (arxiv-complex-search t t t) (arxiv-query nil nil nil) (arxiv-export-bibtex-to-string nil nil nil)))"
    ];
    assert_arxiv_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn repeated_source_loading_is_idempotent_for_features_maps_and_commands() {
    let elisp_form = r##"(let ((source (locate-library "arxiv-mode"))
               snapshots)
         (dotimes (_ 3)
           (load source nil 'nomessage)
           (push
            (list
             (cl-count 'arxiv-mode features)
             (lookup-key arxiv-mode-map (kbd "n"))
             (help-function-arglist 'arxiv-search t)
             (commandp 'arxiv-search-menu/body))
            snapshots))
         (list (cl-count 'arxiv-mode features)
               (and (equal (nth 0 snapshots) (nth 1 snapshots))
                    (equal (nth 1 snapshots) (nth 2 snapshots)))))"##;
    let expect = expect!["OK (1 t)"];
    assert_arxiv_mode_parity(elisp_form, expect);
}
