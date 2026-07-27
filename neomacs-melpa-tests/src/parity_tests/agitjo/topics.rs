use expect_test::expect;

use super::assert_agitjo_parity;

#[test]
fn agitjo_topics_are_isolated_by_project_root_and_can_be_replaced_or_cleared() {
    let elisp_form = r##"(let ((agitjo--current-topics nil)
               (root "/projects/one/"))
         (cl-letf (((symbol-function 'project-current)
                    (lambda (&rest _) 'project))
                   ((symbol-function 'project-root)
                    (lambda (_project) root)))
           (list
            (agitjo--get-current-topic)
            (agitjo--set-current-topic "topic-a")
            (agitjo--get-current-topic)
            (progn
              (setq root "/projects/two/")
              (agitjo--get-current-topic))
            (agitjo--set-current-topic "topic-b")
            agitjo--current-topics
            (progn
              (setq root "/projects/one/")
              (agitjo--set-current-topic nil)
              (agitjo--get-current-topic))
            agitjo--current-topics)))"##;
    let expect = expect![[
        r#"OK (nil "topic-a" "topic-a" nil "topic-b" #1=(("/projects/two/" . "topic-b") ("/projects/one/")) nil #1#)"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_topics_without_project_are_noops_and_do_not_create_global_state() {
    let elisp_form = r##"(let ((agitjo--current-topics '(("/kept/" . "value"))))
         (cl-letf (((symbol-function 'project-current) (lambda (&rest _) nil)))
           (list
            (agitjo--get-current-topic)
            (agitjo--set-current-topic "ignored")
            agitjo--current-topics)))"##;
    let expect = expect![[r#"OK (nil nil (("/kept/" . "value")))"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_topic_reader_clears_existing_value_path_or_forwards_prompt_arguments() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'read-string)
                    (lambda (&rest args)
                      (push args calls)
                      "entered")))
           (list
            (cl-letf (((symbol-function 'agitjo--get-current-topic)
                       (lambda () "existing")))
              (agitjo--topic-reader "Topic: " "seed" 'history))
            (cl-letf (((symbol-function 'agitjo--get-current-topic)
                       (lambda () nil)))
              (agitjo--topic-reader "Topic: " "seed" 'history))
            (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil "entered" (("Topic: " "seed" history)))"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_topic_infix_set_and_format_delegate_to_project_topic_state() {
    let elisp_form = r##"(let ((obj (agitjo--topic-variable-infix))
               set-values)
         (cl-letf (((symbol-function 'agitjo--set-current-topic)
                    (lambda (value)
                      (push value set-values)))
                   ((symbol-function 'agitjo--get-current-topic)
                    (lambda () "release/2")))
           (list
            (transient-infix-set obj "new-topic")
            (substring-no-properties (transient-format-value obj))
            (nreverse set-values)
            (cl-letf (((symbol-function 'agitjo--get-current-topic)
                       (lambda () nil)))
              (substring-no-properties
               (transient-format-value obj))))))"##;
    let expect =
        expect![[r#"OK (#1=("new-topic") "(release/2)" #1# "(<use source branch/ref>)")"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_pull_request_type_switches_initialize_and_cycle_real_transient_values() {
    let elisp_form = r##"(let* ((transient--prefix
                 (transient-prefix :command 'agitjo-test-prefix))
                (empty
                (agitjo--pullreq-type-switches-infix
                 :argument-format "%s"
                 :argument-regexp "\\`\\(normal\\|draft\\)\\'"
                 :choices '("normal" "draft")))
               (normal
                (agitjo--pullreq-type-switches-infix
                 :argument-format "%s"
                 :argument-regexp "\\`\\(normal\\|draft\\)\\'"
                 :choices '("normal" "draft")))
               (draft
                (agitjo--pullreq-type-switches-infix
                 :argument-format "%s"
                 :argument-regexp "\\`\\(normal\\|draft\\)\\'"
                 :choices '("normal" "draft"))))
         (oset transient--prefix value nil)
         (oset normal value "normal")
         (oset draft value "draft")
         (list
          (transient-init-value empty)
          (oref empty value)
          (transient-infix-read normal)
          (transient-infix-read draft)))"##;
    let expect = expect![[r#"OK ("normal" "normal" "draft" "normal")"#]];
    assert_agitjo_parity(elisp_form, expect);
}

#[test]
fn agitjo_current_branch_heading_renders_branch_or_detached_state() {
    let elisp_form = r##"(list
         (cl-letf (((symbol-function 'magit-get-current-branch)
                    (lambda () "feature/widgets")))
           (substring-no-properties
            (agitjo-push--pullreq-current-description)))
         (cl-letf (((symbol-function 'magit-get-current-branch)
                    (lambda () nil)))
           (agitjo-push--pullreq-current-description)))"##;
    let expect = expect![[
        r#"OK ("Push PR from feature/widgets to" "Push PR from <no current branch> to")"#
    ]];
    assert_agitjo_parity(elisp_form, expect);
}
