use super::assert_ace_popup_menu_parity;
use expect_test::expect;

#[test]
fn ace_popup_menu_supported_menu_forwards_exact_buffer_menu_header_and_result() {
    let elisp_form = r##"(mapcar
         (lambda (header)
           (let ((ace-popup-menu-show-pane-header
                  header))
             (setq
              ace-popup-menu--test-events
              nil)
             (cl-letf
                 (((symbol-function 'avy-menu)
                   (lambda
                       (buffer menu show-header)
                     (push
                      (list
                       'avy
                       buffer
                       menu
                       show-header)
                      ace-popup-menu--test-events)
                     (list
                      'avy-result
                      show-header)))
                  ((symbol-function
                    'ace-popup-menu--test-original)
                   (lambda (&rest arguments)
                     (push
                      (cons
                       'original
                       arguments)
                      ace-popup-menu--test-events)
                     'original-result)))
               (list
                header
                (ace-popup-menu
                 #'ace-popup-menu--test-original
                 '(10 20)
                 '("Menu"
                   ("Pane"
                    ("First" . first)
                    ("Second" . second))))
                (nreverse
                 ace-popup-menu--test-events)))))
         '(nil t))"##;
    let expect = expect![[
        r#"OK ((nil (avy-result nil) ((avy "*ace-popup-menu*" #1=("Menu" ("Pane" ("First" . first) ("Second" . second))) nil))) (t (avy-result t) ((avy "*ace-popup-menu*" #1# t))))"#
    ]];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_falls_back_for_nil_position_keymap_and_keymap_list() {
    let elisp_form = r##"(let* ((keymap
                (make-sparse-keymap))
               (other-keymap
                (make-sparse-keymap))
               (fixtures
                (list
                 (list
                  'nil-position
                  nil
                  '("Menu"
                    ("Pane"
                     ("Choice" . selected))))
                 (list
                  'keymap
                  t
                  keymap)
                 (list
                  'keymap-list
                  t
                  (list
                   keymap
                   other-keymap)))))
         (mapcar
          (lambda (fixture)
            (setq
             ace-popup-menu--test-events
             nil
             ace-popup-menu--test-tag
             (nth 0 fixture))
            (cl-letf
                (((symbol-function
                   'avy-menu)
                  (lambda (&rest _arguments)
                    (push
                     (list
                      'avy
                      ace-popup-menu--test-tag)
                     ace-popup-menu--test-events)
                    'avy-result))
                 ((symbol-function
                   'ace-popup-menu--test-original)
                  (lambda
                      (_position _menu)
                    (push
                     (list
                      'original
                      ace-popup-menu--test-tag)
                     ace-popup-menu--test-events)
                    (list
                     'original-result
                     ace-popup-menu--test-tag))))
              (list
               ace-popup-menu--test-tag
               (ace-popup-menu
                #'ace-popup-menu--test-original
                (nth 1 fixture)
                (nth 2 fixture))
               (nreverse
                ace-popup-menu--test-events))))
          fixtures))"##;
    let expect = expect![
        "OK ((nil-position (original-result nil-position) ((original nil-position))) (keymap (original-result keymap) ((original keymap))) (keymap-list (original-result keymap-list) ((original keymap-list))))"
    ];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_uses_avy_for_empty_atomic_vector_and_later_keymap_menus() {
    let elisp_form = r##"(let ((later-keymap
              (make-sparse-keymap))
             (fixtures
              (list
               (list 'empty nil)
               (list 'atom 'menu-symbol)
               (list 'vector [menu vector])
               (list
                'later-keymap
                (list
                 'plain-first
                 (make-sparse-keymap))))))
         (mapcar
          (lambda (fixture)
            (setq
             ace-popup-menu--test-events
             nil
             ace-popup-menu--test-tag
             (car fixture))
            (cl-letf
                (((symbol-function
                   'avy-menu)
                  (lambda
                      (buffer _menu header)
                    (push
                     (list
                      'avy
                      ace-popup-menu--test-tag
                      buffer
                      header)
                     ace-popup-menu--test-events)
                    (list
                     'avy-result
                     ace-popup-menu--test-tag)))
                 ((symbol-function
                   'ace-popup-menu--test-original)
                  (lambda (&rest _arguments)
                    (push
                     (list
                      'original
                      ace-popup-menu--test-tag)
                     ace-popup-menu--test-events)
                    'original-result)))
              (list
               ace-popup-menu--test-tag
               (ace-popup-menu
                #'ace-popup-menu--test-original
                t
                (cadr fixture))
               (nreverse
                ace-popup-menu--test-events))))
          fixtures))"##;
    let expect = expect![[
        r#"OK ((empty (avy-result empty) ((avy empty "*ace-popup-menu*" nil))) (atom (avy-result atom) ((avy atom "*ace-popup-menu*" nil))) (vector (avy-result vector) ((avy vector "*ace-popup-menu*" nil))) (later-keymap (avy-result later-keymap) ((avy later-keymap "*ace-popup-menu*" nil))))"#
    ]];
    assert_ace_popup_menu_parity(elisp_form, expect);
}

#[test]
fn ace_popup_menu_propagates_selected_backend_errors_without_calling_other_backend() {
    let elisp_form = r##"(mapcar
         (lambda (fixture)
           (setq
            ace-popup-menu--test-events
            nil
            ace-popup-menu--test-branch
            (car fixture))
           (cl-letf
               (((symbol-function 'avy-menu)
                 (lambda (&rest _arguments)
                   (push 'avy
                         ace-popup-menu--test-events)
                   (if (eq
                        ace-popup-menu--test-branch
                        'avy)
                       (error "avy failure")
                     'unexpected-avy)))
                ((symbol-function
                  'ace-popup-menu--test-original)
                 (lambda (&rest _arguments)
                   (push
                    'original
                    ace-popup-menu--test-events)
                   (if (eq
                        ace-popup-menu--test-branch
                        'original)
                       (error
                        "original failure")
                     'unexpected-original))))
             (list
              ace-popup-menu--test-branch
              (condition-case error
                  (list
                   'ok
                   (ace-popup-menu
                    #'ace-popup-menu--test-original
                    (nth 1 fixture)
                    (nth 2 fixture)))
                (error
                 (list 'error error)))
              (nreverse
               ace-popup-menu--test-events))))
         '((avy t ("Menu"))
           (original nil ("Menu"))))"##;
    let expect = expect![[
        r#"OK ((avy (error (error "avy failure")) (avy)) (original (error (error "original failure")) (original)))"#
    ]];
    assert_ace_popup_menu_parity(elisp_form, expect);
}
