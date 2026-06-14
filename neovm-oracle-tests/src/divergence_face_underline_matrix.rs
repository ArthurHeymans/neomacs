//! Per-face *face-attribute :underline* matrix.
//!

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_face_under_abbrev_table_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'abbrev-table-name :underline)");
}

#[test]
fn div_face_under_blink_matching_paren_offscreen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'blink-matching-paren-offscreen :underline)");
}

#[test]
fn div_face_under_bold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'bold :underline)");
}

#[test]
fn div_face_under_bold_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'bold-italic :underline)");
}

#[test]
fn div_face_under_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'border :underline)");
}

#[test]
fn div_face_under_buffer_menu_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'buffer-menu-buffer :underline)");
}

#[test]
fn div_face_under_button() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'button :underline)");
}

#[test]
fn div_face_under_child_frame_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'child-frame-border :underline)");
}

#[test]
fn div_face_under_completions_annotations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-annotations :underline)");
}

#[test]
fn div_face_under_completions_common_part() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-common-part :underline)");
}

#[test]
fn div_face_under_completions_first_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-first-difference :underline)");
}

#[test]
fn div_face_under_completions_group_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-group-separator :underline)");
}

#[test]
fn div_face_under_completions_group_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-group-title :underline)");
}

#[test]
fn div_face_under_completions_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-highlight :underline)");
}

#[test]
fn div_face_under_confusingly_reordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'confusingly-reordered :underline)");
}

#[test]
fn div_face_under_cursor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'cursor :underline)");
}

#[test]
fn div_face_under_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'default :underline)");
}

#[test]
fn div_face_under_eldoc_highlight_function_argument() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'eldoc-highlight-function-argument :underline)");
}

#[test]
fn div_face_under_elisp_ampersand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-ampersand :underline)");
}

#[test]
fn div_face_under_elisp_binding_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-binding-variable :underline)");
}

#[test]
fn div_face_under_elisp_bound_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-bound-variable :underline)");
}

#[test]
fn div_face_under_elisp_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-charset :underline)");
}

#[test]
fn div_face_under_elisp_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-coding :underline)");
}

#[test]
fn div_face_under_elisp_completion_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-completion-category :underline)");
}

#[test]
fn div_face_under_elisp_completion_category_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-completion-category-definition :underline)");
}

#[test]
fn div_face_under_elisp_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-condition :underline)");
}

#[test]
fn div_face_under_elisp_constant() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-constant :underline)");
}

#[test]
fn div_face_under_elisp_defcharset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defcharset :underline)");
}

#[test]
fn div_face_under_elisp_defcoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defcoding :underline)");
}

#[test]
fn div_face_under_elisp_defface() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defface :underline)");
}

#[test]
fn div_face_under_elisp_deficon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-deficon :underline)");
}

#[test]
fn div_face_under_elisp_defmacro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defmacro :underline)");
}

#[test]
fn div_face_under_elisp_defoclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defoclosure :underline)");
}

#[test]
fn div_face_under_elisp_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defun :underline)");
}

#[test]
fn div_face_under_elisp_defvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defvar :underline)");
}

#[test]
fn div_face_under_elisp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-face :underline)");
}

#[test]
fn div_face_under_elisp_feature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-feature :underline)");
}

#[test]
fn div_face_under_elisp_free_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-free-variable :underline)");
}

#[test]
fn div_face_under_elisp_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-function :underline)");
}

#[test]
fn div_face_under_elisp_function_property_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-function-property-declaration :underline)");
}

#[test]
fn div_face_under_elisp_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-group :underline)");
}

#[test]
fn div_face_under_elisp_icon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-icon :underline)");
}

#[test]
fn div_face_under_elisp_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-macro :underline)");
}

#[test]
fn div_face_under_elisp_major_mode_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-major-mode-name :underline)");
}

#[test]
fn div_face_under_elisp_nnoo_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-nnoo-backend :underline)");
}

