//! Per-face *face-attribute :background* matrix.
//!

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_face_bg_abbrev_table_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'abbrev-table-name :background)");
}

#[test]
fn div_face_bg_blink_matching_paren_offscreen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'blink-matching-paren-offscreen :background)");
}

#[test]
fn div_face_bg_bold() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'bold :background)");
}

#[test]
fn div_face_bg_bold_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'bold-italic :background)");
}

#[test]
fn div_face_bg_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'border :background)");
}

#[test]
fn div_face_bg_buffer_menu_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'buffer-menu-buffer :background)");
}

#[test]
fn div_face_bg_button() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'button :background)");
}

#[test]
fn div_face_bg_child_frame_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'child-frame-border :background)");
}

#[test]
fn div_face_bg_completions_annotations() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-annotations :background)");
}

#[test]
fn div_face_bg_completions_common_part() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-common-part :background)");
}

#[test]
fn div_face_bg_completions_first_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-first-difference :background)");
}

#[test]
fn div_face_bg_completions_group_separator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-group-separator :background)");
}

#[test]
fn div_face_bg_completions_group_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-group-title :background)");
}

#[test]
fn div_face_bg_completions_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'completions-highlight :background)");
}

#[test]
fn div_face_bg_confusingly_reordered() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'confusingly-reordered :background)");
}

#[test]
fn div_face_bg_cursor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'cursor :background)");
}

#[test]
fn div_face_bg_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'default :background)");
}

#[test]
fn div_face_bg_eldoc_highlight_function_argument() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'eldoc-highlight-function-argument :background)");
}

#[test]
fn div_face_bg_elisp_ampersand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-ampersand :background)");
}

#[test]
fn div_face_bg_elisp_binding_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-binding-variable :background)");
}

#[test]
fn div_face_bg_elisp_bound_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-bound-variable :background)");
}

#[test]
fn div_face_bg_elisp_charset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-charset :background)");
}

#[test]
fn div_face_bg_elisp_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-coding :background)");
}

#[test]
fn div_face_bg_elisp_completion_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-completion-category :background)");
}

#[test]
fn div_face_bg_elisp_completion_category_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-completion-category-definition :background)");
}

#[test]
fn div_face_bg_elisp_condition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-condition :background)");
}

#[test]
fn div_face_bg_elisp_constant() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-constant :background)");
}

#[test]
fn div_face_bg_elisp_defcharset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defcharset :background)");
}

#[test]
fn div_face_bg_elisp_defcoding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defcoding :background)");
}

#[test]
fn div_face_bg_elisp_defface() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defface :background)");
}

#[test]
fn div_face_bg_elisp_deficon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-deficon :background)");
}

#[test]
fn div_face_bg_elisp_defmacro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defmacro :background)");
}

#[test]
fn div_face_bg_elisp_defoclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defoclosure :background)");
}

#[test]
fn div_face_bg_elisp_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defun :background)");
}

#[test]
fn div_face_bg_elisp_defvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-defvar :background)");
}

#[test]
fn div_face_bg_elisp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-face :background)");
}

#[test]
fn div_face_bg_elisp_feature() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-feature :background)");
}

#[test]
fn div_face_bg_elisp_free_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-free-variable :background)");
}

#[test]
fn div_face_bg_elisp_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-function :background)");
}

#[test]
fn div_face_bg_elisp_function_property_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-function-property-declaration :background)");
}

#[test]
fn div_face_bg_elisp_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-group :background)");
}

#[test]
fn div_face_bg_elisp_icon() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-icon :background)");
}

#[test]
fn div_face_bg_elisp_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-macro :background)");
}

#[test]
fn div_face_bg_elisp_major_mode_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-major-mode-name :background)");
}

#[test]
fn div_face_bg_elisp_nnoo_backend() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-nnoo-backend :background)");
}

#[test]
fn div_face_bg_elisp_non_local_exit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-non-local-exit :background)");
}

#[test]
fn div_face_bg_elisp_oclosure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-oclosure :background)");
}

