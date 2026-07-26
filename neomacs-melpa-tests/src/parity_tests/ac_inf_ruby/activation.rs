use expect_test::expect;

use super::assert_ac_inf_ruby_parity;

#[test]
fn ac_inf_ruby_enable_prepends_once_returns_the_live_list_and_makes_sources_local() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                ac-sources
                '(fixture-source))
               (let ((first
                      (ac-inf-ruby-enable))
                     (after-first
                      ac-sources))
                 (let ((second
                        (ac-inf-ruby-enable)))
                   (list
                    first
                    after-first
                    second
                    ac-sources
                    (eq first after-first)
                    (eq
                     after-first
                     ac-sources)
                    (local-variable-p
                     'ac-sources)))))"##;
    let expect = expect![[r#"OK (#1=(ac-source-inf-ruby fixture-source) #1# #1# #1# t t t)"#]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

#[test]
fn ac_inf_ruby_enable_preserves_an_existing_source_and_its_position() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                ac-sources
                '(before-source
                  ac-source-inf-ruby
                  after-source))
               (let ((before ac-sources))
                 (list
                  (ac-inf-ruby-enable)
                  ac-sources
                  (eq before ac-sources)
                  (local-variable-p
                   'ac-sources))))"##;
    let expect = expect![[r#"OK (#1=(before-source ac-source-inf-ruby after-source) #1# t t)"#]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}

#[test]
fn ac_inf_ruby_enable_is_buffer_local_and_preserves_the_default_sources() {
    let elisp_form = r##"(let ((default-before
                    (default-value
                     'ac-sources))
                   (first
                    (get-buffer-create
                     " *ac-inf-ruby first*"))
                   (second
                    (get-buffer-create
                     " *ac-inf-ruby second*")))
               (unwind-protect
                   (progn
                     (with-current-buffer first
                       (ac-inf-ruby-enable))
                     (list
                      (with-current-buffer first
                        ac-sources)
                      (with-current-buffer second
                        ac-sources)
                      (eq
                       default-before
                       (default-value
                        'ac-sources))
                      (default-value
                       'ac-sources)))
                 (kill-buffer first)
                 (kill-buffer second)))"##;
    let expect = expect![[
        r#"OK ((ac-source-inf-ruby . #1=(ac-source-words-in-same-mode-buffers)) #1# t #1#)"#
    ]];

    assert_ac_inf_ruby_parity(elisp_form, expect);
}
