//! Oracle parity tests for the public GNU inotify primitives.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_inotify_public_lifecycle_and_error_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((missing "/tmp/neomacs-oracle-inotify-missing-path"))
  (list
   (featurep 'inotify)
   (fboundp 'inotify-add-watch)
   (fboundp 'inotify-rm-watch)
   (fboundp 'inotify-valid-p)
   (inotify-valid-p 0)
   (condition-case err
       (inotify-rm-watch 0)
     (error (cons (car err) (cdr err))))
   (condition-case err
       (inotify-add-watch 0 nil nil)
     (error (cons (car err) (cdr err))))
   (condition-case err
       (inotify-add-watch missing t #'ignore)
     (error (cons (car err) (cdr err))))
   (condition-case err
       (inotify-add-watch "/tmp" 'neomacs-unknown-aspect #'ignore)
     (error (cons (car err) (cdr err))))
   ;; These are event-report symbols, not add-watch aspect symbols.
   (condition-case err
       (inotify-add-watch "/tmp" 'isdir #'ignore)
     (error (cons (car err) (cdr err))))
   (condition-case err
       (inotify-add-watch "/tmp" 'q-overflow #'ignore)
     (error (cons (car err) (cdr err))))
   (mapcar (lambda (descriptor)
             (condition-case err
                 (inotify-rm-watch descriptor)
               (error (cons (car err) (cdr err)))))
           '((-1 . 0) (0 . -1) (0 . "id") ("wd" . 0) (0 0)))
   (let ((w (inotify-add-watch "/tmp" '(modify) #'ignore)))
     (list (consp w)
           (integerp (car w))
           (integerp (cdr w))
           (inotify-valid-p w)
           (inotify-rm-watch w)
           (inotify-valid-p w)
           ;; GNU validates descriptor shape but treats removing an already
           ;; inactive valid-shaped descriptor as a successful no-op.
           (inotify-rm-watch w)
           (inotify-rm-watch (cons 0 0))))))
"#;

    assert_oracle_parity(form);
}
