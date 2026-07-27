use expect_test::expect;

use super::assert_archive_region_parity;

#[test]
fn archive_region_dispatch_plain_prefix_performs_real_kill_region_and_updates_kill_ring() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "keep before\n"
          "kill α β\n"
          "keep after\n")
         (goto-char
          (point-min))
         (forward-line 1)
         (let ((start
                (point)))
           (forward-line 1)
           (let ((end
                  (point))
                 (kill-ring
                  nil)
                 (kill-ring-yank-pointer
                  nil))
             (list
              (kill-region-or-archive-region
               1
               start
               end)
              (buffer-string)
              kill-ring
              (current-kill
               0
               t)
              (point)
              (file-exists-p
               (archive-region-test-path
                "unrelated_archive"))))))"##;
    let expect =
        expect![[r#"OK (nil "keep before\nkeep after\n" ("kill α β\n") "kill α β\n" 13 nil)"#]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_dispatch_plain_prefix_routes_cua_rectangle_without_killing_linear_region() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "alpha\nbeta\ngamma\n")
         (let ((before
                (buffer-string))
               (kill-ring
                '("existing"))
               (kill-ring-yank-pointer
                nil)
               calls)
           (setq
            kill-ring-yank-pointer
            kill-ring)
           (set
            'cua--rectangle
            '((1 . 2)
              (3 . 4)))
           (unwind-protect
               (cl-letf
                   (((symbol-function
                      'cua-cut-rectangle)
                     (lambda (argument)
                       (push argument calls)
                       :rectangle-cut)))
                 (list
                  (kill-region-or-archive-region
                   1
                   2
                   7)
                  calls
                  before
                  (buffer-string)
                  kill-ring))
             (makunbound
              'cua--rectangle))))"##;
    let expect = expect![[
        r#"OK (:rectangle-cut (nil) "alpha\nbeta\ngamma\n" "alpha\nbeta\ngamma\n" ("existing"))"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_dispatch_single_universal_prefix_copies_original_then_archives_real_region() {
    let elisp_form = r##"(let* ((source
                 (archive-region-test-path
                  "dispatch-four.el"))
                (archive
                 (concat
                  source
                  archive-region-filename-suffix))
                (kill-ring
                 nil)
                (kill-ring-yank-pointer
                 nil))
         (unwind-protect
             (with-temp-buffer
               (setq-local
                buffer-file-name
                source)
               (emacs-lisp-mode)
               (insert
                "(keep)\n"
                ";; (archive-through-prefix)\n"
                "(tail)\n")
               (goto-char
                (point-min))
               (forward-line 1)
               (let ((start
                      (point)))
                 (forward-line 1)
                 (cl-letf
                     (((symbol-function
                        'format-time-string)
                       (lambda (&rest _)
                         "PREFIX-DATE")))
                   (let ((result
                          (kill-region-or-archive-region
                           4
                           start
                           (point))))
                     (list
                      result
                      (buffer-string)
                      kill-ring
                      (current-kill
                       0
                       t)
                      (archive-region-test-read-file
                       archive))))))
           (when
               (file-exists-p
                archive)
             (delete-file archive))))"##;
    let expect = expect![[
        r#"OK (nil #("(keep)\n(tail)\n" 0 7 (fontified nil) 7 14 (fontified nil)) (#(";; (archive-through-prefix)\n" 0 28 (fontified nil))) #(";; (archive-through-prefix)\n" 0 28 (fontified nil)) ";; PREFIX-DATE\n;; (archive-region-pos \"(keep)\")\n(archive-through-prefix)\n\n")"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_dispatch_double_universal_prefix_opens_archive_other_window_only() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "unchanged")
         (let ((before
                (buffer-string))
               calls)
           (cl-letf
               (((symbol-function
                  'archive-region-open-archive-file-other-window)
                 (lambda ()
                   (push
                    :open-other-window
                    calls)
                   :opened)))
             (list
              (kill-region-or-archive-region
               16
               (point-min)
               (point-max))
              calls
              before
              (buffer-string)
              kill-ring))))"##;
    let expect = expect![[r#"OK (:opened (:open-other-window) "unchanged" "unchanged" nil)"#]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_dispatch_unsupported_numeric_prefixes_are_exact_no_ops() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "unchanged")
         (let ((before
                (buffer-string))
               (kill-ring
                '("existing"))
               (kill-ring-yank-pointer
                nil)
               calls)
           (setq
            kill-ring-yank-pointer
            kill-ring)
           (cl-letf
               (((symbol-function
                  'kill-region)
                 (lambda (&rest arguments)
                   (push
                    (cons
                     :kill
                     arguments)
                    calls)))
                ((symbol-function
                  'archive-region)
                 (lambda (&rest arguments)
                   (push
                    (cons
                     :archive
                     arguments)
                    calls)))
                ((symbol-function
                  'archive-region-open-archive-file-other-window)
                 (lambda ()
                   (push
                    :open
                    calls))))
             (list
              (mapcar
               (lambda (argument)
                 (list
                  argument
                  (kill-region-or-archive-region
                   argument
                   1
                   3)))
               '(0 2 3 5 8 64 -1))
              calls
              before
              (buffer-string)
              kill-ring))))"##;
    let expect = expect![[
        r#"OK (((0 nil) (2 nil) (3 nil) (5 nil) (8 nil) (64 nil) (-1 nil)) nil "unchanged" "unchanged" ("existing"))"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}

#[test]
fn archive_region_dispatch_archive_without_filename_updates_kill_ring_before_error() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "before\n"
          "archive me\n"
          "after\n")
         (goto-char
          (point-min))
         (forward-line 1)
         (let ((start
                (point))
               (kill-ring
                nil)
               (kill-ring-yank-pointer
                nil))
           (forward-line 1)
           (let ((end
                  (point))
                 (before
                  (buffer-string)))
             (list
              (condition-case error-data
                  (list
                   :ok
                   (kill-region-or-archive-region
                    4
                    start
                    end))
                (error
                 (list
                  :error
                  (car error-data)
                  (cdr error-data))))
              before
              (buffer-string)
              kill-ring
              (current-kill
               0
               t)
              buffer-file-name))))"##;
    let expect = expect![[
        r#"OK ((:error error ("Need filename")) "before\narchive me\nafter\n" "before\narchive me\nafter\n" ("archive me\n") "archive me\n" nil)"#
    ]];

    assert_archive_region_parity(elisp_form, expect);
}
