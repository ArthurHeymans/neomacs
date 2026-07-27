use expect_test::expect;

use super::assert_all_the_icons_parity;

#[test]
fn all_the_icons_builtin_cache_reuses_identity_and_keys_on_all_arguments() {
    let elisp_form = r##"(let* ((first
                 (all-the-icons-icon-for-file
                  "main.rs" :height 1))
                (second
                 (all-the-icons-icon-for-file
                  "main.rs" :height 1))
                (different
                 (all-the-icons-icon-for-file
                  "main.rs" :height 2)))
         (list
          (eq first second)
          (equal first second)
          (eq first different)
          (text-properties-at 0 first)
          (text-properties-at 0 different)
          (get 'all-the-icons-icon-for-file
               'all-the-icons--cached)))"##;
    let expect = expect![[
        r#"OK (t t nil (face #1=(:family "all-the-icons" :height 1.2 :inherit all-the-icons-maroon) font-lock-face #1# display (raise -0.24) rear-nonsticky t) (face #2=(:family "all-the-icons" :height 2.4 :inherit all-the-icons-maroon) font-lock-face #2# display (raise -0.24) rear-nonsticky t) t)"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_cache_wraps_once_and_memoizes_nil_and_non_nil_results() {
    let elisp_form = r##"(let ((calls 0))
         (fset 'all-the-icons-test-cache
               (lambda (value)
                 (setq calls (1+ calls))
                 (and (> value 0) (list value calls))))
         (all-the-icons-cache 'all-the-icons-test-cache)
         (let ((wrapped (symbol-function
                         'all-the-icons-test-cache)))
           (all-the-icons-cache 'all-the-icons-test-cache)
           (list
            (funcall 'all-the-icons-test-cache 3)
            (funcall 'all-the-icons-test-cache 3)
            (funcall 'all-the-icons-test-cache 0)
            (funcall 'all-the-icons-test-cache 0)
            calls
            (eq wrapped
                (symbol-function
                 'all-the-icons-test-cache))
            (get 'all-the-icons-test-cache
                 'all-the-icons--cached))))"##;
    let expect = expect!["OK (#1=(3 1) #1# nil nil 3 t t)"];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_cache_clears_after_limit_and_recomputes_evicted_values() {
    let elisp_form = r##"(let ((all-the-icons--cache-limit 2)
               (calls 0))
         (fset 'all-the-icons-test-eviction
               (lambda (value)
                 (setq calls (1+ calls))
                 (list value calls)))
         (all-the-icons-cache 'all-the-icons-test-eviction)
         (let ((a1 (funcall 'all-the-icons-test-eviction 'a))
               (b1 (funcall 'all-the-icons-test-eviction 'b))
               (c1 (funcall 'all-the-icons-test-eviction 'c))
               (d1 (funcall 'all-the-icons-test-eviction 'd))
               (a2 (funcall 'all-the-icons-test-eviction 'a)))
           (list a1 b1 c1 d1 a2 calls)))"##;
    let expect = expect!["OK ((a 1) (b 2) (c 3) (d 4) (a 5) 5)"];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_cached_file_result_retains_original_global_style_snapshot() {
    let elisp_form = r##"(let ((all-the-icons-scale-factor 1)
               first second third)
         (setq first
               (all-the-icons-icon-for-file
                "cache-style.rs" :height 1.2345))
         (setq all-the-icons-scale-factor 3)
         (setq second
               (all-the-icons-icon-for-file
                "cache-style.rs" :height 1.2345))
         (setq third
               (all-the-icons-icon-for-file
                "cache-style.rs" :height 1.2346))
         (list
          (eq first second)
          (text-properties-at 0 first)
          (text-properties-at 0 second)
          (text-properties-at 0 third)))"##;
    let expect = expect![[
        r#"OK (t #2=(face #1=(:family "all-the-icons" :height 1.2345 :inherit all-the-icons-maroon) font-lock-face #1# display (raise -0.2) rear-nonsticky t) #2# (face #3=(:family "all-the-icons" :height 3.7037999999999998 :inherit all-the-icons-maroon) font-lock-face #3# display (raise -0.6000000000000001) rear-nonsticky t))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}