#[test]
fn div_face_under_elisp_non_local_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-non-local-exit :underline)");
}

#[test]
fn div_face_under_elisp_oclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-oclosure :underline)");
}

#[test]
fn div_face_under_elisp_rx() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-rx :underline)");
}

#[test]
fn div_face_under_elisp_shadowed_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shadowed-variable :underline)");
}

#[test]
fn div_face_under_elisp_shadowing_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shadowing-variable :underline)");
}

#[test]
fn div_face_under_elisp_shorthand_font_lock_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shorthand-font-lock-face :underline)");
}

#[test]
fn div_face_under_elisp_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-slot :underline)");
}

#[test]
fn div_face_under_elisp_special_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-special-form :underline)");
}

#[test]
fn div_face_under_elisp_special_variable_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-special-variable-declaration :underline)");
}

#[test]
fn div_face_under_elisp_symbol_at_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-at-mouse :underline)");
}

#[test]
fn div_face_under_elisp_symbol_role() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-role :underline)");
}

#[test]
fn div_face_under_elisp_symbol_role_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-role-definition :underline)");
}

#[test]
fn div_face_under_elisp_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-theme :underline)");
}

#[test]
fn div_face_under_elisp_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-thing :underline)");
}

#[test]
fn div_face_under_elisp_throw_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-throw-tag :underline)");
}

#[test]
fn div_face_under_elisp_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-type :underline)");
}

#[test]
fn div_face_under_elisp_unknown_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-unknown-call :underline)");
}

#[test]
fn div_face_under_elisp_variable_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-variable-at-point :underline)");
}

#[test]
fn div_face_under_elisp_warning_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-warning-type :underline)");
}

#[test]
fn div_face_under_elisp_widget_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-widget-type :underline)");
}

#[test]
fn div_face_under_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'error :underline)");
}

#[test]
fn div_face_under_escape_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'escape-glyph :underline)");
}

#[test]
fn div_face_under_file_name_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'file-name-shadow :underline)");
}

#[test]
fn div_face_under_fill_column_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fill-column-indicator :underline)");
}

#[test]
fn div_face_under_fixed_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fixed-pitch :underline)");
}

#[test]
fn div_face_under_fixed_pitch_serif() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fixed-pitch-serif :underline)");
}

#[test]
fn div_face_under_font_lock_bracket_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-bracket-face :underline)");
}

#[test]
fn div_face_under_font_lock_builtin_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-builtin-face :underline)");
}

#[test]
fn div_face_under_font_lock_comment_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-comment-delimiter-face :underline)");
}

#[test]
fn div_face_under_font_lock_comment_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-comment-face :underline)");
}

#[test]
fn div_face_under_font_lock_constant_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-constant-face :underline)");
}

#[test]
fn div_face_under_font_lock_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-delimiter-face :underline)");
}

#[test]
fn div_face_under_font_lock_doc_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-doc-face :underline)");
}

#[test]
fn div_face_under_font_lock_doc_markup_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-doc-markup-face :underline)");
}

#[test]
fn div_face_under_font_lock_escape_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-escape-face :underline)");
}

#[test]
fn div_face_under_font_lock_function_call_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-function-call-face :underline)");
}

#[test]
fn div_face_under_font_lock_function_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-function-name-face :underline)");
}

#[test]
fn div_face_under_font_lock_keyword_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-keyword-face :underline)");
}

#[test]
fn div_face_under_font_lock_misc_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-misc-punctuation-face :underline)");
}

#[test]
fn div_face_under_font_lock_negation_char_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-negation-char-face :underline)");
}

#[test]
fn div_face_under_font_lock_number_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-number-face :underline)");
}

#[test]
fn div_face_under_font_lock_operator_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-operator-face :underline)");
}

#[test]
fn div_face_under_font_lock_preprocessor_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-preprocessor-face :underline)");
}

#[test]
fn div_face_under_font_lock_property_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-property-name-face :underline)");
}

