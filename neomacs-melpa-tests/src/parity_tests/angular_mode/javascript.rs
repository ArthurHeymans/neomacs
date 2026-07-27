use expect_test::expect;

use super::assert_angular_mode_parity;

#[test]
fn angular_mode_initializes_real_javascript_buffer_contract() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "angular.module('map', []);\n")
         (angular-mode)
         (list
          major-mode mode-name
          (derived-mode-p
           'angular-mode
           'javascript-mode
           'js-mode)
          comment-start
          comment-end
          indent-line-function
          (mapcar
           (lambda (key)
             (list
              key
              (key-binding
               (kbd key))))
           '("C-M-a" "C-M-e"
             "C-M-h" "M-."
             "C-c C-j"))
          (length
           font-lock-keywords)
          (memq
           angular-font-lock-keywords
           font-lock-keywords)))"##;
    let expect = expect![[
        r#"OK (angular-mode "JavaScript[Angular]" angular-mode "// " "" js-indent-line (("C-M-a" beginning-of-defun) ("C-M-e" end-of-defun) ("C-M-h" mark-defun) ("M-." xref-find-definitions) ("C-c C-j" nil)) 36 nil)"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_fontifies_real_module_controller_service_workflow() {
    let elisp_form = r##"(with-temp-buffer
         (angular-mode)
         (insert
          "angular.module('excavation', [])\n"
          "  .controller('MapCtrl', function($scope, $http) {\n"
          "    angular.forEach($scope.ruins, function(ruin) {\n"
          "      $http.get(ruin.url);\n"
          "    });\n"
          "  })\n"
          "  .service('Survey', function($q) { return $q; });\n")
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
          '("angular.module" ".controller"
            "$scope" "$http"
            "angular.forEach" ".forEach"
            ".service" "$q" "return"
            "MapCtrl" "Survey")))"##;
    let expect = expect![[
        r#"OK (("angular.module" font-lock-builtin-face) (".controller" font-lock-builtin-face) ("$scope" font-lock-variable-name-face) ("$http" font-lock-builtin-face) ("angular.forEach" font-lock-builtin-face) (".forEach" font-lock-builtin-face) (".service" font-lock-builtin-face) ("$q" font-lock-builtin-face) ("return" font-lock-keyword-face) ("MapCtrl" font-lock-string-face) ("Survey" font-lock-string-face))"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_fontifies_directive_objects_and_mocha_tests() {
    let elisp_form = r##"(with-temp-buffer
         (angular-mode)
         (insert
          "describe('panel', function() {\n"
          "  beforeEach(module('app'));\n"
          "  before(function() {});\n"
          "  afterEach(function() {});\n"
          "  it('renders', function() {});\n"
          "});\n"
          "var directive = {\n"
          "  controller: PanelCtrl,\n"
          "  controllerAs: 'panel',\n"
          "  link: attach,\n"
          "  scope: {},\n"
          "  templateUrl: 'panel.html',\n"
          "  transclude: true\n"
          "};\n")
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
          '("describe(" "beforeEach("
            "before(" "afterEach(" "it("
            "controller:" "controllerAs:"
            "link:" "scope:" "templateUrl:"
            "transclude:")))"##;
    let expect = expect![[
        r#"OK (("describe(" font-lock-type-face) ("beforeEach(" font-lock-type-face) ("before(" font-lock-type-face) ("afterEach(" font-lock-type-face) ("it(" font-lock-type-face) ("controller:" font-lock-type-face) ("controllerAs:" font-lock-type-face) ("link:" font-lock-type-face) ("scope:" font-lock-type-face) ("templateUrl:" font-lock-type-face) ("transclude:" font-lock-type-face))"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_global_and_service_regexps_observe_real_boundaries() {
    let elisp_form = r##"(let ((global
                        (caar
                         angular-font-lock-keywords))
               (services
                (caadr
                 angular-font-lock-keywords)))
         (mapcar
          (lambda (sample)
            (list
             sample
             (and
              (string-match global sample)
              (list
               (match-string 0 sample)
               (match-beginning 0)
               (match-end 0)))
             (and
              (string-match services sample)
              (list
               (match-string 0 sample)
               (match-beginning 0)
               (match-end 0)))))
          '("angular.module('x')"
            "myangular.module('x')"
            "items.forEach(render)"
            "beforeEach(setup)"
            "$httpBackend.flush()"
            "custom$httpClient"
            "rootScope.$broadcast('x')"
            "angular.isUndefined(value)"
            "angular.version.full")))"##;
    let expect = expect![[
        r#"OK (("angular.module('x')" ("angular.module" 0 14) nil) ("myangular.module('x')" ("angular.module" 2 16) nil) ("items.forEach(render)" (".forEach" 5 13) nil) ("beforeEach(setup)" nil nil) ("$httpBackend.flush()" nil ("$httpBackend" 0 12)) ("custom$httpClient" nil ("$http" 6 11)) ("rootScope.$broadcast('x')" ("$broadcast" 10 20) ("rootScope" 0 9)) ("angular.isUndefined(value)" ("angular.isUndefined" 0 19) nil) ("angular.version.full" ("angular.version" 0 15) nil))"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_refontification_tracks_live_buffer_edits() {
    let elisp_form = r##"(with-temp-buffer
         (angular-mode)
         (insert
          "plain.call(value);\n")
         (font-lock-ensure)
         (let ((before
                (get-text-property
                 (point-min) 'face)))
           (goto-char (point-min))
           (delete-region
            (point)
            (progn
              (search-forward
               "plain.call")
              (point)))
           (insert "angular.copy")
           (font-lock-flush)
           (font-lock-ensure)
           (goto-char (point-min))
           (list
            before
            (buffer-string)
            (get-text-property
             (point) 'face)
            (next-single-property-change
             (point) 'face nil
             (point-max)))))"##;
    let expect = expect![[
        r#"OK (nil #("angular.copy(value);\n" 0 12 (face font-lock-builtin-face)) font-lock-builtin-face 13)"#
    ]];
    assert_angular_mode_parity(elisp_form, expect);
}

#[test]
fn angular_mode_two_buffers_keep_font_lock_state_buffer_local() {
    let elisp_form = r##"(let ((angular-buffer
                        (generate-new-buffer
                         " *angular-js*"))
               (plain-buffer
                (generate-new-buffer
                 " *plain-js*")))
         (unwind-protect
             (progn
               (with-current-buffer
                   angular-buffer
                 (angular-mode)
                 (insert
                  "angular.copy(source);\n")
                 (font-lock-ensure))
               (with-current-buffer
                   plain-buffer
                 (javascript-mode)
                 (insert
                  "angular.copy(source);\n")
                 (font-lock-ensure))
               (list
                (with-current-buffer
                    angular-buffer
                  (list
                   major-mode
                   (get-text-property
                    (point-min) 'face)
                   (local-variable-p
                    'font-lock-keywords)))
                (with-current-buffer
                    plain-buffer
                  (list
                   major-mode
                   (get-text-property
                    (point-min) 'face)
                   (local-variable-p
                    'font-lock-keywords)))))
           (kill-buffer angular-buffer)
           (kill-buffer plain-buffer)))"##;
    let expect = expect!["OK ((angular-mode font-lock-builtin-face t) (js-mode nil t))"];
    assert_angular_mode_parity(elisp_form, expect);
}
