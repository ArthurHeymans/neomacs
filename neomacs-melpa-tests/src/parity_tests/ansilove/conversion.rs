use expect_test::expect;

use super::{assert_ansilove_parity, assert_ansilove_signal_parity};

#[test]
fn converter_builds_the_exact_shell_command_and_routes_process_output_to_its_log_buffer() {
    let elisp_form = r##"(let ((ansilove-executable "/opt/tools/ansilove")
      captured)
  (with-temp-buffer
    (insert "ANSI payload")
    (let ((input (expand-file-name "source.ans" temporary-file-directory))
          (output (expand-file-name "rendered.png" temporary-file-directory)))
      (write-region (point-min) (point-max) input nil 'silent)
      (unwind-protect
          (cl-letf (((symbol-function 'call-process-shell-command)
                     (lambda (command infile destination &rest arguments)
                       (setq captured
                             (list command
                                   infile
                                   (buffer-name destination)
                                   arguments))
                       (with-current-buffer destination
                         (erase-buffer)
                         (insert "converter diagnostic"))
                       17)))
            (list
             (ansilove--convert-file-to-png input output)
             captured
             (with-current-buffer "*Ansilove-Output*"
               (buffer-string))))
        (delete-file input)
        (when (get-buffer "*Ansilove-Output*")
          (kill-buffer "*Ansilove-Output*"))))))"##;
    let expect = expect![[
        r#"OK (17 ("/opt/tools/ansilove -o [ORACLE-TMPDIR]/rendered.png [ORACLE-TMPDIR]/source.ans" nil "*Ansilove-Output*" nil) "converter diagnostic")"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn converter_rejects_an_unreadable_input_before_invoking_the_external_process() {
    let elisp_form = r##"(let ((ansilove-executable "ansilove")
      (input (expand-file-name "missing-input.ans" temporary-file-directory)))
  (cl-letf (((symbol-function 'call-process-shell-command)
             (lambda (&rest _arguments)
               (error "external process must not run"))))
    (ansilove--convert-file-to-png
     input
     (expand-file-name "unused.png" temporary-file-directory))))"##;
    let expect = expect![[
        r#"ERR (user-error "Fatal error: The file [ORACLE-TMPDIR]/missing-input.ans is not readable!")"#
    ]];
    assert_ansilove_signal_parity(elisp_form, expect);
}

#[test]
fn executable_check_prefers_path_lookup_then_falls_back_to_an_explicit_executable_file() {
    let elisp_form = r##"(let ((ansilove-executable "chosen-tool")
      calls)
  (cl-letf (((symbol-function 'executable-find)
             (lambda (name)
               (push (list 'find name) calls)
               "/tools/chosen-tool"))
            ((symbol-function 'file-executable-p)
             (lambda (name)
               (push (list 'file name) calls)
               'fallback)))
    (let ((path-result (ansilove--check-executable)))
      (setq ansilove-executable "/private/chosen-tool")
      (cl-letf (((symbol-function 'executable-find)
                 (lambda (name)
                   (push (list 'find name) calls)
                   nil)))
        (let ((file-result (ansilove--check-executable)))
          (setq ansilove-executable "missing-tool")
          (cl-letf (((symbol-function 'file-executable-p)
                     (lambda (name)
                       (push (list 'file name) calls)
                       nil)))
            (list
             path-result
             file-result
             (ansilove--check-executable)
             (nreverse calls))))))))"##;
    let expect = expect![[
        r#"OK ("/tools/chosen-tool" fallback nil ((find "chosen-tool") (find "/private/chosen-tool") (file "/private/chosen-tool") (find "missing-tool") (file "missing-tool")))"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn file_backed_buffer_conversion_uses_the_visited_file_and_deterministic_png_destination() {
    let elisp_form = r##"(let* ((directory
        (expand-file-name "ansilove-file-backed" temporary-file-directory))
       (ansilove-temporary-directory (file-name-as-directory directory))
       (input (expand-file-name "art.ans" directory))
       captured)
  (make-directory directory t)
  (with-temp-file input
    (insert "\e[31mREAL ANSI ART\e[0m\n"))
  (unwind-protect
      (with-current-buffer (find-file-noselect input)
        (cl-letf (((symbol-function 'random) (lambda (&rest _arguments) -4242))
                  ((symbol-function 'ansilove--convert-file-to-png)
                   (lambda (source destination)
                     (setq captured (list source destination))
                     'converted)))
          (list
           (ansilove--buffer-to-png (current-buffer))
           captured
           (buffer-file-name)
           (buffer-string))))
    (when (get-file-buffer input)
      (kill-buffer (get-file-buffer input)))
    (delete-directory directory t)))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-TMPDIR]/ansilove-file-backed/ansilove_4242.png" ("[ORACLE-TMPDIR]/ansilove-file-backed/art.ans" "[ORACLE-TMPDIR]/ansilove-file-backed/ansilove_4242.png") "[ORACLE-TMPDIR]/ansilove-file-backed/art.ans" "\33[31mREAL ANSI ART\33[0m\n")"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn non_file_buffer_conversion_stages_exact_contents_then_removes_the_temporary_input() {
    let elisp_form = r##"(let* ((directory
        (expand-file-name "ansilove-memory-buffer" temporary-file-directory))
       (ansilove-temporary-directory (file-name-as-directory directory))
       captured)
  (make-directory directory t)
  (unwind-protect
      (with-temp-buffer
        (insert "first line\n\e[1;34mblue block\e[0m\n最後の行")
        (cl-letf (((symbol-function 'random) (lambda (&rest _arguments) 731))
                  ((symbol-function 'ansilove--convert-file-to-png)
                   (lambda (source destination)
                     (setq captured
                           (list
                            source
                            destination
                            (file-exists-p source)
                            (with-temp-buffer
                              (insert-file-contents-literally source)
                              (buffer-string))))
                     'converted)))
          (let* ((result (ansilove--buffer-to-png (current-buffer)))
                 (input (expand-file-name "ansilove_731.txt" directory)))
            (list
             result
             captured
             (file-exists-p input)
             (get-buffer "ansilove_731.txt")))))
    (delete-directory directory t)))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-TMPDIR]/ansilove-memory-buffer/ansilove_731.png" ("[ORACLE-TMPDIR]/ansilove-memory-buffer/ansilove_731.txt" "[ORACLE-TMPDIR]/ansilove-memory-buffer/ansilove_731.png" t "first line\n\33[1;34mblue block\33[0m\n\346\234\200\345\276\214\343\201\256\350\241\214\n") nil nil)"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}

#[test]
fn converter_shell_contract_keeps_embedded_spaces_visible_in_all_three_paths() {
    let elisp_form = r##"(let ((ansilove-executable "/tools/Ansi Love/bin/ansilove")
      captured)
  (with-temp-buffer
    (insert "payload")
    (let* ((directory
            (expand-file-name "directory with spaces" temporary-file-directory))
           (input (expand-file-name "source art.ans" directory))
           (output (expand-file-name "output art.png" directory)))
      (make-directory directory t)
      (write-region (point-min) (point-max) input nil 'silent)
      (unwind-protect
          (cl-letf (((symbol-function 'call-process-shell-command)
                     (lambda (command &rest _arguments)
                       (setq captured command)
                       0)))
            (list
             (ansilove--convert-file-to-png input output)
             captured))
        (delete-directory directory t)))))"##;
    let expect = expect![[
        r#"OK (0 "/tools/Ansi Love/bin/ansilove -o [ORACLE-TMPDIR]/directory with spaces/output art.png [ORACLE-TMPDIR]/directory with spaces/source art.ans")"#
    ]];
    assert_ansilove_parity(elisp_form, expect);
}
