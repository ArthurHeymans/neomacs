use expect_test::expect;

use super::assert_apib_mode_parity;

#[test]
fn mode_activation_derives_from_markdown_and_establishes_the_api_blueprint_environment() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apib-drafter-executable "drafter")
        events)
    (cl-letf
        (((symbol-function 'executable-find)
          (lambda (program)
            (push (list 'find program) events)
            "/tools/drafter"))
         ((symbol-function 'display-warning)
          (lambda (&rest arguments)
            (push (cons 'warning arguments) events))))
      (apib-mode)
      (list
       major-mode mode-name
       (derived-mode-p 'markdown-mode)
       (eq (current-local-map) apib-mode-map)
       (eq (keymap-parent apib-mode-map) markdown-mode-map)
       indent-tabs-mode
       apib-drafter-executable
       (length font-lock-keywords)
       (seq-every-p
        (lambda (rule) (member rule font-lock-keywords))
        apib-mode-font-lock-keywords)
       (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (apib-mode "API Blueprint" markdown-mode t t nil "/tools/drafter" 49 nil ((find "drafter")))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn mode_activation_warns_and_clears_the_runtime_when_drafter_is_unavailable() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apib-drafter-executable "/missing/drafter")
        events)
    (cl-letf
        (((symbol-function 'executable-find)
          (lambda (program)
            (push (list 'find program) events)
            nil))
         ((symbol-function 'display-warning)
          (lambda (type message &rest arguments)
            (push (list 'warning type message arguments) events)
            'warned)))
      (list
       (apib-mode)
       major-mode
       apib-drafter-executable
       (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (nil apib-mode nil ((find "/missing/drafter") (warning apib-mode "drafter binary not found, please install it in your exec-path" nil)))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn mode_activation_resolves_custom_executable_names_exactly_once_per_buffer() {
    let elisp_form = r##"(let ((apib-drafter-executable "drafter-v4")
      events)
  (cl-letf
      (((symbol-function 'executable-find)
        (lambda (program)
          (push program events)
          (concat "/opt/apiary/" program))))
    (list
     (with-temp-buffer
       (apib-mode)
       (list major-mode apib-drafter-executable))
     (with-temp-buffer
       (apib-mode)
       (list major-mode apib-drafter-executable))
     (nreverse events))))"##;
    let expect = expect![[
        r#"OK ((apib-mode "/opt/apiary/drafter-v4") (apib-mode "/opt/apiary//opt/apiary/drafter-v4") ("drafter-v4" "/opt/apiary/drafter-v4"))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn practical_api_blueprint_font_lock_distinguishes_responses_attributes_and_properties() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apib-drafter-executable "drafter"))
    (cl-letf (((symbol-function 'executable-find)
               (lambda (_program) "/tools/drafter")))
    (apib-mode))
    (insert
     "# Orders API\n\n"
     "#" "# Order [/orders/{id}]\n\n"
     "+ Parameters\n"
     "    + id: 42 (number, required) - Order identifier\n"
     "+ Response 200 (application/json)\n"
     "    + Attributes\n"
     "        + status: shipped (string)\n"
     "    + Body\n")
    (font-lock-ensure)
    (mapcar
     (lambda (needle)
       (goto-char (point-min))
       (search-forward needle)
       (let ((start (- (point) (length needle))))
         (list needle
               (get-text-property start 'face)
               (get-text-property start 'font-lock-face))))
     '("Orders API" "Parameters" "id" "42" "number, required"
       "Response" "200" "application/json" "Attributes"
       "status" "shipped" "string" "Body"))))"##;
    let expect = expect![[
        r#"OK (("Orders API" markdown-header-face-1 nil) ("Parameters" font-lock-keyword-face nil) ("id" markdown-header-face-2 nil) ("42" font-lock-constant-face nil) ("number, required" font-lock-constant-face nil) ("Response" font-lock-keyword-face nil) ("200" font-lock-constant-face nil) ("application/json" font-lock-variable-name-face nil) ("Attributes" font-lock-keyword-face nil) ("status" nil nil) ("shipped" font-lock-constant-face nil) ("string" font-lock-constant-face nil) ("Body" font-lock-keyword-face nil))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn property_font_lock_handles_optional_values_types_descriptions_and_nested_names() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apib-drafter-executable "drafter"))
    (cl-letf (((symbol-function 'executable-find)
               (lambda (_program) "/tools/drafter")))
      (apib-mode))
    (insert
     "+ plain\n"
     "+ empty:\n"
     "+ count: 3 (number)\n"
     "+ account-name: primary (string, required) - Human label\n"
     "    + nested_value: false (boolean, optional)\n"
     "- legacy: yes (string)\n")
    (font-lock-ensure)
    (mapcar
     (lambda (needle)
       (goto-char (point-min))
       (search-forward needle)
       (list needle
             (get-text-property
              (- (point) (length needle))
              'face)))
     '("plain" "empty" "count" "3" "number" "account-name"
       "primary" "string, required" "nested_value" "false"
       "boolean, optional" "legacy" "yes"))))"##;
    let expect = expect![[
        r#"OK (("plain" nil) ("empty" nil) ("count" nil) ("3" font-lock-constant-face) ("number" font-lock-constant-face) ("account-name" nil) ("primary" nil) ("string, required" nil) ("nested_value" nil) ("false" font-lock-constant-face) ("boolean, optional" font-lock-constant-face) ("legacy" nil) ("yes" font-lock-constant-face))"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn markdown_parent_outline_level_recognizes_api_blueprint_heading_depths() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apib-drafter-executable "drafter"))
    (cl-letf (((symbol-function 'executable-find)
               (lambda (_program) "/tools/drafter")))
    (apib-mode))
    (insert
     "# Catalog API\n"
     "#" "# Group Products\n"
     "#" "#" "# Product [/products/{id}]\n"
     "#" "#" "#" "# Retrieve [GET]\n")
    (goto-char (point-min))
    (let (levels)
      (dotimes (_ 4)
        (push
         (list
          (line-number-at-pos)
          (markdown-outline-level)
          (buffer-substring-no-properties
           (line-beginning-position) (line-end-position)))
         levels)
        (forward-line))
      (list
       (derived-mode-p 'markdown-mode)
       (nreverse levels)
       outline-regexp
       outline-level))))"##;
    let expect = expect![[
        r#####"OK (markdown-mode ((1 1 "# Catalog API") (2 1 "## Group Products") (3 1 "### Product [/products/{id}]") (4 1 "#### Retrieve [GET]")) "^\\(?:\\(?1:[^\15\n\11 -].*\\)\n\\(?:\\(?2:=+\\)\\|\\(?3:-+\\)\\)\\|\\(?4:#+[ \11]+\\)\\(?5:.*?\\)\\(?6:[ \11]+#+\\)?\\)$" markdown-outline-level)"#####
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn markdown_parent_mode_keeps_inherited_navigation_and_editing_commands() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apib-drafter-executable "drafter"))
    (cl-letf (((symbol-function 'executable-find)
               (lambda (_program) "/tools/drafter")))
      (apib-mode))
    (list
     (derived-mode-p 'markdown-mode)
     (eq (keymap-parent apib-mode-map) markdown-mode-map)
     (lookup-key apib-mode-map (kbd "M-n"))
     (lookup-key apib-mode-map (kbd "M-p"))
     (lookup-key apib-mode-map (kbd "TAB"))
     (lookup-key apib-mode-map (kbd "C-c C-s h"))
     paragraph-start
     comment-start
     comment-end
     fill-paragraph-function)))"##;
    let expect = expect![[
        r#"OK (markdown-mode t markdown-next-link markdown-previous-link markdown-cycle markdown-insert-header-dwim "\f\\|[ \11\f]*$\\|\\(?:[ \11]*>\\)+[ \11\f]*$\\|[ \11]*[*+-][ \11]+\\|[ \11]*\\(?:[0-9]+\\|#\\)\\.[ \11]+\\|[ \11]*\\[\\S-*\\]:[ \11]+\\|[ \11]*:[ \11]+\\|^|" "<!-- " " -->" markdown-fill-paragraph)"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}

#[test]
fn font_lock_keyword_registry_preserves_all_four_api_blueprint_grammar_rules() {
    let elisp_form = r##"(list
 (length apib-mode-font-lock-keywords)
 (copy-tree apib-mode-font-lock-keywords)
 (secure-hash
  'sha256
  (encode-coding-string
   (prin1-to-string apib-mode-font-lock-keywords)
   'utf-8-unix)))"##;
    let expect = expect![[
        r#"OK (4 (("\\(?:\\(?:\\+\\|\\-\\) +\\(?:Body\\|Headers?\\|Model\\|Parameters?\\|Re\\(?:quest\\)\\|Schema\\|Values\\)\\)" 0 font-lock-keyword-face) ("\\(\\(?:\\+\\|\\-\\) +Response\\) +\\([0-9]\\{3\\}\\)+(?\\(.*\\))?" (1 font-lock-keyword-face) (2 font-lock-constant-face) (3 font-lock-variable-name-face)) ("\\(\\(?:\\+\\|\\-\\) +Attributes\\)+(?\\(.*\\))?" (1 font-lock-keyword-face) (2 font-lock-variable-name-face)) ("^ *\\(?:\\+\\|\\-\\) +\\(.+?\\)\\(?:: +\\([^(\n]+\\)\\)?\\(?: +(\\(.*\\))\\)?\\(?: *- *.*\\)?$" (1 nil) (2 font-lock-constant-face nil t) (3 font-lock-constant-face nil t))) "e6c1dcd4f5bd374c05c1daa3b68cbabfdaa062ccd4dd4c384492fba585d7e2eb")"#
    ]];
    assert_apib_mode_parity(elisp_form, expect);
}
