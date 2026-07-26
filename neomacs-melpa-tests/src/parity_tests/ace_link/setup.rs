use super::assert_ace_link_parity;
use expect_test::expect;

#[test]
fn ace_link_setup_default_registers_exact_avy_styles_and_deferred_binding_forms() {
    let elisp_form = r##"(let ((avy-styles-alist
              '((sentinel . fixture)))
             registrations)
         (cl-letf (((symbol-function 'eval-after-load)
                    (lambda (file form)
                      (push
                       (list
                        file
                        form)
                       registrations))))
           (list
            (ace-link-setup-default)
            avy-styles-alist
            (nreverse registrations))))"##;
    let expect = expect![[
        r#"OK (#1=(("cider-inspector" (progn (define-key cider-inspector-mode-map "o" 'ace-link-cider-inspector)))) ((ace-link-slime-inspector . pre) (ace-link-slime-xref . pre) (ace-link-sldb . pre) (ace-link-xref . at) (ace-link-addr . pre) (ace-link-custom . pre) (ace-link-org-agenda . pre) (ace-link-org . pre) (ace-link-widget . pre) (ace-link-mu4e . post) (ace-link-gnus . post) (ace-link-compilation . post) (ace-link-w3m . post) (ace-link-eww . post) (ace-link-woman . post) (ace-link-help . post) (ace-link-info . at) (sentinel . fixture)) (("xref" (define-key xref--xref-buffer-mode-map "o" 'ace-link-xref)) ("info" (define-key Info-mode-map "o" 'ace-link-info)) ("notmuch" (progn (define-key notmuch-show-mode-map "o" 'ace-link-notmuch) (define-key notmuch-hello-mode-map "o" 'ace-link-widget))) ("compile" (define-key compilation-mode-map "o" 'ace-link-compilation)) ("help-mode" (define-key help-mode-map "o" 'ace-link-help)) ("woman" (define-key woman-mode-map "o" 'ace-link-woman)) ("eww" (progn (define-key eww-link-keymap "o" 'ace-link-eww) (define-key eww-mode-map "o" 'ace-link-eww))) (cus-edit (progn (define-key custom-mode-map "o" 'ace-link-custom))) ("helpful" (progn (define-key helpful-mode-map "o" 'ace-link-help))) ("elbank-overview" (progn (define-key elbank-overview-mode-map "o" 'ace-link-help))) ("elbank-report" (progn (define-key elbank-report-mode-map "o" 'ace-link-help))) ("indium-inspector" (progn (define-key indium-inspector-mode-map "o" 'ace-link-indium-inspector))) ("indium-debugger" (progn (define-key indium-debugger-frames-mode-map "o" 'ace-link-indium-debugger-frames))) . #1#))"#
    ]];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_setup_default_is_idempotent_for_styles_but_registers_each_requested_key() {
    let elisp_form = r##"(let ((avy-styles-alist nil)
             registrations)
         (cl-letf (((symbol-function 'eval-after-load)
                    (lambda (file form)
                      (push
                       (list
                        file
                        form)
                       registrations))))
           (ace-link-setup-default "x")
           (ace-link-setup-default "x")
           (let* ((registrations
                   (nreverse registrations))
                  (count
                   (length registrations))
                  (half
                   (/ count 2)))
             (list
              (length avy-styles-alist)
              avy-styles-alist
              count
              (equal
               (cl-subseq
                registrations
                0 half)
               (cl-subseq
                registrations
                half count))))))"##;
    let expect = expect![
        "OK (17 ((ace-link-slime-inspector . pre) (ace-link-slime-xref . pre) (ace-link-sldb . pre) (ace-link-xref . at) (ace-link-addr . pre) (ace-link-custom . pre) (ace-link-org-agenda . pre) (ace-link-org . pre) (ace-link-widget . pre) (ace-link-mu4e . post) (ace-link-gnus . post) (ace-link-compilation . post) (ace-link-w3m . post) (ace-link-eww . post) (ace-link-woman . post) (ace-link-help . post) (ace-link-info . at)) 28 t)"
    ];
    assert_ace_link_parity(elisp_form, expect);
}

#[test]
fn ace_link_setup_default_deferred_forms_bind_every_target_map_to_exact_commands() {
    let elisp_form = r##"(let ((avy-styles-alist nil)
             forms)
         (cl-letf (((symbol-function 'eval-after-load)
                    (lambda (_file form)
                      (push
                       form
                       forms))))
           (ace-link-setup-default "z"))
         (let ((maps
                '(xref--xref-buffer-mode-map
                  Info-mode-map
                  notmuch-show-mode-map
                  notmuch-hello-mode-map
                  compilation-mode-map
                  help-mode-map
                  woman-mode-map
                  eww-link-keymap
                  eww-mode-map
                  custom-mode-map
                  helpful-mode-map
                  elbank-overview-mode-map
                  elbank-report-mode-map
                  indium-inspector-mode-map
                  indium-debugger-frames-mode-map
                  cider-inspector-mode-map)))
           (cl-progv
               maps
               (mapcar
                (lambda (_)
                  (make-sparse-keymap))
                maps)
             (mapc
              #'eval
              (nreverse forms))
             (mapcar
              (lambda (symbol)
                (list
                 symbol
                 (lookup-key
                  (symbol-value symbol)
                  "z")))
              maps))))"##;
    let expect = expect![
        "OK ((xref--xref-buffer-mode-map ace-link-xref) (Info-mode-map ace-link-info) (notmuch-show-mode-map ace-link-notmuch) (notmuch-hello-mode-map ace-link-widget) (compilation-mode-map ace-link-compilation) (help-mode-map ace-link-help) (woman-mode-map ace-link-woman) (eww-link-keymap ace-link-eww) (eww-mode-map ace-link-eww) (custom-mode-map ace-link-custom) (helpful-mode-map ace-link-help) (elbank-overview-mode-map ace-link-help) (elbank-report-mode-map ace-link-help) (indium-inspector-mode-map ace-link-indium-inspector) (indium-debugger-frames-mode-map ace-link-indium-debugger-frames) (cider-inspector-mode-map ace-link-cider-inspector))"
    ];
    assert_ace_link_parity(elisp_form, expect);
}
