use expect_test::expect;

use super::assert_annalist_parity;

#[test]
fn annalist_records_and_renders_a_complete_deployment_inventory() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-test-record-deployments)
         (annalist-test-description 'operations 'deployments))"##;
    let expect = expect![[
        r#"OK (org-mode t 1 373 "| Environment | Service  | Version   | Status    | Owner    |\n|-------------+----------+-----------+-----------+----------|\n| production  | api      | 2.4.0     | healthy   | platform |\n| staging     | worker   | 2.5.0-rc1 | deploying | runtime  |\n| production  | frontend | 8.1.2     | healthy   | web      |\n| development | api      | 2.6.0-dev | degraded  | alice    |\n")"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_plist_records_preserve_metadata_and_pad_unspecified_fields() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-record
          'release-bot
          'deployments
          '(environment "staging"
            service "billing"
            version "3.7.0-rc2"
            status "waiting-for-approval"
            t (:ticket "OPS-742" :approvers ("alice" "bob")))
          :plist t)
         (annalist-record
          'release-bot
          'deployments
          '("production" "search" "9.1.0"))
         (let* ((records
                 (gethash
                  'release-bot
                  (annalist--tome 'deployments)))
                (values
                 (annalist--hash-table-values records)))
           (list
            (mapcar
             (lambda (record)
               (list
                record
                (annalist-plistify-record record 'deployments)))
             values)
            (annalist-test-description
             'release-bot
             'deployments))))"##;
    let expect = expect![[
        r#"OK (((("staging" "billing" "3.7.0-rc2" "waiting-for-approval" nil #1=(:ticket "OPS-742" :approvers ("alice" "bob"))) (environment "staging" service "billing" version "3.7.0-rc2" status "waiting-for-approval" owner nil t #1#)) (("production" "search" "9.1.0" nil nil nil) (environment "production" service "search" version "9.1.0" status nil owner nil t nil))) (org-mode t 1 277 "| Environment | Service | Version   | Status               | Owner |\n|-------------+---------+-----------+----------------------+-------|\n| staging     | billing | 3.7.0-rc2 | waiting-for-approval | nil   |\n| production  | search  | 9.1.0     | nil                  | nil   |\n"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_primary_key_replacement_updates_in_place_without_duplicate_rows() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-test-record-deployments)
         (annalist-record
          'operations
          'deployments
          '("production" "api" "2.4.1" "recovering" "sre"))
         (let ((records
                (gethash
                 'operations
                 (annalist--tome 'deployments))))
           (list
            (hash-table-count records)
            (annalist--hash-table-keys records)
            (annalist--hash-table-values records)
            (annalist-test-description
             'operations
             'deployments))))"##;
    let expect = expect![[
        r#"OK (4 (("production" "api") ("staging" "worker") ("production" "frontend") ("development" "api")) (("production" "api" "2.4.1" "recovering" "sre" nil) ("staging" "worker" "2.5.0-rc1" "deploying" "runtime" nil) ("production" "frontend" "8.1.2" "healthy" "web" nil) ("development" "api" "2.6.0-dev" "degraded" "alice" nil)) (org-mode t 1 373 "| Environment | Service  | Version   | Status     | Owner   |\n|-------------+----------+-----------+------------+---------|\n| production  | api      | 2.4.1     | recovering | sre     |\n| staging     | worker   | 2.5.0-rc1 | deploying  | runtime |\n| production  | frontend | 8.1.2     | healthy    | web     |\n| development | api      | 2.6.0-dev | degraded   | alice   |\n"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_preprocess_and_record_update_build_a_real_status_history() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments
          0
          '(environment service)
          (list
           :preprocess
           (lambda (record _settings)
             (let ((copy (copy-sequence record)))
               (setf (nth 0 copy) (downcase (nth 0 copy)))
               (setf (nth 1 copy) (downcase (nth 1 copy)))
               copy))
           :record-update
           (lambda (old-record new-record settings)
             (let* ((copy (copy-sequence new-record))
                    (metadata-index
                     (plist-get settings :metadata-index))
                    (old-metadata
                     (and old-record
                          (nth metadata-index old-record)))
                    (history
                     (copy-sequence
                      (plist-get old-metadata :status-history))))
               (when old-record
                 (setq history
                       (append
                        history
                        (list (nth 3 old-record)))))
               (setf
                (nth metadata-index copy)
                (list
                 :status-history history
                 :revision
                 (1+
                  (or
                   (plist-get old-metadata :revision)
                   0))))
               copy))))
         (annalist-record
          'delivery
          'deployments
          '("PRODUCTION" "API" "4.0.0" "deploying" "platform"))
         (annalist-record
          'delivery
          'deployments
          '("Production" "Api" "4.0.0" "healthy" "platform"))
         (annalist-record
          'delivery
          'deployments
          '("production" "api" "4.0.1" "degraded" "sre"))
         (let* ((records
                 (gethash
                  'delivery
                  (annalist--tome 'deployments)))
                (record
                 (car
                  (annalist--hash-table-values records))))
           (list
            (hash-table-count records)
            record
            (annalist-plistify-record record 'deployments)
            (annalist-test-description
             'delivery
             'deployments))))"##;
    let expect = expect![[
        r#"OK (1 ("production" "api" "4.0.1" "degraded" "sre" #1=(:status-history ("deploying" "healthy") :revision 3)) (environment "production" service "api" version "4.0.1" status "degraded" owner "sre" t #1#) (org-mode t 1 166 "| Environment | Service | Version | Status   | Owner |\n|-------------+---------+---------+----------+-------|\n| production  | api     |   4.0.1 | degraded | sre   |\n"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_view_postprocessing_changes_display_without_mutating_records() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-define-view
             'deployments
             'release-report
           (list
            :postprocess
            (lambda (record _settings)
              (let ((display-record (copy-sequence record)))
                (setf
                 (nth 3 display-record)
                 (upcase (nth 3 display-record)))
                (setf
                 (nth 4 display-record)
                 (format "%s (on-call)" (nth 4 display-record)))
                display-record))
            '(environment :title "Target")
            '(service :title "Application")
            'version
            'status
            'owner)
           :inherit 'default)
         (annalist-record
          'operations
          'deployments
          '("production" "payments" "6.2.3" "healthy" "alice"))
         (let* ((records
                 (gethash
                  'operations
                  (annalist--tome 'deployments)))
                (before
                 (copy-tree
                  (annalist--hash-table-values records)))
                (description
                 (annalist-test-description
                  'operations
                  'deployments
                  'release-report))
                (after
                 (annalist--hash-table-values records)))
           (list before description after (equal before after))))"##;
    let expect = expect![[
        r#"OK ((("production" "payments" "6.2.3" "healthy" "alice" nil)) (org-mode t 1 202 "| Target     | Application | Version | Status  | Owner           |\n|------------+-------------+---------+---------+-----------------|\n| production | payments    |   6.2.3 | HEALTHY | alice (on-call) |\n") (("production" "payments" "6.2.3" "healthy" "alice" nil)) t)"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_local_records_are_visible_only_in_the_recording_buffer() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-record
          'operations
          'deployments
          '("production" "api" "2.4.0" "healthy" "platform"))
         (let ((recording-buffer
                (generate-new-buffer " *annalist-recording-context*"))
               local-description
               unrelated-description)
           (unwind-protect
               (progn
                 (with-current-buffer recording-buffer
                   (annalist-record
                    'operations
                    'deployments
                    '("development"
                      "api"
                      "2.6.0-dev"
                      "degraded"
                      "alice")
                    :local t)
                   (setq local-description
                         (annalist-test-description
                          'operations
                          'deployments)))
                 (with-temp-buffer
                   (setq unrelated-description
                         (annalist-test-description
                          'operations
                          'deployments)))
                 (list
                  local-description
                  unrelated-description
                  (with-current-buffer recording-buffer
                    (hash-table-count
                     (gethash
                      'operations
                      (annalist--tome 'deployments t))))))
             (kill-buffer recording-buffer))))"##;
    let expect = expect![[
        r#"OK ((org-mode t 1 361 "* Local\n| Environment | Service | Version   | Status   | Owner |\n|-------------+---------+-----------+----------+-------|\n| development | api     | 2.6.0-dev | degraded | alice |\n\n* Global\n| Environment | Service | Version | Status  | Owner    |\n|-------------+---------+---------+---------+----------|\n| production  | api     |   2.4.0 | healthy | platform |\n") (org-mode t 1 172 "| Environment | Service | Version | Status  | Owner    |\n|-------------+---------+---------+---------+----------|\n| production  | api     |   2.4.0 | healthy | platform |\n") 1)"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_view_and_global_hooks_run_in_order_on_the_writable_org_buffer() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (annalist-define-view
             'deployments
             'audited
           (list
            :hooks
            (list
             (lambda ()
               (goto-char (point-max))
               (insert "VIEW-HOOK-ONE\n"))
             (lambda ()
               (goto-char (point-max))
               (insert "VIEW-HOOK-TWO\n")))
            'environment
            'service
            'version
            'status
            'owner)
           :inherit 'default)
         (annalist-record
          'operations
          'deployments
          '("canary" "api" "2.7.0" "observing" "sre"))
         (let ((annalist-describe-hook
                (list
                 (lambda ()
                   (goto-char (point-max))
                   (insert
                    (format
                     "GLOBAL-HOOK:%S:%S\n"
                     major-mode
                     buffer-read-only))))))
           (annalist-test-description
            'operations
            'deployments
            'audited)))"##;
    let expect = expect![[
        r#"OK (org-mode t 1 222 "| Environment | Service | Version | Status    | Owner |\n|-------------+---------+---------+-----------+-------|\n| canary      | api     |   2.7.0 | observing | sre   |\nVIEW-HOOK-ONE\nVIEW-HOOK-TWO\nGLOBAL-HOOK:org-mode:nil\n")"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}

#[test]
fn annalist_recording_policy_prevents_actual_store_mutations() {
    let elisp_form = r##"(progn
         (annalist-test-reset)
         (annalist-test-define-deployments)
         (let ((annalist-record nil))
           (annalist-record
            'blocked
            'deployments
            '("production" "api" "1" "healthy" "nobody")))
         (let ((annalist-record-whitelist
                '((allowed deployments))))
           (annalist-record
            'blocked
            'deployments
            '("production" "api" "2" "healthy" "nobody"))
           (annalist-record
            'allowed
            'deployments
            '("production" "api" "3" "healthy" "alice")))
         (let ((annalist-record-blacklist
                '((blocked deployments))))
           (annalist-record
            'blocked
            'deployments
            '("production" "api" "4" "healthy" "nobody"))
           (annalist-record
            'allowed
            'deployments
            '("staging" "worker" "5" "healthy" "bob")))
         (let ((tome (annalist--tome 'deployments)))
           (list
            (sort
             (annalist--hash-table-keys tome)
             (lambda (left right)
               (string<
                (symbol-name left)
                (symbol-name right))))
            (and (gethash 'blocked tome) :unexpected)
            (annalist--hash-table-values
             (gethash 'allowed tome))
            (annalist-test-description
             'allowed
             'deployments))))"##;
    let expect = expect![[
        r#"OK ((allowed) nil (("production" "api" "3" "healthy" "alice" nil) ("staging" "worker" "5" "healthy" "bob" nil)) (org-mode t 1 217 "| Environment | Service | Version | Status  | Owner |\n|-------------+---------+---------+---------+-------|\n| production  | api     |       3 | healthy | alice |\n| staging     | worker  |       5 | healthy | bob   |\n"))"#
    ]];

    assert_annalist_parity(elisp_form, expect);
}
