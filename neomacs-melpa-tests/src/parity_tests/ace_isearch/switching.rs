use expect_test::expect;

use super::{assert_ace_isearch_parity, assert_ace_isearch_signal_parity};

#[test]
fn ace_isearch_regexp_function_returns_a_truthy_canonical_regexp_function() {
    let elisp_form = r##"(let ((isearch-regexp-function
                    'regexp-function))
               (list
                isearch-regexp-function
                (ace-isearch--isearch-regexp-function)))"##;
    let expect = expect!["OK (regexp-function regexp-function)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_regexp_function_observes_the_isearch_word_variable_alias() {
    let elisp_form = r##"(let ((isearch-regexp-function 'regexp-function)
                   (isearch-word 'word-mode))
               (list
                isearch-regexp-function
                isearch-word
                (ace-isearch--isearch-regexp-function)
                (let ((isearch-regexp-function nil))
                  (ace-isearch--isearch-regexp-function))
                (let ((isearch-regexp-function nil)
                      (isearch-word nil))
                  (ace-isearch--isearch-regexp-function))))"##;
    let expect = expect!["OK (word-mode word-mode word-mode nil nil)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_regexp_function_handles_unbound_search_variables() {
    let elisp_form = r##"(let ((regexp-bound (boundp 'isearch-regexp-function))
                   (regexp-value (and (boundp 'isearch-regexp-function)
                                      isearch-regexp-function))
                   (word-bound (boundp 'isearch-word))
                   (word-value (and (boundp 'isearch-word) isearch-word)))
               (unwind-protect
                   (progn
                     (makunbound 'isearch-regexp-function)
                     (makunbound 'isearch-word)
                     (ace-isearch--isearch-regexp-function))
                 (if regexp-bound
                     (set 'isearch-regexp-function regexp-value)
                   (makunbound 'isearch-regexp-function))
                 (if word-bound
                     (set 'isearch-word word-value)
                   (makunbound 'isearch-word))))"##;
    let expect = expect!["OK nil"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_fboundp_ignores_the_function_when_the_flag_is_nil() {
    let elisp_form = r##"(list
               (ace-isearch--fboundp nil nil)
               (ace-isearch--fboundp 'missing-function nil)
               (ace-isearch--fboundp 'car nil))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_fboundp_accepts_an_existing_function_when_enabled() {
    let elisp_form = r##"(list
               (ace-isearch--fboundp 'car t)
               (ace-isearch--fboundp 'ace-isearch--fboundp t))"##;
    let expect = expect!["OK (t t)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_fboundp_rejects_a_nil_enabled_function() {
    let elisp_form = r##"(ace-isearch--fboundp nil t)"##;
    let expect = expect![[r#"ERR (error "function name must be specified!")"#]];
    assert_ace_isearch_signal_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_fboundp_rejects_an_unbound_enabled_function() {
    let elisp_form = r##"(ace-isearch--fboundp 'definitely-missing t)"##;
    let expect = expect![[r#"ERR (error "function definitely-missing is not bounded!")"#]];
    assert_ace_isearch_signal_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_make_backend_classifies_every_one_character_backend() {
    let elisp_form = r##"(progn
               (setq ace-isearch--ace-jump-or-avy 'before)
               (mapcar
               (lambda (function)
                 (setq ace-isearch-function function
                       ace-isearch--ace-jump-or-avy 'before)
                 (list function
                       (ace-isearch--make-ace-jump-or-avy)
                       ace-isearch--ace-jump-or-avy))
               '(ace-jump-word-mode
                 ace-jump-char-mode
                 avy-goto-word-1
                 avy-goto-subword-1
                 avy-goto-word-or-subword-1
                 avy-goto-char)))"##;
    let expect = expect![
        "OK ((ace-jump-word-mode ace-jump ace-jump) (ace-jump-char-mode ace-jump ace-jump) (avy-goto-word-1 avy avy) (avy-goto-subword-1 avy avy) (avy-goto-word-or-subword-1 avy avy) (avy-goto-char avy avy))"
    ];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_make_backend_accepts_string_names_but_rejects_unknown_and_nil_values() {
    let elisp_form = r##"(progn
               (setq ace-isearch--ace-jump-or-avy 'before)
               (mapcar
               (lambda (function)
                 (setq ace-isearch-function function
                       ace-isearch--ace-jump-or-avy 'before)
                 (list
                  (condition-case error-data
                      (ace-isearch--make-ace-jump-or-avy)
                    (error error-data))
                  ace-isearch--ace-jump-or-avy))
               '(unknown-backend "avy-goto-char" nil)))"##;
    let expect = expect![[
        r#"OK (((error "Function name unknown-backend for ace-isearch is invalid!") before) (avy avy) ((error "Function name nil for ace-isearch is invalid!") before))"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_make_two_backend_classifies_every_avy_backend() {
    let elisp_form = r##"(progn
               (setq ace-isearch--ace-jump-or-avy 'before)
               (mapcar
               (lambda (function)
                 (setq ace-isearch-2-function function
                       ace-isearch--ace-jump-or-avy 'before)
                 (list function
                       (ace-isearch-2--make-ace-jump-or-avy)
                       ace-isearch--ace-jump-or-avy))
               '(avy-goto-char-2
                 avy-goto-char-2-above
                 avy-goto-char-2-below)))"##;
    let expect = expect![
        "OK ((avy-goto-char-2 avy avy) (avy-goto-char-2-above avy avy) (avy-goto-char-2-below avy avy))"
    ];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_make_two_backend_rejects_unknown_values() {
    let elisp_form = r##"(progn
               (setq ace-isearch--ace-jump-or-avy 'before)
               (mapcar
               (lambda (function)
                 (setq ace-isearch-2-function function
                       ace-isearch--ace-jump-or-avy 'before)
                 (list
                  (condition-case error-data
                      (ace-isearch-2--make-ace-jump-or-avy)
                    (error error-data))
                  ace-isearch--ace-jump-or-avy))
               '(unknown-backend "avy-goto-char-2" nil)))"##;
    let expect = expect![[
        r#"OK (((error "Function name unknown-backend for ace-isearch-2 is invalid!") before) (avy avy) ((error "Function name nil for ace-isearch-2 is invalid!") before))"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_switch_function_reads_sets_classifies_and_messages() {
    let elisp_form = r##"(let ((ace-isearch-function 'ace-jump-word-mode)
                   calls)
               (cl-letf (((symbol-function 'completing-read)
                          (lambda (&rest arguments)
                            (push (cons 'read arguments) calls)
                            "avy-goto-char"))
                         ((symbol-function
                           'ace-isearch--make-ace-jump-or-avy)
                          (lambda ()
                            (push (list 'classify ace-isearch-function) calls)
                            'classified))
                         ((symbol-function 'message)
                          (lambda (&rest arguments)
                            (push (cons 'message arguments) calls)
                            'messaged)))
                 (list
                  (ace-isearch-switch-function)
                  ace-isearch-function
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (messaged avy-goto-char ((read "Function for ace-isearch (current is ace-jump-word-mode): " ("ace-jump-word-mode" "ace-jump-char-mode" "avy-goto-word-1" "avy-goto-subword-1" "avy-goto-word-or-subword-1" "avy-goto-char") nil t) (classify avy-goto-char) (message "Function for ace-isearch is set to %s." "avy-goto-char")))"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_two_switch_function_reads_sets_classifies_and_messages() {
    let elisp_form = r##"(let ((ace-isearch-2-function 'avy-goto-char-2)
                   calls)
               (cl-letf (((symbol-function 'completing-read)
                          (lambda (&rest arguments)
                            (push (cons 'read arguments) calls)
                            "avy-goto-char-2-below"))
                         ((symbol-function
                           'ace-isearch-2--make-ace-jump-or-avy)
                          (lambda ()
                            (push (list 'classify ace-isearch-2-function) calls)
                            'classified))
                         ((symbol-function 'message)
                          (lambda (&rest arguments)
                            (push (cons 'message arguments) calls)
                            'messaged)))
                 (list
                  (ace-isearch-2-switch-function)
                  ace-isearch-2-function
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (messaged avy-goto-char-2-below ((read "Function for ace-isearch-2 (current is avy-goto-char-2): " ("avy-goto-char-2" "avy-goto-char-2-above" "avy-goto-char-2-below") nil t) (classify avy-goto-char-2-below) (message "Function for ace-isearch-2 is set to %s." "avy-goto-char-2-below")))"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}
