use expect_test::expect;

use super::{assert_angular_snippets_autoload_parity, assert_angular_snippets_parity};

#[test]
fn angular_snippets_loads_real_yasnippet_dependency_and_all_mode_tables() {
    let elisp_form = r##"(list
  (featurep 'angular-snippets)
  (featurep 'yasnippet)
  (mapcar
   (lambda (mode)
     (let ((table (gethash mode yas--tables)))
       (list mode
             (and table
                  (length
                   (delete-dups
                    (mapcar
                     (lambda (entry)
                       (yas--template-name
                        (cdr entry)))
                     (yas--table-templates table)))))
             (gethash mode yas--parents))))
   '(html-mode js-mode web-mode js2-mode)))"##;
    let expect = expect![
        "OK (t t ((html-mode 42 nil) (js-mode 17 nil) (web-mode nil (html-mode)) (js2-mode nil (js-mode))))"
    ];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_expands_installed_ng_repeat_and_updates_mirrored_collection() {
    let elisp_form = r##"(with-temp-buffer
  (html-mode)
  (yas-minor-mode 1)
  (let* ((file
          (expand-file-name
           "snippets/html-mode/ng-repeat.yasnippet"
           angular-snippets-root))
         (definition
          (with-temp-buffer
            (insert-file-contents file)
            (yas--parse-template file))))
    (yas-define-snippets 'html-mode
                         (list definition))
    (let ((template
           (yas-lookup-snippet
            "ng-repeat"
            'html-mode)))
      (insert "<li ")
      (yas-expand-snippet template)
      (let ((field (yas-current-field)))
        (delete-region
         (yas--field-start field)
         (yas--field-end field))
        (goto-char (yas--field-start field))
        (insert "product"))
      (yas-next-field-or-maybe-expand)
      (let ((field (yas-current-field)))
        (delete-region
         (yas--field-start field)
         (yas--field-end field))
        (goto-char (yas--field-start field))
        (insert "catalog.products"))
      (yas-next-field-or-maybe-expand)
      (insert ">{{ product.name }}</li>")
      (list
       (buffer-string)
       (point)
       (null (yas-active-snippets))
       ng-snip/last-docs-message))))"##;
    let expect = expect![[
        r#"OK ("<li ng-repeat=\"product in catalog.products\">{{ product.name }}</li>" 68 t "ng-repeat")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_package_descriptor_preserves_frozen_release_and_manual_yasnippet_contract() {
    let elisp_form = r##"(let* ((angular
         (cadr
          (assq 'angular-snippets
                package-alist)))
        (yasnippet
         (cadr
          (assq 'yasnippet
                package-alist))))
  (list
   (list
    (package-desc-name angular)
    (package-version-join
     (package-desc-version angular))
    (package-desc-summary angular)
    (package-desc-reqs angular)
    (package-desc-kind angular)
    (package-desc-archive angular))
   (list
    (package-desc-name yasnippet)
    (package-version-join
     (package-desc-version yasnippet))
    (package-desc-reqs yasnippet))
   (not
    (assq 'yasnippet
          (package-desc-reqs angular)))
   (file-name-nondirectory
    (directory-file-name
     (package-desc-dir angular)))
   (file-name-nondirectory
    (directory-file-name
     (package-desc-dir yasnippet)))))"##;
    let expect = expect![[
        r#"OK ((angular-snippets "20140514.523" "Yasnippets for AngularJS." ((s (1 4 0)) (dash (1 2 0))) nil nil) (yasnippet "20250602.1342" ((cl-lib (0 5)) (emacs (24 4)))) t "angular-snippets-20140514.523" "yasnippet-20250602.1342")"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_installed_library_and_descriptor_match_frozen_archive_bytes() {
    let elisp_form = r##"(let* ((descriptor
         (cadr
          (assq 'angular-snippets
                package-alist)))
        (directory
         (package-desc-dir descriptor)))
  (mapcar
   (lambda (name)
     (let ((file
            (expand-file-name
             name directory)))
       (list
        name
        (file-attribute-size
         (file-attributes file))
        (with-temp-buffer
          (insert-file-contents-literally
           file)
          (secure-hash
           'sha256
           (current-buffer))))))
   '("angular-snippets.el"
     "angular-snippets-pkg.el")))"##;
    let expect = expect![[
        r#"OK (("angular-snippets.el" 6730 "27ad19961d9f71995c188899a8e5a4833c58f020c11957ce348288fc75bea784") ("angular-snippets-pkg.el" 443 "552e130e5f1677606ac11cde63b8499a8645b1c9d73d28a14c424b0584b2d493"))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_exports_exact_callable_and_state_surface() {
    let elisp_form = r##"(list
  (mapcar
   (lambda (symbol)
     (list symbol
           (fboundp symbol)
           (interactive-form symbol)
           (help-function-arglist symbol t)))
   '(ng-snip-show-docs-at-point
     -aget
     ng-snip/directive-to-docs
     ng-snip/docs-value
     ng-snip/forget-last-docs-message
     ng-snip/docs
     ng-snip/show-or-browse-docs
     ng-snip/browse-docs
     ng-snip/maybe-space-after-attr
     ng-snip/closest-ng-identifer
     angular-snippets-initialize))
  (mapcar
   (lambda (symbol)
     (list symbol
           (boundp symbol)
           (and
            (boundp symbol)
            (type-of
             (symbol-value symbol)))))
   '(ng-directive-docstrings
     ng-snip/docs-indirection
     ng-snip/directive-root-url
     ng-docs
     ng-snip/last-docs-message
     angular-snippets-root))
  (featurep 'angular-snippets))"##;
    let expect = expect![
        "OK (((ng-snip-show-docs-at-point t (interactive nil) nil) (-aget t nil (alist key)) (ng-snip/directive-to-docs t nil (directive)) (ng-snip/docs-value t nil (id prop)) (ng-snip/forget-last-docs-message t nil nil) (ng-snip/docs t nil (id)) (ng-snip/show-or-browse-docs t nil (id)) (ng-snip/browse-docs t nil (id)) (ng-snip/maybe-space-after-attr t nil nil) (ng-snip/closest-ng-identifer t nil nil) (angular-snippets-initialize t nil nil)) ((ng-directive-docstrings t cons) (ng-snip/docs-indirection t cons) (ng-snip/directive-root-url t string) (ng-docs t cons) (ng-snip/last-docs-message t symbol) (angular-snippets-root t string)) t)"
    ];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_installed_payload_has_exact_mode_files_sizes_and_hashes() {
    let elisp_form = r##"(mapcar
  (lambda (mode)
    (let* ((directory
            (expand-file-name
             (concat "snippets/" mode)
             angular-snippets-root))
           (files
            (sort
             (directory-files
              directory t
              "^[^.]")
             #'string<)))
      (list
       mode
       (mapcar
        (lambda (file)
          (list
           (file-name-nondirectory file)
           (file-attribute-size
            (file-attributes file))
           (with-temp-buffer
             (insert-file-contents-literally
              file)
             (secure-hash
              'sha256
              (current-buffer)))))
        files))))
  '("html-mode" "js-mode"))"##;
    let expect = expect![[
        r#"OK (("html-mode" (("ng-app.yasnippet" 140 "fe995c9aa7a7872f5821fe8fd7a8c9d6c3c9aea00abd9cf6f8e9507e1e9fe362") ("ng-bind-html-unsafe.yasnippet" 163 "8f132de49a2360db9119677d6468309e02441ea2b4522f2fecf094f582eac36f") ("ng-bind-template.yasnippet" 154 "2dae3595c556999affe5d63fa2b47fff0d752d1485215725c5c9838cae120820") ("ng-bind.yasnippet" 127 "f9b70cdb95708f8942830c9acdbedfd44352c438103a57d6ee6a76e83dc79093") ("ng-change.yasnippet" 133 "412df842ec2e48ff7aa22aafd0a177ff7fdb7c26c1382576f049ef64d10cf9aa") ("ng-checked.yasnippet" 136 "ca3df320ca5e8bc7349bff023b93287aedaa578c444a1cf8022496044db6eed8") ("ng-class-even.yasnippet" 145 "0b34f819632faa0e0b1ec080f71a49633ad907a09eee2850e7bcb5be35b534b1") ("ng-class-odd.yasnippet" 142 "58a68343453b2f0ebf2566a5dd507eac1328a0ec9be77ff1a1f030d25233d902") ("ng-class.yasnippet" 160 "02fe75fbc2b952c83f219cd203dad0823571747a404e88cc735585cd944ec58f") ("ng-click.yasnippet" 130 "b392183ef6f73ddf76edd812e452cc75223f92b06108a1a5499f094f9e2aabaf") ("ng-cloak.yasnippet" 125 "990b0c8a019b580771337b1368955e82724d502d3b7126a453eab237c79d7efd") ("ng-controller.yasnippet" 145 "379f7daa035928d013b2e3686f143804ad0c16d8a381240ab17bf50229a87cf1") ("ng-csp.yasnippet" 119 "54f4e9e94c6fec7f93d959e5a4610a59efc637cea1ab4d72fc2796dbab133f80") ("ng-dblclick.yasnippet" 139 "9255da8249726f57f398a37d29a2e780648fbd47a83528f223b5bb6a04097807") ("ng-disabled.yasnippet" 139 "1f0203b4dab8cdb924dfffcafc48dc9e5ebc81b05865e4edec25da250edfcf7a") ("ng-form.yasnippet" 122 "06170b83ee765b2aac732aa389e3f24e52d88b666b1f7b370ce63c10d8059869") ("ng-hide.yasnippet" 127 "d7df5d16bd3687dde04a039f4c83d8df65cdf87b2452f4690a9b1081811d5c01") ("ng-href.yasnippet" 127 "30716f9d798ce6b32d890a31428a787667f9ca8bb682a2c289e3450c349ba842") ("ng-include.yasnippet" 138 "e3746d8cd8610c1c8b66a1572a108c9dbb300480991967569d010a446b8b6f75") ("ng-init.yasnippet" 127 "3914684de2b66e148927104b73dd00a952013a2730ae2e4996d9ce6eeb286c8c") ("ng-list.yasnippet" 122 "f0dc19e9c0d4f705b744afdb8caf71d51d4e99a233270d08051e1e29b28631e7") ("ng-model.yasnippet" 130 "f6b03d1e49038718bddd1d4473081e8afe4debead70d7e92699d4c8010e6de78") ("ng-mousedown.yasnippet" 142 "aad8d8d2ec654879206c5c486b3c2e97a222ec0a04814897d6e47b12c4ae0d88") ("ng-mouseenter.yasnippet" 145 "04a2f490a35fe000739bf68b3274300c3d5ffd7624f0be54868445e105b9a865") ("ng-mouseleave.yasnippet" 145 "071d014c9a45ea2f4ec99ed4a1e1d74e5ac22d643090a82cbe87d0f79862b3d0") ("ng-mousemove.yasnippet" 142 "1f4a9a0c87e593ce6ed351be1c64a83386f095035f7b8cf0f399c9f04dec124c") ("ng-mouseover.yasnippet" 142 "81f91964ad4a6d8d5cb4c1811e051c71291966f08e09595f13e3f1bf7394aab1") ("ng-mouseup.yasnippet" 136 "3e42b9e359ea71afa7af505766c4046b4f044c57ad0280180e690d450d3283d6") ("ng-multiple.yasnippet" 139 "bf3d666c03e7c2a52e1a5c6b9ef543db409b58a2b4e91cb749d8014030cf1879") ("ng-non-bindable.yasnippet" 151 "05ff11cbbe54273a16619d785c47d3ff113f9deaeb98a4223510152cb509c6f2") ("ng-options.yasnippet" 195 "e93dfdb194e7a6b4c1b831cc429186eee38ac59ecbe2cfc8c6e2bd689796563e") ("ng-pluralize.yasnippet" 227 "1f70c5bf5be96450ba08181ebe47c49a6dff006b20b6bfe191591b135e63b083") ("ng-readonly.yasnippet" 134 "986bf45f4ef3fa800bf285873c74cf1b3c361a3b303bbb7e915815795135fe6a") ("ng-repeat.yasnippet" 153 "7163516e916f1f8af68c26aaaf990dc809f27c511d548d26aa775efaee36f083") ("ng-selected.yasnippet" 139 "6a0bf5a4f6629121d2b1b9a32a2b404376c69ec7de2d73954a031841bafeaf3a") ("ng-show.yasnippet" 127 "3bf11a35052623eb4c616c4c74388882af9a580b5c839d5a9cafe15070c2e3a9") ("ng-src.yasnippet" 124 "119d4f19286053853d0af7f5bb5509e2f6cb96dc57ae435833d5f5faf6e35a40") ("ng-style.yasnippet" 130 "cdf074128eecce32c6c0079c979510e7e660a788b3a14334a536dc277a7e25c0") ("ng-submit.yasnippet" 133 "9102c3975356df34eb92dba28256d0a828db3eeaf6e25b936455a4ba1c12424c") ("ng-switch.yasnippet" 133 "b333f3377de4dc0c94fe5cfa67d7dae7f91b256d74c784fee7a0889b209cd1c2") ("ng-transclude.yasnippet" 140 "7c9d87e999b4f9c0e9fa02ecea3657af45e363952d029617871bbab51cce7eda") ("ng-view.yasnippet" 122 "dbf2043253a87bc36a12e637b7fa40fea92c12958a332ab703cf62f217ac26f4"))) ("js-mode" (("$b.yasnippet" 81 "b42058b35634e6e6e079dfdf460ab5763d69ebe261d480876830e492c4ec7e0b") ("$e.yasnippet" 76 "443f05e7c4a3d1a573321b3b6977644f0f602746015247b4e0a6b8f0ccaea021") ("$f.yasnippet" 86 "813bf30d405238ea70b18a6a762ba6adec5265ccf20f07d43756e381e9881e15") ("$on.yasnippet" 100 "3287dbbe606304b6e6deede1ebe5b12e8afe606334565a309b4386c3f686fde0") ("$v.yasnippet" 68 "d9cb2ecb539007ab58dc5503653cfddca91db5b00442388a138f6c63ed34559d") ("$va.yasnippet" 70 "0322d2dd9c26c978c04de18a0c32d18b0529fd617decbbed2c234e0ccb5ae152") ("$w.yasnippet" 110 "ca85547e4d15bba62040eda933d3aaaec8b90a094a1c01b895823ffa5c318d83") ("ngc.yasnippet" 101 "61a3b7cebe04a8e25cc0f3ffe211d50c671ca4e68e0277ac9e2b6ff54d24b060") ("ngd.yasnippet" 146 "3a1cc00c43e67365fa5fddb006ddfb3164a8258ab8e28887018a0db584605faf") ("ngfa.yasnippet" 92 "2f4a98dc47abfba4fcb06054a139c0a071e503afd6018dc14fa0569121b3bc93") ("ngfi.yasnippet" 131 "35ec0b5e827c54181fa331e6b53c9fdfefaa7400f5e4512d8d530883ba0f2e17") ("ngm.yasnippet" 80 "6aca13439a400494b7eaa966997c36175b304f621903490bb4e4b435cdb615a8") ("ngro.yasnippet" 102 "989ec85fc378ca7e0e16abaff4425158a78f864ba6f37eef822636706998f6c9") ("ngrw.yasnippet" 128 "90424790e1e3329a1c32ee4e44e7f27884bf4f503d9e16b2028e42de9187e982") ("ngrwr.yasnippet" 148 "386bf7c1990b916769da80a8117b39ff5c26c0befbc890a9ed34727a75fa03d3") ("ngs.yasnippet" 89 "25251553210de260d04e41ebef0bc0a398aa2938b065686ed5fb361c415185d1") ("ngw.yasnippet" 108 "b39ffe13c4d1516fee0f6bfe7758ea890e7a9fd1d0769d186d710e0ef0c59bf7"))))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_installed_assets_parse_to_exact_template_names_keys_and_bodies() {
    let elisp_form = r##"(mapcar
  (lambda (mode)
    (let* ((directory
            (expand-file-name
             (concat "snippets/" mode)
             angular-snippets-root))
           (files
            (sort
             (directory-files
              directory t
              "\\.yasnippet\\'")
             #'string<)))
      (list
       mode
       (mapcar
        (lambda (file)
          (let ((definition
                 (with-temp-buffer
                   (insert-file-contents file)
                   (yas--parse-template
                    file))))
            (list
             (file-name-nondirectory
              file)
             (nth 0 definition)
             (nth 2 definition)
             (length
              (nth 1 definition))
             (secure-hash
              'sha256
              (nth 1 definition)))))
        files))))
  '("html-mode" "js-mode"))"##;
    let expect = expect![[
        r#"OK (("html-mode" (("ng-app.yasnippet" "ng" "ng-app" 86 "831e362c5af6723f452a05dad4a7ffcc62a46f10f1653e81b8d45e2e1ddf2fe4") ("ng-bind-html-unsafe.yasnippet" "ng" "ng-bind-html-unsafe" 96 "46ddd87f1caa3ddc953e7b0b2b36077cae02bf94017adfce7779cbeed49ec7fa") ("ng-bind-template.yasnippet" "ng" "ng-bind-template" 90 "ea0ecb43e408e951c92949229d6bdcee5948d0719511dad461b32193bd626a40") ("ng-bind.yasnippet" "ng" "ng-bind" 72 "79026a87fbfb7ae3b4a0c325f0a8d5548c23c45f70590d307ed2699c42d1e065") ("ng-change.yasnippet" "ng" "ng-change" 76 "107653dadf036bbebca760d8df2ab61c6499e6eb4becc2ac5d0973c4ab208f73") ("ng-checked.yasnippet" "ng" "ng-checked" 78 "f3d0d0334ac4c035ae75a4280b6773c20f0e372583213671cda6aad59671c3ec") ("ng-class-even.yasnippet" "ng" "ng-class-even" 84 "419f1c3cbc1ec46b3a2848c7c5043fc3717fde39278e12a45c70f52f3b9d6a7a") ("ng-class-odd.yasnippet" "ng" "ng-class-odd" 82 "f92287aaa12184d3aea5ad5d4f8b8e4f887e3b1978d2f23d0ae03b572da0721a") ("ng-class.yasnippet" "ng" "ng-class" 104 "676defadd8a9bb9dbf33ec7db799c091565e4719236e7d2916f1521f15956889") ("ng-click.yasnippet" "ng" "ng-click" 74 "eddcb3c5119370e7f101aeea0b2470502ce0533424d22eb253e748dedc05a06b") ("ng-cloak.yasnippet" "ng" "ng-cloak" 69 "039b93df06c12773354f18621b56a766eb0e55795d1e60c99e3718dcef14c2db") ("ng-controller.yasnippet" "ng" "ng-controller" 84 "ca0aeaa936f30b6c177c6bb5c0f54717fb6e3f81ace9737ad2e30820135df610") ("ng-csp.yasnippet" "ng" "ng-csp" 65 "90be51663f56a0b8030e9024042b850643ab7d22f312755f39407528dbf1ee82") ("ng-dblclick.yasnippet" "ng" "ng-dblclick" 80 "16e2943c6f2e16de162de782e00a8a232945cc01b58f29b57c59187cf5f499e9") ("ng-disabled.yasnippet" "ng" "ng-disabled" 80 "72a1aff76f0c11d6328d431f1da2a798bb5325f5ea2436beda60d3108c699494") ("ng-form.yasnippet" "ng" "ng-form" 67 "1ddd77848838b3775c8f1c9ee5e15e5dc0dd0ee27221fc7419c4978e264481d8") ("ng-hide.yasnippet" "ng" "ng-hide" 72 "271e1c6580e9d8ebee67eb3b75de40b7bea4c01e6afa7ea59b8b80eea6fb976c") ("ng-href.yasnippet" "ng" "ng-href" 72 "965eb7d2ab6dc529a4552101eb2e28f88f5f8f214913cf04e2778381a6ecf62b") ("ng-include.yasnippet" "ng" "ng-include" 80 "5047533faaf22f9b3c4670f69aaf8a2e2a7324b297926c3be5f23ddaf90bd02a") ("ng-init.yasnippet" "ng" "ng-init" 72 "194b89d30e35fa337de2b474908c76f5a057df7669d09d8a791e37a693182278") ("ng-list.yasnippet" "ng" "ng-list" 67 "c8f193e04d965c0a795d3081c7d29707bcdb1b3d8772040598a56105046386a4") ("ng-model.yasnippet" "ng" "ng-model" 74 "9f0874e491976d75d69738dec4c014b0b9f1b9428e850df685bcb5b3c2b5c3b0") ("ng-mousedown.yasnippet" "ng" "ng-mousedown" 82 "c4721b528cb14a6ca84c5ec507c35ff54eb8718333efe51da4aa856c20a24954") ("ng-mouseenter.yasnippet" "ng" "ng-mouseenter" 84 "42fd16423555a0c517ad088733b0d5d58d547e3b54b1a01315e64c2a5b417a09") ("ng-mouseleave.yasnippet" "ng" "ng-mouseleave" 84 "16cff69ca3e5ef785c156017320b646d835e4bac76bee8f7213711687c9e8525") ("ng-mousemove.yasnippet" "ng" "ng-mousemove" 82 "96d1a205c209d1889f9abe8b3f666f3758ec076f947550b6f063200a105a17f7") ("ng-mouseover.yasnippet" "ng" "ng-mouseover" 82 "c8b674074b42b33d283c8b977812960b5bbed906437d3d580351504e41aeaa3e") ("ng-mouseup.yasnippet" "ng" "ng-mouseup" 78 "bbf970741ef7bd9d9212cdd6d34b84b444d4dcd71fc206e7dc15c9c101632815") ("ng-multiple.yasnippet" "ng" "ng-multiple" 80 "d0965458847d33740d932d25e8b6cc52f7c5d0cd3fc9a45dcd097edb040c4203") ("ng-non-bindable.yasnippet" "ng" "ng-non-bindable" 88 "a8d11b4adb6aa9a83fa9e55ca493bfe92fa33443ddfb752aba5eac17f6a919b5") ("ng-options.yasnippet" "ng" "ng-options" 137 "af2608ff99ec06c08ccf58b017bd589d81338d2219a1367435876320ab016d70") ("ng-pluralize.yasnippet" "ng" "ng-pluralize" 167 "5cada81791a745858de5766f08b82d76149ebe79aeabac3402971ce7ad129fe3") ("ng-readonly.yasnippet" "ng" "ng-readonly" 75 "9594ba4eef8e00bab2c858395ac3597defae6c7bda655652d60f4d8cf697af54") ("ng-repeat.yasnippet" "ng" "ng-repeat" 96 "4afa4ea970de29de260390863d21943453a630204d36c624b1e7c58ed4ad7ea9") ("ng-selected.yasnippet" "ng" "ng-selected" 80 "6b4df3874253aec556f8971ea56a3170b8eec72f189a4ddfe1526817da428166") ("ng-show.yasnippet" "ng" "ng-show" 72 "47b352916993d0fadbc3c2dd2d47b6acbefbc09096266ae21e26d2f7d2eb04b4") ("ng-src.yasnippet" "ng" "ng-src" 70 "1624b63233b9f06d276102de315770519ea2dc99430d4eb9f8893954d918d9e8") ("ng-style.yasnippet" "ng" "ng-style" 74 "533a0fe72f091427b66616089090ac24feb6cb5cec437337d6a9ceca0acdb395") ("ng-submit.yasnippet" "ng" "ng-submit" 76 "e35617788a4b47a2baf714accc771e3f5d73df9b3f9bc33459605c46d42a4e1c") ("ng-switch.yasnippet" "ng" "ng-switch" 76 "1150e353938bdf46ed56a4286e9692fb2f13f62eef1f0fc4ef0616f3aab0cc72") ("ng-transclude.yasnippet" "ng" "ng-transclude" 79 "38bcc6a109fc7d4ee5e9f78a7b1f59d3a57a3a9566d7018824887869c8518b9c") ("ng-view.yasnippet" "ng" "ng-view" 67 "9cc84325f1b8f0af1a963c48861ac77c356f90900aaad0783d6bc40ba9e05334"))) ("js-mode" (("$b.yasnippet" "$b" "$b" 31 "609a504e883bb3048bc2ccf00d4c076fcfdf4374462e316f26e6465fe02c16e7") ("$e.yasnippet" "$e" "$e" 26 "92ce1b439e1a1ade72ed6bbb0cf042b02434be5ae12a4e9df5882ec23d8f3b80") ("$f.yasnippet" "$f" "$f" 36 "493ebab3f42e4330beef2b1768eba1dd1458232cdcfa8310c4fef1155fd1d0a1") ("$on.yasnippet" "$on" "$on" 48 "fd77876222e482b6445936676a6e54a56dfd596a8ed5e855155ef1b83358650f") ("$v.yasnippet" "$v" "$v" 18 "05b1cbae3c3cdbfd864955a9a740a9f244654b35beb75b8993a744b0629ebec9") ("$va.yasnippet" "$va" "$va" 18 "232d724b65310891d1b587ac1f4b47b4af462c3423c601a550ebb563e07ee9c7") ("$w.yasnippet" "$w" "$w" 60 "44c5478eedf4be515257dc4e9407074abaa47002de7f016eecff89d83113b7e6") ("ngc.yasnippet" "ngc" "ngc" 49 "62ce27ed063e5845962684bb719a09b635ba9bc90a18b64731cb6ab0625ec218") ("ngd.yasnippet" "ngd" "ngd" 94 "ca7392f26a2b15471d6495e13c156dcded38729f2a728fc94819953b47e28cf1") ("ngfa.yasnippet" "ngfa" "ngfa" 38 "f33ab0867364a3939621f9febd39bea2ece63834f94cfb978e7f9b9634763d3b") ("ngfi.yasnippet" "ngfi" "ngfi" 77 "b0e3a0ab70be271334f0fa7116b2d222002c9471fd19d2fdb4008c43d4c7e257") ("ngm.yasnippet" "ngm" "ngm" 28 "8eb3350c06be390ad29ee4ff25cbcd583d3e76295bbbdfda8f3d46edf77bdf34") ("ngro.yasnippet" "ngro" "ngro" 48 "065dd78ee4b5769dad1c05666bf986477c50f84bfff18422e5048e689007788f") ("ngrw.yasnippet" "ngrw" "ngrw" 74 "bf5ae3d9f92f36ef7f0b1cc89ae2bc518faefc2f9a3cd5e113ced196d203086b") ("ngrwr.yasnippet" "ngrwr" "ngrwr" 92 "6104a7263283ebeca977160e573670ef3d80942f4820a2e962d3c32c9180eb09") ("ngs.yasnippet" "ngs" "ngs" 37 "98e0aa791b4b8c884dbe09d7c25e3d2f27b02a68d49f35b07a89092edfc1f1cc") ("ngw.yasnippet" "ngw" "ngw" 56 "0502357ca18bcdb038a0255f59048094ef64c377ebe62b2fb7598a4bade1352e"))))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_parent_mode_assets_are_exact_and_initialize_is_idempotent() {
    let elisp_form = r##"(let* ((snippet-root
         (expand-file-name
          "snippets"
          angular-snippets-root))
        (web-parent
         (expand-file-name
          "web-mode/.yas-parents"
          snippet-root))
        (js2-parent
         (expand-file-name
          "js2-mode/.yas-parents"
          snippet-root))
        (before
         (cl-count
          snippet-root
          yas-snippet-dirs
          :test #'file-equal-p)))
  (angular-snippets-initialize)
  (angular-snippets-initialize)
  (list
   (with-temp-buffer
     (insert-file-contents web-parent)
     (buffer-string))
   (with-temp-buffer
     (insert-file-contents js2-parent)
     (buffer-string))
   before
   (cl-count
    snippet-root
    yas-snippet-dirs
    :test #'file-equal-p)
   (length yas-snippet-dirs)
   (file-directory-p snippet-root)))"##;
    let expect = expect![[r#"OK ("html-mode\n" "js-mode\n" 1 1 2 t)"#]];
    assert_angular_snippets_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_generated_autoloads_register_commands_without_loading_package() {
    let elisp_form = r##"(list
  (featurep 'angular-snippets)
  (mapcar
   (lambda (symbol)
     (let ((definition
            (symbol-function symbol)))
       (list
        symbol
        (cond
         ((autoloadp definition)
          (list
           'autoload
           (nth 1 definition)
           (nth 4 definition)))
         ((byte-code-function-p
           definition)
          (list
           'byte-code
           (help-function-arglist
            symbol t)
           (interactive-form symbol)))
         ((functionp definition)
          (list
           'function
           (help-function-arglist
            symbol t)
           (interactive-form symbol)))
         (t definition)))))
   '(ng-snip-show-docs-at-point
     angular-snippets-initialize))
  (cl-remove-if-not
   (lambda (entry)
     (eq
      (car-safe entry)
      'angular-snippets))
   load-history))"##;
    let expect = expect![[
        r#"OK (nil ((ng-snip-show-docs-at-point (autoload "angular-snippets" nil)) (angular-snippets-initialize (autoload "angular-snippets" nil))) nil)"#
    ]];
    assert_angular_snippets_autoload_parity(elisp_form, expect);
}

#[test]
fn angular_snippets_manual_initialize_recovers_all_templates_and_parent_modes() {
    let elisp_form = r##"(progn
  (setq yas--tables
        (make-hash-table)
        yas--parents
        (make-hash-table)
        yas-snippet-dirs nil)
  (angular-snippets-initialize)
  (list
   (mapcar
    (lambda (mode)
      (let ((table
             (gethash mode
                      yas--tables)))
        (list
         mode
         (and table
              (length
               (delete-dups
                (mapcar
                 (lambda (entry)
                   (yas--template-name
                    (cdr entry)))
                 (yas--table-templates
                  table)))))
         (gethash mode
                  yas--parents))))
    '(html-mode js-mode
      web-mode js2-mode))
   (mapcar
    #'file-name-nondirectory
    yas-snippet-dirs)))"##;
    let expect = expect![[
        r#"OK (((html-mode 42 nil) (js-mode 17 nil) (web-mode nil (html-mode)) (js2-mode nil (js-mode))) ("snippets"))"#
    ]];
    assert_angular_snippets_parity(elisp_form, expect);
}
