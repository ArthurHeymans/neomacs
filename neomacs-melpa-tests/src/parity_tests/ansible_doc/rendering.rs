use expect_test::expect;

use super::assert_ansible_doc_parity;

#[test]
fn ansible_doc_fontify_module_xrefs_builds_exact_buttons_for_valid_tokens() {
    let elisp_form = r##"(with-temp-buffer
         (insert "See [copy], [community.general.ufw], [bad name], [] and [user].")
         (let ((unrelated (make-overlay 1 4)))
           (overlay-put unrelated 'fixture t)
           (ansible-doc-fontify-module-xrefs (point-min) (point-max))
           (list
            (overlay-buffer unrelated)
            (mapcar
             (lambda (button)
               (list (button-start button)
                     (button-end button)
                     (buffer-substring-no-properties
                      (button-start button) (button-end button))
                     (button-get button 'type)
                     (button-get button 'ansible-module)
                     (button-get button 'action)
                     (button-get button 'face)
                     (button-get button 'help-echo)))
             (sort (cl-remove-if-not
                    (lambda (overlay)
                      (overlay-get overlay 'button))
                    (overlays-in (point-min) (point-max)))
                   (lambda (a b)
                     (< (button-start a) (button-start b))))))))"##;
    let expect = expect![[
        r#"OK (nil ((5 11 "[copy]" ansible-doc-module-xref "copy" ansible-doc-follow-module-xref ansible-doc-module-xref "mouse-2, RET: visit module") (13 36 "[community.general.ufw]" ansible-doc-module-xref "community.general.ufw" ansible-doc-follow-module-xref ansible-doc-module-xref "mouse-2, RET: visit module") (57 63 "[user]" ansible-doc-module-xref "user" ansible-doc-follow-module-xref ansible-doc-module-xref "mouse-2, RET: visit module")))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_fontify_module_xrefs_respects_region_bounds_and_replaces_overlays() {
    let elisp_form = r##"(with-temp-buffer
         (insert "[before] middle [inside] tail [after]")
         (let* ((beg (progn (goto-char (point-min))
                            (search-forward "[inside]")
                            (match-beginning 0)))
                (end (match-end 0))
                (old (make-overlay beg end)))
           (overlay-put old 'old t)
           (ansible-doc-fontify-module-xrefs beg end)
           (list
            (overlay-buffer old)
            (mapcar
             (lambda (button)
               (list (button-start button)
                     (button-end button)
                     (button-get button 'ansible-module)))
             (cl-remove-if-not
              (lambda (overlay)
                (overlay-get overlay 'button))
              (overlays-in (point-min) (point-max)))))))"##;
    let expect = expect![[r#"OK (nil ((17 25 "inside")))"#]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_follow_module_xref_dispatches_exact_button_module() {
    let elisp_form = r##"(with-temp-buffer
         (insert "[community.crypto.openssh_keypair]")
         (make-button (point-min) (point-max)
                      'type 'ansible-doc-module-xref
                      'ansible-module "community.crypto.openssh_keypair")
         (let (calls)
           (cl-letf (((symbol-function 'ansible-doc)
                      (lambda (&rest args)
                        (push args calls)
                        'opened)))
             (let ((button (button-at (point-min))))
               (list (ansible-doc-follow-module-xref button)
                     (nreverse calls))))))"##;
    let expect = expect![[r#"OK (opened (("community.crypto.openssh_keypair")))"#]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_fontify_yaml_without_yaml_mode_returns_same_string_object() {
    let elisp_form = r##"(let ((text (copy-sequence "---\nkey: value\n")))
         (cl-letf (((symbol-function 'yaml-mode) nil))
           (let ((result (ansible-doc-fontify-yaml text)))
             (list (eq text result)
                   (equal text result)
                   result
                   (text-properties-at 0 result)))))"##;
    let expect = expect![[r#"OK (t t "---\nkey: value\n" nil)"#]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_fontify_yaml_converts_face_runs_to_font_lock_face() {
    let elisp_form = r##"(let ((text "alpha beta gamma")
               calls)
         (cl-letf (((symbol-function 'yaml-mode)
                    (lambda ()
                      (push 'yaml-mode calls)
                      (put-text-property 1 6 'face 'font-lock-keyword-face)
                      (put-text-property 7 11 'face 'font-lock-string-face)))
                   ((symbol-function 'font-lock-mode)
                    (lambda (&rest args)
                      (push (cons 'font-lock-mode args) calls)))
                   ((symbol-function 'font-lock-ensure)
                    (lambda (&rest args)
                      (push (cons 'font-lock-ensure args) calls))))
           (let ((result (ansible-doc-fontify-yaml text)))
             (list (eq text result)
                   (nreverse calls)
                   result
                   (mapcar
                    (lambda (position)
                      (list position
                            (get-text-property position 'face result)
                            (get-text-property
                             position 'font-lock-face result)))
                    '(0 1 5 6 7 10 11 15))))))"##;
    let expect = expect![[
        r#"OK (nil (yaml-mode (font-lock-mode) (font-lock-ensure)) #("alpha beta gamma" 0 5 (font-lock-face font-lock-keyword-face face font-lock-keyword-face) 5 6 (font-lock-face nil) 6 10 (font-lock-face font-lock-string-face face font-lock-string-face) 10 16 (font-lock-face nil)) ((0 font-lock-keyword-face font-lock-keyword-face) (1 font-lock-keyword-face font-lock-keyword-face) (5 nil nil) (6 font-lock-string-face font-lock-string-face) (7 font-lock-string-face font-lock-string-face) (10 nil nil) (11 nil nil) (15 nil nil)))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_fontify_yaml_examples_replaces_only_first_example_to_end() {
    let elisp_form = r##"(with-temp-buffer
         (insert "Header\nNotes:  useful\n# - name: Copy\n  copy:\n    src: a\nTail\n")
         (let (calls)
           (cl-letf (((symbol-function 'ansible-doc-fontify-yaml)
                      (lambda (text)
                        (push text calls)
                        (propertize (upcase text) 'fixture 'fontified))))
             (ansible-doc-fontify-yaml-examples)
             (list (buffer-string)
                   (nreverse calls)
                   (get-text-property
                    (save-excursion
                      (goto-char (point-min))
                      (search-forward "# - NAME")
                      (match-beginning 0))
                    'fixture)
                   (get-text-property 1 'fixture)))))"##;
    let expect = expect![[
        r##"OK (#("Header\nNotes:  useful\n# - NAME: COPY\n  COPY:\n    SRC: A\nTAIL\n" 22 61 (fixture fontified)) ("# - name: Copy\n  copy:\n    src: a\nTail\n") fontified nil)"##
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_fontify_yaml_examples_without_marker_is_noop() {
    let elisp_form = r##"(with-temp-buffer
         (insert "Header\n- option\nplain text\n")
         (let ((before (buffer-string))
               calls)
           (cl-letf (((symbol-function 'ansible-doc-fontify-yaml)
                      (lambda (text)
                        (push text calls)
                        text)))
             (ansible-doc-fontify-yaml-examples)
             (list (equal before (buffer-string))
                   calls
                   (point)))))"##;
    let expect = expect!["OK (t nil 28)"];
    assert_ansible_doc_parity(elisp_form, expect);
}

#[test]
fn ansible_doc_real_module_rendering_fontifies_sections_options_literals_and_xrefs() {
    let elisp_form = r##"(with-temp-buffer
         (insert "> COPY (/usr/share/ansible/plugins/modules/copy.py)\n"
                 "Options (= is mandatory):\n"
                 "= src\n"
                 "    [Default: /tmp/source]\n"
                 "    (Choices: yes, no)\n"
                 "- mode\n"
                 "    Use `preserve' or see [file].\n"
                 "Notes:  This is a note.\n"
                 "Requirements:  python\n"
                 "# - name: Example\n"
                 "  copy:\n"
                 "    src: x\n")
         (ansible-doc-module-mode)
         (font-lock-ensure)
         (ansible-doc-fontify-module-xrefs (point-min) (point-max))
         (list
          (buffer-string)
          (mapcar
           (lambda (needle)
             (save-excursion
               (goto-char (point-min))
               (search-forward needle)
               (let ((position (match-beginning 0)))
                 (list needle position
                       (get-char-property position 'face)
                       (get-char-property position 'font-lock-face)))))
           '("> COPY" "Options" "= src" "Default:" "/tmp/source"
             "Choices:" "yes, no" "- mode" "preserve" "Notes:"
             "Requirements:"))
          (let ((button
                 (save-excursion
                   (goto-char (point-min))
                   (search-forward "[file]")
                   (button-at (match-beginning 0)))))
            (list (button-label button)
                  (button-get button 'ansible-module)
                  (button-get button 'action)))))"##;
    let expect = expect![[
        r#"OK (#("> COPY (/usr/share/ansible/plugins/modules/copy.py)\nOptions (= is mandatory):\n= src\n    [Default: /tmp/source]\n    (Choices: yes, no)\n- mode\n    Use `preserve' or see [file].\nNotes:  This is a note.\nRequirements:  python\n# - name: Example\n  copy:\n    src: x\n" 0 51 (face ansible-doc-header fontified nil) 51 52 (fontified nil) 52 77 (face ansible-doc-section fontified nil) 77 78 (fontified nil) 78 83 (face ansible-doc-mandatory-option fontified nil) 83 89 (fontified nil) 89 97 (face ansible-doc-label fontified nil) 97 98 (fontified nil) 98 109 (face ansible-doc-default fontified nil) 109 116 (fontified nil) 116 124 (face ansible-doc-label fontified nil) 124 125 (fontified nil) 125 132 (face ansible-doc-choices fontified nil) 132 134 (fontified nil) 134 140 (face ansible-doc-option fontified nil) 140 150 (fontified nil) 150 158 (face ansible-doc-literal fontified nil) 158 175 (fontified nil) 175 183 (face ansible-doc-section fontified nil) 183 199 (fontified nil) 199 214 (face ansible-doc-section fontified nil) 214 258 (fontified nil)) (("> COPY" 1 ansible-doc-header nil) ("Options" 53 ansible-doc-section nil) ("= src" 79 ansible-doc-mandatory-option nil) ("Default:" 90 ansible-doc-label nil) ("/tmp/source" 99 ansible-doc-default nil) ("Choices:" 117 ansible-doc-label nil) ("yes, no" 126 ansible-doc-choices nil) ("- mode" 135 ansible-doc-option nil) ("preserve" 151 ansible-doc-literal nil) ("Notes:" 176 ansible-doc-section nil) ("Requirements:" 200 ansible-doc-section nil)) ("[file]" #("file" 0 4 (fontified nil)) ansible-doc-follow-module-xref))"#
    ]];
    assert_ansible_doc_parity(elisp_form, expect);
}
