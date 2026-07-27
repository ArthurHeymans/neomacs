use expect_test::expect;

use super::assert_angular_html_mode_parity;

#[test]
fn angular_html_mode_registers_exact_feature_parent_and_keyword_table() {
    let elisp_form = r##"(list
         (featurep 'angular-html-mode)
         (commandp 'angular-html-mode)
         (interactive-form
          'angular-html-mode)
         (get 'angular-html-mode
              'derived-mode-parent)
         (documentation
          'angular-html-mode)
         (help-function-arglist
          'angular-html-mode t)
         (length
          angular-html-font-lock-keywords)
         (secure-hash
          'sha256
          (prin1-to-string
           angular-html-font-lock-keywords)))"##;
    let expect = expect![[
        r#"OK (t t (interactive nil) html-mode "Major HTML mode for AngularJS.\n\nUses keymap ‘html-mode-map’, which is not currently defined.\n\n\nIn addition to any hooks its parent mode ‘html-mode’ might have run,\nthis mode runs the hook ‘angular-html-mode-hook’, as the final or\npenultimate step during initialization." nil 2 "79c32d41155a38d01a2b5e8d667b2a53c2aa24dc1342061011f004f499c6e6d6")"#
    ]];
    assert_angular_html_mode_parity(elisp_form, expect);
}

#[test]
fn angular_html_mode_initializes_real_html_buffer_contract() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "<section ng-app=\"ruins\"></section>")
         (angular-html-mode)
         (list
          major-mode mode-name
          (derived-mode-p
           'angular-html-mode
           'html-mode
           'sgml-mode)
          comment-start
          comment-end
          indent-line-function
          (eq
           (keymap-parent
            (current-local-map))
           html-mode-map)
          (mapcar
           (lambda (key)
             (list
              key
              (key-binding
               (kbd key))))
           '("C-c C-v"
             "C-c C-s"
             "C-c C-c i"
             "C-c C-c h"
             "M-o"))
          (length
           (car font-lock-defaults))))"##;
    let expect = expect![[
        r#"OK (angular-html-mode "HTML[Angular]" angular-html-mode "<!-- " " -->" sgml-indent-line t (("C-c C-v" browse-url-of-buffer) ("C-c C-s" html-autoview-mode) ("C-c C-c i" html-image) ("C-c C-c h" html-href-anchor) ("M-o" facemenu-keymap)) 15)"#
    ]];
    assert_angular_html_mode_parity(elisp_form, expect);
}

#[test]
fn angular_html_mode_fontifies_real_directives_expressions_and_markup() {
    let elisp_form = r##"(with-temp-buffer
         (angular-html-mode)
         (insert
          "<main ng-app=\"excavation\" "
          "ng-controller=\"MapCtrl as map\">\n"
          "  <button ng-click=\"map.open(ruin)\" "
          "ng-disabled=\"!ruin.safe\">\n"
          "    {{ ruin.name }} — {{ map.status }}\n"
          "  </button>\n"
          "  <li ng-repeat=\"item in map.items\" "
          "ng-class=\"{active: item.selected}\">\n"
          "    {{item.label}}\n"
          "  </li>\n"
          "</main>\n")
         (font-lock-ensure)
         (mapcar
          (lambda (needle)
            (goto-char (point-min))
            (search-forward needle)
            (list
             needle
             (get-text-property
              (match-beginning 0)
              'face)))
          '("<main" "ng-app"
            "ng-controller" "MapCtrl"
            "ng-click" "ng-disabled"
            "{{ ruin.name }}" "ng-repeat"
            "ng-class" "{{item.label}}"
            "</main>")))"##;
    let expect = expect![[
        r#"OK (("<main" nil) ("ng-app" font-lock-variable-name-face) ("ng-controller" font-lock-variable-name-face) ("MapCtrl" font-lock-string-face) ("ng-click" font-lock-variable-name-face) ("ng-disabled" font-lock-variable-name-face) ("{{ ruin.name }}" font-lock-keyword-face) ("ng-repeat" font-lock-variable-name-face) ("ng-class" font-lock-variable-name-face) ("{{item.label}}" font-lock-keyword-face) ("</main>" nil))"#
    ]];
    assert_angular_html_mode_parity(elisp_form, expect);
}

#[test]
fn angular_html_mode_directive_regexp_covers_all_legacy_core_names() {
    let elisp_form = r##"(let* ((regexp
                          (car
                           angular-html-font-lock-keywords))
               (directives
                '("ng-app" "ng-bind"
                  "ng-bind-html"
                  "ng-bind-template"
                  "ng-blur" "ng-change"
                  "ng-checked" "ng-class"
                  "ng-class-even" "ng-class-odd"
                  "ng-click" "ng-cloak"
                  "ng-controller" "ng-copy"
                  "ng-csp" "ng-cut"
                  "ng-dblclick" "ng-disabled"
                  "ng-focus" "ng-form"
                  "ng-hide" "ng-href" "ng-if"
                  "ng-include" "ng-init"
                  "ng-keydown" "ng-keypress"
                  "ng-keyup" "ng-list"
                  "ng-model" "ng-mousedown"
                  "ng-mouseenter" "ng-mouseleave"
                  "ng-mousemove" "ng-mouseover"
                  "ng-mouseup" "ng-non-bindable"
                  "ng-open" "ng-paste"
                  "ng-pluralize" "ng-readonly"
                  "ng-repeat" "ng-selected"
                  "ng-show" "ng-src" "ng-srcset"
                  "ng-style" "ng-submit"
                  "ng-switch" "ng-transclude"
                  "ng-value")))
         (list
          (length directives)
          (mapcar
           (lambda (directive)
             (list
              directive
              (string-match-p
               (concat
                "\\`" regexp "\\'")
               directive)))
           directives)
          (mapcar
           (lambda (sample)
             (list
              sample
              (string-match
               regexp sample)
              (and
               (string-match
                regexp sample)
               (match-string 0 sample))))
           '("x-ng-click"
             "data-ng-repeat"
             "ng-click-extra"
             "NG-CLICK"
             "ng-model"))))"##;
    let expect = expect![[
        r#"OK (51 (("ng-app" 0) ("ng-bind" 0) ("ng-bind-html" 0) ("ng-bind-template" 0) ("ng-blur" 0) ("ng-change" 0) ("ng-checked" 0) ("ng-class" 0) ("ng-class-even" 0) ("ng-class-odd" 0) ("ng-click" 0) ("ng-cloak" 0) ("ng-controller" 0) ("ng-copy" 0) ("ng-csp" 0) ("ng-cut" 0) ("ng-dblclick" 0) ("ng-disabled" 0) ("ng-focus" 0) ("ng-form" 0) ("ng-hide" 0) ("ng-href" 0) ("ng-if" 0) ("ng-include" 0) ("ng-init" 0) ("ng-keydown" 0) ("ng-keypress" 0) ("ng-keyup" 0) ("ng-list" 0) ("ng-model" 0) ("ng-mousedown" 0) ("ng-mouseenter" 0) ("ng-mouseleave" 0) ("ng-mousemove" 0) ("ng-mouseover" 0) ("ng-mouseup" 0) ("ng-non-bindable" 0) ("ng-open" 0) ("ng-paste" 0) ("ng-pluralize" 0) ("ng-readonly" 0) ("ng-repeat" 0) ("ng-selected" 0) ("ng-show" 0) ("ng-src" 0) ("ng-srcset" 0) ("ng-style" 0) ("ng-submit" 0) ("ng-switch" 0) ("ng-transclude" 0) ("ng-value" 0)) (("x-ng-click" 2 "ng-click") ("data-ng-repeat" 5 "ng-repeat") ("ng-click-extra" 0 "ng-click") ("NG-CLICK" 0 "NG-CLICK") ("ng-model" 0 "ng-model")))"#
    ]];
    assert_angular_html_mode_parity(elisp_form, expect);
}

