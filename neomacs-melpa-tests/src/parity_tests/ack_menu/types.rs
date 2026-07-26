use super::assert_ack_menu_parity;
use expect_test::expect;

#[test]
fn ack_menu_create_type_preserves_extension_order_and_empty_inputs() {
    let elisp_form = r##"(list
         (ack-create-type nil)
         (ack-create-type
          '("rs"))
         (ack-create-type
          '("c" "h" "inl"))
         (ack-create-type
          '("" "x")))"##;
    let expect = expect![[
        r#"OK (("--type-set" "ack-menu-custom-type=" "--type" "ack-menu-custom-type") ("--type-set" "ack-menu-custom-type=rs" "--type" "ack-menu-custom-type") ("--type-set" "ack-menu-custom-type=c,h,inl" "--type" "ack-menu-custom-type") ("--type-set" "ack-menu-custom-type=,x" "--type" "ack-menu-custom-type"))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_type_for_major_mode_covers_overrides_defaults_extensions_and_missing_modes() {
    let elisp_form = r##"(let ((ack-mode-type-alist
                '((python-mode
                   "custom-a"
                   "custom-b")
                  (fixture-extension-only)))
               (ack-mode-extension-alist
                '((python-mode
                   "pyi"
                   "pyx")
                  (fixture-extension-only
                   "one"
                   "two")
                  (c-mode
                   "header"))))
         (list
          (ack-type-for-major-mode
           'python-mode)
          (ack-type-for-major-mode
           'fixture-extension-only)
          (ack-type-for-major-mode
           'c-mode)
          (ack-type-for-major-mode
           'd-mode)
          (ack-type-for-major-mode
           'fixture-missing)))"##;
    let expect = expect![[
        r#"OK (("--type-add" "custom-a=pyi,pyx" "--type" "custom-b" "--type" "custom-a") ("--type-set" "ack-menu-custom-type=one,two" "--type" "ack-menu-custom-type") ("--type-add" "cc=header" "--type" "cc") ("--type-set" "ack-menu-custom-type=d" "--type" "ack-menu-custom-type") nil)"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_type_uses_major_mode_then_file_extension_fallback() {
    let elisp_form = r##"(let ((ack-mode-type-alist
                '((fixture-mode
                   "fixture")))
               (ack-mode-extension-alist
                nil))
         (list
          (with-temp-buffer
            (setq major-mode
                  'fixture-mode
                  buffer-file-name
                  "/workspace/name.ignored")
            (ack-type))
          (with-temp-buffer
            (setq major-mode
                  'fixture-unknown-mode
                  buffer-file-name
                  "/workspace/name.multi.rs")
            (ack-type))
          (with-temp-buffer
            (setq major-mode
                  'fixture-unknown-mode
                  buffer-file-name
                  "/workspace/Makefile")
            (ack-type))
          (with-temp-buffer
            (setq major-mode
                  'fixture-unknown-mode
                  buffer-file-name
                  nil)
            (ack-type))))"##;
    let expect = expect![[
        r#"OK (("--type" "fixture") ("--type-set" "ack-menu-custom-type=rs" "--type" "ack-menu-custom-type") ("--type-set" "ack-menu-custom-type=" "--type" "ack-menu-custom-type") nil)"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_default_mode_type_and_extension_tables_match_exactly() {
    let elisp_form = r##"(list
         (copy-tree
          ack-mode-default-type-alist)
         (copy-tree
          ack-mode-default-extension-alist))"##;
    let expect = expect![[
        r#"OK (((actionscript-mode "actionscript") (LaTeX-mode "tex") (TeX-mode "tex") (asm-mode "asm") (batch-file-mode "batch") (c++-mode "cpp") (c-mode "cc") (cfmx-mode "cfmx") (cperl-mode "perl") (csharp-mode "csharp") (css-mode "css") (emacs-lisp-mode "elisp") (erlang-mode "erlang") (espresso-mode "js") (f90-mode "fortran") (fortran-mode "fortran") (haskell-mode "haskell") (hexl-mode "binary") (html-mode "html") (java-mode "java") (javascript-mode "js") (jde-mode "java") (js2-mode "js") (jsp-mode "jsp") (latex-mode "tex") (lisp-mode "lisp") (lua-mode "lua") (makefile-mode "make") (mason-mode "mason") (nxml-mode "xml") (objc-mode "objc" "objcpp") (ocaml-mode "ocaml") (parrot-mode "parrot") (perl-mode "perl") (php-mode "php") (plone-mode "plone") (python-mode "python") (ruby-mode "ruby") (scheme-mode "scheme") (shell-script-mode "shell") (smalltalk-mode "smalltalk") (sql-mode "sql") (tcl-mode "tcl") (tex-mode "tex") (text-mode "text") (tt-mode "tt") (vb-mode "vb") (vim-mode "vim") (xml-mode "xml") (yaml-mode "yaml")) ((d-mode "d")))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}
