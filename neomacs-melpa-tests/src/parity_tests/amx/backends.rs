use expect_test::expect;

use super::assert_amx_parity;

#[test]
fn command_name_and_symbol_conversion_cover_symbols_strings_cells_and_errors() {
    let elisp_form = r##"
(list
 (mapcar
  (lambda (value)
    (condition-case error-data
        (amx-get-command-name value)
      (error
       (cons (car error-data)
             (cdr error-data)))))
 '(amx-test-alpha
    "amx-test-beta"
    (amx-test-gamma . 9)))
 (mapcar
  (lambda (entry)
    (condition-case error-data
        (amx-get-command-symbol
         (car entry) (cadr entry))
      (error
       (cons (car error-data)
             (cdr error-data)))))
  '((amx-test-alpha nil)
    ("amx-test-beta" nil)
    ((amx-test-gamma . 2) nil)
    (amx-test-noncommand nil)
    ("amx-test-created-on-demand" t)
    ("amx-test-not-interned" nil)
    (nil nil)))
 (mapcar
  (lambda (value)
    (condition-case nil
        (amx-get-command-name value)
      (error 'invalid-name)))
  '(42 nil))
 (condition-case nil
     (amx-get-command-symbol 17)
   (error 'invalid-symbol))
 (fboundp 'amx-test-created-on-demand)
 (intern-soft "amx-test-created-on-demand"))
"##;
    let expect = expect![[
        r#"OK (("amx-test-alpha" "amx-test-beta" "amx-test-gamma") (amx-test-alpha amx-test-beta amx-test-gamma nil amx-test-created-on-demand nil nil) (invalid-name "nil") invalid-symbol nil amx-test-created-on-demand)"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn default_selection_skips_ignored_entries_and_augments_first_usable_binding() {
    let elisp_form = r##"
(let ((hash (make-hash-table :test 'equal))
      (amx-ignored-command-matchers
       '("\\`amx-test-alpha\\'")))
  (puthash 'amx-test-beta "C-c b" hash)
  (list
   (amx-get-default
    '((amx-test-alpha . 9)
      (amx-test-beta . 4)
      (amx-test-gamma . 1))
    hash)
   (amx-get-default
    (completion-table-dynamic
     (lambda (_)
       '("amx-test-alpha"
         "amx-test-gamma")))
    hash)
   (condition-case nil
       (amx-get-default
        ["amx-test-alpha"
         "amx-test-gamma"]
        hash)
     (error 'vector-signaled))
   (let ((amx-ignored-command-matchers
          '("\\`amx-test-")))
     (amx-get-default
      '(amx-test-alpha amx-test-beta)
      hash))))
"##;
    let expect = expect![[r#"OK ("amx-test-beta (C-c b)" "amx-test-gamma" vector-signaled nil)"#]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn prefix_prompt_matrix_and_one_shot_override_match_interactive_usage() {
    let elisp_form = r##"
(let ((amx-prompt-string "Command: ")
      (amx-temp-prompt-string "First only: "))
  (list
   (let ((current-prefix-arg nil))
     (amx-prompt-with-prefix-arg))
   amx-temp-prompt-string
   (let ((current-prefix-arg '-))
     (amx-prompt-with-prefix-arg))
   (let ((current-prefix-arg 7))
     (amx-prompt-with-prefix-arg))
   (let ((current-prefix-arg '(4)))
     (amx-prompt-with-prefix-arg))
   (let ((current-prefix-arg '(16)))
     (amx-prompt-with-prefix-arg))
   amx-temp-prompt-string))
"##;
    let expect = expect![[
        r#"OK ("First only: " nil "- Command: " "7 Command: " "C-u Command: " "16 Command: " nil)"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn defining_and_resolving_custom_backends_preserves_defaults_and_replacement() {
    let elisp_form = r##"
(let ((amx-known-backends nil))
  (amx-define-backend
   :name 'fixture
   :comp-fun 'completion-a
   :required-feature 'fixture-feature
   :auto-activate '(bound-and-true-p fixture-mode))
  (let ((first (amx-get-backend 'fixture)))
    (amx-define-backend
     :name 'fixture
     :comp-fun 'completion-b
     :get-text-fun 'fixture-text
     :exit-fun 'fixture-exit)
    (let ((second (amx-get-backend 'fixture)))
      (list
       (list
        (amx-backend-name first)
        (amx-backend-required-feature first)
        (amx-backend-comp-fun first)
        (amx-backend-get-text-fun first)
        (amx-backend-exit-fun first)
        (amx-backend-auto-activate first))
       (list
        (amx-backend-name second)
        (amx-backend-required-feature second)
        (amx-backend-comp-fun second)
        (amx-backend-get-text-fun second)
        (amx-backend-exit-fun second)
        (amx-backend-auto-activate second))
       (eq second (amx-get-backend second))
       (condition-case error-data
           (amx-get-backend 'missing)
         (error
          (cons (car error-data)
                (cdr error-data))))))))
"##;
    let expect = expect![[
        r#"OK ((fixture fixture-feature completion-a amx-default-get-text amx-default-exit-minibuffer (bound-and-true-p fixture-mode)) (fixture nil completion-b fixture-text fixture-exit nil) t (error "Unknown amx backed missing"))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn backend_definition_validation_rejects_each_malformed_required_field() {
    let elisp_form = r##"
(mapcar
 (lambda (arguments)
   (condition-case error-data
       (apply #'amx-define-backend arguments)
     (error
      (cons (car error-data)
            (cdr error-data)))))
 '((:name nil :comp-fun complete)
   (:name "string" :comp-fun complete)
   (:name valid :comp-fun 7)
   (:name valid :comp-fun complete :get-text-fun 8)
   (:name valid :comp-fun complete :exit-fun 9)
   (:name valid :comp-fun complete :required-feature "bad")))
"##;
    let expect = expect![[
        r#"OK ((error "Not enough arguments for format string") (error "Not enough arguments for format string") (error "Not enough arguments for format string") (error "Not enough arguments for format string") (error "Not enough arguments for format string") (error "Not enough arguments for format string"))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn loading_backend_requirements_handles_single_multiple_and_missing_features() {
    let elisp_form = r##"
(let ((amx-known-backends nil))
  (provide 'amx-test-present-a)
  (provide 'amx-test-present-b)
  (amx-define-backend
   :name 'single
   :comp-fun 'ignore
   :required-feature 'amx-test-present-a)
  (setq amx-known-backends
        (plist-put
         amx-known-backends
         'multiple
         (make-amx-backend
          :name 'multiple
          :comp-fun 'ignore
          :get-text-fun
          'amx-default-get-text
          :exit-fun
          'amx-default-exit-minibuffer
          :required-feature
          '(amx-test-present-a
            amx-test-present-b))))
  (amx-define-backend
   :name 'missing
   :comp-fun 'ignore
   :required-feature 'amx-test-absent)
  (list
   (condition-case nil
       (list 'single
             (amx-load-backend 'single))
     (error 'single-rejected))
   (condition-case nil
       (list 'multiple
             (amx-load-backend 'multiple))
     (error 'multiple-rejected))
   (condition-case nil
       (amx-load-backend 'missing)
     (error
      'missing-rejected))))
"##;
    let expect = expect!["OK ((single nil) (multiple nil) missing-rejected)"];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn backend_custom_setter_commits_valid_values_and_preserves_old_value_on_failure() {
    let elisp_form = r##"
(let ((amx-known-backends nil)
      (amx-backend 'before))
  (provide 'amx-test-backend-feature)
  (amx-define-backend
   :name 'valid
   :comp-fun 'ignore
   :required-feature 'amx-test-backend-feature)
  (amx-define-backend
   :name 'missing-feature
   :comp-fun 'ignore
   :required-feature 'amx-test-no-feature)
  (list
   (amx-set-backend 'amx-backend 'valid)
   amx-backend
   (condition-case error-data
       (amx-set-backend
        'amx-backend 'unknown)
     (error
      (cons (car error-data)
            (cdr error-data))))
   amx-backend
   (condition-case error-data
       (amx-set-backend
        'amx-backend 'missing-feature)
     (error
      (cons (car error-data)
            (cdr error-data))))
   amx-backend))
"##;
    let expect = expect![[
        r#"OK (valid valid (error "Unknown amx backend: unknown") valid (error "Feature ‘amx-test-no-feature’ is required for backend ‘missing-feature’") valid)"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn auto_backend_selection_obeys_mode_activation_and_priority_order() {
    let elisp_form = r##"
(let ((select
       (lambda ()
         (let ((backend
                (amx-auto-select-backend)))
           (if (amx-backend-p backend)
               (amx-backend-name backend)
             backend))))
      results)
  (setq ido-mode nil
        ido-ubiquitous-mode nil
        ivy-mode nil
        helm-mode nil
        selectrum-mode nil)
  (push (funcall select) results)
  (setq ido-mode t)
  (push (funcall select) results)
  (setq ido-mode nil
        ido-ubiquitous-mode t)
  (push (funcall select) results)
  (setq ido-ubiquitous-mode nil
        ivy-mode t)
  (push (funcall select) results)
  (setq ivy-mode nil
        helm-mode t)
  (push (funcall select) results)
  (setq helm-mode nil
        selectrum-mode t)
  (push (funcall select) results)
  (setq ido-mode t
        ivy-mode t
        helm-mode t)
  (push (funcall select) results)
  (nreverse results))
"##;
    let expect = expect!["OK (standard ido ido ivy helm selectrum ido)"];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn completing_read_dispatches_exact_keywords_and_tracks_minibuffer_ownership() {
    let elisp_form = r##"
(let ((amx-known-backends nil)
      events)
  (amx-define-backend
   :name 'fixture
   :comp-fun
   (lambda (choices &rest arguments)
     (push
      (list choices arguments
            amx-minibuffer-depth
            (minibuffer-depth)
            (amx-active))
      events)
     "chosen"))
  (list
   (amx-completing-read
    '("first" "second")
    :initial-input "sec"
    :predicate #'stringp
    :def "first"
    :backend 'fixture)
   (nreverse events)
   amx-minibuffer-depth))
"##;
    let expect = expect![[
        r#"OK ("chosen" ((("first" "second") (:initial-input "sec" :predicate stringp :def "first") 1 0 t)) -1)"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn automatic_completion_falls_back_to_standard_and_reports_load_failure() {
    let elisp_form = r##"
(let ((amx-known-backends nil)
      events)
  (amx-define-backend
   :name 'standard
   :comp-fun 'ignore)
  (amx-define-backend
   :name 'broken
   :comp-fun 'ignore
   :required-feature 'amx-test-missing
   :auto-activate t)
  (cl-letf
      (((symbol-function 'require)
        (lambda (&rest _) nil))
       ((symbol-function 'display-warning)
        (lambda (&rest arguments)
          (push (cons 'warning arguments) events)))
       ((symbol-function 'amx-completing-read)
        (lambda (choices &rest arguments)
          (push
           (list 'read choices arguments)
           events)
          'fallback-result)))
    (list
     (amx-completing-read-auto
      '("one" "two")
      :initial-input "o"
      :predicate #'stringp
      :def "one")
     (nreverse events))))
"##;
    let expect = expect![[
        r#"OK (fallback-result ((warning amx "Falling back to standard amx backend due to error loading #s(amx-backend broken amx-test-missing ignore amx-default-get-text amx-default-exit-minibuffer t) backend: \"Feature ‘amx-test-missing’ is required for backend ‘broken’\"") (read ("one" "two") (:initial-input "o" :predicate stringp :def "one" :backend standard))))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn read_and_run_supports_custom_action_and_normal_execution_then_ranks_choice() {
    let elisp_form = r##"
(let ((amx-show-key-bindings nil)
      (amx-ignored-command-matchers nil)
      (amx-command-keybind-hash
       (make-hash-table :test 'equal))
      events)
  (cl-letf
      (((symbol-function 'amx-completing-read)
        (lambda (&rest arguments)
          (push (cons 'complete arguments) events)
          "amx-test-beta"))
       ((symbol-function 'execute-extended-command)
        (lambda (&rest arguments)
          (push (cons 'execute arguments) events)
          'executed))
       ((symbol-function 'amx-rank)
        (lambda (command)
          (push (list 'rank command) events))))
    (let ((amx-custom-action
           (lambda (command)
             (push (list 'custom command) events))))
      (amx-read-and-run
       '(amx-test-alpha amx-test-beta)
       "bet"))
    (let ((amx-custom-action nil)
          (current-prefix-arg '(4)))
      (amx-read-and-run
       '(amx-test-alpha amx-test-beta)))
    (list
     amx-custom-action
     (nreverse events))))
"##;
    let expect = expect![[
        r#"OK (nil ((complete (amx-test-alpha amx-test-beta) :initial-input "bet" :def "amx-test-alpha") (custom amx-test-beta) (complete (amx-test-alpha amx-test-beta) :initial-input nil :def "amx-test-alpha") (execute (4) "amx-test-beta") (rank amx-test-beta)))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}

#[test]
fn selected_item_help_actions_exit_then_dispatch_to_exact_emacs_commands() {
    let elisp_form = r##"
(let ((amx-known-backends nil)
      (amx-backend 'fixture)
      events)
  (amx-define-backend
   :name 'fixture
   :comp-fun 'ignore
   :exit-fun
   (lambda ()
     (push 'exit events)))
  (cl-letf
      (((symbol-function 'describe-function)
        (lambda (function)
          (push (list 'describe function) events)))
       ((symbol-function 'pop-to-buffer)
        (lambda (buffer)
          (push (list 'pop buffer) events)))
       ((symbol-function 'where-is)
        (lambda (function)
          (push (list 'where function) events)))
       ((symbol-function 'find-function)
        (lambda (function)
          (push (list 'find function) events))))
    (dolist (command
             '(amx-describe-function
               amx-where-is
               amx-find-function))
      (setq amx-custom-action nil)
      (funcall command)
      (funcall amx-custom-action 'amx-test-alpha))
    (nreverse events)))
"##;
    let expect = expect![[
        r#"OK (exit (describe amx-test-alpha) (pop "*Help*") exit (where amx-test-alpha) exit (find amx-test-alpha))"#
    ]];
    assert_amx_parity(elisp_form, expect);
}
