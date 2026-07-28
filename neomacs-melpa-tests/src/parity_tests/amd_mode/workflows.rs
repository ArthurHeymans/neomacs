use expect_test::expect;

use super::assert_amd_mode_parity;

/// Starting a module from nothing, which is the first thing the package's
/// commentary tells a user to do: `amd-auto-insert' writes an empty `define'
/// with point inside the function body, then `amd-import-module' adds
/// dependencies by name.
///
/// The buffer is asserted after every step, so the two halves amd-mode has to
/// keep in step are both visible: the string goes into the dependency array and
/// the same name goes into the function's parameter list, in the same position.
/// Importing a name that is already required is asserted to change nothing at
/// all - `amd--import' returns early rather than duplicating it - and the
/// imports are separated by an idle period because js2-mode reparses from an
/// idle timer and each command reads the AST.
///
/// The last import is the one where the two halves deliberately differ.  Every
/// import prompts "Import as (DEFAULT):", and the first three answer it with
/// RET and take the default, which is the module path's base name; the fourth
/// answers `shortcut', so `lib/keyboard/bindings' enters the array under its
/// full path and the parameter list under the name the user chose.
#[test]
fn starting_an_empty_module_and_importing_dependencies_by_name() {
    let elisp_form = r##"(let* ((root (amd-test-project "amd-start"))
       (buffer (amd-test-open root "src/app/main.js" "")))
  (amd-test-in buffer
    (amd-auto-insert)
    (let ((template (list (amd-test-text) (point))))
      (amd-test-idle)
      (amd-test-answering "" nil (amd-import-module "lib/router"))
      (let ((first (amd-test-text)))
        (amd-test-idle)
        (amd-test-answering "" nil (amd-import-module "widgets/button"))
        (let ((second (amd-test-text)))
          (amd-test-idle)
          (amd-test-answering "" nil (amd-import-module "lib/router"))
          (let ((duplicate (amd-test-text)))
            (amd-test-idle)
            (amd-test-answering "shortcut" nil (amd-import-module "lib/keyboard/bindings"))
            (list template first second duplicate
                  (equal second duplicate)
                  (amd-test-text))))))))"##;
    let expect = expect![[
        r#"OK (("define([], function() {\n    \n});\n" 29) "define(['lib/router'], function(router) {\n    \n});\n" "define(['lib/router',\n\11'widgets/button'], function(router, button) {\n    \n});\n" "define(['lib/router',\n\11'widgets/button'], function(router, button) {\n    \n});\n" t "define(['lib/router',\n\11'widgets/button',\n\11'lib/keyboard/bindings'], function(router, button, shortcut) {\n    \n});\n")"#
    ]];

    assert_amd_mode_parity(elisp_form, expect);
}

/// Importing a file rather than a name, where amd-mode has to turn a path into
/// a module path.  The project has a file in the buffer's own subtree
/// (`src/app/util/format.js`) and one outside it (`src/vendor/moment.js`), and
/// both are imported into a fresh copy of the same buffer under the default
/// settings and under each of the two documented ones.
///
/// With `amd-use-relative-file-name' on, the commentary promises a relative path
/// only when the file is in the same directory or below it, and that is exactly
/// what the two halves of the report show: `./util/format' for the file inside
/// the subtree and the project path `src/vendor/moment' for the one outside.
/// With `amd-always-use-relative-file-name' on, the file outside becomes
/// `../vendor/moment' as well.  The default - neither set - is the project path
/// in both cases.
#[test]
fn importing_a_file_uses_a_relative_path_only_where_the_settings_say_so() {
    let elisp_form = r##"(let* ((root (amd-test-project "amd-paths"))
       (_ (amd-test-write root "src/app/util/format.js" "define([], function() {});\n"))
       (_ (amd-test-write root "src/vendor/moment.js" "define([], function() {});\n")))
  (cl-flet ((import-both
              (label)
              (let ((buffer (amd-test-open root "src/app/main.js"
                                           "define([], function() {\n\n});\n")))
                (amd-test-in buffer
                  (amd-test-answering "" "src/app/util/format.js" (amd-import-file))
                  (amd-test-idle)
                  (amd-test-answering "" "src/vendor/moment.js" (amd-import-file))
                  (list label (amd-test-text))))))
    (list (import-both :default)
          (let ((amd-use-relative-file-name t))
            (import-both :relative-when-below))
          (let ((amd-always-use-relative-file-name t))
            (import-both :always-relative)))))"##;
    let expect = expect![[
        r#"OK ((:default "define(['src/app/util/format',\n\11'src/vendor/moment'], function(format, moment) {\n\n});\n") (:relative-when-below "define(['./util/format',\n\11'src/vendor/moment'], function(format, moment) {\n\n});\n") (:always-relative "define(['./util/format',\n\11'../vendor/moment'], function(format, moment) {\n\n});\n"))"#
    ]];

    assert_amd_mode_parity(elisp_form, expect);
}

