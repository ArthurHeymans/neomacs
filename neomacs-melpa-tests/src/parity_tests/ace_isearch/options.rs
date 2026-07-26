use expect_test::expect;

use super::assert_ace_isearch_parity;

#[test]
fn ace_isearch_function_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-function
               (get 'ace-isearch-function 'standard-value)
               (get 'ace-isearch-function 'custom-type)
               (get 'ace-isearch-function 'variable-documentation)
               (assq 'ace-isearch-function
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-function 'defvar)))"##;
    let expect = expect![[
        r#"OK (ace-jump-word-mode ((funcall #'#[nil ('ace-jump-word-mode) (t)])) (choice (const :tag "Use ace-jump-word-mode." ace-jump-word-mode) (const :tag "Use ace-jump-char-mode." ace-jump-char-mode) (const :tag "Use avy-goto-word-1." avy-goto-word-1) (const :tag "Use avy-goto-subword-1." avy-goto-subword-1) (const :tag "Use avy-goto-char." avy-goto-char)) "Function name to invoke ace-jump-mode or avy based on 1 character." (ace-isearch-function custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_two_function_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-2-function
               (get 'ace-isearch-2-function 'standard-value)
               (get 'ace-isearch-2-function 'custom-type)
               (get 'ace-isearch-2-function 'variable-documentation)
               (assq 'ace-isearch-2-function
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-2-function 'defvar)))"##;
    let expect = expect![[
        r#"OK (avy-goto-char-2 ((funcall #'#[nil ('avy-goto-char-2) (t)])) (choice (const :tag "Use avy-goto-char-2." avy-goto-char-2) (const :tag "Use avy-goto-char-2-above." avy-goto-char-2-above) (const :tag "Use avy-goto-char-2-below." avy-goto-char-2-below)) "Function name to invoke ace-jump-mode or avy based on 2 characters." (ace-isearch-2-function custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_disable_message_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-disable-isearch-function-from-isearch-message
               (get 'ace-isearch-disable-isearch-function-from-isearch-message
                    'standard-value)
               (get 'ace-isearch-disable-isearch-function-from-isearch-message
                    'custom-type)
               (get 'ace-isearch-disable-isearch-function-from-isearch-message
                    'variable-documentation)
               (assq 'ace-isearch-disable-isearch-function-from-isearch-message
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file
                 'ace-isearch-disable-isearch-function-from-isearch-message
                 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil ((funcall #'#[nil (nil) (t)])) boolean "Disable message shown by `ace-isearch-function-from-isearch'." (ace-isearch-disable-isearch-function-from-isearch-message custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_function_from_isearch_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-function-from-isearch
               (get 'ace-isearch-function-from-isearch 'standard-value)
               (get 'ace-isearch-function-from-isearch 'custom-type)
               (get 'ace-isearch-function-from-isearch
                    'variable-documentation)
               (assq 'ace-isearch-function-from-isearch
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-function-from-isearch 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil ((funcall #'#[nil ((cond ((require 'helm-swoop nil 'noerror) 'ace-isearch-helm-swoop-from-isearch) ((require 'helm-occur nil 'noerror) 'ace-isearch-helm-occur-from-isearch) ((require 'swiper nil 'noerror) 'ace-isearch-swiper-from-isearch) ((require 'consult nil 'noerror) 'ace-isearch-consult-line-from-isearch) ((progn (customize-set-variable 'ace-isearch-use-function-from-isearch nil) (if ace-isearch-disable-isearch-function-from-isearch-message nil (message "You don't have a suitable line-searching package installed.\nIn order to seamlessly transition to a line-searching command\nthrough `ace-isearch', you should install the following\npackage(s) of your choice:\n\n* Helm (in order to use `helm-occur')\n* Helm and helm-swoop (in order to use `helm-swoop')\n* Swiper (in order to use `swiper'), or\n* Consult (in order to use `consult-line')."))) nil))) (t)])) symbol "Symbol name of function which is invoked when the length of `isearch-string'\nis longer than or equal to `ace-isearch-input-length'." (ace-isearch-function-from-isearch custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_lighter_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-lighter
               (get 'ace-isearch-lighter 'standard-value)
               (get 'ace-isearch-lighter 'custom-type)
               (get 'ace-isearch-lighter 'variable-documentation)
               (assq 'ace-isearch-lighter
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-lighter 'defvar)))"##;
    let expect = expect![[
        r#"OK (" AceI" ((funcall #'#[nil (" AceI") (t)])) string "Lighter of ace-isearch-mode." (ace-isearch-lighter custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jump_basis_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-jump-based-on-one-char
               (get 'ace-isearch-jump-based-on-one-char 'standard-value)
               (get 'ace-isearch-jump-based-on-one-char 'custom-type)
               (get 'ace-isearch-jump-based-on-one-char
                    'variable-documentation)
               (assq 'ace-isearch-jump-based-on-one-char
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-jump-based-on-one-char 'defvar)))"##;
    let expect = expect![[
        r#"OK (t ((funcall #'#[nil (t) (t)])) boolean "If true, jump for L=1 after delay of `ace-isearch-jump-delay', otherwise\nrequire L=2 characters to jump." (ace-isearch-jump-based-on-one-char custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jump_delay_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-jump-delay
               (get 'ace-isearch-jump-delay 'standard-value)
               (get 'ace-isearch-jump-delay 'custom-type)
               (get 'ace-isearch-jump-delay 'variable-documentation)
               (assq 'ace-isearch-jump-delay
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-jump-delay 'defvar)))"##;
    let expect = expect![[
        r#"OK (0.3 ((funcall #'#[nil (0.3) (t)])) number "Delay seconds for invoking `ace-jump-mode' or `avy' during isearch." (ace-isearch-jump-delay custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_function_delay_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-func-delay
               (get 'ace-isearch-func-delay 'standard-value)
               (get 'ace-isearch-func-delay 'custom-type)
               (get 'ace-isearch-func-delay 'variable-documentation)
               (assq 'ace-isearch-func-delay
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-func-delay 'defvar)))"##;
    let expect = expect![[
        r#"OK (0.0 ((funcall #'#[nil (0.0) (t)])) number "Delay seconds for invoking `ace-isearch-function-from-isearch' during isearch." (ace-isearch-func-delay custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_input_length_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-input-length
               (get 'ace-isearch-input-length 'standard-value)
               (get 'ace-isearch-input-length 'custom-type)
               (get 'ace-isearch-input-length 'variable-documentation)
               (assq 'ace-isearch-input-length
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-input-length 'defvar)))"##;
    let expect = expect![[
        r#"OK (6 ((funcall #'#[nil (6) (t)])) integer "Length of input string required for invoking `ace-isearch-function-from-isearch'\nduring isearch." (ace-isearch-input-length custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_use_jump_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-use-jump
               (get 'ace-isearch-use-jump 'standard-value)
               (get 'ace-isearch-use-jump 'custom-type)
               (get 'ace-isearch-use-jump 'variable-documentation)
               (assq 'ace-isearch-use-jump
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-use-jump 'defvar)))"##;
    let expect = expect![[
        r#"OK (t ((funcall #'#[nil (t) (t)])) (choice (const :tag "Always" t) (const :tag "Only after a printing character is input" printing-char) (const :tag "Never" nil)) "If `nil', `ace-jump-mode' or `avy' is never invoked.\n\nIf `t', it is always invoked if the length of `isearch-string' is\nequal to 1 or 2, cf. value of `ace-isearch-jump-based-on-one-char'.\n\nIf `printing-char', it is invoked only if you hit a printing\ncharacter to search for as a first input.  This prevents it from\nbeing invoked when repeating a one character search, yanking a\ncharacter or calling `isearch-delete-char' leaving only one\ncharacter." (ace-isearch-use-jump custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_use_transition_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-use-function-from-isearch
               (get 'ace-isearch-use-function-from-isearch 'standard-value)
               (get 'ace-isearch-use-function-from-isearch 'custom-type)
               (get 'ace-isearch-use-function-from-isearch
                    'variable-documentation)
               (assq 'ace-isearch-use-function-from-isearch
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-use-function-from-isearch 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil ((funcall #'#[nil (t) (t)])) boolean "When non-nil, invoke `ace-isearch-function-from-isearch' if the length\nof `isearch-string' is longer than or equal to `ace-isearch-input-length'." (ace-isearch-use-function-from-isearch custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_fallback_function_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-fallback-function
               (get 'ace-isearch-fallback-function 'standard-value)
               (get 'ace-isearch-fallback-function 'custom-type)
               (get 'ace-isearch-fallback-function 'variable-documentation)
               (assq 'ace-isearch-fallback-function
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-fallback-function 'defvar)))"##;
    let expect = expect![[
        r#"OK (ace-isearch-helm-swoop-from-isearch ((funcall #'#[nil ('ace-isearch-helm-swoop-from-isearch) (t)])) symbol "Symbol name of function that is invoked when isearch fails and\n`ace-isearch-use-fallback-function' is non-nil." (ace-isearch-fallback-function custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_use_fallback_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-use-fallback-function
               (get 'ace-isearch-use-fallback-function 'standard-value)
               (get 'ace-isearch-use-fallback-function 'custom-type)
               (get 'ace-isearch-use-fallback-function
                    'variable-documentation)
               (assq 'ace-isearch-use-fallback-function
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-use-fallback-function 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil ((funcall #'#[nil (nil) (t)])) boolean "When non-nil, invoke `ace-isearch-fallback-function' when isearch fails." (ace-isearch-use-fallback-function custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_evil_mode_default_type_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-on-evil-mode
               (get 'ace-isearch-on-evil-mode 'standard-value)
               (get 'ace-isearch-on-evil-mode 'custom-type)
               (get 'ace-isearch-on-evil-mode 'variable-documentation)
               (assq 'ace-isearch-on-evil-mode
                     (get 'ace-isearch 'custom-group))
               (file-name-nondirectory
                (symbol-file 'ace-isearch-on-evil-mode 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil ((funcall #'#[nil (nil) (t)])) boolean "If non nil, ace-isearch-mode can be used on Evil mode." (ace-isearch-on-evil-mode custom-variable) "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}
