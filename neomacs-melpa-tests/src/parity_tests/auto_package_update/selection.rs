use expect_test::expect;

use super::assert_auto_package_update_parity;

#[test]
fn auto_package_update_quelpa_filter_reads_cache_and_removes_each_cached_name() {
    let elisp_form = r##"(let
                             ((packages
                               (list
                                'alpha
                                'beta
                                'gamma
                                'delta))
                              calls)
                           (provide 'quelpa)
                           (cl-letf
                               (((symbol-function
                                  'quelpa-read-cache)
                                 (lambda ()
                                   (push :read-cache calls)
                                   (setq
                                    quelpa-cache
                                    '((beta . first)
                                      (delta . second)
                                      (absent . third))))))
                             (let ((result
                                    (apu--filter-quelpa-packages
                                     packages)))
                               (list
                                result
                                packages
                                (nreverse calls)
                                quelpa-cache
                                (featurep 'quelpa)))))"##;
    let expect = expect![
        "OK (#1=(alpha gamma) #1# (:read-cache) ((beta . first) (delta . second) (absent . third)) t)"
    ];

    assert_auto_package_update_parity(elisp_form, expect);
}
