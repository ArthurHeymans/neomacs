use expect_test::expect;

use super::assert_f_parity;

#[test]
fn f_create_touch_delete_and_empty_predicates_match() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-create-" t))
                    (default-directory (file-name-as-directory root)))
               (unwind-protect
                   (progn
                     (f-mkdir "one" "two")
                     (f-mkdir-full-path "three/four")
                     (f-touch "one/empty")
                     (f-write "content" 'utf-8 "one/full")
                     (let ((before
                            (list
                             (f-exists-p "one/two")
                             (f-directory? "one/two")
                             (f-dir-p "three/four")
                             (f-file? "one/empty")
                             (f-empty-p "one/empty")
                             (f-empty? "one/full")
                             (f-empty-p "one/two")
                             (f-empty-p "one"))))
                       (f-delete "one/full")
                       (f-delete "three" t)
                       (list before
                             (f-exists? "one/full")
                             (f-exists-p "three"))))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK ((t t t t t nil t nil) nil nil)"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_copy_move_symlink_and_copy_contents_preserve_data() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-copy-" t))
                    (default-directory (file-name-as-directory root)))
               (unwind-protect
                   (progn
                     (f-mkdir "source")
                     (f-write "alpha" 'utf-8 "source/a.txt")
                     (f-mkdir "source/nested")
                     (f-write "beta" 'utf-8 "source/nested/b.txt")
                     (f-copy "source/a.txt" "copy.txt")
                     (f-move "copy.txt" "moved.txt")
                     (f-symlink "moved.txt" "linked.txt")
                     (f-mkdir "destination")
                     (f-copy-contents "source" "destination")
                     (list
                      (f-read "moved.txt")
                      (f-read "linked.txt")
                      (f-symlink-p "linked.txt")
                      (f-symlink? "linked.txt")
                      (f-read "destination/a.txt")
                      (f-read "destination/nested/b.txt")
                      (sort
                       (mapcar
                        (lambda (path) (f-relative path "destination"))
                        (f-entries "destination" nil t))
                       #'string<)))
                 (delete-directory root t)))"##;
    let expect =
        expect![[r#"OK ("alpha" "alpha" t t "alpha" "beta" ("a.txt" "nested" "nested/b.txt"))"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_path_relationship_predicates_cover_aliases_and_boundaries() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-relations-" t))
                    (default-directory (file-name-as-directory root)))
               (unwind-protect
                   (progn
                     (f-mkdir "a" "b" "c")
                     (f-touch "a/b/c/file")
                     (f-symlink "a/b/c/file" "alias")
                     (list
                      (f-same-p "a/b/c/file" "alias")
                      (f-same? "a/b/c/file" "alias")
                      (f-equal-p "a/b/c/file" "alias")
                      (f-parent-of-p "a/b/c" "a/b/c/file")
                      (f-parent-of? "a/b" "a/b/c/file")
                      (f-child-of-p "a/b/c/file" "a/b/c")
                      (f-child-of? "a/b/c/file" "a/b")
                      (f-ancestor-of-p "a" "a/b/c/file")
                      (f-ancestor-of? "a/b/c/file" "a/b/c/file")
                      (f-descendant-of-p "a/b/c/file" "a")
                      (f-descendant-of? "a" "a/b/c/file")))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK (t t t t nil t nil t nil t nil)"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_hidden_predicate_modes_cover_nested_dot_components() {
    let elisp_form = r##"(list
              (f-hidden-p ".root/visible/file")
              (f-hidden-p "visible/.nested/file")
              (f-hidden-p "visible/.nested/file" 'any)
              (f-hidden-p "visible/.nested/file" 'last)
              (f-hidden-p "visible/nested/.file" 'last)
              (f-hidden-p "./visible/.file" 'any)
              (f-hidden? "../visible")
              (f-hidden-p "visible/file" 'any))"##;
    let expect = expect![[r#"OK (t nil t nil t t nil nil)"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_glob_entries_directories_and_files_filter_recursively() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-list-" t))
                    (default-directory (file-name-as-directory root)))
               (unwind-protect
                   (progn
                     (f-mkdir "tree" "nested")
                     (f-mkdir "tree" "other")
                     (f-touch "tree/a.el")
                     (f-touch "tree/b.txt")
                     (f-touch "tree/nested/c.el")
                     (f-touch "tree/other/d.txt")
                     (list
                      (sort (mapcar #'f-filename (f-glob "*.el" "tree"))
                            #'string<)
                      (sort
                       (mapcar
                        (lambda (path) (f-relative path "tree"))
                        (f-entries "tree" nil t))
                       #'string<)
                      (sort
                       (mapcar
                        (lambda (path) (f-relative path "tree"))
                        (f-directories
                         "tree"
                         (lambda (path)
                           (not (string= (f-filename path) "other")))
                         t))
                       #'string<)
                      (sort
                       (mapcar
                        (lambda (path) (f-relative path "tree"))
                        (f-files
                         "tree"
                         (lambda (path) (f-ext-p path "el"))
                         t))
                       #'string<)))
                 (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (("a.el") ("a.el" "b.txt" "nested" "nested/c.el" "other" "other/d.txt") ("nested") ("a.el" "nested/c.el"))"#
    ]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_anaphoric_entry_macros_bind_each_path() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-anaphoric-" t))
                    (default-directory (file-name-as-directory root)))
               (unwind-protect
                   (progn
                     (f-mkdir "tree" "nested")
                     (f-touch "tree/a.el")
                     (f-touch "tree/b.txt")
                     (f-touch "tree/nested/c.el")
                     (list
                      (sort
                       (mapcar
                        (lambda (path) (f-relative path "tree"))
                        (f--entries "tree" (f-ext-p it "el") t))
                       #'string<)
                      (sort
                       (mapcar
                        (lambda (path) (f-relative path "tree"))
                        (f--directories "tree"
                          (string= (f-filename it) "nested")
                          t))
                       #'string<)
                      (sort
                       (mapcar
                        (lambda (path) (f-relative path "tree"))
                        (f--files "tree" (f-ext-p it "el") t))
                       #'string<)))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK (("a.el" "nested/c.el") ("nested") ("a.el" "nested/c.el"))"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_size_depth_and_traversal_observe_filesystem_shape() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-stats-" t))
                    (default-directory (file-name-as-directory root)))
               (unwind-protect
                   (progn
                     (f-mkdir "one" "two" "marker")
                     (f-write "abc" 'utf-8 "one/a")
                     (f-write "åß" 'utf-8 "one/two/b")
                     (let ((found
                            (f-traverse-upwards
                             (lambda (dir)
                               (file-directory-p
                                (expand-file-name "marker" dir)))
                             "one/two")))
                       (list
                        (f-size "one/a")
                        (f-size "one/two/b")
                        (f-size "one")
                        (- (f-depth "one/two") (f-depth "."))
                        (f-same-p found "one/two")
                        (f-root-p "/")
                        (f-root? "/"))))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK (3 4 7 2 t t t)"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_file_times_and_comparison_aliases_preserve_ordering() {
    // f--get-time dynamically binds `current-time-list` to nil before asking
    // for file attributes. GNU dired.c delegates those timestamps to
    // timefns.c:make_lisp_time, so this also detects an editor that always
    // returns the legacy four-element time list.
    let elisp_form = r##"(let* ((root (make-temp-file "f-times-" t))
                    (older (expand-file-name "older" root))
                    (newer (expand-file-name "newer" root)))
               (unwind-protect
                   (progn
                     (f-touch older)
                     (f-touch newer)
                     (set-file-times older '(1000 0 0 0))
                     (set-file-times newer '(2000 0 0 0))
                     (list
                      (f-older-p older newer)
                      (f-older? older newer)
                      (f-newer-p newer older)
                      (f-newer? newer older)
                      (f-same-time-p older older)
                      (f-same-time? newer newer)
                      (f-modification-time older 'seconds)
                      (f-modification-time older t)
                      (f-modification-time older)
                      (let ((value (f-change-time older t)))
                        (list
                         (integerp (car value))
                         (equal (cdr value) 1000000000)))
                      (let ((value (f-access-time older t)))
                        (list
                         (integerp (car value))
                         (equal (cdr value) 1000000000)))))
                 (delete-directory root t)))"##;
    let expect = expect![[
        r#"OK (t t t t t t 65536000 (65536000000000000 . 1000000000) (1000 0 0 0) (t t) (t t))"#
    ]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_access_predicates_and_this_file_cover_runtime_context() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-access-" t))
                    (path (expand-file-name "script" root)))
               (unwind-protect
                   (progn
                     (f-write "echo" 'utf-8 path)
                     (set-file-modes path #o600)
                     (let ((before
                            (list
                             (f-readable-p path)
                             (f-readable? path)
                             (f-writable-p path)
                             (f-writable? path)
                             (f-executable-p path)
                             (f-executable? path))))
                       (set-file-modes path #o700)
                       (list
                        before
                        (f-executable-p path)
                        (with-temp-buffer
                          (setq buffer-file-name "/virtual/example.el")
                          (f-this-file)))))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK ((t t t t nil nil) t "/virtual/example.el")"#]];

    assert_f_parity(elisp_form, expect);
}

#[test]
fn f_with_sandbox_allows_descendants_and_resets_guard() {
    let elisp_form = r##"(let* ((root (make-temp-file "f-guard-" t))
                    (allowed (expand-file-name "allowed" root))
                    (inside (expand-file-name "inside.txt" allowed))
                    (outside (expand-file-name "outside.txt" root)))
               (unwind-protect
                   (progn
                     (make-directory allowed)
                     (let ((blocked
                            (condition-case error
                                (f-with-sandbox allowed
                                  (f-write "inside" 'utf-8 inside)
                                  (f-write "outside" 'utf-8 outside)
                                  nil)
                              (f-guard-error
                               (list
                                (car error)
                                (file-name-nondirectory
                                 (cadr error)))))))
                       (f-write "after" 'utf-8 outside)
                       (list
                        (f-read inside)
                        blocked
                        (f-read outside)
                        f--guard-paths)))
                 (delete-directory root t)))"##;
    let expect = expect![[r#"OK ("inside" (f-guard-error "outside.txt") "after" nil)"#]];

    assert_f_parity(elisp_form, expect);
}
