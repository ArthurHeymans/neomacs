use expect_test::expect;

use super::assert_ameba_parity;

/// The package's headline use, run through the key the minor mode binds: open a
/// Crystal file, press `C-c C-r f', and read the linter's findings in a
/// compilation buffer.
///
/// Four things are asserted, in the order the user meets them.  The argv the
/// linter received, which is the only part of the run that is really the
/// package's doing.  The name of the buffer it opened, `*Ameba FILE*'.  That
/// buffer's whole text, with the wall clock taken out - the command line as
/// invoked, the two diagnostics ameba reported, and the abnormal exit status
/// that tells the user the linter found something.  And then the part that
/// makes a compilation buffer worth having: following the first diagnostic
/// lands in `greeter.cr' on line 14, column 4, on the `if true' that
/// `Lint/LiteralInCondition' is about, and following the second lands on the
/// useless assignment on line 6.
///
/// Line 14 is followed first because that is the order ameba prints, not the
/// order the file is written in.
#[test]
fn linting_the_current_file_gives_a_compilation_buffer_you_can_navigate() {
    let elisp_form = r##"(let* ((root (ameba-test-project "check-file" ".projectile"))
       (file (expand-file-name "src/greeter.cr" root)))
  (ameba-test-install-linter root (ameba-test-file-diagnostics file) 1)
  (let ((buffer (find-file-noselect file)))
    (set-window-buffer (selected-window) buffer)
    (with-current-buffer buffer
      (ameba-mode 1)
      (execute-kbd-macro (kbd "C-c C-r f")))
    (let ((compilation (get-buffer (ameba-buffer-name file))))
      (ameba-test-wait compilation)
      (list (mapcar (lambda (argument) (ameba-test-relative argument root))
                    (ameba-test-arguments))
            (ameba-test-relative (buffer-name compilation) root)
            (with-current-buffer compilation major-mode)
            (ameba-test-compilation-text compilation root)
            (ameba-test-jump compilation)
            (ameba-test-jump compilation)))))"##;
    let expect = expect![[
        r#"OK (("--format" "flycheck" "PROJECT/src/greeter.cr") "*Ameba PROJECT/src/greeter.cr*" compilation-mode "-*- mode: compilation; default-directory: \"PROJECT/\" -*-\nCompilation started at [TIME]\n\nameba --format flycheck PROJECT/src/greeter.cr\nPROJECT/src/greeter.cr:14:5: W: [Lint/LiteralInCondition] Literal value found in conditional\nPROJECT/src/greeter.cr:6:5: W: [Lint/UselessAssign] Useless assignment to variable `unused`\n\nCompilation exited abnormally with code 1 at [TIME], duration [DURATION]\n" ("greeter.cr" 14 4 "    if true") ("greeter.cr" 6 4 "    unused = \"not used anywhere\""))"#
    ]];

    assert_ameba_parity(elisp_form, expect);
}

/// `ameba-check-project' is the command that does more than pass a path along:
/// it builds its argument out of the project root and `ameba-project-lib', so
/// the linter is asked to check the whole shard *except* its vendored `lib'
/// directory, which is the point of the command.
///
/// The argv is the assertion that matters here, because it is the whole of what
/// the package contributes - two positional arguments, the root and `!ROOT/lib'.
/// The output replayed against it is what real ameba 1.6.4 answered that exact
/// argv with when the shard was linted for real: three diagnostics from `src',
/// and nothing from `lib/vendored.cr', which is faulty in the same way as the
/// others and is left out because the exclusion took effect.  The buffer name
/// carries the whole argument, spaces and all.
#[test]
fn checking_the_project_asks_the_linter_to_skip_the_vendored_lib_directory() {
    let elisp_form = r##"(let* ((root (ameba-test-project "check-project" ".projectile"))
       (file (expand-file-name "src/greeter.cr" root))
       (argument (concat root " !" root "lib")))
  (ameba-test-install-linter root (ameba-test-project-diagnostics root) 1)
  (let ((buffer (find-file-noselect file)))
    (with-current-buffer buffer
      (ameba-mode 1)
      (ameba-check-project))
    (let ((compilation (get-buffer (ameba-buffer-name argument))))
      (ameba-test-wait compilation)
      (list (mapcar (lambda (recorded) (ameba-test-relative recorded root))
                    (ameba-test-arguments))
            (ameba-test-relative (ameba-buffer-name argument) root)
            (ameba-test-relative (buffer-name compilation) root)
            (ameba-test-compilation-text compilation root)
            (ameba-test-jump compilation)))))"##;
    let expect = expect![[
        r#"OK (("--format" "flycheck" "PROJECT/" "!PROJECT/lib") "*Ameba PROJECT/ !PROJECT/lib*" "*Ameba PROJECT/ !PROJECT/lib*" "-*- mode: compilation; default-directory: \"PROJECT/\" -*-\nCompilation started at [TIME]\n\nameba --format flycheck PROJECT/ !PROJECT/lib\nPROJECT/src/greeter.cr:14:5: W: [Lint/LiteralInCondition] Literal value found in conditional\nPROJECT/src/greeter.cr:6:5: W: [Lint/UselessAssign] Useless assignment to variable `unused`\nPROJECT/src/util.cr:3:5: W: [Lint/UselessAssign] Useless assignment to variable `x`\n\nCompilation exited abnormally with code 1 at [TIME], duration [DURATION]\n" ("greeter.cr" 14 4 "    if true"))"#
    ]];

    assert_ameba_parity(elisp_form, expect);
}