/// Removing and reordering dependencies, through the keys the mode binds rather
/// than by calling the commands: `C-k' on a module line, then `C-S-down' and
/// `C-S-up' on another.  `where-is-internal' pins that those are the bindings
/// `amd-mode' installs, and the keys are sent with `execute-kbd-macro' into the
/// selected window's buffer.
///
/// Each of the three edits has to touch two places at once, and the buffer text
/// after each shows whether it did: killing `'lib/router'` has to drop `router`
/// from the parameter list, and moving a module up or down has to carry its
/// parameter with it.  The body of the function is left alone throughout, which
/// is why it still refers to `router` after the kill - amd-mode edits the module
/// header, not the code.
#[test]
fn killing_and_reordering_a_dependency_keeps_the_parameter_list_in_step() {
    let elisp_form = r##"(let* ((root (amd-test-project "amd-edit"))
       (buffer (amd-test-open root "src/app/main.js" amd-test-two-module-source)))
  (amd-test-in buffer
    (let ((bindings (list (key-description
                           (where-is-internal 'amd-kill-line amd-mode-map t))
                          (key-description
                           (where-is-internal 'amd-move-line-up amd-mode-map t))
                          (key-description
                           (where-is-internal 'amd-move-line-down amd-mode-map t))))
          (original (amd-test-text)))
      (goto-char (point-min))
      (search-forward "'lib/router'")
      (beginning-of-line)
      (execute-kbd-macro (kbd "C-k"))
      (let ((killed (amd-test-text)))
        (amd-test-idle)
        (amd-test-answering "" nil (amd-import-module "lib/router"))
        (amd-test-idle)
        (let ((reimported (amd-test-text)))
          (goto-char (point-min))
          (search-forward "'widgets/button'")
          (execute-kbd-macro (kbd "<C-S-down>"))
          (amd-test-idle)
          (let ((moved-down (amd-test-text)))
            (goto-char (point-min))
            (search-forward "'widgets/button'")
            (execute-kbd-macro (kbd "<C-S-up>"))
            (list bindings original killed reimported moved-down
                  (amd-test-text))))))))"##;
    let expect = expect![[
        r#"OK (("C-k" "C-S-<up>" "C-S-<down>") "define([\n    'lib/router',\n    'widgets/button'\n], function(router, button) {\n    return router;\n});\n" "define([\n    'widgets/button'\n], function(button) {\n    return router;\n});\n" "define([\n    'widgets/button',\n    'lib/router'\n], function(button, router) {\n    return router;\n});\n" "define([\n    'lib/router',\n    'widgets/button'\n], function(router, button) {\n    return router;\n});\n" "define([\n    'widgets/button',\n    'lib/router'\n], function(button, router) {\n    return router;\n});\n")"#
    ]];

    assert_amd_mode_parity(elisp_form, expect);
}

/// Copying the current file's module path, which is what a user does to paste it
/// into another module's dependency array.  `amd-kill-buffer-module' puts it on
/// the kill ring already quoted, ready to yank between the brackets.
///
/// The path is the file's location relative to the project root with the
/// extension dropped, and `amd-rewrite-rules-alist' rewrites it afterwards - the
/// documented use is a directory-local variable stripping a leading source
/// directory, which is what the second and third parts do, including two rules
/// applied in order.  The buffer's own module path ignores both relative-name
/// settings: `amd--use-relative-file-name-p' answers nil when the file is the
/// buffer's own, so a file cannot be required relative to itself.
///
/// The last part is the same command with no project around it, which is the
/// package's one documented failure mode - every command begins with
/// `amd--guard'.  A project is not something the workflow can take away by
/// deleting a marker file, because the sandbox lives inside the Neomacs
/// checkout and projectile finds that instead; a buffer whose
/// `default-directory' is the filesystem root has no project above it and needs
/// nothing faked.  All three commands refuse with the same error, the buffer is
/// left untouched and nothing reaches the kill ring.
#[test]
fn copying_the_buffers_module_path_applies_the_projects_rewrite_rules() {
    let elisp_form = r##"(let* ((root (amd-test-project "amd-copy"))
       (buffer (amd-test-open root "src/widgets/forms/button.js"
                              "define([], function() {\n\n});\n")))
  (amd-test-in buffer
    (cl-flet ((copied
                ()
                (let ((kill-ring nil))
                  (amd-kill-buffer-module)
                  (copy-sequence (car kill-ring)))))
      (list (copied)
            (let ((amd-rewrite-rules-alist '(("^src/" . "")))) (copied))
            (let ((amd-rewrite-rules-alist '(("^src/" . "") ("widgets/" . "ui/"))))
              (copied))
            (let ((amd-use-relative-file-name t)) (copied))
            (let ((amd-always-use-relative-file-name t)) (copied))
            (amd-test-text)
            (with-temp-buffer
              (setq default-directory "/")
              (js2-mode)
              (amd-mode 1)
              (let ((kill-ring nil))
                (list (projectile-project-p)
                      (mapcar (lambda (command)
                                (condition-case error (funcall command)
                                  (error (list (car error) (cadr error)))))
                              '(amd-kill-buffer-module amd-auto-insert
                                amd-search-references))
                      (amd-test-text)
                      kill-ring)))))))"##;
    let expect = expect![[
        r#"OK ("'src/widgets/forms/button'" "'widgets/forms/button'" "'ui/forms/button'" "'src/widgets/forms/button'" "'src/widgets/forms/button'" "define([], function() {\n\n});\n" (nil ((error "Not within a project") (error "Not within a project") (error "Not within a project")) "" nil))"#
    ]];

    assert_amd_mode_parity(elisp_form, expect);
}

/// Finding which modules require this one.  `amd-search-references' builds a
/// regexp from the file's base name, runs `ag' with the configured arguments and
/// ignore lists, parses the `file:line:match' lines and shows the survivors as
/// xrefs.
///
/// The stand-in records argv, so the whole command line is pinned: the two
/// default arguments, one `--ignore-dir' pair per entry of
/// `amd-ag-ignored-dirs', one `--ignore' per entry of `amd-ag-ignored-files',
/// and the search regexp last.  A customized ignore list is asserted to change
/// the argv accordingly.
///
/// The output the stand-in prints has three hits, and only two of them are
/// references: `var buttonlike = 1;' contains the module's name but not as a
/// quoted module, so `amd--xref-false-positive' drops it.  What reaches the user
/// is the `*xref*' buffer, so that is what the workflow asserts, grouped by file
/// with the line numbers `ag' reported.  The empty-output case takes the other
/// branch and only messages.
#[test]
fn searching_for_references_runs_ag_with_the_configured_ignores() {
    let elisp_form = r##"(let* ((root (amd-test-project "amd-refs"))
       (log (amd-test-configure-ag
             root
             (concat "src/app/main.js:3:    'widgets/button',\n"
                     "src/app/other.js:2:define(['widgets/button'], function(button) {\n"
                     "src/vendor/bundle.js:1:var buttonlike = 1;\n")))
       (buffer (amd-test-open root "src/widgets/button.js"
                              "define([], function() {\n\n});\n")))
  (amd-test-in buffer
    (amd-search-references)
    (let ((found (list (amd-test-ag-arguments log)
                       (amd-test-xref-text "amd-refs"))))
      (kill-buffer "*xref*")
      (setenv "AMD_TEST_AG_OUTPUT" "")
      (let* ((message-start (with-current-buffer "*Messages*" (point-max)))
             (result (let ((amd-ag-ignored-dirs '("dist"))
                           (amd-ag-ignored-files '("*.bundle.js" "*.min.js")))
                       (amd-search-references))))
        (list found
              result
              (amd-test-ag-arguments log)
              (with-current-buffer "*Messages*"
                (buffer-substring-no-properties message-start (point-max)))
              (and (get-buffer "*xref*") t))))))"##;
    let expect = expect![[
        r#"OK ((("--js" "--noheading" "--ignore-dir" "bower_components" "--ignore-dir" "node_modules" "--ignore-dir" "build" "--ignore-dir" "lib" "--ignore" "*.min.js" "define\\([^])]+['|\"](.*/)?button['|\"]") "src/app/other.js\n2:define(['widgets/button'], function(button) {\nsrc/app/main.js\n3:'widgets/button',\n") "No reference found" ("--js" "--noheading" "--ignore-dir" "dist" "--ignore" "*.bundle.js" "--ignore" "*.min.js" "define\\([^])]+['|\"](.*/)?button['|\"]") "No reference found\n" nil)"#
    ]];

    assert_amd_mode_parity(elisp_form, expect);
}

/// The case the package tried to handle and gets wrong.  `amd--xref-candidate'
/// means to shorten a match longer than a hundred characters - its comment says
/// minified JavaScript would otherwise clutter the results - but the call is
/// `(seq-take 100 match)', with the sequence and the count the wrong way round.
/// `seq-take' then computes `(min 100 match)' on the string and signals.
///
/// So a single long line anywhere in the search output aborts the whole command,
/// and the user gets no results at all rather than a truncated one - from
/// exactly the minified file the guard was written for.  The workflow pins the
/// signal, that no `*xref*' buffer is produced, and that `ag' had already run
/// with the right arguments, which is what places the failure after the search
/// rather than in it.  A short match through the same code path is asserted
/// alongside so the length is visibly what makes the difference.
#[test]
fn a_reference_longer_than_a_hundred_characters_aborts_the_search() {
    let elisp_form = r##"(let* ((root (amd-test-project "amd-long"))
       (long-line (concat "var b=" (make-string 120 ?x) "'button';"))
       (log (amd-test-configure-ag
             root
             (concat "src/app/main.js:3:define(['widgets/button'], function(button) {\n"
                     "src/vendor/all.min.js:1:" long-line "\n")))
       (buffer (amd-test-open root "src/widgets/button.js"
                              "define([], function() {\n\n});\n")))
  (amd-test-in buffer
    (let ((failure (condition-case error (amd-search-references)
                     (error (list :signal (car error)
                                  :on-a-line-of-length (length long-line))))))
      (setenv "AMD_TEST_AG_OUTPUT"
              "src/app/main.js:3:define(['widgets/button'], function(button) {\n")
      (let ((short (amd-search-references)))
        (list failure
              (amd-test-ag-arguments log)
              (and (bufferp short) t)
              (amd-test-xref-text "amd-long"))))))"##;
    let expect = expect![[
        r#"OK ((:signal wrong-type-argument :on-a-line-of-length 135) ("--js" "--noheading" "--ignore-dir" "bower_components" "--ignore-dir" "node_modules" "--ignore-dir" "build" "--ignore-dir" "lib" "--ignore" "*.min.js" "define\\([^])]+['|\"](.*/)?button['|\"]") t "src/app/main.js\n3:define(['widgets/button'], function(button) {\n")"#
    ]];

    assert_amd_mode_parity(elisp_form, expect);
}