#[test]
fn div_face_under_font_lock_property_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-property-use-face :underline)");
}

#[test]
fn div_face_under_font_lock_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-punctuation-face :underline)");
}

#[test]
fn div_face_under_font_lock_regexp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-face :underline)");
}

#[test]
fn div_face_under_font_lock_regexp_grouping_backslash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-grouping-backslash :underline)");
}

#[test]
fn div_face_under_font_lock_regexp_grouping_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-grouping-construct :underline)");
}

#[test]
fn div_face_under_font_lock_string_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-string-face :underline)");
}

#[test]
fn div_face_under_font_lock_type_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-type-face :underline)");
}

#[test]
fn div_face_under_font_lock_variable_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-variable-name-face :underline)");
}

#[test]
fn div_face_under_font_lock_variable_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-variable-use-face :underline)");
}

#[test]
fn div_face_under_font_lock_warning_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-warning-face :underline)");
}

#[test]
fn div_face_under_fringe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fringe :underline)");
}

#[test]
fn div_face_under_glyphless_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'glyphless-char :underline)");
}

#[test]
fn div_face_under_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line :underline)");
}

#[test]
fn div_face_under_header_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-active :underline)");
}

#[test]
fn div_face_under_header_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-highlight :underline)");
}

#[test]
fn div_face_under_header_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-inactive :underline)");
}

#[test]
fn div_face_under_help_argument_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-argument-name :underline)");
}

#[test]
fn div_face_under_help_for_help_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-for-help-header :underline)");
}

#[test]
fn div_face_under_help_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-key-binding :underline)");
}

#[test]
fn div_face_under_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'highlight :underline)");
}

#[test]
fn div_face_under_homoglyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'homoglyph :underline)");
}

#[test]
fn div_face_under_internal_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'internal-border :underline)");
}

#[test]
fn div_face_under_isearch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch :underline)");
}

#[test]
fn div_face_under_isearch_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-fail :underline)");
}

#[test]
fn div_face_under_isearch_group_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-group-1 :underline)");
}

#[test]
fn div_face_under_isearch_group_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-group-2 :underline)");
}

#[test]
fn div_face_under_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'italic :underline)");
}

#[test]
fn div_face_under_lazy_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'lazy-highlight :underline)");
}

#[test]
fn div_face_under_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number :underline)");
}

#[test]
fn div_face_under_line_number_current_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-current-line :underline)");
}

#[test]
fn div_face_under_line_number_major_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-major-tick :underline)");
}

#[test]
fn div_face_under_line_number_minor_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-minor-tick :underline)");
}

#[test]
fn div_face_under_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'link :underline)");
}

#[test]
fn div_face_under_link_visited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'link-visited :underline)");
}

#[test]
fn div_face_under_margin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'margin :underline)");
}

#[test]
fn div_face_under_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'match :underline)");
}

#[test]
fn div_face_under_menu() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'menu :underline)");
}

#[test]
fn div_face_under_minibuffer_nonselected() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'minibuffer-nonselected :underline)");
}

#[test]
fn div_face_under_minibuffer_prompt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'minibuffer-prompt :underline)");
}

#[test]
fn div_face_under_mode_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line :underline)");
}

#[test]
fn div_face_under_mode_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-active :underline)");
}

#[test]
fn div_face_under_mode_line_buffer_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-buffer-id :underline)");
}

#[test]
fn div_face_under_mode_line_emphasis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-emphasis :underline)");
}

#[test]
fn div_face_under_mode_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-highlight :underline)");
}

#[test]
fn div_face_under_mode_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-inactive :underline)");
}

#[test]
fn div_face_under_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mouse :underline)");
}

#[test]
fn div_face_under_mouse_drag_and_drop_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mouse-drag-and-drop-region :underline)");
}

#[test]
fn div_face_under_next_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'next-error :underline)");
}

#[test]
fn div_face_under_next_error_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'next-error-message :underline)");
}

#[test]
fn div_face_under_nobreak_hyphen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'nobreak-hyphen :underline)");
}