/// Which directory the linter runs from, which decides what the paths in its
/// output mean.  `ameba-project-root' looks for the first entry of
/// `ameba-project-root-files' that has *any* dominating directory, and takes
/// that one - so the list is searched in order and the nearest marker does not
/// necessarily win.
///
/// The fixture is a shard with `shard.yml' at its root, inside the Neomacs
/// checkout, which is itself a git repository.  With the default list, `.git'
/// is checked before `shard.yml' and matches far above, so the root comes out as
/// the checkout and ameba would be run from there rather than from the shard.
/// Adding `.projectile' to the shard moves it back, because `.projectile' is
/// first in the list.  Setting the documented option to `("shard.yml")' does the
/// same thing without adding a file.
///
/// The three answers are reported relative to the sandbox, so a root that is the
/// shard shows as the empty string and one that is not shows as the path above
/// it.  `ameba-project-lib' is included because that is what
/// `ameba-check-project' exclusion is built from, and it inherits whichever root
/// was chosen.
#[test]
fn the_project_root_is_the_first_marker_in_the_list_not_the_nearest_one() {
    let elisp_form = r##"(let* ((root (ameba-test-project "roots"))
       (file (expand-file-name "src/greeter.cr" root))
       (buffer (find-file-noselect file)))
  (with-current-buffer buffer
    (let* ((sandbox (file-name-as-directory
                     (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
           (describe (lambda (path)
                       (cond ((null path) nil)
                             ((equal path root) :the-shard)
                             ((string-prefix-p path sandbox) :above-the-sandbox)
                             (t (ameba-test-relative path root))))))
      (list (list :markers ameba-project-root-files
                  :default (funcall describe (ameba-project-root))
                  :lib (funcall describe (ameba-project-lib)))
            (let ((ameba-project-root-files '("shard.yml")))
              (list :only-shard (funcall describe (ameba-project-root))))
            (progn
              (write-region "" nil (expand-file-name ".projectile" root) nil 'silent)
              (list :with-projectile (funcall describe (ameba-project-root))
                    :lib (funcall describe (ameba-project-lib))))))))"##;
    let expect = expect![[
        r#"OK ((:markers (".projectile" ".git" ".hg" ".ameba.yml" "shard.yml") :default :above-the-sandbox :lib "![ORACLE-WORKSPACE]/lib") (:only-shard :the-shard) (:with-projectile :the-shard :lib "!PROJECT/lib"))"#
    ]];

    assert_ameba_parity(elisp_form, expect);
}

/// The three ways the package refuses, all of which a user meets in practice:
/// the linter is not installed, the buffer is not visiting a file, and there is
/// no project anywhere above.
///
/// Each is checked through the public command rather than the guard behind it,
/// and each is asserted to leave nothing behind - no compilation buffer is
/// created in any of the three cases, so the refusal happens before
/// `compilation-start' rather than as a failed run.  `ameba-project-root' is
/// also called with its documented `no-error' argument, which turns the third
/// case into nil instead of a signal; that is the form the two command paths use
/// internally, which is why a file outside any project is still linted, from
/// whatever directory it is in.
#[test]
fn the_commands_refuse_without_a_linter_a_file_or_a_project() {
    let elisp_form = r##"(let* ((root (ameba-test-project "refusals" ".projectile"))
       (file (expand-file-name "src/greeter.cr" root))
       (buffer (find-file-noselect file))
       (compilation-buffers
        (lambda ()
          (sort (delq nil (mapcar (lambda (candidate)
                                    (and (string-prefix-p "*Ameba " (buffer-name candidate))
                                         (buffer-name candidate)))
                                  (buffer-list)))
                #'string<))))
  (list
   (let ((exec-path '("/nonexistent")))
     (with-current-buffer buffer
       (list (condition-case error (ameba-check-current-file)
               (error (list (car error) (cadr error))))
             (funcall compilation-buffers))))
   (progn
     (ameba-test-install-linter root "" 0)
     (with-temp-buffer
       (list (condition-case error (ameba-check-current-file)
               (error (list (car error) (cadr error))))
             (funcall compilation-buffers))))
   (with-temp-buffer
     (setq default-directory "/")
     (list (condition-case error (ameba-project-root)
             (error (list (car error) (cadr error))))
           (ameba-project-root 'no-error)
           (funcall compilation-buffers)))
   (list :keymap-prefix (key-description ameba-keymap-prefix)
         :binding (key-description
                   (where-is-internal 'ameba-check-current-file ameba-mode-map t))
         :command ameba-check-command)))"##;
    let expect = expect![[
        r#"OK (((error "Ameba is not installed") nil) ((error "Buffer is not visiting a file") nil) ((error "You’re not into a project") nil nil) (:keymap-prefix "C-c C-r" :binding "C-c C-r f" :command "ameba --format flycheck"))"#
    ]];

    assert_ameba_parity(elisp_form, expect);
}
