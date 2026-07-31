use expect_test::expect;

use super::assert_affe_autoload_batch;

#[test]
fn autoloads_public_surface_batch() {
    assert_affe_autoload_batch(&[
        (
            "affe_generated_autoloads_register_both_commands_without_loading_package",
            r##"(list
               (featurep 'affe)
               (mapcar
                (lambda (command)
                  (let ((definition
                         (symbol-function command)))
                    (list
                     command
                     (autoloadp definition)
                     (and (autoloadp definition)
                          (file-name-nondirectory
                           (nth 1 definition)))
                     (and (autoloadp definition)
                          (nth 2 definition))
                     (and (autoloadp definition)
                          (nth 4 definition)))))
                '(affe-grep affe-find))
               (get 'affe-count 'custom-autoload)
               (get 'affe-find-command
                    'custom-autoload)
               (get 'affe-grep-command
                    'custom-autoload)
               (get 'affe-regexp-compiler
                    'custom-autoload))"##,
            true,
            expect![[
        r#"OK (nil ((affe-grep t "affe" "Fuzzy grep in DIR with optional INITIAL input.\n\n(fn &optional DIR INITIAL)" nil) (affe-find t "affe" "Fuzzy find in DIR with optional INITIAL input.\n\n(fn &optional DIR INITIAL)" nil)) nil nil nil nil)"#
    ]],
        ),
    ]);
}