#[test]
fn div_face_bg_elisp_rx() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-rx :background)");
}

#[test]
fn div_face_bg_elisp_shadowed_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shadowed-variable :background)");
}

#[test]
fn div_face_bg_elisp_shadowing_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shadowing-variable :background)");
}

#[test]
fn div_face_bg_elisp_shorthand_font_lock_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-shorthand-font-lock-face :background)");
}

#[test]
fn div_face_bg_elisp_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-slot :background)");
}

#[test]
fn div_face_bg_elisp_special_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-special-form :background)");
}

#[test]
fn div_face_bg_elisp_special_variable_declaration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-special-variable-declaration :background)");
}

#[test]
fn div_face_bg_elisp_symbol_at_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-at-mouse :background)");
}

#[test]
fn div_face_bg_elisp_symbol_role() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-role :background)");
}

#[test]
fn div_face_bg_elisp_symbol_role_definition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-symbol-role-definition :background)");
}

#[test]
fn div_face_bg_elisp_theme() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-theme :background)");
}

#[test]
fn div_face_bg_elisp_thing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-thing :background)");
}

#[test]
fn div_face_bg_elisp_throw_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-throw-tag :background)");
}

#[test]
fn div_face_bg_elisp_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-type :background)");
}

#[test]
fn div_face_bg_elisp_unknown_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-unknown-call :background)");
}

#[test]
fn div_face_bg_elisp_variable_at_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-variable-at-point :background)");
}

#[test]
fn div_face_bg_elisp_warning_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-warning-type :background)");
}

#[test]
fn div_face_bg_elisp_widget_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'elisp-widget-type :background)");
}

#[test]
fn div_face_bg_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'error :background)");
}

#[test]
fn div_face_bg_escape_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'escape-glyph :background)");
}

#[test]
fn div_face_bg_file_name_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'file-name-shadow :background)");
}

#[test]
fn div_face_bg_fill_column_indicator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fill-column-indicator :background)");
}

#[test]
fn div_face_bg_fixed_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fixed-pitch :background)");
}

#[test]
fn div_face_bg_fixed_pitch_serif() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fixed-pitch-serif :background)");
}

#[test]
fn div_face_bg_font_lock_bracket_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-bracket-face :background)");
}

#[test]
fn div_face_bg_font_lock_builtin_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-builtin-face :background)");
}

#[test]
fn div_face_bg_font_lock_comment_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-comment-delimiter-face :background)");
}

#[test]
fn div_face_bg_font_lock_comment_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-comment-face :background)");
}

#[test]
fn div_face_bg_font_lock_constant_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-constant-face :background)");
}

#[test]
fn div_face_bg_font_lock_delimiter_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-delimiter-face :background)");
}

#[test]
fn div_face_bg_font_lock_doc_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-doc-face :background)");
}

#[test]
fn div_face_bg_font_lock_doc_markup_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-doc-markup-face :background)");
}

#[test]
fn div_face_bg_font_lock_escape_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-escape-face :background)");
}

#[test]
fn div_face_bg_font_lock_function_call_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-function-call-face :background)");
}

#[test]
fn div_face_bg_font_lock_function_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-function-name-face :background)");
}

#[test]
fn div_face_bg_font_lock_keyword_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-keyword-face :background)");
}

#[test]
fn div_face_bg_font_lock_misc_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-misc-punctuation-face :background)");
}

#[test]
fn div_face_bg_font_lock_negation_char_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-negation-char-face :background)");
}

#[test]
fn div_face_bg_font_lock_number_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-number-face :background)");
}

#[test]
fn div_face_bg_font_lock_operator_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-operator-face :background)");
}

#[test]
fn div_face_bg_font_lock_preprocessor_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-preprocessor-face :background)");
}

#[test]
fn div_face_bg_font_lock_property_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-property-name-face :background)");
}

#[test]
fn div_face_bg_font_lock_property_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-property-use-face :background)");
}

#[test]
fn div_face_bg_font_lock_punctuation_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-punctuation-face :background)");
}

