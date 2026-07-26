use super::{assert_achievements_functions_parity, assert_achievements_parity};
use expect_test::expect;

#[test]
fn achievements_save_writes_complete_readable_structure_without_print_limits() {
    let elisp_form = r##"(let* ((achievements-file
                 (expand-file-name
                  "achievements-save-fixture.el"
                  (getenv "TMPDIR")))
                (achievements-list
                 (list
                  (make-achievement
                   "Long"
                   (make-string 20 120)
                   :points 7
                   :predicate t)
                  '(nested
                    (one two three)))))
         (unwind-protect
             (progn
               (when
                   (file-exists-p
                    achievements-file)
                 (delete-file
                  achievements-file))
               (list
                (achievements-save-achievements)
                (file-exists-p
                 achievements-file)
                (with-temp-buffer
                  (insert-file-contents
                   achievements-file)
                  (buffer-string))
                (with-temp-buffer
                  (insert-file-contents
                   achievements-file)
                  (read
                   (current-buffer)))))
           (when
               (file-exists-p
                achievements-file)
             (delete-file
              achievements-file))))"##;
    let expect = expect![[
        r#"OK (nil t "(#s(emacs-achievement \"Long\" \"xxxxxxxxxxxxxxxxxxxx\"\n\11\11      (lambda nil (and t)) nil nil 7 0 nil)\n   (nested (one two three)))\n" (#s(emacs-achievement "Long" "xxxxxxxxxxxxxxxxxxxx" (lambda nil (and t)) nil nil 7 0 nil) (nested (one two three))))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_load_handles_missing_empty_invalid_non_list_and_valid_files() {
    let elisp_form = r##"(let ((achievements-file
              (expand-file-name
               "achievements-load-fixture.el"
               (getenv "TMPDIR")))
             (achievements-debug t))
         (unwind-protect
             (mapcar
              (lambda (contents)
                (setq
                 achievements-list
                 '(stale)
                 achievements--test-messages
                 nil)
                (when
                    (file-exists-p
                     achievements-file)
                  (delete-file
                   achievements-file))
                (when
                    (stringp contents)
                  (with-temp-file
                      achievements-file
                    (insert contents)))
                (cl-letf
                    (((symbol-function 'message)
                      (lambda
                          (format-string
                           &rest arguments)
                        (push
                         (apply
                          #'format
                          format-string
                          arguments)
                         achievements--test-messages))))
                  (list
                   contents
                   (achievements-load-achievements)
                   achievements-list
                   (length
                    achievements--test-messages)
                   (and
                    achievements--test-messages
                    (and
                     (string-match-p
                      "does not contain valid data"
                      (car
                       achievements--test-messages))
                     t)))))
              (list
               'missing
               ""
               "("
               "42"
               "((alpha 1) beta)"))
           (when
               (file-exists-p
                achievements-file)
             (delete-file
              achievements-file))))"##;
    let expect = expect![[
        r#"OK ((missing nil nil 0 nil) ("" nil nil 1 t) ("(" nil nil 1 t) ("42" nil nil 1 t) ("((alpha 1) beta)" #1=((alpha 1) beta) #1# 0 nil))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_init_loads_once_installs_hooks_lighter_and_basic_catalog() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            achievements--test-events
            nil)
           (let ((achievements-list
                  (and
                   (nth 0 fixture)
                   '(already-loaded)))
                 (minor-mode-alist
                  '((achievements-mode
                     " Achieve"))))
             (cl-letf
                 (((symbol-function
                    'achievements-load-achievements)
                   (lambda ()
                     (push '(load)
                           achievements--test-events)
                     (setq
                      achievements-list
                      '(loaded))))
                  ((symbol-function 'add-hook)
                   (lambda
                       (hook function
                        &optional append local)
                     (push
                      (list
                       'add-hook
                       hook
                       function
                       append
                       local)
                      achievements--test-events)))
                  ((symbol-function
                    'internal-char-font)
                   (lambda
                       (_position character)
                     (push
                      (list
                       'font
                       character)
                      achievements--test-events)
                     (nth 1 fixture)))
                  ((symbol-function 'require)
                   (lambda
                       (feature
                        &optional _filename
                        _noerror)
                     (push
                      (list
                       'require
                       feature)
                      achievements--test-events)
                     feature)))
               (list
                fixture
                (achievements-init)
                achievements-list
                minor-mode-alist
                (nreverse
                 achievements--test-events)))))
         '((nil nil)
           (nil t)
           (t t)))"##;
    let expect = expect![[
        r#"OK (((nil nil) basic-achievements #1=(loaded) #2=((achievements-mode " 🏆")) (#3=(load) (add-hook kill-emacs-hook achievements-save-achievements nil nil) (font 127942) (require basic-achievements))) ((nil t) basic-achievements #1# #2# (#3# (add-hook kill-emacs-hook achievements-save-achievements nil nil) (font 127942) (require basic-achievements))) ((t t) basic-achievements (already-loaded) #2# ((add-hook kill-emacs-hook achievements-save-achievements nil nil) (font 127942) (require basic-achievements))))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_main_load_initializes_catalog_and_kill_hook_once() {
    let elisp_form = r##"(list
         (length achievements-list)
         (mapcar
          #'emacs-achievement-name
          (seq-take
           achievements-list
           5))
         (memq
          #'achievements-save-achievements
          kill-emacs-hook)
         (cl-count
          #'achievements-save-achievements
          kill-emacs-hook)
         achievements-score
         achievements-total)"##;
    let expect = expect![[
        r#"OK (101 ("Achiever" "Not All There" "Unlocker" "Over Achiever" "Cheater") (achievements-save-achievements) 1 0 0)"#
    ]];
    assert_achievements_parity(elisp_form, expect);
}
