use expect_test::expect;

use super::assert_apdl_mode_parity;

#[test]
fn alignment_turns_real_parameter_definitions_into_a_readable_engineering_table() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apdl-mode-hook nil)
        (apdl-dynamic-highlighting-flag nil))
    (apdl-mode)
    (insert
     "youngs=210000 ! MPa\n"
     "poisson_ratio =0.3 ! dimensionless\n"
     "density= 7.85e-9 ! tonne/mm3\n")
    (goto-char (point-min))
    (push-mark (point-max) nil t)
    (setq mark-active t)
    (apdl-align (region-beginning) (region-end))
    (buffer-string)))"##;
    let expect = expect![[
        r#"OK "youngs\11      =\011210000\11     ! MPa\npoisson_ratio =\11     0.3     ! dimensionless\ndensity\11      =\11     7.85e-9 ! tonne/mm3\n""#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn number_block_hiding_creates_precise_overlays_and_unhiding_removes_them() {
    let elisp_form = r##"(with-temp-buffer
  (let ((apdl-mode-hook nil)
        (apdl-dynamic-highlighting-flag nil))
    (apdl-mode)
    (insert
     "nblock,3,solid\n"
     "(1i9,3e20.9e3)\n"
     "1,0.0,0.0,0.0\n"
     "2,1.0,0.0,0.0\n"
     "3,1.0,1.0,0.0\n"
     "4,0.0,1.0,0.0\n"
     "5,0.0,0.0,1.0\n"
     "6,1.0,0.0,1.0\n"
     "7,1.0,1.0,1.0\n"
     "-1\n"
     "type,1\n")
    (goto-char (point-min))
    (forward-line 1)
    (let ((original-point (point)))
      (apdl-hide-number-blocks)
      (let ((hidden
             (mapcar
              (lambda (overlay)
                (list
                 (line-number-at-pos (overlay-start overlay))
                 (line-number-at-pos (overlay-end overlay))
                 (overlay-get overlay 'invisible)
                 (overlay-get overlay 'intangible)
                 (substring-no-properties
                  (overlay-get overlay 'before-string))
                 (substring-no-properties
                  (overlay-get overlay 'after-string))))
              apdl-hide-region-overlays)))
        (apdl-unhide-number-blocks)
        (list
         (= original-point (point))
         hidden
         apdl-hide-region-overlays
         (overlays-in (point-min) (point-max)))))))"##;
    let expect = expect![[
        r#"OK (t ((4 8 t t "![ ... hidden" " region ... ]")) nil (#<overlay in no buffer>))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn close_block_inserts_the_matching_case_preserving_end_keyword_and_indents_it() {
    let elisp_form = r##"(mapcar
 (lambda (program)
   (with-temp-buffer
     (let ((apdl-mode-hook nil)
           (apdl-dynamic-highlighting-flag nil)
           (apdl-blink-matching-block-flag nil))
       (apdl-mode)
       (insert program)
       (goto-char (point-max))
       (list (apdl-close-block) (buffer-string)
             (line-number-at-pos) (current-column)))))
 '("*if,active,eq,1,then\n  solve\n"
   "*DO,index,1,3\n  solve\n"
   "*create,worker,mac\n  /com,body\n"))"##;
    let expect = expect![[
        r#"OK ((t "*if,active,eq,1,then\n  solve\n*endif" 3 6) (t "*DO,index,1,3\n  solve\n*ENDDO" 3 6) (t "*create,worker,mac\n  /com,body\n*end" 3 4))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn insert_pi_and_do_skeleton_build_executable_apdl_fragments_with_stable_point_placement() {
    let elisp_form = r##"(list
 (with-temp-buffer
   (let ((apdl-mode-hook nil)
         (apdl-dynamic-highlighting-flag nil))
     (apdl-mode)
     (insert "*if,active,eq,1,then\n")
     (apdl-insert-pi)
     (list (buffer-string) (line-number-at-pos) (current-column))))
 (with-temp-buffer
   (let ((apdl-mode-hook nil)
         (apdl-dynamic-highlighting-flag nil)
         (answers '("1" "load_steps" "1")))
     (apdl-mode)
     (cl-letf (((symbol-function 'read-string)
                (lambda (&rest _arguments) (pop answers))))
       (apdl-do "step")
       (list (buffer-string) (line-number-at-pos) (current-column))))))"##;
    let expect = expect![[
        r#"OK (("*if,active,eq,1,then\n  Pi = acos(-1) ! 3.14159265359\n  " 3 2) ("*do,step,1,load_steps,1\n  \n*enddo" 2 2))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn abort_file_workflow_derives_jobname_from_model_and_confirms_without_touching_disk() {
    let elisp_form = r##"(with-temp-buffer
  (insert "/prep7\n/filname,production_beam\nsolve\n")
  (let ((default-directory "/workspace/run/")
        (apdl-job "fallback")
        events)
    (cl-letf
        (((symbol-function 'yes-or-no-p)
          (lambda (prompt)
            (push (list 'confirm prompt) events)
            t))
         ((symbol-function 'apdl-write-abort-file)
          (lambda (filename)
            (push (list 'write filename default-directory) events)))
         ((symbol-function 'message)
          (lambda (format-string &rest arguments)
            (let ((text (apply #'format format-string arguments)))
              (push (list 'message text) events)
              text))))
      (list (apdl-abort-file 1) (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("Wrote MAPDL stop file production.abt in /workspace/run/." ((confirm "Write stop file \"/workspace/run/production.abt\"? ") (write "production.abt" "/workspace/run/") (message "Wrote MAPDL stop file production.abt in /workspace/run/.")))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}

#[test]
fn installation_discovery_checks_the_newest_ansys_environment_root_and_rejects_it_if_unreadable() {
    let elisp_form = r##"(let ((process-environment
       '("PATH=/usr/bin"
         "AWP_ROOT201=/opt/ansys/v201"
         "AWP_ROOT232=/opt/ansys/v232"
         "AWP_ROOT251=/opt/ansys/v251"
         "OTHER=value"))
      events)
  (cl-letf
      (((symbol-function 'file-readable-p)
        (lambda (path)
          (push (list 'readable path) events)
          (not (string-suffix-p "v251" path))))
       ((symbol-function 'message)
        (lambda (format-string &rest arguments)
          (let ((text (apply #'format format-string arguments)))
            (push (list 'message text) events)
            text))))
    (list (apdl-find-path-environment-value)
          (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil ((readable "/opt/ansys/v251") (message "Environment AWP_ROOTXXX set but value is not readable")))"#
    ]];
    assert_apdl_mode_parity(elisp_form, expect);
}