#[test]
fn div_face_bg_font_lock_regexp_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-face :background)");
}

#[test]
fn div_face_bg_font_lock_regexp_grouping_backslash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-grouping-backslash :background)");
}

#[test]
fn div_face_bg_font_lock_regexp_grouping_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-regexp-grouping-construct :background)");
}

#[test]
fn div_face_bg_font_lock_string_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-string-face :background)");
}

#[test]
fn div_face_bg_font_lock_type_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-type-face :background)");
}

#[test]
fn div_face_bg_font_lock_variable_name_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-variable-name-face :background)");
}

#[test]
fn div_face_bg_font_lock_variable_use_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-variable-use-face :background)");
}

#[test]
fn div_face_bg_font_lock_warning_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'font-lock-warning-face :background)");
}

#[test]
fn div_face_bg_fringe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'fringe :background)");
}

#[test]
fn div_face_bg_glyphless_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'glyphless-char :background)");
}

#[test]
fn div_face_bg_header_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line :background)");
}

#[test]
fn div_face_bg_header_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-active :background)");
}

#[test]
fn div_face_bg_header_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-highlight :background)");
}

#[test]
fn div_face_bg_header_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'header-line-inactive :background)");
}

#[test]
fn div_face_bg_help_argument_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-argument-name :background)");
}

#[test]
fn div_face_bg_help_for_help_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-for-help-header :background)");
}

#[test]
fn div_face_bg_help_key_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'help-key-binding :background)");
}

#[test]
fn div_face_bg_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'highlight :background)");
}

#[test]
fn div_face_bg_homoglyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'homoglyph :background)");
}

#[test]
fn div_face_bg_internal_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'internal-border :background)");
}

#[test]
fn div_face_bg_isearch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch :background)");
}

#[test]
fn div_face_bg_isearch_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-fail :background)");
}

#[test]
fn div_face_bg_isearch_group_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-group-1 :background)");
}

#[test]
fn div_face_bg_isearch_group_2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'isearch-group-2 :background)");
}

#[test]
fn div_face_bg_italic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'italic :background)");
}

#[test]
fn div_face_bg_lazy_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'lazy-highlight :background)");
}

#[test]
fn div_face_bg_line_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number :background)");
}

#[test]
fn div_face_bg_line_number_current_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-current-line :background)");
}

#[test]
fn div_face_bg_line_number_major_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-major-tick :background)");
}

#[test]
fn div_face_bg_line_number_minor_tick() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'line-number-minor-tick :background)");
}

#[test]
fn div_face_bg_link() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'link :background)");
}

#[test]
fn div_face_bg_link_visited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'link-visited :background)");
}

#[test]
fn div_face_bg_margin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'margin :background)");
}

#[test]
fn div_face_bg_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'match :background)");
}

#[test]
fn div_face_bg_menu() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'menu :background)");
}

#[test]
fn div_face_bg_minibuffer_nonselected() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'minibuffer-nonselected :background)");
}

#[test]
fn div_face_bg_minibuffer_prompt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'minibuffer-prompt :background)");
}

#[test]
fn div_face_bg_mode_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line :background)");
}

#[test]
fn div_face_bg_mode_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-active :background)");
}

#[test]
fn div_face_bg_mode_line_buffer_id() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-buffer-id :background)");
}

#[test]
fn div_face_bg_mode_line_emphasis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-emphasis :background)");
}

#[test]
fn div_face_bg_mode_line_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-highlight :background)");
}

#[test]
fn div_face_bg_mode_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mode-line-inactive :background)");
}

#[test]
fn div_face_bg_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mouse :background)");
}

#[test]
fn div_face_bg_mouse_drag_and_drop_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'mouse-drag-and-drop-region :background)");
}

#[test]
fn div_face_bg_next_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'next-error :background)");
}

#[test]
fn div_face_bg_next_error_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'next-error-message :background)");
}

#[test]
fn div_face_bg_nobreak_hyphen() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'nobreak-hyphen :background)");
}

#[test]
fn div_face_bg_nobreak_space() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'nobreak-space :background)");
}

