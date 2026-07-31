use expect_test::expect;

use super::assert_auto_package_update_batch;

#[test]
fn selection_public_surface_batch() {
    assert_auto_package_update_batch(&[
        (
            "auto_package_update_quelpa_filter_reads_cache_and_removes_each_cached_name",
            r##"(let
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
                                (featurep 'quelpa)))))"##,
            true,
            expect![
        "OK (#1=(alpha gamma) #1# (:read-cache) ((beta . first) (delta . second) (absent . third)) t)"
    ],
        ),
    ]);
}
