use expect_test::expect;

use super::assert_ant_parity;

#[test]
fn ant_explicit_task_compiles_in_discovered_root_and_updates_last_task() {
    let elisp_form = r##"(let ((ant-command "ant -emacs")
               (ant-build-file-name "build.xml")
               (ant-last-task "old")
               calls)
         (cl-letf (((symbol-function 'ant-find-root)
                    (lambda (&rest args)
                      (push (list 'root args default-directory) calls)
                      "/workspace/project/"))
                   ((symbol-function 'compile)
                    (lambda (&rest args)
                      (push (list 'compile args default-directory) calls)
                      'compilation-buffer)))
           (list (ant "clean test")
                 ant-last-task
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (compilation-buffer "clean test" ((root ("build.xml") "[ORACLE-SANDBOX]/") (compile ("ant -emacs clean test") "/workspace/project/")))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_without_task_reads_completion_inside_project_root() {
    let elisp_form = r##"(let ((ant-command "/opt/ant -emacs")
               (ant-last-task "compile")
               calls)
         (cl-letf (((symbol-function 'ant-find-root)
                    (lambda (indicator)
                      (push (list 'root indicator default-directory) calls)
                      "/repo/"))
                   ((symbol-function 'ant-get-task)
                    (lambda (directory)
                      (push (list 'task directory default-directory) calls)
                      "verify package"))
                   ((symbol-function 'compile)
                    (lambda (command)
                      (push (list 'compile command default-directory) calls)
                      'started)))
           (list (ant)
                 ant-last-task
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (started "verify package" ((root "build.xml" "[ORACLE-SANDBOX]/") (task "/repo/" "/repo/") (compile "/opt/ant -emacs verify package" "/repo/")))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_without_project_reports_message_and_skips_task_and_compile() {
    let elisp_form = r##"(let (calls messages)
         (cl-letf (((symbol-function 'ant-find-root)
                    (lambda (&rest args)
                      (push (cons 'root args) calls)
                      nil))
                   ((symbol-function 'ant-get-task)
                    (lambda (&rest args)
                      (push (cons 'task args) calls)
                      "compile"))
                   ((symbol-function 'compile)
                    (lambda (&rest args)
                      (push (cons 'compile args) calls)))
                   ((symbol-function 'message)
                    (lambda (&rest args)
                      (push args messages))))
           (list (ant)
                 (nreverse calls)
                 (nreverse messages))))"##;
    let expect =
        expect![[r#"OK (#1=(("Couldn't find an ant project.")) ((root "build.xml")) #1#)"#]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_empty_task_preserves_trailing_command_space_and_updates_state() {
    let elisp_form = r##"(let ((ant-command "ant -emacs")
               (ant-last-task "compile")
               calls)
         (cl-letf (((symbol-function 'ant-find-root)
                    (lambda (&rest _) "/repo/"))
                   ((symbol-function 'compile)
                    (lambda (command)
                      (push command calls)
                      'done)))
           (list (ant "")
                 ant-last-task
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK (done "" ("ant -emacs "))"#]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_last_forwards_current_task_and_falls_back_to_empty_string() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ant)
                    (lambda (&rest args)
                      (push args calls)
                      (car args))))
           (let ((ant-last-task "deploy"))
             (ant-last))
           (let ((ant-last-task nil))
             (ant-last))
           (list (nreverse calls))))"##;
    let expect = expect![[r#"OK ((("deploy") ("")))"#]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_compile_clean_and_test_forward_exact_standard_tasks() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ant)
                    (lambda (&rest args)
                      (push args calls)
                      (car args))))
           (list (ant-compile)
                 (ant-clean)
                 (ant-test)
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK ("compile" "clean" "test" (("compile") ("clean") ("test")))"#]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_custom_build_name_and_command_drive_practical_compile_workflow() {
    let elisp_form = r##"(let ((ant-build-file-name "project.xml")
               (ant-command "java -jar /tools/ant.jar -emacs -f project.xml")
               calls)
         (cl-letf (((symbol-function 'ant-find-root)
                    (lambda (indicator)
                      (push (list 'root indicator) calls)
                      "/workspace/service/"))
                   ((symbol-function 'compile)
                    (lambda (command)
                      (push (list 'compile command default-directory) calls)
                      'buffer)))
           (list (ant "integration-test")
                 ant-last-task
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (buffer "integration-test" ((root "project.xml") (compile "java -jar /tools/ant.jar -emacs -f project.xml integration-test" "/workspace/service/")))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}
