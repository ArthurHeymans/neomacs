use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn fragment_models_normalize_empty_labels_and_preserve_group_intent() {
    let elisp_form = r##"
(list
 (agent-shell-ui-make-fragment-model
  :namespace-id "turn-7"
  :block-id "tool-3"
  :label-left "run"
  :label-right ""
  :body "cargo nextest"
  :group-id "activity-2"
  :group-label "Working"
  :group-expanded nil)
 (agent-shell-ui-make-group-model
  :namespace-id "turn-7"
  :block-id "activity-2"
  :label-left "Working"
  :label-right "2/4"
  :expanded nil))
"##;
    let expect = expect![[
        r#"OK (((:namespace-id . "turn-7") (:block-id . "tool-3") (:label-left . "run") (:label-right) (:body . "cargo nextest") (:group-id . "activity-2") (:group-label . "Working") (:group-expanded)) ((:namespace-id . "turn-7") (:block-id . "activity-2") (:kind . group) (:label-left . "Working") (:label-right . "2/4") (:expanded)))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn indenting_rendered_multiline_body_preserves_every_caller_property() {
    let elisp_form = r##"
(let* ((input
        (propertize "line one\nline two\nline three"
                    'agent-shell-markdown-frozen t
                    'face 'diff-added))
       (output (agent-shell-ui--indent-text input "│ "))
       (runs nil))
  (dotimes (index (length output))
    (push
     (list (char-to-string (aref output index))
           (get-text-property index 'agent-shell-markdown-frozen output)
           (get-text-property index 'face output)
           (get-text-property index 'line-prefix output)
           (get-text-property index 'wrap-prefix output))
     runs))
  (list (substring-no-properties output)
        (delete-dups (nreverse runs))))
"##;
    let expect = expect![[
        r#"OK ("line one\nline two\nline three" (("l" t diff-added "│ " "│ ") ("i" t diff-added "│ " "│ ") ("n" t diff-added "│ " "│ ") ("e" t diff-added "│ " "│ ") (" " t diff-added "│ " "│ ") ("o" t diff-added "│ " "│ ") ("\n" t diff-added "│ " "│ ") ("t" t diff-added "│ " "│ ") ("w" t diff-added "│ " "│ ") ("h" t diff-added "│ " "│ ") ("r" t diff-added "│ " "│ ")))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn whitespace_body_visibility_and_newline_padding_match_rendered_transcripts() {
    let elisp_form = r##"
(list
 (with-temp-buffer
   (insert "\n\n")
   (add-text-properties (point-min) (point-max) '(invisible t))
   (agent-shell-ui--body-invisible-p (point-min) (point-max)))
 (with-temp-buffer
   (insert "\n\n")
   (agent-shell-ui--body-invisible-p (point-min) (point-max)))
 (with-temp-buffer
   (insert "answer\n\n")
   (agent-shell-ui--required-newlines 3))
 (with-temp-buffer
   (insert "answer\n\nhidden\n")
   (add-text-properties 9 (point-max) '(invisible t))
   (agent-shell-ui--required-newlines 3)))
"##;
    let expect = expect![[r#"OK (t nil "\n" "\n")"#]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn activity_group_lifecycle_nests_streams_folds_and_restores_members() {
    let elisp_form = r##"
(with-temp-buffer
  (agent-shell-ui-mode 1)
  (agent-shell-ui-update-fragment
   (agent-shell-ui-make-fragment-model
    :namespace-id "turn" :block-id "read"
    :group-id "activity" :group-label "Working"
    :label-left "read" :label-right "src/lib.rs"
    :body "line one")
   :expanded nil :navigation 'always)
  (agent-shell-ui-update-fragment
   (agent-shell-ui-make-fragment-model
    :namespace-id "turn" :block-id "test"
    :group-id "activity" :group-label "Working"
    :label-left "run" :label-right "nextest"
    :body "running")
   :expanded t :navigation 'always)
  (agent-shell-ui-update-fragment
   (agent-shell-ui-make-fragment-model
    :namespace-id "turn" :block-id "test"
    :group-id "different-late-id" :group-label "Wrong"
    :label-left "done" :label-right "nextest"
    :body "\n24 passed")
   :append t :navigation 'always)
  (let* ((children
          (agent-shell-ui--group-children
           :group-qualified-id "turn-activity"))
         (before
          (mapcar
           (lambda (child)
             (list (map-elt child :qualified-id)
                   (get-text-property (map-elt child :start)
                                      'invisible)))
           children)))
    (let ((inhibit-read-only t))
      (agent-shell-ui--set-group-collapsed "turn-activity" t))
    (let ((folded
           (mapcar
            (lambda (child)
              (get-text-property (map-elt child :start) 'invisible))
            children)))
      (let ((inhibit-read-only t))
        (agent-shell-ui--set-group-collapsed "turn-activity" nil))
      (list (substring-no-properties (buffer-string))
            (mapcar (lambda (child) (map-elt child :qualified-id))
                    children)
            before
            folded
            (mapcar
             (lambda (child)
               (get-text-property (map-elt child :start) 'invisible))
             children)
            (agent-shell-ui--group-header-range
             "turn-different-late-id")))))
"##;
    let expect = expect![""];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn property_ranges_recover_whole_blocks_and_nested_body_sections() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "header\nbody\nnext\n")
  (let ((first '((:qualified-id . "turn-tool")
                 (:kind . fragment)
                 (:navigatable . t)))
        (second '((:qualified-id . "turn-next")
                  (:kind . fragment)
                  (:navigatable . t))))
    (add-text-properties 1 13
                         `(agent-shell-ui-state ,first))
    (add-text-properties 13 (point-max)
                         `(agent-shell-ui-state ,second))
    (add-text-properties 1 8
                         '(agent-shell-ui-section labels))
    (add-text-properties 8 13
                         '(agent-shell-ui-section body))
    (goto-char 10)
    (list
     (agent-shell-ui--block-range :position (point))
     (agent-shell-ui--nearest-range-matching-property
      :property 'agent-shell-ui-section :value 'body
      :from 1 :to 13)
     (agent-shell-ui--nearest-range-matching-property
      :property 'agent-shell-ui-state :value "turn-next"
      :predicate
      (lambda (qualified-id state)
        (equal qualified-id (map-elt state :qualified-id)))))))
"##;
    let expect = expect![
        "OK (((:start . 1) (:end . 13)) ((:start . 8) (:end . 13)) ((:start . 13) (:end . 18)))"
    ];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn navigation_skips_invisible_activity_but_keeps_collapsed_headers_reachable() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "one\nhidden\nthree\n")
  (let ((one '((:qualified-id . "turn-one")
               (:navigatable . t)))
        (hidden '((:qualified-id . "turn-hidden")
                  (:navigatable . t)))
        (three '((:qualified-id . "turn-three")
                 (:navigatable . t)
                 (:collapsed . t))))
    (add-text-properties 1 5 `(agent-shell-ui-state ,one))
    (add-text-properties 5 12
                         `(agent-shell-ui-state ,hidden invisible t))
    (add-text-properties 12 (point-max)
                         `(agent-shell-ui-state ,three))
    (goto-char 1)
    (let ((forward (agent-shell-ui-forward-block)))
      (goto-char 15)
      (list forward
            (map-elt
             (get-text-property forward 'agent-shell-ui-state)
             :qualified-id)
            (agent-shell-ui-backward-block)
            (point)))))
"##;
    let expect = expect![[r#"OK (12 "turn-three" 12 12)"#]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn majority_fold_state_counts_distinct_navigatable_fragments() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "one\ntwo\nthree\nignored\n")
  (let ((one '((:qualified-id . "one")
               (:navigatable . t)
               (:collapsed . t)))
        (two '((:qualified-id . "two")
               (:navigatable . t)
               (:collapsed . t)))
        (three '((:qualified-id . "three")
                 (:navigatable . t)
                 (:collapsed . nil)))
        (ignored '((:qualified-id . "ignored")
                   (:navigatable . nil)
                   (:collapsed . t))))
    (add-text-properties 1 5 `(agent-shell-ui-state ,one))
    (add-text-properties 5 9 `(agent-shell-ui-state ,two))
    (add-text-properties 9 15 `(agent-shell-ui-state ,three))
    (add-text-properties 15 (point-max)
                         `(agent-shell-ui-state ,ignored))
    (let ((mostly-collapsed
           (agent-shell-ui--majority-collapsed-p)))
      (setf (map-elt one :collapsed) nil)
      (list mostly-collapsed
            (agent-shell-ui--majority-collapsed-p)))))
"##;
    let expect = expect!["OK (t t)"];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn actionable_labels_expose_keyboard_mouse_help_and_face_contracts() {
    let elisp_form = r##"
(let* ((calls nil)
       (text
        (agent-shell-ui-add-action-to-text
         "open diff"
         (lambda () (interactive) (push 'activated calls))
         (lambda () (push 'entered calls))
         'font-lock-keyword-face))
       (map (get-text-property 0 'keymap text))
       (action (lookup-key map (kbd "RET"))))
  (funcall action)
  (list (substring-no-properties text)
        (keymapp map)
        (get-text-property 0 'mouse-face text)
        (get-text-property 0 'help-echo text)
        (get-text-property 0 'font-lock-face text)
        calls))
"##;
    let expect = expect![[r#"OK ("open diff" t nil nil font-lock-keyword-face (activated))"#]];
    assert_agent_shell_parity(elisp_form, expect);
}
