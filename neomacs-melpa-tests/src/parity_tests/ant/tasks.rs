use expect_test::expect;

use super::assert_ant_parity;

#[test]
fn ant_find_tasks_builds_exact_shell_command_and_parses_realistic_targets() {
    let elisp_form = r##"(let ((ant-build-file-name "build.xml")
               (*ant-tasks-command* "fixture-grep")
               calls messages)
         (cl-letf (((symbol-function 'shell-command-to-string)
                    (lambda (command)
                      (push command calls)
                      "<target name=\"compile\" description=\"Compile\"/>\n<target depends=\"compile\" name=\"package\"/>\n<target name=\"test.integration\"/>\n"))
                   ((symbol-function 'message)
                    (lambda (&rest args)
                      (push args messages))))
           (list (ant-find-tasks "/workspace/project")
                 (nreverse calls)
                 (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (("compile" "package" "test.integration" "") ("fixture-grep /workspace/project/build.xml") (("<target name=\"compile\" description=\"Compile\"/>\n<target depends=\"compile\" name=\"package\"/>\n<target name=\"test.integration\"/>\n")))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_find_tasks_returns_nil_for_empty_command_output() {
    let elisp_form = r##"(let ((ant-build-file-name "custom.xml")
               (*ant-tasks-command* "scan")
               calls)
         (cl-letf (((symbol-function 'shell-command-to-string)
                    (lambda (command)
                      (push command calls)
                      "")))
           (list (ant-find-tasks "/project/")
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK (nil ("scan /project//custom.xml"))"#]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_find_tasks_preserves_parser_edge_cases_and_trailing_entry() {
    let elisp_form = r##"(let (messages)
         (cl-letf (((symbol-function 'shell-command-to-string)
                    (lambda (&rest _)
                      "<target name=\"-private\"/>\n<target name=\"clean-all\"/>\nnoise\n<target name=\"a b\"/>\n"))
                   ((symbol-function 'message)
                    (lambda (&rest args) (push args messages))))
           (list (ant-find-tasks ".")
                 (nreverse messages))))"##;
    let expect = expect![[
        r#"OK (("<target name=\"-private\"/>" "clean-all" "noise" "a b" "") (("<target name=\"-private\"/>\n<target name=\"clean-all\"/>\nnoise\n<target name=\"a b\"/>\n")))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_tasks_cache_miss_exposes_first_result_shape_then_reuses_full_cache() {
    let elisp_form = r##"(let ((*ant-tasks-cache* nil)
               calls)
         (cl-letf (((symbol-function 'ant-find-tasks)
                    (lambda (directory)
                      (push directory calls)
                      '("compile" "test" "package"))))
           (let ((first (ant-tasks "/repo/"))
                 (cache-after-first *ant-tasks-cache*))
             (let ((second (ant-tasks "/repo/")))
               (list first second cache-after-first
                     *ant-tasks-cache* (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (#1=("test" "package") #2=("compile" . #1#) #3=(("/repo/" . #2#)) #3# ("/repo/"))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_tasks_falls_back_to_defaults_and_caches_independent_directories() {
    let elisp_form = r##"(let ((*ant-tasks-cache* nil)
               (ant-tasks-default '("compile" "verify" "clean"))
               calls)
         (cl-letf (((symbol-function 'ant-find-tasks)
                    (lambda (directory)
                      (push directory calls)
                      nil)))
           (let ((first (ant-tasks "/one/"))
                 (second (ant-tasks "/two/"))
                 (one-again (ant-tasks "/one/")))
             (list first second one-again
                   *ant-tasks-cache* (nreverse calls)))))"##;
    let expect = expect![[
        r#"OK (#1=("verify" "clean") #1# #2=("compile" . #1#) (("/two/" . #2#) ("/one/" . #2#)) ("/one/" "/two/"))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_get_task_joins_multiple_selections_and_preserves_completion_arguments() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ant-tasks)
                    (lambda (directory)
                      (push (list 'tasks directory) calls)
                      '("compile" "test" "clean")))
                   ((symbol-function 'completing-read-multiple)
                    (lambda (&rest args)
                      (push (cons 'read args) calls)
                      '("clean" "compile" "test"))))
           (list (ant-get-task "/repo/")
                 (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("clean compile test" ((tasks "/repo/") (read "Task (default): " ("compile" "test" "clean"))))"#
    ]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_get_task_empty_selection_returns_empty_command_suffix() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function 'ant-tasks)
                    (lambda (&rest args)
                      (push (cons 'tasks args) calls)
                      '("compile")))
                   ((symbol-function 'completing-read-multiple)
                    (lambda (&rest args)
                      (push (cons 'read args) calls)
                      nil)))
           (list (ant-get-task "/repo/")
                 (nreverse calls))))"##;
    let expect = expect![[r#"OK ("" ((tasks "/repo/") (read "Task (default): " ("compile"))))"#]];
    assert_ant_parity(elisp_form, expect);
}

#[test]
fn ant_kill_cache_replaces_nonempty_cache_with_fresh_empty_list() {
    let elisp_form = r##"(let ((*ant-tasks-cache*
                '(("/one/" "compile") ("/two/" "test"))))
         (let ((before *ant-tasks-cache*)
               (result (ant-kill-cache)))
           (list result
                 *ant-tasks-cache*
                 (eq before *ant-tasks-cache*))))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_ant_parity(elisp_form, expect);
}
