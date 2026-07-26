use super::assert_ack_menu_parity;
use expect_test::expect;

#[test]
fn ack_menu_region_helpers_cover_active_empty_inactive_and_symbol_defaults() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert
            "alpha beta")
           (goto-char 2)
           (set-mark 6)
           (setq mark-active t
                 transient-mark-mode t)
           (cl-letf
               (((symbol-function
                  'use-region-p)
                 (lambda () t)))
             (list
              (ack--use-region-p)
              (ack--initial-contents-for-read)
              (ack--default-for-read))))
         (with-temp-buffer
           (insert
            "alpha beta")
           (goto-char 3)
           (cl-letf
               (((symbol-function
                  'use-region-p)
                 (lambda () nil)))
             (list
              (ack--use-region-p)
              (ack--initial-contents-for-read)
              (ack--default-for-read))))
         (with-temp-buffer
           (insert
            "alpha")
           (goto-char 3)
           (set-mark 3)
           (setq mark-active t
                 transient-mark-mode t)
           (cl-letf
               (((symbol-function
                  'use-region-p)
                 (lambda () nil)))
             (ack--use-region-p))))"##;
    let expect = expect![[r#"OK ((t "lpha" nil) (nil nil "alpha") nil)"#]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_read_builds_exact_literal_and_regexp_prompts_and_history_arguments() {
    let elisp_form = r##"(let (calls)
         (cl-labels
             ((scenario
               (regexp default initial)
               (cl-letf
                   (((symbol-function
                      'ack--default-for-read)
                     (lambda ()
                       default))
                    ((symbol-function
                      'ack--initial-contents-for-read)
                     (lambda ()
                       initial))
                    ((symbol-function
                      'read-string)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       "answer")))
                 (ack--read regexp))))
           (list
            (scenario nil
                      "symbol"
                      "region")
            (scenario t nil nil)
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("answer" "answer" (("ack literal search (default symbol): " "region" ack-literal-history "symbol") ("ack pattern search: " nil ack-regexp-history nil)))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_read_file_dispatches_to_ido_and_iswitchb_with_exact_choices() {
    let elisp_form = r##"(progn
         (defvar iswitchb-temp-buflist)
         (let (calls)
         (list
          (let ((ido-mode t))
            (cl-letf
                (((symbol-function
                   'ido-completing-read)
                  (lambda (&rest arguments)
                    (push
                     (cons 'ido arguments)
                     calls)
                    "ido-choice")))
              (ack-read-file
               "Pick: "
               '("a" "b"))))
          (condition-case error
              (progn
                (require
                 'iswitchb)
                (let ((ido-mode nil))
                  (cl-letf
                      (((symbol-function
                         'iswitchb-read-buffer)
                        (lambda (&rest arguments)
                          (run-hooks
                           'iswitchb-make-buflist-hook)
                          (push
                           (list
                            'iswitchb
                            arguments
                            (copy-sequence
                             iswitchb-temp-buflist))
                           calls)
                          "iswitchb-choice")))
                    (ack-read-file
                     "Pick: "
                     '("x" "y")))))
            (error
             (list
              'error
              (car error))))
          (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("ido-choice" "iswitchb-choice" ((ido "Pick: " ("a" "b") nil t) (iswitchb ("Pick: " nil t) ("x" "y"))))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_find_file_commands_compose_directory_types_choices_and_target_path() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'ack-read-dir)
               (lambda ()
                 "/fixture/root/"))
              ((symbol-function
                'ack-type)
               (lambda ()
                 '("--type"
                   "elisp")))
              ((symbol-function
                'ack-list-files)
               (lambda (&rest arguments)
                 (push
                  (cons
                   'list-files
                   arguments)
                  calls)
                 '("one.el"
                   "two.el")))
              ((symbol-function
                'ack-read-file)
               (lambda (&rest arguments)
                 (push
                  (cons
                   'read-file
                   arguments)
                  calls)
                 "two.el"))
              ((symbol-function
                'find-file)
               (lambda (path)
                 (push
                  (list
                   'find-file
                   path)
                  calls)
                 'visited)))
           (list
            (ack-find-same-file
             "/fixture/same/")
            (ack-find-file
             "/fixture/all/")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (visited visited ((list-files "/fixture/same/" "--type" "elisp") (read-file "Find file: " #1=("one.el" "two.el")) (find-file "/fixture/same/two.el") (list-files "/fixture/all/") (read-file "Find file: " #1#) (find-file "/fixture/all/two.el")))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}
