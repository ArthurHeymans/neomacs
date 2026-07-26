use super::assert_ack_menu_autoload_parity;
use expect_test::expect;

#[test]
fn ack_menu_autoload_file_registers_all_public_entry_points_without_loading_runtime() {
    let elisp_form = r##"(list
         (featurep
          'ack-menu)
         (mapcar
          (lambda (symbol)
            (let* ((definition
                    (symbol-function
                     symbol))
                   (file
                    (nth
                     1
                     definition)))
              (list
               symbol
               (autoloadp definition)
               (nth
                3
                definition)
               (nth
                4
                definition)
               (nth
                2
                definition)
               (cond
                ((stringp file)
                 (file-name-nondirectory
                  file))
                ((and
                  (consp file)
                  (stringp
                   (car file)))
                 (list
                  (file-name-nondirectory
                   (car file))))
                (t file)))))
          '(ack-find-same-file
            ack-find-file
            ack-menu))
         (featurep
          'ack-menu-autoloads))"##;
    let expect = expect![[
        r#"OK (nil ((ack-find-same-file t t nil "Prompt to find a file found by ack in DIRECTORY.\n\n(fn &optional DIRECTORY)" "ack-menu") (ack-find-file t t nil "Prompt to find a file found by ack in DIRECTORY.\n\n(fn &optional DIRECTORY)" "ack-menu") (ack-menu t t nil "Invoke the ack menu. When finished, ack will be run with the\nspecified options." "ack-menu")) t)"#
    ]];
    assert_ack_menu_autoload_parity(elisp_form, expect);
}

#[test]
fn ack_menu_find_file_autoload_loads_runtime_and_composes_selected_path() {
    let elisp_form = r##"(let ((ido-mode t)
              calls)
         (cl-letf
             (((symbol-function
                'call-process)
               (lambda (&rest arguments)
                 (push
                  (cons
                   'call-process
                   arguments)
                  calls)
                 (insert
                  "one.el\0two.el\0")
                 0))
              ((symbol-function
                'ido-completing-read)
               (lambda (&rest arguments)
                 (push
                  (cons
                   'ido
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
            (ack-find-file
             temporary-file-directory)
            (featurep
             'ack-menu)
            (autoloadp
             (symbol-function
              'ack-find-file))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (visited t nil ((call-process nil nil t nil "-f" "--print0") (ido "Find file: " ("two.el" "one.el") nil t) (find-file "[ORACLE-TMPDIR]/two.el")))"#
    ]];
    assert_ack_menu_autoload_parity(elisp_form, expect);
}
