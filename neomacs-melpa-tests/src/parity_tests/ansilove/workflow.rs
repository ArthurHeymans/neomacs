use expect_test::expect;

use super::{assert_ansilove_parity, assert_ansilove_signal_parity};

#[test]
fn convert_and_display_visits_the_rendered_png_from_the_current_buffer() {
    let elisp_form = r##"(with-temp-buffer
  (insert "ANSI art to render")
  (let (events)
    (cl-letf (((symbol-function 'ansilove--check-executable)
               (lambda ()
                 (push '(check-executable) events)
                 t))
              ((symbol-function 'ansilove--buffer-to-png)
               (lambda (buffer)
                 (push
                  (list 'buffer-to-png
                        (eq buffer (current-buffer))
                        (with-current-buffer buffer (buffer-string)))
                  events)
                 "/workspace/rendered.png"))
              ((symbol-function 'find-file)
               (lambda (file)
                 (push (list 'find-file file) events)
                 'visited)))
      (list
       (ansilove-convert-and-display-now)
       (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (visited ((check-executable) (buffer-to-png t "ANSI art to render") (find-file "/workspace/rendered.png")))"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn convert_and_display_rejects_an_unusable_executable_before_conversion() {
    let elisp_form = r##"(let ((ansilove-executable "/missing/ansilove"))
  (cl-letf (((symbol-function 'ansilove--check-executable) (lambda () nil))
            ((symbol-function 'ansilove--buffer-to-png)
             (lambda (_buffer)
               (error "conversion must not start")))
            ((symbol-function 'find-file)
             (lambda (_file)
               (error "display must not start"))))
    (ansilove-convert-and-display-now)))"##;
    let expect = expect![[
        r#"ERR (user-error "Fatal error: The required executable /missing/ansilove is unusable!")"#
    ]];
    assert_ansilove_signal_parity(elisp_form, expect);
}

#[test]
fn top_level_conversion_initializes_then_cleans_then_converts_when_cleanup_is_enabled() {
    let elisp_form = r##"(let ((ansilove-clean-temporary-directory-before-conversion t)
      events)
  (cl-letf (((symbol-function 'ansilove--init-temporary-directory)
             (lambda () (push 'initialize events) 'initialized))
            ((symbol-function 'ansilove-clean-temporary-directory)
             (lambda () (push 'clean events) 'cleaned))
            ((symbol-function 'ansilove-convert-and-display-now)
             (lambda () (push 'convert events) 'displayed)))
    (list
     (ansilove)
     (nreverse events))))"##;
    let expect = expect!["OK (displayed (initialize clean convert))"];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn top_level_conversion_skips_cleanup_but_preserves_initialize_then_convert_order_by_default() {
    let elisp_form = r##"(let ((ansilove-clean-temporary-directory-before-conversion nil)
      events)
  (cl-letf (((symbol-function 'ansilove--init-temporary-directory)
             (lambda () (push 'initialize events) 'initialized))
            ((symbol-function 'ansilove-clean-temporary-directory)
             (lambda () (push 'unexpected-clean events) 'cleaned))
            ((symbol-function 'ansilove-convert-and-display-now)
             (lambda () (push 'convert events) 'displayed)))
    (list
     (ansilove)
     (nreverse events))))"##;
    let expect = expect!["OK (displayed (initialize convert))"];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn quick_example_downloads_once_opens_the_file_enters_mode_and_runs_the_full_conversion() {
    let elisp_form = r##"(let* ((directory
         (expand-file-name "ansilove-quick-new" temporary-file-directory))
        (ansilove-temporary-directory (file-name-as-directory directory))
        (ansilove-quick-test-example-url "https://example.invalid/practical.ans")
        events)
  (when (file-exists-p directory)
    (delete-directory directory t))
  (unwind-protect
      (cl-letf (((symbol-function 'ansilove--init-temporary-directory)
                 (lambda ()
                   (push '(initialize) events)
                   (make-directory directory t)))
                ((symbol-function 'url-copy-file)
                 (lambda (url file &rest arguments)
                   (push (list 'download url file arguments) events)
                   (with-temp-file file
                     (insert "\e[31mDOWNLOADED ART\e[0m\n"))
                   nil))
                ((symbol-function 'find-file-noselect)
                 (lambda (file &rest arguments)
                   (push (list 'open file arguments) events)
                   (get-buffer-create "*ansilove-quick-new*")))
                ((symbol-function 'ansilove-mode)
                 (lambda ()
                   (push
                    (list 'mode
                          ansilove-clean-temporary-directory-before-conversion)
                    events)
                   'mode-entered))
                ((symbol-function 'ansilove)
                 (lambda ()
                   (push
                    (list 'convert
                          ansilove-clean-temporary-directory-before-conversion)
                    events)
                   'converted)))
        (list
         (ansilove-quick-test-example)
         (nreverse events)
         (with-temp-buffer
           (insert-file-contents-literally
            (expand-file-name "test.txt" directory))
           (buffer-string))))
    (when (get-buffer "*ansilove-quick-new*")
      (kill-buffer "*ansilove-quick-new*"))
    (when (file-exists-p directory)
      (delete-directory directory t))))"##;
    let expect = expect![[
        r#"OK (converted ((initialize) (download "https://example.invalid/practical.ans" "[ORACLE-TMPDIR]/ansilove-quick-new/test.txt" nil) (open "[ORACLE-TMPDIR]/ansilove-quick-new/test.txt" nil) (mode nil) (convert nil)) "\33[31mDOWNLOADED ART\33[0m\n")"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn quick_example_reuses_an_existing_download_and_still_enters_mode_before_conversion() {
    let elisp_form = r##"(let* ((directory
         (expand-file-name "ansilove-quick-existing" temporary-file-directory))
        (ansilove-temporary-directory (file-name-as-directory directory))
        (test-file (expand-file-name "test.txt" directory))
        events)
  (make-directory directory t)
  (with-temp-file test-file
    (insert "already cached"))
  (unwind-protect
      (cl-letf (((symbol-function 'ansilove--init-temporary-directory)
                 (lambda () (push '(initialize) events)))
                ((symbol-function 'url-copy-file)
                 (lambda (&rest arguments)
                   (push (cons 'unexpected-download arguments) events)))
                ((symbol-function 'find-file-noselect)
                 (lambda (file &rest _arguments)
                   (push (list 'open file) events)
                   (get-buffer-create "*ansilove-quick-existing*")))
                ((symbol-function 'ansilove-mode)
                 (lambda () (push '(mode) events)))
                ((symbol-function 'ansilove)
                 (lambda () (push '(convert) events) 'converted)))
        (list
         (ansilove-quick-test-example)
         (nreverse events)
         (with-temp-buffer
           (insert-file-contents-literally test-file)
           (buffer-string))))
    (when (get-buffer "*ansilove-quick-existing*")
      (kill-buffer "*ansilove-quick-existing*"))
    (delete-directory directory t)))"##;
    let expect = expect![[
        r#"OK (converted ((initialize) (open "[ORACLE-TMPDIR]/ansilove-quick-existing/test.txt") (mode) (convert)) "already cached")"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}
