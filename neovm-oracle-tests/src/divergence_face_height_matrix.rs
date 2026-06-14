//! Per-face *face-attribute :height* matrix (all GNU faces).
//!

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_face_height_abbrev_table_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'abbrev-table-name :height)");
}

#[test]
fn div_face_height_blink_matching_paren_offscreen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'blink-matching-paren-offscreen :height)");
}

#[test]
fn div_face_height_bold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'bold :height)");
}

#[test]
fn div_face_height_bold_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'bold-italic :height)");
}

#[test]
fn div_face_height_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'border :height)");
}

#[test]
fn div_face_height_buffer_menu_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'buffer-menu-buffer :height)");
}

#[test]
fn div_face_height_button() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'button :height)");
}

#[test]
fn div_face_height_child_frame_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'child-frame-border :height)");
}

#[test]
fn div_face_height_completions_annotations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-annotations :height)");
}

#[test]
fn div_face_height_completions_common_part() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-common-part :height)");
}

#[test]
fn div_face_height_completions_first_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-first-difference :height)");
}

#[test]
fn div_face_height_completions_group_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-group-separator :height)");
}

#[test]
fn div_face_height_completions_group_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-group-title :height)");
}

#[test]
fn div_face_height_completions_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-highlight :height)");
}

#[test]
fn div_face_height_confusingly_reordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'confusingly-reordered :height)");
}

#[test]
fn div_face_height_cursor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'cursor :height)");
}

#[test]
fn div_face_height_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'default :height)");
}

#[test]
fn div_face_height_eldoc_highlight_function_argument() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'eldoc-highlight-function-argument :height)");
}

#[test]
fn div_face_height_elisp_ampersand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-ampersand :height)");
}

#[test]
fn div_face_height_elisp_binding_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-binding-variable :height)");
}

#[test]
fn div_face_height_elisp_bound_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-bound-variable :height)");
}

#[test]
fn div_face_height_elisp_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-charset :height)");
}

#[test]
fn div_face_height_elisp_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-coding :height)");
}

#[test]
fn div_face_height_elisp_completion_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-completion-category :height)");
}

#[test]
fn div_face_height_elisp_completion_category_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-completion-category-definition :height)");
}

#[test]
fn div_face_height_elisp_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-condition :height)");
}

#[test]
fn div_face_height_elisp_constant() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-constant :height)");
}

#[test]
fn div_face_height_elisp_defcharset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defcharset :height)");
}

#[test]
fn div_face_height_elisp_defcoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defcoding :height)");
}

#[test]
fn div_face_height_elisp_defface() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defface :height)");
}

#[test]
fn div_face_height_elisp_deficon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-deficon :height)");
}

#[test]
fn div_face_height_elisp_defmacro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defmacro :height)");
}

#[test]
fn div_face_height_elisp_defoclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defoclosure :height)");
}

#[test]
fn div_face_height_elisp_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defun :height)");
}

#[test]
fn div_face_height_elisp_defvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defvar :height)");
}

#[test]
fn div_face_height_elisp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-face :height)");
}

#[test]
fn div_face_height_elisp_feature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-feature :height)");
}

#[test]
fn div_face_height_elisp_free_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-free-variable :height)");
}

#[test]
fn div_face_height_elisp_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-function :height)");
}

#[test]
fn div_face_height_elisp_function_property_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-function-property-declaration :height)");
}

#[test]
fn div_face_height_elisp_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-group :height)");
}

#[test]
fn div_face_height_elisp_icon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-icon :height)");
}

#[test]
fn div_face_height_elisp_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-macro :height)");
}

#[test]
fn div_face_height_elisp_major_mode_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-major-mode-name :height)");
}

#[test]
fn div_face_height_elisp_nnoo_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-nnoo-backend :height)");
}

#[test]
fn div_face_height_elisp_non_local_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-non-local-exit :height)");
}

#[test]
fn div_face_height_elisp_oclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-oclosure :height)");
}

#[test]
fn div_face_height_elisp_rx() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-rx :height)");
}

#[test]
fn div_face_height_elisp_shadowed_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shadowed-variable :height)");
}

#[test]
fn div_face_height_elisp_shadowing_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shadowing-variable :height)");
}

#[test]
fn div_face_height_elisp_shorthand_font_lock_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shorthand-font-lock-face :height)");
}

#[test]
fn div_face_height_elisp_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-slot :height)");
}

#[test]
fn div_face_height_elisp_special_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-special-form :height)");
}

#[test]
fn div_face_height_elisp_special_variable_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-special-variable-declaration :height)");
}

#[test]
fn div_face_height_elisp_symbol_at_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-at-mouse :height)");
}

#[test]
fn div_face_height_elisp_symbol_role() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-role :height)");
}

#[test]
fn div_face_height_elisp_symbol_role_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-role-definition :height)");
}

#[test]
fn div_face_height_elisp_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-theme :height)");
}

#[test]
fn div_face_height_elisp_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-thing :height)");
}

#[test]
fn div_face_height_elisp_throw_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-throw-tag :height)");
}

