use super::{assert_achievements_functions_parity, assert_achievements_parity};
use expect_test::expect;

#[test]
fn achievements_main_source_reload_preserves_catalog_and_deduplicates_kill_hook() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'achievements
                    package-alist)))
                 (source
                  (expand-file-name
                   "achievements.el"
                   (package-desc-dir
                    descriptor)))
             (before-list
              achievements-list)
             (before-count
              (cl-count
               #'achievements-save-achievements
               kill-emacs-hook)))
         (load source nil t)
         (list
          (eq before-list
              achievements-list)
          (length
           achievements-list)
          before-count
          (cl-count
           #'achievements-save-achievements
           kill-emacs-hook)
          (featurep 'achievements)
          (featurep
           'basic-achievements)))"##;
    let expect = expect!["OK (t 101 1 1 t t)"];
    assert_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_basic_catalog_load_preserves_saved_record_with_same_name() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'achievements
                    package-alist)))
                 (source
                  (expand-file-name
                   "basic-achievements.el"
                   (package-desc-dir
                    descriptor)))
                 (saved
                  (make-achievement
                   "Achiever"
                   "Saved description"
                   :points 99))
                 (achievements-list
                  (list saved)))
         (setf
          (emacs-achievement-predicate
           saved)
          nil)
         (load source nil t)
         (let ((result
                (achievements-get-achievements-by-name
                 "Achiever")))
           (list
            (length achievements-list)
            (eq result saved)
            (emacs-achievement-description
             result)
            (emacs-achievement-points
             result)
            (emacs-achievement-predicate
             result)
            (featurep
             'basic-achievements))))"##;
    let expect = expect![[r#"OK (101 t "Saved description" 99 nil t)"#]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_advanced_catalog_load_is_feature_idempotent() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'achievements
                    package-alist)))
                 (source
                  (expand-file-name
                   "advanced-achievements.el"
                   (package-desc-dir
                    descriptor)))
                 (achievements-list nil))
         (load source nil t)
         (let ((first
                (length
                 achievements-list)))
           (load source nil t)
           (list
            first
            (length
             achievements-list)
            (featurep
             'advanced-achievements)
            (mapcar
             #'emacs-achievement-name
             achievements-list))))"##;
    let expect = expect![[
        r#"OK (18 18 t ("Inception" "Narrow minded" "Forbidden Fruits" "Enabler" "Case Changer" "CASE CHANGER" "The Great Destroyer" "Goal Setter" "Wide Load" "Dired reuse" "Yes Man" "Leaving Home" "The Examined Life" "Playing it Safe" "Arbitrator" "Surfs up" "Polyglot" "Org-anizer"))"#
    ]];
    assert_achievements_functions_parity(elisp_form, expect);
}

#[test]
fn achievements_packaged_ideas_source_preserves_its_upstream_load_error() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'achievements
                    package-alist)))
                 (source
                  (expand-file-name
                   "ideas-achievements.el"
                   (package-desc-dir
                    descriptor))))
         (condition-case error
             (list
              'ok
              (load source nil t t))
           (error
            (list
             'error
             error
             (featurep
              'ideas-achievements)))))"##;
    let expect = expect!["OK (error (void-variable This) nil)"];
    assert_achievements_functions_parity(elisp_form, expect);
}
