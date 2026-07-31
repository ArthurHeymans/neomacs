use expect_test::expect;

use super::{ParityBatchCase, assert_auto_package_update_batch};

fn auto_package_update_old_version_cleanup_recursively_deletes_real_sandbox_directories()
-> ParityBatchCase {
    ParityBatchCase::new(
        "auto_package_update_old_version_cleanup_recursively_deletes_real_sandbox_directories",
        r##"(let*
                             ((root
                               (auto-package-update-test-root
                                "delete-old"))
                              (alpha
                               (auto-package-update-test-path
                                root
                                "alpha-1.0/"))
                              (beta
                               (auto-package-update-test-path
                                root
                                "nested/beta-2.0/"))
                              (alpha-file
                               (auto-package-update-test-path
                                alpha
                                "alpha.el"))
                              (beta-file
                               (auto-package-update-test-path
                                beta
                                "data/state")))
                           (auto-package-update-test-write
                            alpha-file
                            "alpha")
                           (auto-package-update-test-write
                            beta-file
                            "beta")
                           (let
                               ((apu--old-versions-dirs-list
                                 (list alpha beta)))
                             (let ((result
                                    (apu--delete-old-versions-dirs-list)))
                               (list
                                result
                                apu--old-versions-dirs-list
                                (file-exists-p alpha)
                                (file-exists-p beta)
                                (file-exists-p alpha-file)
                                (file-exists-p beta-file)
                                (file-directory-p root)))))"##,
        true,
        expect!["OK (nil nil nil nil nil nil t)"],
    )
}

#[test]
fn install_public_surface_batch() {
    let cases: Vec<ParityBatchCase> = vec![
        auto_package_update_old_version_cleanup_recursively_deletes_real_sandbox_directories(),
    ];
    assert_auto_package_update_batch(&cases);
}
