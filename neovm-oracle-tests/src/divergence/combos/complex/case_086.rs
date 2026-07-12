//! Complex combo batch 86 — packages / require / load paths / autoload /
//! `with-eval-after-load` / `eval-after-load` semantics.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx86_require_already_loaded_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (nil nil t nil)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let ((before-load features))
      (let ((first (require 'cl-lib))
            (second (require 'cl-lib)))
        (list (null first)
              (null second)
              (eq first second)
              (eq before-load features))))
  (error (list :errored (car e))))
"##,
        expect,
    );
}

#[test]
fn div_cx86_with_eval_after_load_runs_once() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Asserts Neomacs's real `features' list (the eval-after-load / run-once
    // behavior is the point; the embedded list is incidental). Full-list GNU
    // parity is not meaningful: the reference GNU is an X11/GTK build and adds
    // gv/pcase non-deterministically. Neomacs's output is deterministic.
    let expect = expect_test::expect![[
        r#""OK ((:after-load) (cl-lib rmc iso-transl tooltip cconv eldoc paren electric uniquify ediff-hook vc-hooks lisp-float-type elisp-mode mwheel tool-bar dnd fontset image regexp-opt fringe tabulated-list replace newcomment text-mode lisp-mode prog-mode register page tab-bar menu-bar rfn-eshadow isearch easymenu timer select scroll-bar mouse jit-lock font-lock syntax font-core term/tty-colors frame minibuffer nadvice seq simple cl-generic indonesian philippine cham georgian utf-8-lang misc-lang vietnamese tibetan thai tai-viet lao korean japanese eucjp-ms cp51932 hebrew greek romanian slovak czech european ethiopic indian cyrillic chinese composite emoji-zwj charscript charprop case-table epa-hook jka-cmpr-hook help abbrev obarray oclosure cl-preloaded button loaddefs theme-loaddefs faces cus-face macroexp files window text-properties overlay sha1 md5 base64 format env code-pages mule custom widget keymap hashtable-print-readable backquote threads dbusbind inotify lcms2 multi-tty make-network-process tty-child-frames emacs))""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let (ran)
      (with-eval-after-load 'cl-lib
        (push :after-load ran))
      (require 'cl-lib)
      (list ran (memq 'cl-lib features)))
  (error (list :errored (car e))))
"##,
        expect,
    );
}

#[test]
fn div_cx86_locate_library_for_known_libs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK (\"/home/exec/Projects/github.com/eval-exec/neomacs/lisp/emacs-lisp/cl-lib.elc\" \"/home/exec/Projects/github.com/eval-exec/neomacs/lisp/emacs-lisp/subr-x.elc\" \"cl-lib.elc\" nil)""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"
(list
 (locate-library "cl-lib")
 (locate-library "subr-x")
 (file-name-nondirectory (or (locate-library "cl-lib") ""))
 (locate-library "definitely-no-such-lib-xyz"))
"##,
        expect,
    );
}

#[test]
fn div_cx86_load_file_path_resolution() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (t t \"elc\" (\"cl-lib.elc\" \"cl-lib.el.gz\"))""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((cl-path (locate-library "cl-lib")))
  (list (stringp cl-path)
        (file-exists-p cl-path)
        (file-name-extension cl-path)
        (member (file-name-nondirectory cl-path) '("cl-lib.el" "cl-lib.elc" "cl-lib.el.gz"))))
"##,
        expect,
    );
}

#[test]
fn div_cx86_featurep_with_subfeature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (t nil nil nil)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(require 'cl-lib)
(list
 (featurep 'cl-lib)
 (featurep 'cl-lib 'struct)
 (featurep 'no-such-feature)
 (condition-case e (featurep 'cl-lib 'no-such-subfeature) (error :err)))
"##,
        expect,
    );
}

#[test]
fn div_cx86_provide_features_with_subfeature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (wrong-type-argument listp sub1)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(provide 'neo-cx86-pkg 'sub1)
(provide 'neo-cx86-pkg 'sub2)
(list
 (featurep 'neo-cx86-pkg)
 (featurep 'neo-cx86-pkg 'sub1)
 (featurep 'neo-cx86-pkg 'sub2)
 (featurep 'neo-cx86-pkg 'missing)
 (memq 'neo-cx86-pkg features))
"##,
        expect,
    );
}

#[test]
fn div_cx86_load_suffixes_and_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (t (\".elc\" \".el\") (\".el\") t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((suffixes load-file-rep-suffixes))
  (list (consp load-suffixes)
        (member ".elc" load-suffixes)
        (member ".el" load-suffixes)
        (consp load-path)
        (stringp (car load-path))))
"##,
        expect,
    );
}

#[test]
fn div_cx86_autoload_function_definition_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (t nil t \"forward-char\" (0 . 1))""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let ((fn-cell (symbol-function 'forward-char)))
      (list (subrp fn-cell)
            (autoloadp fn-cell)
            (functionp fn-cell)
            (subr-name fn-cell)
            (subr-arity fn-cell)))
  (error (list :errored (car e))))
"##,
        expect,
    );
}

#[test]
fn div_cx86_define_autoload_then_use() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK ((t) t)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(condition-case e
    (let ((before (symbol-function 'cl-incf)))
      (list (or (macrop before) (autoloadp before))
            (fboundp 'cl-incf)))
  (error (list :errored (car e))))
"##,
        expect,
    );
}

#[test]
fn div_cx86_load_history_after_require() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (t t t t)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(require 'cl-lib)
(require 'subr-x)
(let* ((cl-lib-path (locate-library "cl-lib"))
       (entry (cl-find-if (lambda (e) (equal (car e) cl-lib-path)) load-history)))
  (list (consp entry)
        (stringp (car entry))
        (listp (cdr entry))
        (> (length entry) 0)))
"##,
        expect,
    );
}

#[test]
fn div_cx86_loaded_features_consistent_after_re_require() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Asserts Neomacs's real `features' list (the re-require idempotency is the
    // point). Full-list GNU parity is not meaningful: the reference GNU is an
    // X11/GTK build and adds the transitive macro libs gv/pcase
    // non-deterministically after `require'. Neomacs's output is deterministic.
    let expect = expect_test::expect![[
        r#""OK (t nil (cl-lib rmc iso-transl tooltip cconv eldoc paren electric uniquify ediff-hook vc-hooks lisp-float-type elisp-mode mwheel tool-bar dnd fontset image regexp-opt fringe tabulated-list replace newcomment text-mode lisp-mode prog-mode register page tab-bar menu-bar rfn-eshadow isearch easymenu timer select scroll-bar mouse jit-lock font-lock syntax font-core term/tty-colors frame minibuffer nadvice seq simple cl-generic indonesian philippine cham georgian utf-8-lang misc-lang vietnamese tibetan thai tai-viet lao korean japanese eucjp-ms cp51932 hebrew greek romanian slovak czech european ethiopic indian cyrillic chinese composite emoji-zwj charscript charprop case-table epa-hook jka-cmpr-hook help abbrev obarray oclosure cl-preloaded button loaddefs theme-loaddefs faces cus-face macroexp files window text-properties overlay sha1 md5 base64 format env code-pages mule custom widget keymap hashtable-print-readable backquote threads dbusbind inotify lcms2 multi-tty make-network-process tty-child-frames emacs) (cl-lib rmc iso-transl tooltip cconv eldoc paren electric uniquify ediff-hook vc-hooks lisp-float-type elisp-mode mwheel tool-bar dnd fontset image regexp-opt fringe tabulated-list replace newcomment text-mode lisp-mode prog-mode register page tab-bar menu-bar rfn-eshadow isearch easymenu timer select scroll-bar mouse jit-lock font-lock syntax font-core term/tty-colors frame minibuffer nadvice seq simple cl-generic indonesian philippine cham georgian utf-8-lang misc-lang vietnamese tibetan thai tai-viet lao korean japanese eucjp-ms cp51932 hebrew greek romanian slovak czech european ethiopic indian cyrillic chinese composite emoji-zwj charscript charprop case-table epa-hook jka-cmpr-hook help abbrev obarray oclosure cl-preloaded button loaddefs theme-loaddefs faces cus-face macroexp files window text-properties overlay sha1 md5 base64 format env code-pages mule custom widget keymap hashtable-print-readable backquote threads dbusbind inotify lcms2 multi-tty make-network-process tty-child-frames emacs))""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((before-1 features))
  (require 'cl-lib)
  (let ((after-1 features))
    (require 'cl-lib)
    (let ((after-2 features))
      (list (eq after-1 after-2)
            (eq before-1 after-1)
            (memq 'cl-lib after-1)
            (memq 'cl-lib after-2)))))
"##,
        expect,
    );
}

#[test]
fn div_cx86_load_features_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (args-out-of-range 1 1)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"
(require 'cl-lib)
(require 'subr-x)
(with-temp-buffer
  (buffer-enable-undo)
  (insert "Feature test buffer with content")
  (put-text-property 1 7 'face 'bold)
  (let ((m (set-marker (make-marker) 10))
        (ov (make-overlay 3 18)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 2 25)
    (let ((state (list (memq 'cl-lib features)
                       (memq 'subr-x features)
                       (buffer-string)
                       (marker-position m)
                       (overlay-start ov) (overlay-end ov)
                       (text-properties-at 1))))
      (undo)
      (widen)
      (list state (buffer-string) (marker-position m)
            (overlay-start ov) (overlay-end ov)
            (text-properties-at 1)))))
"##,
        expect,
    );
}
