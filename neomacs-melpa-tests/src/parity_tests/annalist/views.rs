use expect_test::expect;

use super::assert_annalist_parity;

#[test]
fn annalist_table_start_index_switches_between_rows_and_nested_headings() {
    let elisp_form = r##"(mapcar
         (lambda (start-index)
           (annalist-test-reset)
           (annalist-test-define-deployments start-index)
           (annalist-test-record-deployments)
           (list
            start-index
            (annalist-test-description
             'operations
             'deployments)))
         '(0 1 2))"##;
    let expect = expect![[
        r#"OK ((0 (org-mode t 1 373 "| Environment | Service  | Version   | Status    | Owner    |\n|-------------+----------+-----------+-----------+----------|\n| production  | api      | 2.4.0     | healthy   | platform |\n| staging     | worker   | 2.5.0-rc1 | deploying | runtime  |\n| production  | frontend | 8.1.2     | healthy   | web      |\n| development | api      | 2.6.0-dev | degraded  | alice    |\n")) (1 (org-mode t 1 483 "* production\n| Service  | Version | Status  | Owner    |\n|----------+---------+---------+----------|\n| api      |   2.4.0 | healthy | platform |\n| frontend |   8.1.2 | healthy | web      |\n\n* staging\n| Service | Version   | Status    | Owner   |\n|---------+-----------+-----------+---------|\n| worker  | 2.5.0-rc1 | deploying | runtime |\n\n* development\n| Service | Version   | Status   | Owner |\n|---------+-----------+----------+-------|\n| api     | 2.6.0-dev | degraded | alice |\n")) (2 (org-mode t 1 473 "* production\n** api\n| Version | Status  | Owner    |\n|---------+---------+----------|\n|   2.4.0 | healthy | platform |\n\n** frontend\n| Version | Status  | Owner |\n|---------+---------+-------|\n|   8.1.2 | healthy | web   |\n\n* staging\n** worker\n| Version   | Status    | Owner   |\n|-----------+-----------+---------|\n| 2.5.0-rc1 | deploying | runtime |\n\n* development\n** api\n| Version   | Status   | Owner |\n|-----------+----------+-------|\n| 2.6.0-dev | degraded | alice |\n")))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_row_predicate_and_sort_produce_an_actionable_release_queue() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-test-record-deployments)
         (annalist-define-view
             'deployments
             'unhealthy-first
           (list
            :predicate
            (lambda (record)
              (not
               (string= (nth 3 record) "healthy")))
            :sort
            (lambda (left right)
              (let ((left-status (nth 3 left))
                    (right-status (nth 3 right)))
                (if
                    (string= left-status right-status)
                    (string< (nth 1 left) (nth 1 right))
                  (string< left-status right-status))))
            'environment
            'service
            'version
            'status
            'owner)
           :inherit 'default)
         (annalist-test-description
          'operations
          'deployments
          'unhealthy-first))"##;
    let expect = expect![[
        r#"OK (org-mode t 1 241 "| Environment | Service | Version   | Status    | Owner   |\n|-------------+---------+-----------+-----------+---------|\n| development | api     | 2.6.0-dev | degraded  | alice   |\n| staging     | worker  | 2.5.0-rc1 | deploying | runtime |\n")"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_heading_predicates_priorities_and_sorters_shape_nested_runbooks() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments 2)
         (annalist-test-record-deployments)
         (annalist-record
          'operations
          'deployments
          '("staging" "api" "2.5.0-rc2" "queued" "platform"))
         (annalist-define-view
             'deployments
             'runbook
           (list
            (list
             'environment
             :predicate
             (lambda (environment)
               (member
                environment
                '("production" "staging")))
             :prioritize
             '("staging")
             :sort
             #'string>)
            (list
             'service
             :predicate
             (lambda (service)
               (not (string= service "frontend")))
             :prioritize
             '("worker")
             :sort
             #'string<)
            'version
            'status
            'owner)
           :inherit 'default)
         (annalist-test-description
          'operations
          'deployments
          'runbook))"##;
    let expect = expect![[
        r#"OK (org-mode t 1 359 "* staging\n** worker\n| Version   | Status    | Owner   |\n|-----------+-----------+---------|\n| 2.5.0-rc1 | deploying | runtime |\n\n** api\n| Version   | Status | Owner    |\n|-----------+--------+----------|\n| 2.5.0-rc2 | queued | platform |\n\n* production\n** api\n| Version | Status  | Owner    |\n|---------+---------+----------|\n|   2.4.0 | healthy | platform |\n")"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_titles_formatters_defaults_widths_and_pipe_escaping_compose() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-record
          'operations
          'deployments
          '("production|canary"
            "billing-worker"
            "release-2026.07.27+abcdef"
            "healthy|observing"
            "platform-engineering"))
         (annalist-define-view
             'deployments
             'compact
           (list
            :defaults
            (list
             :format #'annalist-capitalize
             :max-width 14)
            (list
             'environment
             :title "Target|Ring"
             :format #'annalist-verbatim
             :max-width nil)
            (list
             'service
             :title "Workload"
             :format
             (annalist-compose
              #'annalist-code
              #'annalist-capitalize))
            (list
             'version
             :title nil
             :format #'annalist-code
             :max-width 12)
            (list
             'status
             :format nil)
            'owner))
         (annalist-test-description
          'operations
          'deployments
          'compact))"##;
    let expect = expect![[
        r#"OK (org-mode t 1 283 "| Target¦Ring         | Workload         | Version        | Status         | Owner          |\n|---------------------+------------------+----------------+----------------+----------------|\n| =production¦canary= | ~Billing-Worker~ | ~release-2026~ | healthy¦observ | Platform-Engin |\n")"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_extracts_long_values_to_numbered_footnotes_with_formatting() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-record
          'operations
          'deployments
          '("production"
            "api"
            "2026.07.27-very-long-release-identifier"
            "blocked pending database migration approval"
            "platform"))
         (annalist-define-view
             'deployments
             'footnotes
           (list
            'environment
            'service
            (list
             'version
             :max-width 12
             :extractp #'stringp
             :format #'annalist-code)
            (list
             'status
             :max-width 15
             :extractp
             (lambda (value)
               (string-match-p " " value))
             :format #'annalist-verbatim)
            'owner)
           :inherit 'default)
         (annalist-test-description
          'operations
          'deployments
          'footnotes))"##;
    let expect = expect![[
        r#"OK (org-mode t 1 273 "| Environment | Service | Version | Status | Owner    |\n|-------------+---------+---------+--------+----------|\n| production  | api     | [fn:1]  | [fn:2] | platform |\n\n[fn:1]\n~2026.07.27-very-long-release-identifier~\n\n[fn:2]\n=blocked pending database migration approval=\n")"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_extracts_long_elisp_values_into_org_source_blocks() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-define-tome
             'automations
           '(:primary-key name
             :table-start-index 0
             name trigger body owner))
         (annalist-define-view
             'automations
             'default
           (list
            '(name :title "Automation")
            'trigger
            (list
             'body
             :max-width 20
             :extractp #'listp
             :src-block-p #'listp)
            'owner))
         (annalist-record
          'runbooks
          'automations
          '(deploy-on-green
            "CI success"
            (progn
              (validate-artifact artifact)
              (promote artifact 'production)
              (notify-on-call artifact))
            "release-engineering"))
         (annalist-test-description
          'runbooks
          'automations))"##;
    let expect = expect![[
        r#"OK (org-mode t 1 328 "| Automation      | Trigger    | Body   | Owner               |\n|-----------------+------------+--------+---------------------|\n| deploy-on-green | CI success | [fn:1] | release-engineering |\n\n[fn:1]\n#+begin_src emacs-lisp\n(progn (validate-artifact artifact) (promote artifact 'production) (notify-on-call artifact))\n#+end_src\n")"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_view_inheritance_merges_nested_field_settings_and_renders_overrides() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-define-view
             'deployments
             'operator
           (list
            :defaults
            (list :max-width 24 :format #'annalist-capitalize)
            (list
             'environment
             :title "Cluster"
             :prioritize '("production"))
            (list
             'service
             :title "Component"
             :format #'annalist-code)
            (list
             'status
             :title "Health"))
           :inherit 'default)
         (annalist-define-view
             'deployments
             'incident
           (list
            :predicate
            (lambda (record)
              (member
               (nth 3 record)
               '("degraded" "deploying")))
            :defaults
            (list :max-width 12)
            (list
             'status
             :format #'upcase)
            (list
             'owner
             :title "Responder"
             :format #'annalist-verbatim))
           :inherit 'operator)
         (annalist-test-record-deployments)
         (let ((settings
                (annalist--get-view-settings
                 'deployments
                 'incident)))
           (list
            (list
             (annalist--item-get
              settings
              'environment
              :title)
             (annalist--item-get
              settings
              'environment
              :prioritize)
             (annalist--item-get
              settings
              'service
              :title)
             (annalist--item-get
              settings
              'service
              :format)
             (annalist--item-get
              settings
              'status
              :title)
             (annalist--item-get
              settings
              'status
              :format)
             (annalist--item-get
              settings
              'owner
              :title)
             (mapcar
              (lambda (record)
                (funcall
                 (plist-get settings :predicate)
                 record))
              '(("production"
                 "api"
                 "2.4.0"
                 "healthy"
                 "platform")
                ("development"
                 "api"
                 "2.6.0-dev"
                 "degraded"
                 "alice"))))
          (annalist-test-description
           'operations
           'deployments
           'incident))))"##;
    let expect = expect![[
        r#"OK (("Cluster" ("production") "Component" annalist-code "Health" upcase "Responder" (nil ("degraded" "deploying"))) (org-mode t 1 257 "| Cluster     | Component | Version   | Health    | Responder |\n|-------------+-----------+-----------+-----------+-----------|\n| Staging     | ~worker~  | 2.5.0-Rc1 | DEPLOYING | =runtime= |\n| Development | ~api~     | 2.6.0-Dev | DEGRADED  | =alice=   |\n"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_default_view_alias_and_generalized_setter_share_one_registry_entry() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (let ((replacement
                '(:predicate identity
                  environment (:name environment :index 0 :title "Ring"))))
           (setf
            (annalist--get-view-settings 'deployments nil)
            replacement)
           (list
            (annalist--get-view-settings
             'deployments
             nil)
            (annalist--get-view-settings
             'deployments
             'default)
            (gethash
             '(deployments . default)
             annalist--tomes-views)
            (hash-table-count
             annalist--tomes-views))))"##;
    let expect = expect![[
        r#"OK (#1=(:predicate identity environment (:name environment :index 0 :title "Ring")) #1# #1# 1)"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_describe_without_records_leaves_an_empty_non_org_output_buffer() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-test-description
          'nobody
          'deployments))"##;
    let expect = expect![[r#"OK (help-mode t 1 2 "\n")"#]];

    assert_annalist_parity(elisp_form, expect);
}
