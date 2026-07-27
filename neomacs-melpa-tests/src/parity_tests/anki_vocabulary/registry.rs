use expect_test::expect;

use super::{assert_anki_vocabulary_autoload_parity, assert_anki_vocabulary_parity};

#[test]
fn package_descriptor_preserves_the_exact_frozen_release_and_duplicate_dependency_contract() {
    let elisp_form = r##"(let* ((description
        (cadr (assq 'anki-vocabulary package-alist)))
       (directory
        (file-name-as-directory (package-desc-dir description))))
  (list
   (featurep 'anki-vocabulary)
   (package-installed-p 'anki-vocabulary)
   (package-desc-name description)
   (package-version-join (package-desc-version description))
   (package-desc-summary description)
   (package-desc-reqs description)
   (mapcar
    (lambda (requirement)
      (list
       (car requirement)
       (package-version-join (cadr requirement))
       (or (package-installed-p (car requirement))
           (package-built-in-p (car requirement)))))
    (package-desc-reqs description))
   (file-name-nondirectory (directory-file-name directory))))"##;
    let expect = expect![[
        r#"OK (t t anki-vocabulary "20200103.325" "Help you to create vocabulary cards in Anki." ((emacs (24 4)) (s (1 0)) (youdao-dictionary (0 4)) (anki-connect (1 0)) (s (1 10))) ((emacs "24.4" t) (s "1.0" t) (youdao-dictionary "0.4" t) (anki-connect "1.0" t) (s "1.10" t)) "anki-vocabulary-20200103.325")"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn installed_library_and_descriptor_match_the_exact_frozen_archive_bytes() {
    let elisp_form = r##"(let* ((description
         (cadr (assq 'anki-vocabulary package-alist)))
        (directory (package-desc-dir description)))
  (mapcar
   (lambda (name)
     (let ((file (expand-file-name name directory)))
       (list
        name
        (file-attribute-size (file-attributes file))
        (with-temp-buffer
          (insert-file-contents-literally file)
          (secure-hash 'sha256 (current-buffer))))))
   '("anki-vocabulary.el" "anki-vocabulary-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("anki-vocabulary.el" 11557 "045c82caae196b72d9efc2d207a89bc682c7b2019f195d4e11525f9e36e951a6") ("anki-vocabulary-pkg.el" 596 "d2f891cd66becdf198ada833a7b308bc067bcf56372e7eab0316d8ac4186860b"))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn installed_source_preserves_revision_requirements_and_complete_definition_counts() {
    let elisp_form = r##"(let ((source (locate-library "anki-vocabulary")))
  (with-temp-buffer
    (insert-file-contents-literally source)
    (let ((contents (buffer-string)))
      (list
       (file-name-nondirectory source)
       (count-lines (point-min) (point-max))
       (how-many "^(defun anki-vocabulary")
       (how-many "^(defcustom anki-vocabulary")
       (string-match-p "Package-Version: 20200103\\.325" contents)
       (string-match-p "Package-Revision: 863fe0219577" contents)
       (string-match-p
        (regexp-quote
         "Package-Requires: ((emacs \"24.4\") (s \"1.0\") (youdao-dictionary \"0.4\") (anki-connect \"1.0\") (s \"1.10\"))")
        contents)
       (string-match-p "(provide 'anki-vocabulary)" contents)))))"##;
    let expect = expect![[r#"OK ("anki-vocabulary.el" 267 9 8 248 281 315 11496)"#]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_preserves_commands_arglists_interactive_specs_and_docs() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (commandp symbol)
    (help-function-arglist symbol t)
    (interactive-form symbol)
    (documentation symbol t)))
 '(anki-vocabulary-set-ankiconnect
   anki-vocabulary--get-normal-text
   anki-vocabulary--get-pdf-text
   anki-vocabulary--get-text
   anki-vocabulary--select-word-in-string
   anki-vocabulary--get-word
   anki-vocabulary--word-searcher-youdao
   anki-vocabulary--sentence-translator-youdao
   anki-vocabulary))"##;
    let expect = expect![[
        r#"OK ((anki-vocabulary-set-ankiconnect t t nil (interactive nil) "Set the correspondence relation for fields in card.") (anki-vocabulary--get-normal-text t nil nil nil "Get the text in normal mode.") (anki-vocabulary--get-pdf-text t nil nil nil "Get the text in pdf mode.") (anki-vocabulary--get-text t nil nil nil "Get the region text.") (anki-vocabulary--select-word-in-string t nil (str &optional default-word) nil "Select word in STR.\nOptional argument DEFAULT-WORD specify the default word.") (anki-vocabulary--get-word t nil nil nil "Get the word at point.") (anki-vocabulary--word-searcher-youdao t nil (word) nil "Search WORD using youdao.\n\nIt returns an alist like\n    `((expression . ,expression)\n      (glossary . ,glossary)\n      (phonetic . ,phonetic))") (anki-vocabulary--sentence-translator-youdao t nil (sentence) nil "Translate SENTENCE using youdao.") (anki-vocabulary t t (&optional sentence word) (interactive nil) "Translate SENTENCE and WORD, and then create an anki card."))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn complete_customization_surface_preserves_values_types_groups_and_standard_values() {
    let elisp_form = r##"(list
 (get 'anki-vocabulary 'custom-group)
 (get 'anki-vocabulary 'group-documentation)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (copy-tree (symbol-value symbol))
     (get symbol 'custom-type)
     (get symbol 'custom-group)
     (get symbol 'standard-value)
     (get symbol 'variable-documentation)
     (local-variable-if-set-p symbol)))
  '(anki-vocabulary-deck-name
    anki-vocabulary-model-name
    anki-vocabulary-field-alist
    anki-vocabulary-audio-fileds
    anki-vocabulary-before-addnote-functions
    anki-vocabulary-after-addnote-functions
    anki-vocabulary-word-searcher
    anki-vocabulary-sentence-translator)))"##;
    let expect = expect![[
        r#"OK (((anki-vocabulary-deck-name custom-variable) (anki-vocabulary-model-name custom-variable) (anki-vocabulary-field-alist custom-variable) (anki-vocabulary-audio-fileds custom-variable) (anki-vocabulary-before-addnote-functions custom-variable) (anki-vocabulary-after-addnote-functions custom-variable) (anki-vocabulary-word-searcher custom-variable) (anki-vocabulary-sentence-translator custom-variable)) "" ((anki-vocabulary-deck-name "" string nil ("") "Which deck would the word stored." nil) (anki-vocabulary-model-name "" string nil ("") "Specify the model name." nil) (anki-vocabulary-field-alist nil string nil (nil) "Specify the corresponding relationship for fields in card." nil) (anki-vocabulary-audio-fileds nil list nil (nil) "Specify fields used to store audio." nil) (anki-vocabulary-before-addnote-functions nil hook nil (nil) "List of hook functions run before add note.\n\nThe functions should accept those arguments:\n+ expression(单词)\n+ sentence(单词所在句子)\n+ sentence_bold(单词所在句子,单词加粗)\n+ translation(翻译的句子)\n+ glossary(单词释义)\n+ phonetic(音标)" nil) (anki-vocabulary-after-addnote-functions nil hook nil (nil) "List of hook functions run after add note.\n\nThe functions should accept those arguments:\n+ expression(单词)\n+ sentence(单词所在句子)\n+ sentence_bold(单词所在句子,单词加粗)\n+ translation(翻译的句子)\n+ glossary(单词释义)\n+ phonetic(音标)" nil) (anki-vocabulary-word-searcher anki-vocabulary--word-searcher-youdao function nil (#'anki-vocabulary--word-searcher-youdao) "Function used to search word's meaning.\n\nThe function should return an alist like\n    `((expression . ,expression)\n      (glossary . ,glossary)\n      (phonetic . ,phonetic))" nil) (anki-vocabulary-sentence-translator anki-vocabulary--sentence-translator-youdao function nil (#'anki-vocabulary--sentence-translator-youdao) "Function used to translate sentence.\n\nThe function should return the translation in a string." nil)))"#
    ]];
    assert_anki_vocabulary_parity(elisp_form, expect);
}

#[test]
fn generated_autoloads_register_only_the_two_public_user_commands() {
    let elisp_form = r##"(list
 (featurep 'anki-vocabulary)
 (featurep 'anki-vocabulary-autoloads)
 (mapcar
  (lambda (symbol)
    (list
     symbol
     (fboundp symbol)
     (and (fboundp symbol)
          (autoloadp (symbol-function symbol)))
     (interactive-form symbol)))
  '(anki-vocabulary-set-ankiconnect
    anki-vocabulary
    anki-vocabulary--get-normal-text
    anki-vocabulary--word-searcher-youdao))
 (mapcar #'boundp
         '(anki-vocabulary-deck-name
           anki-vocabulary-field-alist
           anki-vocabulary-word-searcher)))"##;
    let expect = expect![
        "OK (nil t ((anki-vocabulary-set-ankiconnect t t (interactive nil)) (anki-vocabulary t nil (interactive nil)) (anki-vocabulary--get-normal-text t nil nil) (anki-vocabulary--word-searcher-youdao t nil nil)) (t t t))"
    ];
    assert_anki_vocabulary_autoload_parity(elisp_form, expect);
}