#[test]
fn div_face_height_elisp_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-type :height)");
}

#[test]
fn div_face_height_elisp_unknown_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-unknown-call :height)");
}

#[test]
fn div_face_height_elisp_variable_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-variable-at-point :height)");
}

#[test]
fn div_face_height_elisp_warning_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-warning-type :height)");
}

#[test]
fn div_face_height_elisp_widget_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-widget-type :height)");
}

#[test]
fn div_face_height_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'error :height)");
}

#[test]
fn div_face_height_escape_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'escape-glyph :height)");
}

#[test]
fn div_face_height_file_name_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'file-name-shadow :height)");
}

#[test]
fn div_face_height_fill_column_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fill-column-indicator :height)");
}

#[test]
fn div_face_height_fixed_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fixed-pitch :height)");
}

#[test]
fn div_face_height_fixed_pitch_serif() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fixed-pitch-serif :height)");
}

#[test]
fn div_face_height_font_lock_bracket_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-bracket-face :height)");
}

#[test]
fn div_face_height_font_lock_builtin_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-builtin-face :height)");
}

#[test]
fn div_face_height_font_lock_comment_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-comment-delimiter-face :height)");
}

#[test]
fn div_face_height_font_lock_comment_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-comment-face :height)");
}

#[test]
fn div_face_height_font_lock_constant_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-constant-face :height)");
}

#[test]
fn div_face_height_font_lock_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-delimiter-face :height)");
}

#[test]
fn div_face_height_font_lock_doc_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-doc-face :height)");
}

#[test]
fn div_face_height_font_lock_doc_markup_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-doc-markup-face :height)");
}

#[test]
fn div_face_height_font_lock_escape_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-escape-face :height)");
}

#[test]
fn div_face_height_font_lock_function_call_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-function-call-face :height)");
}

#[test]
fn div_face_height_font_lock_function_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-function-name-face :height)");
}

#[test]
fn div_face_height_font_lock_keyword_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-keyword-face :height)");
}

#[test]
fn div_face_height_font_lock_misc_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-misc-punctuation-face :height)");
}

#[test]
fn div_face_height_font_lock_negation_char_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-negation-char-face :height)");
}

#[test]
fn div_face_height_font_lock_number_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-number-face :height)");
}

#[test]
fn div_face_height_font_lock_operator_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-operator-face :height)");
}

#[test]
fn div_face_height_font_lock_preprocessor_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-preprocessor-face :height)");
}

#[test]
fn div_face_height_font_lock_property_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-property-name-face :height)");
}

#[test]
fn div_face_height_font_lock_property_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-property-use-face :height)");
}

#[test]
fn div_face_height_font_lock_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-punctuation-face :height)");
}

#[test]
fn div_face_height_font_lock_regexp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-face :height)");
}

#[test]
fn div_face_height_font_lock_regexp_grouping_backslash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-grouping-backslash :height)");
}

#[test]
fn div_face_height_font_lock_regexp_grouping_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-grouping-construct :height)");
}

#[test]
fn div_face_height_font_lock_string_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-string-face :height)");
}

#[test]
fn div_face_height_font_lock_type_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-type-face :height)");
}

#[test]
fn div_face_height_font_lock_variable_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-variable-name-face :height)");
}

#[test]
fn div_face_height_font_lock_variable_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-variable-use-face :height)");
}

#[test]
fn div_face_height_font_lock_warning_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-warning-face :height)");
}

#[test]
fn div_face_height_fringe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fringe :height)");
}

#[test]
fn div_face_height_glyphless_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'glyphless-char :height)");
}

#[test]
fn div_face_height_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line :height)");
}

#[test]
fn div_face_height_header_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-active :height)");
}

#[test]
fn div_face_height_header_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-highlight :height)");
}

#[test]
fn div_face_height_header_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-inactive :height)");
}

#[test]
fn div_face_height_help_argument_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-argument-name :height)");
}

#[test]
fn div_face_height_help_for_help_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-for-help-header :height)");
}

#[test]
fn div_face_height_help_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-key-binding :height)");
}

#[test]
fn div_face_height_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'highlight :height)");
}

#[test]
fn div_face_height_homoglyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'homoglyph :height)");
}

#[test]
fn div_face_height_internal_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'internal-border :height)");
}

#[test]
fn div_face_height_isearch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch :height)");
}

#[test]
fn div_face_height_isearch_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-fail :height)");
}

#[test]
fn div_face_height_isearch_group_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-group-1 :height)");
}

#[test]
fn div_face_height_isearch_group_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-group-2 :height)");
}

#[test]
fn div_face_height_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'italic :height)");
}

#[test]
fn div_face_height_lazy_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'lazy-highlight :height)");
}

#[test]
fn div_face_height_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number :height)");
}

#[test]
fn div_face_height_line_number_current_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-current-line :height)");
}

#[test]
fn div_face_height_line_number_major_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-major-tick :height)");
}

#[test]
fn div_face_height_line_number_minor_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-minor-tick :height)");
}

#[test]
fn div_face_height_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'link :height)");
}

#[test]
fn div_face_height_link_visited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'link-visited :height)");
}

