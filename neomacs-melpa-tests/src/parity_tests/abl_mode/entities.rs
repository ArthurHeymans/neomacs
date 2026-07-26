use expect_test::expect;

use super::{assert_abl_mode_parity, assert_abl_mode_signal_parity};

const PYTHON_FIXTURE: &str = r##"(insert
                    "import unittest\n"
                    "\n"
                    "def test_free_standing_one():\n"
                    "#markerone\n"
                    "    pass\n"
                    "\n"
                    "class AblTest(unittest.TestCase):\n"
                    "#markertwo\n"
                    "    def test_abl_mode(self):\n"
                    "        self.fail('A FAILING TEST')\n"
                    "\n"
                    "    def test_other_thing(self):\n"
                    "        pass\n"
                    "\n"
                    "    def test_one_more_thing(self):\n"
                    "        self.fail()\n"
                    "\n"
                    "def test_free_standing_two():\n"
                    "#markerthree\n"
                    "    pass\n")"##;

#[test]
fn abl_class_and_function_detection_match_upstream_fixture_locations_and_indentation() {
    let elisp_form = format!(
        r##"(with-temp-buffer
               {PYTHON_FIXTURE}
               (list
                (progn
                  (goto-char (point-min))
                  (list
                   (abl-class-and-indent)
                   (abl-function-and-indent)))
                (progn
                  (search-forward "#markerone")
                  (list
                   (abl-class-and-indent)
                   (abl-function-and-indent)))
                (progn
                  (search-forward "#markertwo")
                  (list
                   (abl-class-and-indent)
                   (abl-function-and-indent)))
                (progn
                  (search-forward "self.fail")
                  (list
                   (abl-class-and-indent)
                   (abl-function-and-indent)))
                (progn
                  (search-forward "#markerthree")
                  (list
                   (abl-class-and-indent)
                   (abl-function-and-indent)))))"##
    );
    let expect = expect![[
        r#"OK (((nil nil nil) (nil nil nil)) ((nil nil nil) ("test_free_standing_one" 0 3)) (("AblTest" 0 7) ("test_free_standing_one" 0 3)) (("AblTest" 0 7) ("test_abl_mode" 4 9)) (("AblTest" 0 7) ("test_free_standing_two" 0 18)))"#
    ]];

    assert_abl_mode_parity(&elisp_form, expect);
}

#[test]
fn abl_mode_get_test_entity_ports_every_upstream_fixture_position() {
    let elisp_form = format!(
        r##"(with-temp-buffer
               (setq buffer-file-name "/workspace/project/project_tests.py"
                     abl-package-base "/workspace/project/"
                     abl-use-test-file-path t)
               {PYTHON_FIXTURE}
               (list
                (progn
                  (goto-char (point-min))
                  (abl-mode-get-test-entity))
                (progn
                  (search-forward "#markerone")
                  (abl-mode-get-test-entity))
                (progn
                  (search-forward "#markertwo")
                  (abl-mode-get-test-entity))
                (progn
                  (search-forward "self.fail")
                  (abl-mode-get-test-entity))
                (progn
                  (search-forward "        pass")
                  (abl-mode-get-test-entity))
                (progn
                  (search-forward "#markerthree")
                  (abl-mode-get-test-entity))))"##
    );
    let expect = expect![[
        r#"OK ("project_tests.py" "project_tests.py::test_free_standing_one" "project_tests.py::AblTest" "project_tests.py::AblTest::test_abl_mode" "project_tests.py::AblTest::test_other_thing" "project_tests.py::test_free_standing_two")"#
    ]];

    assert_abl_mode_parity(&elisp_form, expect);
}

#[test]
fn abl_mode_get_test_entity_supports_module_names_and_custom_separators() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name
                     "/workspace/project/tests/unit/sample_tests.py"
                     abl-package-base "/workspace/project/"
                     abl-use-test-file-path nil
                     abl-file-class-separator ":"
                     abl-class-method-separator ".")
               (insert
                "class Sample:\n"
                "    def test_value(self):\n"
                "        pass\n")
               (goto-char (point-max))
               (abl-mode-get-test-entity))"##;
    let expect = expect![[r#"OK "tests.unit.sample_tests:Sample:.test_value""#]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_get_test_entity_selects_new_class_after_a_free_function() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name "/p/mixed_tests.py"
                     abl-package-base "/p/"
                     abl-use-test-file-path t)
               (insert
                "def test_before():\n"
                "    pass\n"
                "\n"
                "class Later:\n"
                "    helper = 1\n")
               (goto-char (point-max))
               (abl-mode-get-test-entity))"##;
    let expect = expect![[r#"OK "mixed_tests.py::Later:""#]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_get_test_entity_selects_free_function_after_an_older_class() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name "/p/mixed_tests.py"
                     abl-package-base "/p/"
                     abl-use-test-file-path t)
               (insert
                "class Earlier:\n"
                "    helper = 1\n"
                "\n"
                "def test_after():\n"
                "    pass\n")
               (goto-char (point-max))
               (abl-mode-get-test-entity))"##;
    let expect = expect![[r#"OK "mixed_tests.py::test_after""#]];

    assert_abl_mode_parity(elisp_form, expect);
}

#[test]
fn abl_mode_get_test_entity_signals_when_class_and_function_context_is_ambiguous() {
    let elisp_form = r##"(with-temp-buffer
               (setq buffer-file-name "/p/odd_tests.py"
                     abl-package-base "/p/"
                     abl-use-test-file-path t)
               (insert
                "  def test_indented():\n"
                "    pass\n"
                "    class SameIndent:\n"
                "      helper = 1\n")
               (goto-char (point-max))
               (abl-mode-get-test-entity))"##;
    let expect = expect![[r#"ERR (error "You do not appear to be in a recognized test entity")"#]];

    assert_abl_mode_signal_parity(elisp_form, expect);
}
