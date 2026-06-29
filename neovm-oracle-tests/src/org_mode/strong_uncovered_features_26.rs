//! Strong uncovered-features-26 oracle tests — org-export and publishing.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-html
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_html() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (void-function org-export-as-html)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-html nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-latex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_latex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (void-function org-export-as-latex)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-latex nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-ascii
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (void-function org-export-as-ascii)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-ascii nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-utf8
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (void-function org-export-as-utf8)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-utf8 nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-html-to-buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_html_buf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (void-function org-export-as-html-to-buffer)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-html-to-buffer nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-latex-to-buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_latex_buf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (void-function org-export-as-latex-to-buffer)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (org-export-as-latex-to-buffer nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-region-as-html
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (void-function org-export-region-as-html)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (goto-char (point-min))
  (search-forward "Body")
  (beginning-of-line)
  (org-export-region-as-html (point) (point-max) nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-pdf
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_pdf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (condition-case nil
      (org-export-as-pdf nil)
    (error nil)))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-as-pdf-and-open
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_pdf_open() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (condition-case nil
      (org-export-as-pdf-and-open nil)
    (error nil)))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-dispatch
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_dispatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r##""OK (:error (user-error \"Export aborted\") \"#+TITLE: Test\n* H1\nBody *bold* /italic/\")""##
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (condition-case err
      (let ((unread-command-events (list ?q)))
        (org-export-dispatch nil)
        (list :ok (buffer-string)))
    (error (list :error err (buffer-string)))))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-html-export-as-html
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_html_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r##""OK \"<?xml version=\\\"1.0\\\" encoding=\\\"utf-8\\\"?>\n<!DOCTYPE html PUBLIC \\\"-//W3C//DTD XHTML 1.0 Strict//EN\\\"\n\\\"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\\\">\n<html xmlns=\\\"http://www.w3.org/1999/xhtml\\\" lang=\\\"en\\\" xml:lang=\\\"en\\\">\n<head>\n<meta http-equiv=\\\"Content-Type\\\" content=\\\"text/html;charset=utf-8\\\" />\n<meta name=\\\"viewport\\\" content=\\\"width=device-width, initial-scale=1\\\" />\n<title>Test</title>\n<meta name=\\\"generator\\\" content=\\\"Org Mode\\\" />\n<style type=\\\"text/css\\\">\n  #content { max-width: 60em; margin: auto; }\n  .title  { text-align: center;\n             margin-bottom: .2em; }\n  .subtitle { text-align: center;\n              font-size: medium;\n              font-weight: bold;\n              margin-top:0; }\n  .todo   { font-family: monospace; color: red; }\n  .done   { font-family: monospace; color: green; }\n  .priority { font-family: monospace; color: orange; }\n  .tag    { background-color: #eee; font-family: monospace;\n            padding: 2px; font-size: 80%; font-weight: normal; }\n  .timestamp { color: #bebebe; }\n  .timestamp-kwd { color: #5f9ea0; }\n  .org-right  { margin-left: auto; margin-right: 0px;  text-align: right; }\n  .org-left   { margin-left: 0px;  margin-right: auto; text-align: left; }\n  .org-center { margin-left: auto; margin-right: auto; text-align: center; }\n  .underline { text-decoration: underline; }\n  #postamble p, #preamble p { font-size: 90%; margin: .2em; }\n  p.verse { margin-left: 3%; }\n  pre {\n    border: 1px solid #e6e6e6;\n    border-radius: 3px;\n    background-color: #f2f2f2;\n    padding: 8pt;\n    font-family: monospace;\n    overflow: auto;\n    margin: 1.2em;\n  }\n  pre.src {\n    position: relative;\n    overflow: auto;\n  }\n  pre.src:before {\n    display: none;\n    position: absolute;\n    top: -8px;\n    right: 12px;\n    padding: 3px;\n    color: #555;\n    background-color: #f2f2f299;\n  }\n  pre.src:hover:before { display: inline; margin-top: 14px;}\n  /* Languages per Org manual */\n  pre.src-asymptote:before { content: 'Asymptote'; }\n  pre.src-awk:before { content: 'Awk'; }\n  pre.src-authinfo::before { content: 'Authinfo'; }\n  pre.src-c:before { content: 'C'; }\n  pre.src-C:before { content: 'C'; }\n  /* pre.src-C++ doesn't work in CSS */\n  pre.src-clojure:before { content: 'Clojure'; }\n  pre.src-css:before { content: 'CSS'; }\n  pre.src-D:before { content: 'D'; }\n  pre.src-ditaa:before { content: 'ditaa'; }\n  pre.src-dot:before { content: 'Graphviz'; }\n  pre.src-calc:before { content: 'Emacs Calc'; }\n  pre.src-emacs-lisp:before { content: 'Emacs Lisp'; }\n  pre.src-fortran:before { content: 'Fortran'; }\n  pre.src-gnuplot:before { content: 'gnuplot'; }\n  pre.src-haskell:before { content: 'Haskell'; }\n  pre.src-hledger:before { content: 'hledger'; }\n  pre.src-java:before { content: 'Java'; }\n  pre.src-js:before { content: 'JavaScript'; }\n  pre.src-latex:before { content: 'LaTeX'; }\n  pre.src-ledger:before { content: 'Ledger'; }\n  pre.src-lisp:before { content: 'Lisp'; }\n  pre.src-lilypond:before { content: 'Lilypond'; }\n  pre.src-lua:before { content: 'Lua'; }\n  pre.src-matlab:before { content: 'MATLAB'; }\n  pre.src-mscgen:before { content: 'Mscgen'; }\n  pre.src-ocaml:before { content: 'Objective Caml'; }\n  pre.src-octave:before { content: 'Octave'; }\n  pre.src-org:before { content: 'Org mode'; }\n  pre.src-oz:before { content: 'OZ'; }\n  pre.src-plantuml:before { content: 'Plantuml'; }\n  pre.src-processing:before { content: 'Processing.js'; }\n  pre.src-python:before { content: 'Python'; }\n  pre.src-R:before { content: 'R'; }\n  pre.src-ruby:before { content: 'Ruby'; }\n  pre.src-sass:before { content: 'Sass'; }\n  pre.src-scheme:before { content: 'Scheme'; }\n  pre.src-screen:before { content: 'Gnu Screen'; }\n  pre.src-sed:before { content: 'Sed'; }\n  pre.src-sh:before { content: 'shell'; }\n  pre.src-sql:before { content: 'SQL'; }\n  pre.src-sqlite:before { content: 'SQLite'; }\n  /* additional languages in org.el's org-babel-load-languages alist */\n  pre.src-forth:before { content: 'Forth'; }\n  pre.src-io:before { content: 'IO'; }\n  pre.src-J:before { content: 'J'; }\n  pre.src-makefile:before { content: 'Makefile'; }\n  pre.src-maxima:before { content: 'Maxima'; }\n  pre.src-perl:before { content: 'Perl'; }\n  pre.src-picolisp:before { content: 'Pico Lisp'; }\n  pre.src-scala:before { content: 'Scala'; }\n  pre.src-shell:before { content: 'Shell Script'; }\n  pre.src-ebnf2ps:before { content: 'ebfn2ps'; }\n  /* additional language identifiers per \\\"defun org-babel-execute\\\"\n       in ob-*.el */\n  pre.src-cpp:before  { content: 'C++'; }\n  pre.src-abc:before  { content: 'ABC'; }\n  pre.src-coq:before  { content: 'Coq'; }\n  pre.src-groovy:before  { content: 'Groovy'; }\n  /* additional language identifiers from org-babel-shell-names in\n     ob-shell.el: ob-shell is the only babel language using a lambda to put\n     the execution function name together. */\n  pre.src-bash:before  { content: 'bash'; }\n  pre.src-csh:before  { content: 'csh'; }\n  pre.src-ash:before  { content: 'ash'; }\n  pre.src-dash:before  { content: 'dash'; }\n  pre.src-ksh:before  { content: 'ksh'; }\n  pre.src-mksh:before  { content: 'mksh'; }\n  pre.src-posh:before  { content: 'posh'; }\n  /* Additional Emacs modes also supported by the LaTeX listings package */\n  pre.src-ada:before { content: 'Ada'; }\n  pre.src-asm:before { content: 'Assembler'; }\n  pre.src-caml:before { content: 'Caml'; }\n  pre.src-delphi:before { content: 'Delphi'; }\n  pre.src-html:before { content: 'HTML'; }\n  pre.src-idl:before { content: 'IDL'; }\n  pre.src-mercury:before { content: 'Mercury'; }\n  pre.src-metapost:before { content: 'MetaPost'; }\n  pre.src-modula-2:before { content: 'Modula-2'; }\n  pre.src-pascal:before { content: 'Pascal'; }\n  pre.src-ps:before { content: 'PostScript'; }\n  pre.src-prolog:before { content: 'Prolog'; }\n  pre.src-simula:before { content: 'Simula'; }\n  pre.src-tcl:before { content: 'tcl'; }\n  pre.src-tex:before { content: 'TeX'; }\n  pre.src-plain-tex:before { content: 'Plain TeX'; }\n  pre.src-verilog:before { content: 'Verilog'; }\n  pre.src-vhdl:before { content: 'VHDL'; }\n  pre.src-xml:before { content: 'XML'; }\n  pre.src-nxml:before { content: 'XML'; }\n  /* add a generic configuration mode; LaTeX export needs an additional\n     (add-to-list 'org-latex-listings-langs '(conf \\\" \\\")) in .emacs */\n  pre.src-conf:before { content: 'Configuration File'; }\n\n  table { border-collapse:collapse; }\n  caption.t-above { caption-side: top; }\n  caption.t-bottom { caption-side: bottom; }\n  td, th { vertical-align:top;  }\n  th.org-right  { text-align: center;  }\n  th.org-left   { text-align: center;   }\n  th.org-center { text-align: center; }\n  td.org-right  { text-align: right;  }\n  td.org-left   { text-align: left;   }\n  td.org-center { text-align: center; }\n  dt { font-weight: bold; }\n  .footpara { display: inline; }\n  .footdef  { margin-bottom: 1em; }\n  .figure { padding: 1em; }\n  .figure p { text-align: center; }\n  .equation-container {\n    display: table;\n    text-align: center;\n    width: 100%;\n  }\n  .equation {\n    vertical-align: middle;\n  }\n  .equation-label {\n    display: table-cell;\n    text-align: right;\n    vertical-align: middle;\n  }\n  .inlinetask {\n    padding: 10px;\n    border: 2px solid gray;\n    margin: 10px;\n    background: #ffffcc;\n  }\n  #org-div-home-and-up\n   { text-align: right; font-size: 70%; white-space: nowrap; }\n  textarea { overflow-x: auto; }\n  .linenr { font-size: smaller }\n  .code-highlighted { background-color: #ffff00; }\n  .org-info-js_info-navigation { border-style: none; }\n  #org-info-js_console-label\n    { font-size: 10px; font-weight: bold; white-space: nowrap; }\n  .org-info-js_search-highlight\n    { background-color: #ffff00; color: #000000; font-weight: bold; }\n  .org-svg { }\n</style>\n</head>\n<body>\n<div id=\\\"content\\\" class=\\\"content\\\">\n<h1 class=\\\"title\\\">Test</h1>\n<div id=\\\"table-of-contents\\\" role=\\\"doc-toc\\\">\n<h2>Table of Contents</h2>\n<div id=\\\"text-table-of-contents\\\" role=\\\"doc-toc\\\">\n<ul>\n<li><a href=\\\"#orgXXXXXXX\\\">1. H1</a></li>\n</ul>\n</div>\n</div>\n<div id=\\\"outline-container-orgXXXXXXX\\\" class=\\\"outline-2\\\">\n<h2 id=\\\"orgXXXXXXX\\\"><span class=\\\"section-number-2\\\">1.</span> H1</h2>\n<div class=\\\"outline-text-2\\\" id=\\\"text-1\\\">\n<p>\nBody <b>bold</b> <i>italic</i></p>\n</div>\n</div>\n</div>\n<div id=\\\"postamble\\\" class=\\\"status\\\">\n<p class=\\\"validation\\\"><a href=\\\"https://validator.w3.org/check?uri=referer\\\">Validate</a></p>\n</div>\n</body>\n</html>\"""##
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (let ((org-export-time-stamp-file nil))
    (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
    (condition-case nil
        (org-html-export-as-html)
      (error nil))
    (replace-regexp-in-string
     "org[0-9a-f]\\{7\\}" "orgXXXXXXX"
     (buffer-string))))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-latex-export-as-latex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_latex_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK \"% Intended LaTeX compiler: pdflatex\n\\\\documentclass[11pt]{article}\n\n\\\\usepackage[utf8]{inputenc}\n\\\\usepackage[T1]{fontenc}\n\\\\usepackage{graphicx}\n\\\\usepackage{longtable}\n\\\\usepackage{wrapfig}\n\\\\usepackage{rotating}\n\\\\usepackage[normalem]{ulem}\n\\\\usepackage{amsmath}\n\\\\usepackage{amssymb}\n\\\\usepackage{capt-of}\n\\\\usepackage{hyperref}\n\\\\date{\\\\today}\n\\\\title{Test}\n\\\\hypersetup{\n pdfauthor={},\n pdftitle={Test},\n pdfkeywords={},\n pdfsubject={},\n pdfcreator={},\n pdflang={English}}\n\\\\begin{document}\n\n\\\\maketitle\n\\\\tableofcontents\n\n\\\\section{H1}\n\\\\label{sec:orgXXXXXXX}\nBody \\\\textbf{bold} \\\\emph{italic}\n\\\\end{document}\"""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (let ((org-export-time-stamp-file nil))
    (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
    (condition-case nil
        (org-latex-export-as-latex)
      (error nil))
    (replace-regexp-in-string
     "org[0-9a-f]\\{7\\}" "orgXXXXXXX"
     (buffer-string))))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-ascii-export-as-ascii
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_ascii_export() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK \"                                 ______\n\n                                  TEST\n                                 ______\n\n\nTable of Contents\n_________________\n\n1. H1\n\n\n1 H1\n====\n\n  Body *bold* /italic/\n\"""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody *bold* /italic/")
  (condition-case nil
      (org-ascii-export-as-ascii)
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-publish
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_publish() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK t""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case nil
    (org-publish "test" nil)
  (error nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-publish-all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_publish_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case nil
    (org-publish-all nil)
  (error nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-publish-current-file
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_publish_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case nil
    (org-publish-current-file nil)
  (error nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-publish-current-project
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_publish_project() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case nil
    (org-publish-current-project nil)
  (error nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-publish-sitemap
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_publish_sitemap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK nil""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case nil
    (org-publish-sitemap "test")
  (error nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-define-backend
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK (#s(org-export-backend test nil ((template lambda (contents info) contents)) nil nil nil nil) #s(org-export-backend odt nil ((bold . org-odt-bold) (center-block . org-odt-center-block) (clock . org-odt-clock) (code . org-odt-code) (drawer . org-odt-drawer) (dynamic-block . org-odt-dynamic-block) (entity . org-odt-entity) (example-block . org-odt-example-block) (export-block . org-odt-export-block) (export-snippet . org-odt-export-snippet) (fixed-width . org-odt-fixed-width) (footnote-definition . org-odt-footnote-definition) (footnote-reference . org-odt-footnote-reference) (headline . org-odt-headline) (horizontal-rule . org-odt-horizontal-rule) (inline-src-block . org-odt-inline-src-block) (inlinetask . org-odt-inlinetask) (italic . org-odt-italic) (item . org-odt-item) (keyword . org-odt-keyword) (latex-environment . org-odt-latex-environment) (latex-fragment . org-odt-latex-fragment) (line-break . org-odt-line-break) (link . org-odt-link) (node-property . org-odt-node-property) (paragraph . org-odt-paragraph) (plain-list . org-odt-plain-list) (plain-text . org-odt-plain-text) (planning . org-odt-planning) (property-drawer . org-odt-property-drawer) (quote-block . org-odt-quote-block) (radio-target . org-odt-radio-target) (section . org-odt-section) (special-block . org-odt-special-block) (src-block . org-odt-src-block) (statistics-cookie . org-odt-statistics-cookie) (strike-through . org-odt-strike-through) (subscript . org-odt-subscript) (superscript . org-odt-superscript) (table . org-odt-table) (table-cell . org-odt-table-cell) (table-row . org-odt-table-row) (target . org-odt-target) (template . org-odt-template) (timestamp . org-odt-timestamp) (underline . org-odt-underline) (verbatim . org-odt-verbatim) (verse-block . org-odt-verse-block)) ((:odt-styles-file \"ODT_STYLES_FILE\" nil org-odt-styles-file t) (:description \"DESCRIPTION\" nil nil newline) (:keywords \"KEYWORDS\" nil nil space) (:subtitle \"SUBTITLE\" nil nil parse) (:odt-with-forbidden-chars nil nil org-odt-with-forbidden-chars) (:odt-content-template-file nil nil org-odt-content-template-file) (:odt-display-outline-level nil nil org-odt-display-outline-level) (:odt-fontify-srcblocks nil nil org-odt-fontify-srcblocks) (:odt-format-drawer-function nil nil org-odt-format-drawer-function) (:odt-format-headline-function nil nil org-odt-format-headline-function) (:odt-format-inlinetask-function nil nil org-odt-format-inlinetask-function) (:odt-inline-formula-rules nil nil org-odt-inline-formula-rules) (:odt-inline-image-rules nil nil org-odt-inline-image-rules) (:odt-pixels-per-inch nil nil org-odt-pixels-per-inch) (:odt-table-styles nil nil org-odt-table-styles) (:odt-use-date-fields nil nil org-odt-use-date-fields) (:with-latex nil \"tex\" org-odt-with-latex) (:latex-header \"LATEX_HEADER\" nil nil newline)) ((:filter-parse-tree org-odt--strip-trailing-newlines org-odt--translate-latex-fragments org-odt--translate-description-lists org-odt--translate-list-tables org-odt--translate-image-links) (:filter-final-output . org-odt--remove-forbidden)) nil (111 \"Export to ODT\" ((111 \"As ODT file\" org-odt-export-to-odt) (79 \"As ODT file and open\" (lambda (a s v b) (if a (org-odt-export-to-odt t s v) (org-open-file (org-odt-export-to-odt nil s v) 'system))))))) #s(org-export-backend latex nil ((bold . org-latex-bold) (center-block . org-latex-center-block) (clock . org-latex-clock) (code . org-latex-code) (drawer . org-latex-drawer) (dynamic-block . org-latex-dynamic-block) (entity . org-latex-entity) (example-block . org-latex-example-block) (export-block . org-latex-export-block) (export-snippet . org-latex-export-snippet) (fixed-width . org-latex-fixed-width) (footnote-definition . org-latex-footnote-definition) (footnote-reference . org-latex-footnote-reference) (headline . org-latex-headline) (horizontal-rule . org-latex-horizontal-rule) (inline-src-block . org-latex-inline-src-block) (inlinetask . org-latex-inlinetask) (italic . org-latex-italic) (item . org-latex-item) (keyword . org-latex-keyword) (latex-environment . org-latex-latex-environment) (latex-fragment . org-latex-latex-fragment) (line-break . org-latex-line-break) (link . org-latex-link) (node-property . org-latex-node-property) (paragraph . org-latex-paragraph) (plain-list . org-latex-plain-list) (plain-text . org-latex-plain-text) (planning . org-latex-planning) (property-drawer . org-latex-property-drawer) (quote-block . org-latex-quote-block) (radio-target . org-latex-radio-target) (section . org-latex-section) (special-block . org-latex-special-block) (src-block . org-latex-src-block) (statistics-cookie . org-latex-statistics-cookie) (strike-through . org-latex-strike-through) (subscript . org-latex-subscript) (superscript . org-latex-superscript) (table . org-latex-table) (table-cell . org-latex-table-cell) (table-row . org-latex-table-row) (target . org-latex-target) (template . org-latex-template) (timestamp . org-latex-timestamp) (underline . org-latex-underline) (verbatim . org-latex-verbatim) (verse-block . org-latex-verse-block) (latex-math-block . org-latex-math-block) (latex-matrices . org-latex-matrices)) ((:latex-class \"LATEX_CLASS\" nil org-latex-default-class t) (:latex-class-options \"LATEX_CLASS_OPTIONS\" nil nil t) (:latex-header \"LATEX_HEADER\" nil nil newline) (:latex-header-extra \"LATEX_HEADER_EXTRA\" nil nil newline) (:latex-class-pre \"LATEX_CLASS_PRE\" nil nil newline) (:description \"DESCRIPTION\" nil nil parse) (:keywords \"KEYWORDS\" nil nil parse) (:subtitle \"SUBTITLE\" nil nil parse) (:latex-active-timestamp-format nil nil org-latex-active-timestamp-format) (:latex-caption-above nil nil org-latex-caption-above) (:latex-classes nil nil org-latex-classes) (:latex-default-figure-position nil nil org-latex-default-figure-position) (:latex-default-table-environment nil nil org-latex-default-table-environment) (:latex-default-quote-environment nil nil org-latex-default-quote-environment) (:latex-default-table-mode nil nil org-latex-default-table-mode) (:latex-default-footnote-command \"LATEX_FOOTNOTE_COMMAND\" nil org-latex-default-footnote-command) (:latex-diary-timestamp-format nil nil org-latex-diary-timestamp-format) (:latex-engraved-options nil nil org-latex-engraved-options) (:latex-engraved-preamble nil nil org-latex-engraved-preamble) (:latex-engraved-theme \"LATEX_ENGRAVED_THEME\" nil org-latex-engraved-theme) (:latex-footnote-defined-format nil nil org-latex-footnote-defined-format) (:latex-footnote-separator nil nil org-latex-footnote-separator) (:latex-format-drawer-function nil nil org-latex-format-drawer-function) (:latex-format-headline-function nil nil org-latex-format-headline-function) (:latex-format-inlinetask-function nil nil org-latex-format-inlinetask-function) (:latex-hyperref-template nil nil org-latex-hyperref-template t) (:latex-image-default-scale nil nil org-latex-image-default-scale) (:latex-image-default-height nil nil org-latex-image-default-height) (:latex-image-default-option nil nil org-latex-image-default-option) (:latex-image-default-width nil nil org-latex-image-default-width) (:latex-images-centered nil nil org-latex-images-centered) (:latex-inactive-timestamp-format nil nil org-latex-inactive-timestamp-format) (:latex-inline-image-rules nil nil org-latex-inline-image-rules) (:latex-link-with-unknown-path-format nil nil org-latex-link-with-unknown-path-format) (:latex-src-block-backend nil nil org-latex-src-block-backend) (:latex-listings-langs nil nil org-latex-listings-langs) (:latex-listings-options nil nil org-latex-listings-options) (:latex-listings-src-omit-language nil nil org-latex-listings-src-omit-language) (:latex-minted-langs nil nil org-latex-minted-langs) (:latex-minted-options nil nil org-latex-minted-options) (:latex-prefer-user-labels nil nil org-latex-prefer-user-labels) (:latex-subtitle-format nil nil org-latex-subtitle-format) (:latex-subtitle-separate nil nil org-latex-subtitle-separate) (:latex-table-scientific-notation nil nil org-latex-table-scientific-notation) (:latex-tables-booktabs nil nil org-latex-tables-booktabs) (:latex-tables-centered nil nil org-latex-tables-centered) (:latex-text-markup-alist nil nil org-latex-text-markup-alist) (:latex-title-command nil nil org-latex-title-command) (:latex-toc-command nil nil org-latex-toc-command) (:latex-compiler \"LATEX_COMPILER\" nil org-latex-compiler) (:latex-use-sans nil \"latex-use-sans\" org-latex-use-sans) (:date \"DATE\" nil \"\\\\today\" parse)) ((:filter-options . org-latex-math-block-options-filter) (:filter-paragraph . org-latex-clean-invalid-line-breaks) (:filter-parse-tree org-latex-math-block-tree-filter org-latex-matrices-tree-filter org-latex-image-link-filter) (:filter-verse-block . org-latex-clean-invalid-line-breaks)) nil (108 \"Export to LaTeX\" ((76 \"As LaTeX buffer\" org-latex-export-as-latex) (108 \"As LaTeX file\" org-latex-export-to-latex) (112 \"As PDF file\" org-latex-export-to-pdf) (111 \"As PDF file and open\" (lambda (a s v b) (if a (org-latex-export-to-pdf t s v b) (org-open-file (org-latex-export-to-pdf nil s v b)))))))) #s(org-export-backend icalendar ascii ((clock) (footnote-definition) (footnote-reference) (headline . org-icalendar-entry) (inner-template . org-icalendar-inner-template) (inlinetask) (planning) (section) (template . org-icalendar-template)) ((:exclude-tags \"ICALENDAR_EXCLUDE_TAGS\" nil org-icalendar-exclude-tags split) (:with-timestamps nil \"<\" org-icalendar-with-timestamps) (:icalendar-alarm-time nil nil org-icalendar-alarm-time) (:icalendar-categories nil nil org-icalendar-categories) (:icalendar-date-time-format nil nil org-icalendar-date-time-format) (:icalendar-include-bbdb-anniversaries nil nil org-icalendar-include-bbdb-anniversaries) (:icalendar-include-body nil nil org-icalendar-include-body) (:icalendar-include-sexps nil nil org-icalendar-include-sexps) (:icalendar-include-todo nil nil org-icalendar-include-todo) (:icalendar-store-UID nil nil org-icalendar-store-UID) (:icalendar-timezone nil nil org-icalendar-timezone) (:icalendar-use-deadline nil nil org-icalendar-use-deadline) (:icalendar-use-scheduled nil nil org-icalendar-use-scheduled) (:icalendar-scheduled-summary-prefix nil nil org-icalendar-scheduled-summary-prefix) (:icalendar-deadline-summary-prefix nil nil org-icalendar-deadline-summary-prefix) (:icalendar-ttl \"ICAL-TTL\" nil org-icalendar-ttl)) ((:filter-headline . org-icalendar-clear-blank-lines)) nil (99 \"Export to iCalendar\" ((102 \"Current file\" org-icalendar-export-to-ics) (97 \"All agenda files\" (lambda (a s v b) (org-icalendar-export-agenda-files a))) (99 \"Combine all agenda files\" (lambda (a s v b) (org-icalendar-combine-agenda-files a)))))) #s(org-export-backend html nil ((bold . org-html-bold) (center-block . org-html-center-block) (clock . org-html-clock) (code . org-html-code) (drawer . org-html-drawer) (dynamic-block . org-html-dynamic-block) (entity . org-html-entity) (example-block . org-html-example-block) (export-block . org-html-export-block) (export-snippet . org-html-export-snippet) (fixed-width . org-html-fixed-width) (footnote-reference . org-html-footnote-reference) (headline . org-html-headline) (horizontal-rule . org-html-horizontal-rule) (inline-src-block . org-html-inline-src-block) (inlinetask . org-html-inlinetask) (inner-template . org-html-inner-template) (italic . org-html-italic) (item . org-html-item) (keyword . org-html-keyword) (latex-environment . org-html-latex-environment) (latex-fragment . org-html-latex-fragment) (line-break . org-html-line-break) (link . org-html-link) (node-property . org-html-node-property) (paragraph . org-html-paragraph) (plain-list . org-html-plain-list) (plain-text . org-html-plain-text) (planning . org-html-planning) (property-drawer . org-html-property-drawer) (quote-block . org-html-quote-block) (radio-target . org-html-radio-target) (section . org-html-section) (special-block . org-html-special-block) (src-block . org-html-src-block) (statistics-cookie . org-html-statistics-cookie) (strike-through . org-html-strike-through) (subscript . org-html-subscript) (superscript . org-html-superscript) (table . org-html-table) (table-cell . org-html-table-cell) (table-row . org-html-table-row) (target . org-html-target) (template . org-html-template) (timestamp . org-html-timestamp) (underline . org-html-underline) (verbatim . org-html-verbatim) (verse-block . org-html-verse-block)) ((:html-doctype \"HTML_DOCTYPE\" nil org-html-doctype) (:html-container \"HTML_CONTAINER\" nil org-html-container-element) (:html-content-class \"HTML_CONTENT_CLASS\" nil org-html-content-class) (:description \"DESCRIPTION\" nil nil newline) (:keywords \"KEYWORDS\" nil nil space) (:html-html5-fancy nil \"html5-fancy\" org-html-html5-fancy) (:html-link-use-abs-url nil \"html-link-use-abs-url\" org-html-link-use-abs-url) (:html-link-home \"HTML_LINK_HOME\" nil org-html-link-home) (:html-link-up \"HTML_LINK_UP\" nil org-html-link-up) (:html-mathjax \"HTML_MATHJAX\" nil \"\" space) (:html-equation-reference-format \"HTML_EQUATION_REFERENCE_FORMAT\" nil org-html-equation-reference-format t) (:html-postamble nil \"html-postamble\" org-html-postamble) (:html-preamble nil \"html-preamble\" org-html-preamble) (:html-head \"HTML_HEAD\" nil org-html-head newline) (:html-head-extra \"HTML_HEAD_EXTRA\" nil org-html-head-extra newline) (:subtitle \"SUBTITLE\" nil nil parse) (:html-head-include-default-style nil \"html-style\" org-html-head-include-default-style) (:html-head-include-scripts nil \"html-scripts\" org-html-head-include-scripts) (:html-allow-name-attribute-in-anchors nil nil org-html-allow-name-attribute-in-anchors) (:html-divs nil nil org-html-divs) (:html-checkbox-type nil nil org-html-checkbox-type) (:html-extension nil nil org-html-extension) (:html-footnote-format nil nil org-html-footnote-format) (:html-footnote-separator nil nil org-html-footnote-separator) (:html-footnotes-section nil nil org-html-footnotes-section) (:html-format-drawer-function nil nil org-html-format-drawer-function) (:html-format-headline-function nil nil org-html-format-headline-function) (:html-format-inlinetask-function nil nil org-html-format-inlinetask-function) (:html-home/up-format nil nil org-html-home/up-format) (:html-indent nil nil org-html-indent) (:html-infojs-options nil nil org-html-infojs-options) (:html-infojs-template nil nil org-html-infojs-template) (:html-inline-image-rules nil nil org-html-inline-image-rules) (:html-link-org-files-as-html nil nil org-html-link-org-files-as-html) (:html-mathjax-options nil nil org-html-mathjax-options) (:html-mathjax-template nil nil org-html-mathjax-template) (:html-metadata-timestamp-format nil nil org-html-metadata-timestamp-format) (:html-postamble-format nil nil org-html-postamble-format) (:html-preamble-format nil nil org-html-preamble-format) (:html-prefer-user-labels nil nil org-html-prefer-user-labels) (:html-self-link-headlines nil \"html-self-link-headlines\" org-html-self-link-headlines) (:html-table-align-individual-fields nil nil org-html-table-align-individual-fields) (:html-table-caption-above nil nil org-html-table-caption-above) (:html-table-data-tags nil nil org-html-table-data-tags) (:html-table-header-tags nil nil org-html-table-header-tags) (:html-table-use-header-tags-for-first-column nil nil org-html-table-use-header-tags-for-first-column) (:html-tag-class-prefix nil nil org-html-tag-class-prefix) (:html-text-markup-alist nil nil org-html-text-markup-alist) (:html-todo-kwd-class-prefix nil nil org-html-todo-kwd-class-prefix) (:html-toplevel-hlevel nil nil org-html-toplevel-hlevel) (:html-use-infojs nil nil org-html-use-infojs) (:html-validation-link nil nil org-html-validation-link) (:html-viewport nil nil org-html-viewport) (:html-inline-images nil nil org-html-inline-images) (:html-table-attributes nil nil org-html-table-default-attributes) (:html-table-row-open-tag nil nil org-html-table-row-open-tag) (:html-table-row-close-tag nil nil org-html-table-row-close-tag) (:html-xml-declaration nil nil org-html-xml-declaration) (:html-wrap-src-lines nil nil org-html-wrap-src-lines) (:html-klipsify-src nil nil org-html-klipsify-src) (:html-klipse-css nil nil org-html-klipse-css) (:html-klipse-js nil nil org-html-klipse-js) (:html-klipse-selection-script nil nil org-html-klipse-selection-script) (:infojs-opt \"INFOJS_OPT\" nil nil) (:creator \"CREATOR\" nil org-html-creator-string) (:with-latex nil \"tex\" org-html-with-latex) (:latex-header \"LATEX_HEADER\" nil nil newline)) ((:filter-options . org-html-infojs-install-script) (:filter-parse-tree . org-html-image-link-filter) (:filter-final-output . org-html-final-function)) nil (104 \"Export to HTML\" ((72 \"As HTML buffer\" org-html-export-as-html) (104 \"As HTML file\" org-html-export-to-html) (111 \"As HTML file and open\" (lambda (a s v b) (if a (org-html-export-to-html t s v b) (org-open-file (org-html-export-to-html nil s v b)))))))) #s(org-export-backend ascii nil ((bold . org-ascii-bold) (center-block . org-ascii-center-block) (clock . org-ascii-clock) (code . org-ascii-code) (drawer . org-ascii-drawer) (dynamic-block . org-ascii-dynamic-block) (entity . org-ascii-entity) (example-block . org-ascii-example-block) (export-block . org-ascii-export-block) (export-snippet . org-ascii-export-snippet) (fixed-width . org-ascii-fixed-width) (footnote-reference . org-ascii-footnote-reference) (headline . org-ascii-headline) (horizontal-rule . org-ascii-horizontal-rule) (inline-src-block . org-ascii-inline-src-block) (inlinetask . org-ascii-inlinetask) (inner-template . org-ascii-inner-template) (italic . org-ascii-italic) (item . org-ascii-item) (keyword . org-ascii-keyword) (latex-environment . org-ascii-latex-environment) (latex-fragment . org-ascii-latex-fragment) (line-break . org-ascii-line-break) (link . org-ascii-link) (node-property . org-ascii-node-property) (paragraph . org-ascii-paragraph) (plain-list . org-ascii-plain-list) (plain-text . org-ascii-plain-text) (planning . org-ascii-planning) (property-drawer . org-ascii-property-drawer) (quote-block . org-ascii-quote-block) (radio-target . org-ascii-radio-target) (section . org-ascii-section) (special-block . org-ascii-special-block) (src-block . org-ascii-src-block) (statistics-cookie . org-ascii-statistics-cookie) (strike-through . org-ascii-strike-through) (subscript . org-ascii-subscript) (superscript . org-ascii-superscript) (table . org-ascii-table) (table-cell . org-ascii-table-cell) (table-row . org-ascii-table-row) (target . org-ascii-target) (template . org-ascii-template) (timestamp . org-ascii-timestamp) (underline . org-ascii-underline) (verbatim . org-ascii-verbatim) (verse-block . org-ascii-verse-block)) ((:subtitle \"SUBTITLE\" nil nil parse) (:ascii-bullets nil nil org-ascii-bullets) (:ascii-caption-above nil nil org-ascii-caption-above) (:ascii-charset nil nil org-ascii-charset) (:ascii-global-margin nil nil org-ascii-global-margin) (:ascii-format-drawer-function nil nil org-ascii-format-drawer-function) (:ascii-format-inlinetask-function nil nil org-ascii-format-inlinetask-function) (:ascii-headline-spacing nil nil org-ascii-headline-spacing) (:ascii-indented-line-width nil nil org-ascii-indented-line-width) (:ascii-inlinetask-width nil nil org-ascii-inlinetask-width) (:ascii-inner-margin nil nil org-ascii-inner-margin) (:ascii-links-to-notes nil nil org-ascii-links-to-notes) (:ascii-list-margin nil nil org-ascii-list-margin) (:ascii-paragraph-spacing nil nil org-ascii-paragraph-spacing) (:ascii-quote-margin nil nil org-ascii-quote-margin) (:ascii-table-keep-all-vertical-lines nil nil org-ascii-table-keep-all-vertical-lines) (:ascii-table-use-ascii-art nil nil org-ascii-table-use-ascii-art) (:ascii-table-widen-columns nil nil org-ascii-table-widen-columns) (:ascii-text-width nil nil org-ascii-text-width) (:ascii-underline nil nil org-ascii-underline) (:ascii-verbatim-format nil nil org-ascii-verbatim-format)) ((:filter-headline . org-ascii-filter-headline-blank-lines) (:filter-parse-tree org-ascii-filter-paragraph-spacing org-ascii-filter-comment-spacing) (:filter-section . org-ascii-filter-headline-blank-lines)) nil (116 \"Export to Plain Text\" ((65 \"As ASCII buffer\" (lambda (a s v b) (org-ascii-export-as-ascii a s v b '(:ascii-charset ascii)))) (97 \"As ASCII file\" (lambda (a s v b) (org-ascii-export-to-ascii a s v b '(:ascii-charset ascii)))) (76 \"As Latin1 buffer\" (lambda (a s v b) (org-ascii-export-as-ascii a s v b '(:ascii-charset latin1)))) (108 \"As Latin1 file\" (lambda (a s v b) (org-ascii-export-to-ascii a s v b '(:ascii-charset latin1)))) (85 \"As UTF-8 buffer\" (lambda (a s v b) (org-ascii-export-as-ascii a s v b '(:ascii-charset utf-8)))) (117 \"As UTF-8 file\" (lambda (a s v b) (org-ascii-export-to-ascii a s v b '(:ascii-charset utf-8))))))))""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case nil
    (org-export-define-backend 'test '((template . (lambda (contents info) contents))))
  (error nil))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-get-environment
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_env() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK ((#(\"Test\" 0 4 (:parent (#(\"Test\" 0 4 (:parent #4)))))) (#(\"Me\" 0 2 (:parent (#(\"Me\" 0 2 (:parent #4)))))))""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n#+AUTHOR: Me\n* H1\nBody")
  (let ((env (org-export-get-environment nil)))
    (list (plist-get env :title)
          (plist-get env :author))))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-get-contents
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_contents() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""ERR (void-function org-export-get-contents)""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+TITLE: Test\n* H1\nBody\n** H2\nSub")
  (let ((info (org-export-get-environment nil)))
    (org-export-get-contents (current-buffer) info)))"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-string-as
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r##""OK \"<div id=\\\"table-of-contents\\\" role=\\\"doc-toc\\\">\n<h2>Table of Contents</h2>\n<div id=\\\"text-table-of-contents\\\" role=\\\"doc-toc\\\">\n<ul>\n<li><a href=\\\"#orgf4a6f64\\\">1. H</a></li>\n</ul>\n</div>\n</div>\n<div id=\\\"outline-container-orgf4a6f64\\\" class=\\\"outline-2\\\">\n<h2 id=\\\"orgf4a6f64\\\"><span class=\\\"section-number-2\\\">1.</span> H</h2>\n<div class=\\\"outline-text-2\\\" id=\\\"text-1\\\">\n<p>\nBody <b>bold</b></p>\n</div>\n</div>\n\"""##
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(org-export-string-as "* H\nBody *bold*" 'html t)"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-string-as latex
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_string_latex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK \"\\\\section{H}\n\\\\label{sec:org06165f6}\nBody \\\\textbf{bold}\n\"""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(org-export-string-as "* H\nBody *bold*" 'latex t)"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// org-export-string-as ascii
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uf26_export_string_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK \"1 H\n===\n\n  Body *bold*\n\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(org-export-string-as "* H\nBody *bold*" 'ascii t)"##,
        expect,
    );
}
