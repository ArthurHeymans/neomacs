use expect_test::expect;

use super::assert_annalist_parity;

#[test]
fn annalist_make_list_handles_atoms_lists_nil_vectors_and_lexical_closures() {
    let elisp_form = r##"(let ((closure
                (let ((prefix "deploy"))
                  (lambda (suffix)
                    (concat prefix suffix)))))
         (mapcar
          (lambda (value)
            (let ((result (annalist--make-list value)))
              (list
               (type-of value)
               (length result)
               (eq value result)
               (and
                (functionp value)
                (funcall (car result) "-now"))
               (cond
                ((functionp value)
                 :function)
                ((vectorp (car result))
                 (append (car result) nil))
                (t
                 (car result))))))
          (list
           'alpha
           '(alpha beta)
           nil
           [alpha beta]
           closure)))"##;
    let expect = expect![[
        r#"OK ((symbol 1 nil nil alpha) (cons 2 t nil alpha) (symbol 0 t nil nil) (vector 1 nil nil (alpha beta)) (interpreted-function 1 nil "deploy-now" :function))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_merge_lists_preserves_order_and_supports_custom_equivalence() {
    let elisp_form = r##"(list
         (annalist--merge-lists
          '(production staging)
          '(staging development production canary))
         (annalist--merge-lists
          '("API" "Worker")
          '("api" "Frontend" "worker")
          #'string-equal-ignore-case)
         (let ((left '(a b))
               (right '(b c)))
           (list
            (annalist--merge-lists left right)
            left
            right)))"##;
    let expect = expect![[
        r#"OK ((production staging development canary) ("API" "Worker" "Frontend") ((a b c) (a b) (b c)))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_merge_plists_and_nested_views_preserve_overrides_and_false_values() {
    let elisp_form = r##"(list
         (annalist--merge-plists
          '(:title "Service" :format nil :width 20)
          '(:title "Fallback" :format upcase :sort string<))
         (annalist--merge-nested-plists
          '(:predicate nil
            service (:title "Application" :format annalist-code)
            :defaults (:max-width 30))
          '(:predicate identity
            :sort annalist-string-<
            service (:title "Service" :sort annalist-string-<)
            owner (:title "Owner")
            :defaults (:format upcase :max-width 50))))"##;
    let expect = expect![[
        r#"OK ((:title "Service" :format upcase :width 20 :sort string<) (owner (:title "Owner") :sort annalist-string-< :predicate identity service (:title "Application" :format annalist-code :sort annalist-string-<) :defaults (:max-width 30 :format upcase)))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_get_and_item_get_distinguish_missing_properties_from_explicit_nil() {
    let elisp_form = r##"(let ((settings
                '(service
                  (:title nil :format annalist-code)
                  owner
                  (:title "Maintainer")
                  :defaults
                  (:title "Default" :format upcase :max-width 40))))
         (list
          (annalist--get
           '(:enabled nil)
           '(:enabled t)
           :enabled
           :missing)
          (annalist--get nil '(:enabled t) :enabled :missing)
          (annalist--get nil nil :enabled :missing)
          (annalist--item-get settings 'service :title :missing)
          (annalist--item-get settings 'service :format :missing)
          (annalist--item-get settings 'service :max-width :missing)
          (annalist--item-get settings 'owner :title :missing)
          (annalist--item-get settings 'version :title :missing)))"##;
    let expect = expect![[r#"OK (nil t :missing nil annalist-code 40 "Maintainer" "Default")"#]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_plistify_settings_generates_exact_indices_primary_keys_and_metadata_slot() {
    let elisp_form = r##"(let ((settings
                (annalist--plistify-settings
                 '(:primary-key (environment service)
                   :table-start-index 2
                   :preprocess identity
                   (environment :test string=)
                   service
                   (version :format annalist-code)
                   status
                   owner)
                 'deployments)))
         (list
          (plist-get settings :type)
          (plist-get settings :key-indices)
          (plist-get settings :final-index)
          (plist-get settings :metadata-index)
          (plist-get settings :table-start-index)
          (plist-get settings :preprocess)
          (mapcar
           (lambda (item)
             (list
              item
              (plist-get settings item)
              (plist-get
               settings
               (plist-get (plist-get settings item) :index))))
           '(environment service version status owner))))"##;
    let expect = expect![
        "OK (deployments (0 1) 4 5 2 identity ((environment #1=(:name environment :index 0 :test string=) #1#) (service #2=(:name service :index 1) #2#) (version #3=(:name version :index 2 :format annalist-code) #3#) (status #4=(:name status :index 3) #4#) (owner #5=(:name owner :index 4) #5#)))"
    ];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_record_list_and_plist_conversion_round_trip_metadata_and_missing_values() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (let* ((ordered
                 '("production"
                   "api"
                   "2.4.0"
                   nil
                   "platform"
                   (:ticket "OPS-42" :rollback t)))
                (plist
                 (annalist-plistify-record ordered 'deployments))
                (roundtrip
                 (annalist-listify-record plist 'deployments))
                (partial
                 (annalist-listify-record
                  '(environment "staging"
                    service "worker"
                    status "queued"
                    t (:ticket "OPS-43"))
                  'deployments)))
           (list ordered plist roundtrip partial)))"##;
    let expect = expect![[
        r#"OK (("production" "api" "2.4.0" nil "platform" #1=(:ticket "OPS-42" :rollback t)) (environment "production" service "api" version "2.4.0" status nil owner "platform" t #1#) ("production" "api" "2.4.0" nil "platform" #1#) ("staging" "worker" nil "queued" nil (:ticket "OPS-43")))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_formatting_composition_sorting_and_pipe_escaping_support_real_labels() {
    let elisp_form = r##"(let ((formatter
                (annalist-compose
                 #'annalist-verbatim
                 #'annalist-capitalize)))
         (list
          (annalist-verbatim "C-c d")
          (annalist-code 'deploy-service)
          (annalist-capitalize 'release_candidate)
          (funcall formatter "production api")
          (annalist--safe-pipe "healthy|degraded|offline")
          (sort
           '("worker" api "Frontend" 42)
           #'annalist-string-<)
          (mapcar
           #'key-description
           (sort
            (list (kbd "C-c z") (kbd "C-c a") (kbd "M-x"))
            #'annalist-key-<))))"##;
    let expect = expect![[
        r#"OK ("=C-c d=" "~deploy-service~" "Release_Candidate" "=Production Api=" "healthy¦degraded¦offline" (42 "Frontend" api "worker") ("C-c a" "C-c z" "M-x"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_record_policy_combines_global_switch_whitelist_blacklist_and_wildcards() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (let ((annalist-record (nth 0 case))
                 (annalist-record-whitelist (nth 1 case))
                 (annalist-record-blacklist (nth 2 case)))
             (list
              (nth 3 case)
              (annalist--should-record-p 'operations 'deployments)
              (annalist--should-record-p 'security 'incidents))))
         '((t nil nil baseline)
           (nil nil nil disabled)
           (t ((operations deployments)) nil exact-whitelist)
           (t ((operations t)) nil annalist-wildcard)
           (t ((t deployments)) nil type-wildcard)
           (t ((t t)) nil full-whitelist)
           (t ((security incidents)) nil unmatched-whitelist)
           (t nil ((operations deployments)) exact-blacklist)
           (t nil ((operations t)) annalist-blacklist)
           (t nil ((t deployments)) type-blacklist)
           (t nil ((t t)) full-blacklist)))"##;
    let expect = expect![
        "OK ((baseline t t) (disabled nil nil) (exact-whitelist t nil) (annalist-wildcard t nil) (type-wildcard t nil) (full-whitelist t t) (unmatched-whitelist nil t) (exact-blacklist nil t) (annalist-blacklist nil t) (type-blacklist nil t) (full-blacklist nil nil))"
    ];

    assert_annalist_parity(elisp_form, expect);
}