#[test]
fn div_face_under_nobreak_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'nobreak-space :underline)");
}

#[test]
fn div_face_under_query_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'query-replace :underline)");
}

#[test]
fn div_face_under_read_multiple_choice_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'read-multiple-choice-face :underline)");
}

#[test]
fn div_face_under_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'region :underline)");
}

#[test]
fn div_face_under_scroll_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'scroll-bar :underline)");
}

#[test]
fn div_face_under_secondary_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'secondary-selection :underline)");
}

#[test]
fn div_face_under_separator_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'separator-line :underline)");
}

#[test]
fn div_face_under_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'shadow :underline)");
}

#[test]
fn div_face_under_show_paren_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-match :underline)");
}

#[test]
fn div_face_under_show_paren_match_expression() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-match-expression :underline)");
}

#[test]
fn div_face_under_show_paren_mismatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-mismatch :underline)");
}

#[test]
fn div_face_under_success() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'success :underline)");
}

#[test]
fn div_face_under_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar :underline)");
}

#[test]
fn div_face_under_tab_bar_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab :underline)");
}

#[test]
fn div_face_under_tab_bar_tab_group_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-group-current :underline)");
}

#[test]
fn div_face_under_tab_bar_tab_group_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-group-inactive :underline)");
}

#[test]
fn div_face_under_tab_bar_tab_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-highlight :underline)");
}

#[test]
fn div_face_under_tab_bar_tab_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-inactive :underline)");
}

#[test]
fn div_face_under_tab_bar_tab_ungrouped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-ungrouped :underline)");
}

#[test]
fn div_face_under_tab_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line :underline)");
}

#[test]
fn div_face_under_tab_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line-active :underline)");
}

#[test]
fn div_face_under_tab_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line-inactive :underline)");
}

#[test]
fn div_face_under_tabulated_list_fake_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tabulated-list-fake-header :underline)");
}

#[test]
fn div_face_under_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tool-bar :underline)");
}

#[test]
fn div_face_under_tooltip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tooltip :underline)");
}

#[test]
fn div_face_under_trailing_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'trailing-whitespace :underline)");
}

#[test]
fn div_face_under_tty_menu_disabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-disabled-face :underline)");
}

#[test]
fn div_face_under_tty_menu_enabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-enabled-face :underline)");
}

#[test]
fn div_face_under_tty_menu_selected_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-selected-face :underline)");
}

#[test]
fn div_face_under_underline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'underline :underline)");
}

#[test]
fn div_face_under_variable_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'variable-pitch :underline)");
}

#[test]
fn div_face_under_variable_pitch_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'variable-pitch-text :underline)");
}

#[test]
fn div_face_under_vc_conflict_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-conflict-state :underline)");
}

#[test]
fn div_face_under_vc_edited_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-edited-state :underline)");
}

#[test]
fn div_face_under_vc_ignored_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-ignored-state :underline)");
}

#[test]
fn div_face_under_vc_locally_added_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-locally-added-state :underline)");
}

#[test]
fn div_face_under_vc_locked_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-locked-state :underline)");
}

#[test]
fn div_face_under_vc_missing_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-missing-state :underline)");
}

#[test]
fn div_face_under_vc_needs_update_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-needs-update-state :underline)");
}

#[test]
fn div_face_under_vc_removed_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-removed-state :underline)");
}

#[test]
fn div_face_under_vc_state_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-state-base :underline)");
}

#[test]
fn div_face_under_vc_up_to_date_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-up-to-date-state :underline)");
}

#[test]
fn div_face_under_vertical_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vertical-border :underline)");
}

#[test]
fn div_face_under_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'warning :underline)");
}

#[test]
fn div_face_under_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider :underline)");
}

#[test]
fn div_face_under_window_divider_first_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider-first-pixel :underline)");
}

#[test]
fn div_face_under_window_divider_last_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider-last-pixel :underline)");
}