#[test]
fn div_face_bg_query_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'query-replace :background)");
}

#[test]
fn div_face_bg_read_multiple_choice_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'read-multiple-choice-face :background)");
}

#[test]
fn div_face_bg_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'region :background)");
}

#[test]
fn div_face_bg_scroll_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'scroll-bar :background)");
}

#[test]
fn div_face_bg_secondary_selection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'secondary-selection :background)");
}

#[test]
fn div_face_bg_separator_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'separator-line :background)");
}

#[test]
fn div_face_bg_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'shadow :background)");
}

#[test]
fn div_face_bg_show_paren_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-match :background)");
}

#[test]
fn div_face_bg_show_paren_match_expression() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-match-expression :background)");
}

#[test]
fn div_face_bg_show_paren_mismatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'show-paren-mismatch :background)");
}

#[test]
fn div_face_bg_success() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'success :background)");
}

#[test]
fn div_face_bg_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar :background)");
}

#[test]
fn div_face_bg_tab_bar_tab() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab :background)");
}

#[test]
fn div_face_bg_tab_bar_tab_group_current() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-group-current :background)");
}

#[test]
fn div_face_bg_tab_bar_tab_group_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-group-inactive :background)");
}

#[test]
fn div_face_bg_tab_bar_tab_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-highlight :background)");
}

#[test]
fn div_face_bg_tab_bar_tab_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-inactive :background)");
}

#[test]
fn div_face_bg_tab_bar_tab_ungrouped() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-bar-tab-ungrouped :background)");
}

#[test]
fn div_face_bg_tab_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line :background)");
}

#[test]
fn div_face_bg_tab_line_active() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line-active :background)");
}

#[test]
fn div_face_bg_tab_line_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tab-line-inactive :background)");
}

#[test]
fn div_face_bg_tabulated_list_fake_header() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tabulated-list-fake-header :background)");
}

#[test]
fn div_face_bg_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tool-bar :background)");
}

#[test]
fn div_face_bg_tooltip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tooltip :background)");
}

#[test]
fn div_face_bg_trailing_whitespace() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'trailing-whitespace :background)");
}

#[test]
fn div_face_bg_tty_menu_disabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-disabled-face :background)");
}

#[test]
fn div_face_bg_tty_menu_enabled_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-enabled-face :background)");
}

#[test]
fn div_face_bg_tty_menu_selected_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'tty-menu-selected-face :background)");
}

#[test]
fn div_face_bg_underline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'underline :background)");
}

#[test]
fn div_face_bg_variable_pitch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'variable-pitch :background)");
}

#[test]
fn div_face_bg_variable_pitch_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'variable-pitch-text :background)");
}

#[test]
fn div_face_bg_vc_conflict_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-conflict-state :background)");
}

#[test]
fn div_face_bg_vc_edited_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-edited-state :background)");
}

#[test]
fn div_face_bg_vc_ignored_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-ignored-state :background)");
}

#[test]
fn div_face_bg_vc_locally_added_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-locally-added-state :background)");
}

#[test]
fn div_face_bg_vc_locked_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-locked-state :background)");
}

#[test]
fn div_face_bg_vc_missing_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-missing-state :background)");
}

#[test]
fn div_face_bg_vc_needs_update_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-needs-update-state :background)");
}

#[test]
fn div_face_bg_vc_removed_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-removed-state :background)");
}

#[test]
fn div_face_bg_vc_state_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-state-base :background)");
}

#[test]
fn div_face_bg_vc_up_to_date_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vc-up-to-date-state :background)");
}

#[test]
fn div_face_bg_vertical_border() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'vertical-border :background)");
}

#[test]
fn div_face_bg_warning() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'warning :background)");
}

#[test]
fn div_face_bg_window_divider() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider :background)");
}

#[test]
fn div_face_bg_window_divider_first_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider-first-pixel :background)");
}

#[test]
fn div_face_bg_window_divider_last_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(face-attribute 'window-divider-last-pixel :background)");
}
