use expect_test::expect;

use super::assert_advent_mode_parity;

#[test]
fn advent_mode_directory_creation_distinguishes_new_and_existing_paths() {
    let elisp_form = r##"(let* ((root
                (expand-file-name
                 "advent-fs/"
                 temporary-file-directory))
               (target (expand-file-name "nested/day/" root))
               messages)
         (make-directory root t)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest arguments)
                      (push (apply #'format format-string arguments)
                            messages))))
           (list
            (advent--maybe-create-dir target)
            (file-directory-p target)
            (advent--maybe-create-dir target)
            (nreverse messages))))"##;
    let expect = expect![[r#"OK (t t nil ("Created [ORACLE-TMPDIR]/advent-fs/nested/day/"))"#]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_template_copying_resolves_relative_and_absolute_sources_and_overwrites() {
    let elisp_form = r##"(let* ((root (make-temp-file "advent-fs-" t))
               (outside (make-temp-file "advent-absolute-" t))
               (target (expand-file-name "target/" root))
               (relative "templates/solution.el")
               (relative-source (expand-file-name relative root))
               (absolute-source (expand-file-name "helper.py" outside)))
         (make-directory (file-name-directory relative-source) t)
         (make-directory target t)
         (with-temp-file relative-source
           (insert "relative-v1"))
         (with-temp-file absolute-source
           (insert "absolute"))
         (with-temp-file (expand-file-name "solution.el" target)
           (insert "old"))
         (advent--copy-templates
          (list relative absolute-source)
          target
          root)
         (list
          (with-temp-buffer
            (insert-file-contents
             (expand-file-name "solution.el" target))
            (buffer-string))
          (with-temp-buffer
            (insert-file-contents
             (expand-file-name "helper.py" target))
            (buffer-string))
          (sort (directory-files target nil
                                 directory-files-no-dot-files-regexp)
                #'string<)))"##;
    let expect = expect![[r#"OK ("relative-v1" "absolute" ("helper.py" "solution.el"))"#]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_template_copying_warns_for_missing_files_and_continues() {
    let elisp_form = r##"(let* ((root (make-temp-file "advent-fs-" t))
               (target (expand-file-name "target/" root))
               (present (expand-file-name "present.txt" root))
               warnings)
         (make-directory target t)
         (with-temp-file present (insert "present"))
         (cl-letf (((symbol-function 'display-warning)
                    (lambda (type message &rest arguments)
                      (push (list type message arguments) warnings))))
           (advent--copy-templates
            '("missing.txt" "present.txt")
            target
            root)
           (list
            (file-exists-p (expand-file-name "missing.txt" target))
            (with-temp-buffer
              (insert-file-contents
               (expand-file-name "present.txt" target))
              (buffer-string))
            (nreverse warnings))))"##;
    let expect = expect![[r#"OK (nil "present" nil)"#]];
    assert_advent_mode_parity(elisp_form, expect);
}

#[test]
fn advent_mode_input_writer_creates_parent_and_writes_exact_puzzle_body() {
    let elisp_form = r##"(let* ((root
                (expand-file-name
                 "advent-fs/"
                 temporary-file-directory))
               (file (expand-file-name "deep/input.txt" root))
               calls)
         (make-directory root t)
         (cl-letf (((symbol-function 'advent--http-request)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      "1000\n2000\n\n3000\n")))
           (list
            (advent--write-url-to-file "https://example.test/input" file)
            (file-exists-p file)
            (with-temp-buffer
              (insert-file-contents file)
              (buffer-string))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-TMPDIR]/advent-fs/deep/input.txt" t "1000\n2000\n\n3000\n" (("https://example.test/input" "GET" nil t)))"#
    ]];
    assert_advent_mode_parity(elisp_form, expect);
}