#[test]
fn angular_html_mode_expression_regexp_handles_adjacent_and_multiline_templates() {
    let elisp_form = r##"(let ((regexp
                        (cadr
                         angular-html-font-lock-keywords)))
         (mapcar
          (lambda (sample)
            (let ((start 0)
                  matches)
              (while
                  (string-match
                   regexp sample start)
                (push
                 (list
                  (match-string 0 sample)
                  (match-beginning 0)
                  (match-end 0))
                 matches)
                (setq start
                      (match-end 0)))
              (list sample
                    (nreverse matches))))
          '("{{one}}{{ two + 3 }}"
            "prefix {{user.name}} suffix"
            "{{line-one\nline-two}}"
            "{{}}"
            "{{ outer {{ inner }} tail }}"
            "{not-angular}")))"##;
    let expect = expect![[
        r#"OK (("{{one}}{{ two + 3 }}" (("{{one}}" 0 7) ("{{ two + 3 }}" 7 20))) ("prefix {{user.name}} suffix" (("{{user.name}}" 7 20))) ("{{line-one\nline-two}}" nil) ("{{}}" nil) ("{{ outer {{ inner }} tail }}" (("{{ outer {{ inner }}" 0 20))) ("{not-angular}" nil))"#
    ]];
    assert_angular_html_mode_parity(elisp_form, expect);
}

#[test]
fn angular_html_mode_refontification_tracks_inserted_directive_and_expression() {
    let elisp_form = r##"(with-temp-buffer
         (angular-html-mode)
         (insert
          "<div class=\"panel\">plain</div>")
         (font-lock-ensure)
         (goto-char (point-min))
         (search-forward "class")
         (let ((before
                (get-text-property
                 (match-beginning 0)
                 'face)))
           (goto-char (point-min))
           (search-forward "<div")
           (insert
            " ng-show=\"ready\"")
           (search-forward "plain")
           (replace-match
            "{{status}}" t t)
           (font-lock-flush)
           (font-lock-ensure)
           (list
            before
            (buffer-string)
            (progn
              (goto-char (point-min))
              (search-forward "ng-show")
              (get-text-property
               (match-beginning 0)
               'face))
            (progn
              (goto-char (point-min))
              (search-forward "{{status}}")
              (get-text-property
               (match-beginning 0)
               'face)))))"##;
    let expect = expect![[
        r#"OK (font-lock-variable-name-face #("<div ng-show=\"ready\" class=\"panel\">{{status}}</div>" 1 4 (face font-lock-function-name-face) 5 12 (face font-lock-variable-name-face) 13 20 (face font-lock-string-face) 21 26 (face font-lock-variable-name-face) 27 34 (face font-lock-string-face) 35 45 (face font-lock-keyword-face) 47 50 (face font-lock-function-name-face)) font-lock-variable-name-face font-lock-keyword-face)"#
    ]];
    assert_angular_html_mode_parity(elisp_form, expect);
}
