use expect_test::expect;

use super::assert_anakondo_parity;

#[test]
fn clj_kondo_analysis_builds_exact_commands_parses_json_and_cleans_process_buffer() {
    let elisp_form = r##"(let (commands)
                      (cl-letf
                          (((symbol-function
                             'call-shell-region)
                            (lambda
                                (_start
                                 _end
                                 command
                                 _delete
                                 destination
                                 &rest _)
                              (push command commands)
                              (with-current-buffer
                                  (get-buffer-create
                                   destination)
                                (erase-buffer)
                                (insert
                                 "{\"analysis\":{\"var-definitions\":[{\"ns\":\"app.core\",\"name\":\"run\",\"row\":7}],\"namespace-definitions\":[{\"name\":\"app.core\"}],\"namespace-usages\":[]}}"))
                              0)))
                        (let* ((project
                                (anakondo--clj-kondo-analyse-sync
                                 "src:vendor/lib.jar"
                                 nil))
                               (buffer
                                (anakondo--clj-kondo-analyse-sync
                                 "-"
                                 "cljs"))
                               (project-var
                                (car
                                 (gethash
                                  :var-definitions
                                  project)))
                               (buffer-ns
                                (car
                                 (gethash
                                  :namespace-definitions
                                  buffer))))
                          (list
                           (nreverse commands)
                           (list
                            (gethash :ns project-var)
                            (gethash :name project-var)
                            (gethash :row project-var))
                           (gethash :name buffer-ns)
                           (get-buffer "*anakondo*")))))"##;
    let expect = expect![[
        r#"OK (("clj-kondo --lint 'src:vendor/lib.jar' --config '{:output {:analysis true :format :json}}'" "clj-kondo --lint '-' --config '{:output {:analysis true :format :json}}' --lang 'cljs'") ("app.core" "run" 7) "app.core" nil)"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn clj_kondo_analysis_cleans_process_buffer_when_json_parsing_signals() {
    let elisp_form = r##"(cl-letf
                      (((symbol-function
                         'call-shell-region)
                        (lambda
                            (_start
                             _end
                             _command
                             _delete
                             destination
                             &rest _)
                          (with-current-buffer
                              (get-buffer-create destination)
                            (erase-buffer)
                            (insert "{not-json"))
                          1)))
                      (list
                       (condition-case error-data
                           (anakondo--clj-kondo-analyse-sync
                            "-" "clj")
                         (error
                          (list
                           (car error-data)
                           (cadr error-data))))
                       (get-buffer "*anakondo*")))"##;
    let expect = expect!["OK ((json-end-of-file nil) nil)"];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn project_path_uses_clojure_tools_deps_and_preserves_command_output_exactly() {
    let elisp_form = r##"(let (commands)
                      (cl-letf
                          (((symbol-function
                             'shell-command-to-string)
                            (lambda (command)
                              (push command commands)
                              "/workspace/src:/deps/lib.jar\n")))
                        (list
                         (anakondo--get-project-path)
                         (nreverse commands))))"##;
    let expect = expect![[r#"OK ("/workspace/src:/deps/lib.jar\n" ("clojure -Spath"))"#]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn project_analysis_runs_clojure_then_java_and_emits_complete_progress_messages() {
    let elisp_form = r##"(let ((var-cache 'vars)
                          (ns-cache 'namespaces)
                          (usage-cache 'usages)
                          (java-cache 'java)
                          events)
                      (cl-letf
                          (((symbol-function
                             'anakondo--clj-kondo-project-analyse-sync)
                            (lambda (&rest caches)
                              (push
                               (cons 'clojure caches)
                               events)
                              'project-root))
                           ((symbol-function
                             'anakondo--java-project-analyse-sync)
                            (lambda (cache)
                              (push
                               (list 'java cache)
                               events)
                              nil))
                           ((symbol-function 'message)
                            (lambda
                                (format-string
                                 &rest arguments)
                              (push
                               (cons
                                'message
                                (apply
                                 #'format
                                 format-string
                                 arguments))
                               events)
                              nil)))
                        (list
                         (anakondo--project-analyse-sync
                          var-cache
                          ns-cache
                          usage-cache
                          java-cache)
                         (nreverse events))))"##;
    let expect = expect![[
        r#"OK (nil ((message . "Analysing project for completion...") (clojure vars namespaces usages) (java java) (message . "Analysing project for completion...done")))"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}
