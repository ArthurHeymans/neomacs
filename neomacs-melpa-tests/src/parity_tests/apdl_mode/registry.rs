use expect_test::expect;

use super::{assert_apdl_mode_autoload_parity, assert_apdl_mode_parity};

#[test]
fn package_descriptor_preserves_the_exact_frozen_release_and_dependency_contract() {
    let elisp_form = r##"(let* ((description (cadr (assq 'apdl-mode package-alist)))
       (directory (package-desc-dir description)))
  (list
   (featurep 'apdl-mode)
   (package-installed-p 'apdl-mode)
   (package-desc-name description)
   (package-version-join (package-desc-version description))
   (package-desc-summary description)
   (package-desc-reqs description)
   (package-desc-extras description)
   (file-name-nondirectory (directory-file-name directory))))"##;
    let expect = expect![[
        r#"OK (t t apdl-mode "20250508.908" "Major mode for the APDL programming language." ((emacs (25 1))) ((:authors ("H. Dieter Wilhelm" . "dieter@duenenhof-wilhelm.de")) (:keywords "languages" "convenience" "tools" "ansys" "apdl") (:revdesc . "4883ab085811") (:commit . "4883ab085811b85cc75c44b5af478ab8f7e98386") (:url . "https://github.com/dieter-wilhelm/apdl-mode")) "apdl-mode-20250508.908")"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn installed_archive_has_the_complete_recipe_selected_payload_without_vendoring_it() {
    let elisp_form = r##"(progn
 (require 'seq)
 (let* ((description (cadr (assq 'apdl-mode package-alist)))
       (directory (package-desc-dir description))
       (entries
        (sort
         (seq-remove
          (lambda (path)
            (or (string-suffix-p ".elc" path)
                (string-suffix-p "README-elpa" path)
                (string-suffix-p "-autoloads.el" path)))
          (directory-files-recursively
           directory "." nil
           (lambda (path)
             (not (member (file-name-nondirectory path)
                          '("." ".."))))))
         #'string-lessp)))
  (mapcar
   (lambda (path)
     (list
      (file-relative-name path directory)
      (file-directory-p path)
      (unless (file-directory-p path)
        (file-attribute-size (file-attributes path)))))
   entries)))"##;
    let expect = expect![[
        r#"OK (("apdl-initialise.el" nil 22403) ("apdl-keyword.el" nil 288457) ("apdl-mode-pkg.el" nil 430) ("apdl-mode.el" nil 144094) ("apdl-mode.info" nil 204431) ("apdl-process.el" nil 93503) ("apdl-template.el" nil 119824) ("apdl-wb-template.el" nil 8365) ("dir" nil 635) ("matlib/42CrMo4_biso_Rp850.MPA_MPL" nil 1264) ("matlib/AlSi9Cu3_biso.MPA_MPL" nil 593) ("matlib/Al_a2024-T3.SI_MPL" nil 797) ("matlib/Al_a6061-T6.SI_MPL" nil 797) ("matlib/Al_a7079-T6.SI_MPL" nil 797) ("matlib/C75s_hardened_kinh.MPA_MPL" nil 886) ("matlib/Cu_pure.SI_MPL" nil 793) ("matlib/M250-35A_aniso.MPA_MPL" nil 909) ("matlib/M250-35A_biso.MPA_MPL" nil 498) ("matlib/M250-35A_orthotropic_elastic.MPA_MPL" nil 702) ("matlib/M800-65A_biso.MPA_MPL" nil 527) ("matlib/Mg_AZ31B-H24.SI_MPL" nil 798) ("matlib/Mg_HK31A-H24.SI_MPL" nil 798) ("matlib/NdFeB_magnet.MPA_MPL" nil 692) ("matlib/Ni_pure.SI_MPL" nil 793) ("matlib/PPS.MPA_MPL" nil 691) ("matlib/PPS_Fortron1140L4_70degC_kinh.MPA_MPL" nil 730) ("matlib/README.org" nil 3577) ("matlib/St37.MPA_MPL" nil 625) ("matlib/St37_elastic.MPA_MPL" nil 624) ("matlib/St70_biso.MPA_MPL" nil 511) ("matlib/Stl_AISI-304.SI_MPL" nil 798) ("matlib/Stl_AISI-C1020.SI_MPL" nil 800) ("matlib/Ti_B-120VCA.SI_MPL" nil 797) ("matlib/X46Cr13.MPA_MPL" nil 464) ("matlib/construction_steel.MPA_MPL" nil 445) ("matlib/copper.MPA_MPL" nil 607) ("matlib/creep_curves_PPS_Fortron1140l4_120degC.csv" nil 767) ("matlib/emagCopper.SI_MPL" nil 1148) ("matlib/emagM3.SI_MPL" nil 1464) ("matlib/emagM54.SI_MPL" nil 1260) ("matlib/emagSa1010.SI_MPL" nil 1430) ("matlib/emagSilicon.SI_MPL" nil 1027) ("matlib/emagVanad.SI_MPL" nil 1315) ("template/3d_press-fit_torque_calculations.mac" nil 625) ("template/harmonic_acceleration_results.mac" nil 3605) ("template/plane-stress_structural_example.mac" nil 1850) ("template/plane_stress_press-fit_torque_calculations.mac" nil 1108) ("template/post26_output.mac" nil 791))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn installed_core_libraries_match_the_exact_frozen_archive_bytes() {
    let elisp_form = r##"(let* ((description (cadr (assq 'apdl-mode package-alist)))
       (directory (package-desc-dir description)))
  (mapcar
   (lambda (name)
     (let ((file (expand-file-name name directory)))
       (list
        name
        (file-attribute-size (file-attributes file))
        (with-temp-buffer
          (set-buffer-multibyte nil)
          (insert-file-contents-literally file)
          (secure-hash 'sha256 (current-buffer))))))
   '("apdl-initialise.el" "apdl-keyword.el" "apdl-mode.el"
     "apdl-process.el" "apdl-template.el" "apdl-wb-template.el"
     "apdl-mode.info" "dir" "apdl-mode-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("apdl-initialise.el" 22403 "289f13e326c8f91c7544aa8cb9784301b86d7f18d47293f246ebdd2d46a251c7") ("apdl-keyword.el" 288457 "446fb9c431d62814df0974b9ae1708e2ac009522f23123302be83970a29e33bf") ("apdl-mode.el" 144094 "208607cf7709dfb00bd6f92d6ede7f660cb91c95e1ae125771a17b1151074767") ("apdl-process.el" 93503 "69edb918a2cc25dbd6e31cecedcc8c392fd5ce02d1c2fe733b984d26adc3df42") ("apdl-template.el" 119824 "c1f810a14ff2fe5852bc0c7183a6979232fc24e9aa4dce40514f16d676633fe0") ("apdl-wb-template.el" 8365 "7e1c7fda71c0002830fa716a70465c95835a7284b55a2a4dc09a51abdd33fe76") ("apdl-mode.info" 204431 "da530874831b84b5bbeadf9d57b46b56c73a9c4ad6cf0323b5b9ce18a2926cd1") ("dir" 635 "6856295f32de11a98bda8af68f224d223427e52a90ad906e50b290fb7151f51d") ("apdl-mode-pkg.el" 430 "0e6628ba4e797c971659b01aa35cec3c2f903e16edff3568215510e0e2b19b97"))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn complete_package_owned_callable_surface_has_stable_names_contracts_and_origins() {
    let elisp_form = r##"(let* ((description (cadr (assq 'apdl-mode package-alist)))
       (directory (file-truename (package-desc-dir description)))
       callables)
  (mapatoms
   (lambda (symbol)
     (when
         (and
          (string-prefix-p "apdl-" (symbol-name symbol))
          (fboundp symbol)
          (let ((file (symbol-file symbol 'defun)))
            (and file
                 (string-prefix-p directory (file-truename file)))))
       (push
        (list
         symbol
         (copy-tree (help-function-arglist symbol t))
         (commandp symbol)
         (concat
          (file-name-sans-extension
           (file-name-nondirectory (symbol-file symbol 'defun)))
          ".el"))
        callables))))
  (setq callables
        (sort callables
              (lambda (left right)
                (string-lessp (symbol-name (car left))
                              (symbol-name (car right))))))
  (list
   (length callables)
   (secure-hash
    'sha256
    (encode-coding-string (prin1-to-string callables) 'utf-8-unix))
   (car callables)
   (nth 50 callables)
   (nth 100 callables)
   (nth 150 callables)
   (car (last callables))))"##;
    let expect = expect![[
        r#"OK (195 "4225bd2bda4dadf7fa70f2c669630c53525da8b06e45fb92afe831efb1625808" (apdl-abbrev-start nil t "apdl-mode.el") (apdl-find-user-variables (&optional _a _b _c) t "apdl-mode.el") (apdl-not-in-code-line-p nil nil "apdl-mode.el") (apdl-skeleton-multi-plot (&optional str arg) t "apdl-template.el") (apdl-zoom-out nil t "apdl-process.el"))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn generated_autoloads_preserve_entry_points_extensions_and_custom_prefix() {
    let elisp_form = r##"(progn
 (require 'cus-edit)
 (list
  (featurep 'apdl-mode)
  (featurep 'apdl-mode-autoloads)
  (mapcar
   (lambda (symbol)
     (list
      symbol
      (fboundp symbol)
      (and (fboundp symbol) (autoloadp (symbol-function symbol)))
      (and (fboundp symbol) (commandp symbol))))
   '(apdl-mode apdl apdl-initialise apdl-start-classics
     apdl-start-launcher apdl-user-license-status
     apdl-license-status apdl-license))
  (mapcar
   (lambda (extension)
     (cons extension (cdr (assoc extension auto-mode-alist))))
   '("\\.mac\\'" "\\.ans\\'" "\\.dat\\'" "\\.inp\\'"))
  (get 'apdl-mode 'custom-autoload)
  (get 'apdl 'custom-autoload)
  (member "apdl-" custom-prefix-list)))"##;
    let expect = expect![[
        r#"OK (nil t ((apdl-mode t t t) (apdl t t t) (apdl-initialise t t nil) (apdl-start-classics t t t) (apdl-start-launcher t t t) (apdl-user-license-status t t t) (apdl-license-status t t t) (apdl-license t t t)) (("\\.mac\\'" . apdl-mode) ("\\.ans\\'" . apdl-mode) ("\\.dat\\'" . apdl-mode) ("\\.inp\\'" . apdl-mode)) nil nil nil)"#
    ]];
    assert_apdl_mode_autoload_parity(elisp_form, expect);
}