#[test]
fn div_face_height_margin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'margin :height)");
}

#[test]
fn div_face_height_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'match :height)");
}

#[test]
fn div_face_height_menu() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'menu :height)");
}

#[test]
fn div_face_height_minibuffer_nonselected() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'minibuffer-nonselected :height)");
}

#[test]
fn div_face_height_minibuffer_prompt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'minibuffer-prompt :height)");
}

#[test]
fn div_face_height_mode_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line :height)");
}

#[test]
fn div_face_height_mode_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-active :height)");
}

#[test]
fn div_face_height_mode_line_buffer_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-buffer-id :height)");
}

#[test]
fn div_face_height_mode_line_emphasis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-emphasis :height)");
}

#[test]
fn div_face_height_mode_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-highlight :height)");
}

#[test]
fn div_face_height_mode_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-inactive :height)");
}

#[test]
fn div_face_height_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mouse :height)");
}

#[test]
fn div_face_height_mouse_drag_and_drop_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mouse-drag-and-drop-region :height)");
}

#[test]
fn div_face_height_next_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'next-error :height)");
}

#[test]
fn div_face_height_next_error_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'next-error-message :height)");
}

#[test]
fn div_face_height_nobreak_hyphen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'nobreak-hyphen :height)");
}

#[test]
fn div_face_height_nobreak_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'nobreak-space :height)");
}

#[test]
fn div_face_height_query_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'query-replace :height)");
}

#[test]
fn div_face_height_read_multiple_choice_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'read-multiple-choice-face :height)");
}

#[test]
fn div_face_height_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'region :height)");
}

#[test]
fn div_face_height_scroll_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'scroll-bar :height)");
}

#[test]
fn div_face_height_secondary_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'secondary-selection :height)");
}

#[test]
fn div_face_height_separator_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'separator-line :height)");
}

#[test]
fn div_face_height_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'shadow :height)");
}

#[test]
fn div_face_height_show_paren_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-match :height)");
}

#[test]
fn div_face_height_show_paren_match_expression() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-match-expression :height)");
}

#[test]
fn div_face_height_show_paren_mismatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-mismatch :height)");
}

#[test]
fn div_face_height_success() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'success :height)");
}

#[test]
fn div_face_height_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar :height)");
}

#[test]
fn div_face_height_tab_bar_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab :height)");
}

#[test]
fn div_face_height_tab_bar_tab_group_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-group-current :height)");
}

#[test]
fn div_face_height_tab_bar_tab_group_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-group-inactive :height)");
}

#[test]
fn div_face_height_tab_bar_tab_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-highlight :height)");
}

#[test]
fn div_face_height_tab_bar_tab_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-inactive :height)");
}

#[test]
fn div_face_height_tab_bar_tab_ungrouped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-ungrouped :height)");
}

#[test]
fn div_face_height_tab_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line :height)");
}

#[test]
fn div_face_height_tab_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line-active :height)");
}

#[test]
fn div_face_height_tab_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line-inactive :height)");
}

#[test]
fn div_face_height_tabulated_list_fake_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tabulated-list-fake-header :height)");
}

#[test]
fn div_face_height_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tool-bar :height)");
}

#[test]
fn div_face_height_tooltip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tooltip :height)");
}

#[test]
fn div_face_height_trailing_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'trailing-whitespace :height)");
}

#[test]
fn div_face_height_tty_menu_disabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-disabled-face :height)");
}

#[test]
fn div_face_height_tty_menu_enabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-enabled-face :height)");
}

#[test]
fn div_face_height_tty_menu_selected_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-selected-face :height)");
}

#[test]
fn div_face_height_underline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'underline :height)");
}

#[test]
fn div_face_height_variable_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'variable-pitch :height)");
}

#[test]
fn div_face_height_variable_pitch_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'variable-pitch-text :height)");
}

#[test]
fn div_face_height_vc_conflict_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-conflict-state :height)");
}

#[test]
fn div_face_height_vc_edited_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-edited-state :height)");
}

#[test]
fn div_face_height_vc_ignored_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-ignored-state :height)");
}

#[test]
fn div_face_height_vc_locally_added_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-locally-added-state :height)");
}

#[test]
fn div_face_height_vc_locked_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-locked-state :height)");
}

#[test]
fn div_face_height_vc_missing_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-missing-state :height)");
}

#[test]
fn div_face_height_vc_needs_update_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-needs-update-state :height)");
}

#[test]
fn div_face_height_vc_removed_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-removed-state :height)");
}

#[test]
fn div_face_height_vc_state_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-state-base :height)");
}

#[test]
fn div_face_height_vc_up_to_date_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-up-to-date-state :height)");
}

#[test]
fn div_face_height_vertical_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vertical-border :height)");
}

#[test]
fn div_face_height_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'warning :height)");
}

#[test]
fn div_face_height_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider :height)");
}

#[test]
fn div_face_height_window_divider_first_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider-first-pixel :height)");
}

#[test]
fn div_face_height_window_divider_last_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider-last-pixel :height)");
}
