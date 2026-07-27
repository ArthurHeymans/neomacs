use expect_test::expect;

use super::assert_anaconda_mode_parity;

#[test]
fn host_selection_routes_local_docker_and_ssh_sessions_to_the_right_endpoint() {
    let elisp_form = r##"(let ((anaconda-mode-localhost-address "127.9.8.7")
      (scenario 'local))
  (cl-letf (((symbol-function 'pythonic-remote-docker-p)
             (lambda () (eq scenario 'docker)))
            ((symbol-function 'pythonic-remote-p)
             (lambda () (memq scenario '(docker ssh))))
            ((symbol-function 'pythonic-remote-host)
             (lambda () "python.example.test")))
    (mapcar
     (lambda (next)
       (setq scenario next)
       (list next (anaconda-mode-host)))
     '(local docker ssh))))"##;
    let expect =
        expect![[r#"OK ((local "127.9.8.7") (docker "127.9.8.7") (ssh "python.example.test"))"#]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn server_directory_and_local_working_directory_honor_the_configured_cache_root() {
    let elisp_form = r##"(let ((anaconda-mode-installation-directory "/workspace/anaconda-cache")
      (directory-exists nil)
      calls)
  (cl-letf (((symbol-function 'pythonic-local-p) (lambda () t))
            ((symbol-function 'file-directory-p)
             (lambda (path)
               (push (list 'file-directory-p path) calls)
               directory-exists))
            ((symbol-function 'make-directory)
             (lambda (path parents)
               (push (list 'make-directory path parents) calls)
               (setq directory-exists t))))
    (list
     (anaconda-mode-server-directory)
     (anaconda-mode-get-server-process-cwd)
     (anaconda-mode-get-server-process-cwd)
     (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("/workspace/anaconda-cache/0.1.17" "/workspace/anaconda-cache" "/workspace/anaconda-cache" ((file-directory-p "/workspace/anaconda-cache") (make-directory "/workspace/anaconda-cache" t) (file-directory-p "/workspace/anaconda-cache")))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn remote_working_directory_does_not_touch_the_local_filesystem() {
    let elisp_form = r##"(let ((anaconda-mode-installation-directory "/workspace/should-not-exist")
      calls)
  (cl-letf (((symbol-function 'pythonic-local-p) (lambda () nil))
            ((symbol-function 'file-directory-p)
             (lambda (&rest args) (push (cons 'file-directory-p args) calls)))
            ((symbol-function 'make-directory)
             (lambda (&rest args) (push (cons 'make-directory args) calls))))
    (list (anaconda-mode-get-server-process-cwd) (nreverse calls))))"##;
    let expect = expect!["OK (nil nil)"];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn process_health_port_binding_and_restart_decisions_track_every_runtime_identity_field() {
    let elisp_form = r##"(let ((anaconda-mode-process 'server)
      (anaconda-mode-socat-process 'socat)
      (anaconda-mode-ssh-process 'ssh)
      (python-shell-interpreter "python3.12")
      (python-shell-virtualenv-root "/venvs/project")
      (local t)
      (properties
       '((port . 43117)
         (interpreter . "python3.12")
         (virtualenv . "/venvs/project")
         (remote-p)
         (remote-method . "ssh")
         (remote-user . "alice")
         (remote-host . "compute.example")
         (remote-port . 2207))))
  (cl-letf (((symbol-function 'process-live-p)
             (lambda (process) (memq process '(server socat ssh))))
            ((symbol-function 'process-get)
             (lambda (_process key) (cdr (assq key properties))))
            ((symbol-function 'pythonic-local-p) (lambda () local))
            ((symbol-function 'pythonic-remote-p) (lambda () (not local)))
            ((symbol-function 'pythonic-remote-method) (lambda () "ssh"))
            ((symbol-function 'pythonic-remote-user) (lambda () "alice"))
            ((symbol-function 'pythonic-remote-host) (lambda () "compute.example"))
            ((symbol-function 'pythonic-remote-port) (lambda () 2207)))
    (let ((baseline
           (list
            (anaconda-mode-running-p)
            (anaconda-mode-socat-running-p)
            (anaconda-mode-ssh-running-p)
            (anaconda-mode-port)
            (anaconda-mode-bound-p)
            (anaconda-mode-need-restart))))
      (setcdr (assq 'interpreter properties) "python3.13")
      (let ((interpreter-change (anaconda-mode-need-restart)))
        (setcdr (assq 'interpreter properties) "python3.12")
        (setq local nil)
        (setcdr (assq 'remote-p properties) t)
        (let ((matching-remote (anaconda-mode-need-restart)))
          (setcdr (assq 'remote-host properties) "stale.example")
          (list baseline interpreter-change matching-remote
                (anaconda-mode-need-restart)))))))"##;
    let expect = expect!["OK (((server . #1=(socat . #2=(ssh))) #1# #2# 43117 t nil) t nil nil)"];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn local_server_command_uses_the_installed_python_artifact_and_configured_runtime_arguments() {
    let elisp_form = r##"(let ((anaconda-mode-installation-directory "/workspace/cache")
      (anaconda-mode-localhost-address "127.6.5.4")
      (python-shell-virtualenv-root "/workspace/venv"))
  (cl-letf (((symbol-function 'locate-library)
             (lambda (_library) "/opt/elpa/anaconda-mode/anaconda-mode.el"))
            ((symbol-function 'pythonic-remote-p) (lambda () nil)))
    (anaconda-mode-server-command-args)))"##;
    let expect = expect![[
        r#"OK ("/opt/elpa/anaconda-mode/anaconda-mode.py" "/workspace/cache/0.1.17" "127.6.5.4" "/workspace/venv")"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn remote_server_command_embeds_the_exact_installed_python_program_without_remote_file_io() {
    let elisp_form = r##"(let ((anaconda-mode-installation-directory "/workspace/cache")
      (python-shell-virtualenv-root nil))
  (cl-letf (((symbol-function 'pythonic-remote-p) (lambda () t)))
    (let* ((arguments (anaconda-mode-server-command-args))
           (program (cadr arguments)))
      (list
       (car arguments)
       (secure-hash 'sha256 program)
       (length program)
       (substring program 0 47)
       (last arguments 3)))))"##;
    let expect = expect![[
        r#"OK ("-c" "fc3c32bc90a567bc2007c5670e6d07d4f457f01dd75fa830ce66f143a02d4945" 6024 "from __future__ import print_function\nimport sy" ("/workspace/cache/0.1.17" "0.0.0.0" ""))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn bootstrap_passes_a_complete_process_spec_and_records_remote_connection_identity() {
    let elisp_form = r##"(let ((anaconda-mode-process nil)
      (anaconda-mode-process-name "test-anaconda")
      (anaconda-mode-process-buffer "*test-anaconda*")
      (python-shell-interpreter "python3.12")
      (python-shell-virtualenv-root "/venvs/api")
      start-arguments
      properties)
  (cl-letf (((symbol-function 'anaconda-mode-get-server-process-cwd)
             (lambda () nil))
            ((symbol-function 'anaconda-mode-server-command-args)
             (lambda () '("-c" "server()" "/cache/0.1.17" "0.0.0.0" "/venvs/api")))
            ((symbol-function 'get-buffer-create)
             (lambda (name) (list 'buffer name)))
            ((symbol-function 'pythonic-start-process)
             (lambda (&rest arguments)
               (setq start-arguments arguments)
               'server-process))
            ((symbol-function 'process-put)
             (lambda (process key value)
               (push (list process key value) properties)
               value))
            ((symbol-function 'pythonic-remote-p) (lambda () t))
            ((symbol-function 'pythonic-remote-method) (lambda () "ssh"))
            ((symbol-function 'pythonic-remote-user) (lambda () "deploy"))
            ((symbol-function 'pythonic-remote-host) (lambda () "api.example"))
            ((symbol-function 'pythonic-remote-port) (lambda () 2222)))
    (anaconda-mode-bootstrap (lambda () 'ready))
    (list
     anaconda-mode-process
     (let ((copy (copy-sequence start-arguments)))
       (plist-put copy :filter (functionp (plist-get copy :filter)))
       (plist-put copy :sentinel (functionp (plist-get copy :sentinel)))
       copy)
     (nreverse properties))))"##;
    let expect = expect![[
        r#"OK (server-process (:process "test-anaconda" :cwd nil :buffer (buffer "*test-anaconda*") :query-on-exit nil :filter t :sentinel t :args ("-c" "server()" "/cache/0.1.17" "0.0.0.0" "/venvs/api")) ((server-process interpreter "python3.12") (server-process virtualenv "/venvs/api") (server-process port nil) (server-process remote-p t) (server-process remote-method "ssh") (server-process remote-user "deploy") (server-process remote-host "api.example") (server-process remote-port 2222)))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn bootstrap_filter_extracts_ansi_wrapped_ports_once_and_invokes_the_ready_callback_once() {
    let elisp_form = r##"(let ((anaconda-mode-process 'server-process)
      (port nil)
      (callback-count 0)
      writes)
  (cl-letf (((symbol-function 'process-buffer) (lambda (_process) nil))
            ((symbol-function 'process-get)
             (lambda (_process key) (and (eq key 'port) port)))
            ((symbol-function 'process-put)
             (lambda (process key value)
               (push (list process key value) writes)
               (when (eq key 'port) (setq port value))))
            ((symbol-function 'pythonic-remote-docker-p) (lambda () nil))
            ((symbol-function 'pythonic-remote-ssh-p) (lambda () nil)))
    (anaconda-mode-bootstrap-filter
     'server-process
     "\e[32mservice starting\e[0m\nanaconda_mode port 45821\n"
     (lambda () (setq callback-count (1+ callback-count))))
    (anaconda-mode-bootstrap-filter
     'server-process
     "anaconda_mode port 59999\n"
     (lambda () (setq callback-count (1+ callback-count))))
    (list port callback-count (nreverse writes))))"##;
    let expect = expect!["OK (45821 1 ((server-process port 45821)))"];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn jump_proxy_string_preserves_every_tramp_hop_user_host_and_explicit_or_default_port() {
    let elisp_form = r##"(let ((scenario 'direct)
      (paths
       '((direct . "/ssh:deploy@api.example:/work/")
         (single-hop . "/ssh:alice@jump#2201|ssh:deploy@api.example:/work/")
         (two-hops . "/ssh:alice@jump#2201|ssh:relay@bastion|ssh:deploy@api.example:/work/"))))
  (cl-letf (((symbol-function 'pythonic-aliased-path)
             (lambda (_path) (cdr (assq scenario paths)))))
    (mapcar
     (lambda (next)
       (setq scenario next)
       (list next (anaconda-jump-proxy-string)))
     '(direct single-hop two-hops))))"##;
    let expect = expect![[
        r#"OK ((direct nil) (single-hop "-J alice@jump:2201") (two-hops "-J alice@jump:2201,relay@bastion:22"))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn docker_bootstrap_filter_maps_the_container_ip_to_a_local_socat_listener() {
    let elisp_form = r##"(let ((anaconda-mode-process 'server)
      (anaconda-mode-socat-process nil)
      (port nil)
      events)
  (cl-letf (((symbol-function 'process-buffer) (lambda (_process) nil))
            ((symbol-function 'process-get)
             (lambda (_process key) (and (eq key 'port) port)))
            ((symbol-function 'process-put)
             (lambda (_process key value)
               (when (eq key 'port) (setq port value))))
            ((symbol-function 'pythonic-remote-docker-p) (lambda () t))
            ((symbol-function 'pythonic-remote-ssh-p) (lambda () nil))
            ((symbol-function 'pythonic-remote-host) (lambda () "python-api"))
            ((symbol-function 'call-process)
             (lambda (&rest arguments)
               (push (cons 'inspect arguments) events)
               (insert
                "[{\"NetworkSettings\":{\"Networks\":{\"bridge\":{\"IPAddress\":\"172.18.0.7\"}}}}]")
               0))
            ((symbol-function 'start-process)
             (lambda (&rest arguments)
               (push (cons 'start arguments) events)
               'socat-process))
            ((symbol-function 'set-process-query-on-exit-flag)
             (lambda (process flag)
               (push (list 'query-on-exit process flag) events))))
    (anaconda-mode-bootstrap-filter
     'server
     "ready\nanaconda_mode port 49152\n")
    (list port anaconda-mode-socat-process (nreverse events))))"##;
    let expect = expect![[
        r#"OK (49152 socat-process ((inspect "docker" nil t nil "inspect" "python-api") (start "anaconda-socat" "*anaconda-socat*" "socat" "TCP4-LISTEN:49152" "TCP4:172.18.0.7:49152") (query-on-exit socat-process nil)))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn ssh_bootstrap_filter_builds_jump_and_direct_tunnel_process_specs_before_callback() {
    let elisp_form = r##"(let ((anaconda-mode-process 'server)
      (anaconda-mode-ssh-process nil)
      (anaconda-mode-tunnel-setup-sleep 2)
      (scenario 'jump)
      (port nil)
      events)
  (cl-letf (((symbol-function 'process-buffer) (lambda (_process) nil))
            ((symbol-function 'process-get)
             (lambda (_process key) (and (eq key 'port) port)))
            ((symbol-function 'process-put)
             (lambda (_process key value)
               (when (eq key 'port) (setq port value))))
            ((symbol-function 'pythonic-remote-docker-p) (lambda () nil))
            ((symbol-function 'pythonic-remote-ssh-p) (lambda () t))
            ((symbol-function 'anaconda-jump-proxy-string)
             (lambda ()
               (and (eq scenario 'jump)
                    "-J alice@jump:2201")))
            ((symbol-function 'pythonic-remote-user) (lambda () "deploy"))
            ((symbol-function 'pythonic-remote-host) (lambda () "api.example"))
            ((symbol-function 'pythonic-remote-port) (lambda () 2222))
            ((symbol-function 'message)
             (lambda (format-string &rest arguments)
               (push (list 'message
                           (apply #'format format-string arguments))
                     events)))
            ((symbol-function 'start-process)
             (lambda (&rest arguments)
               (push (cons 'start arguments) events)
               (intern (format "%s-process" scenario))))
            ((symbol-function 'sleep-for)
             (lambda (seconds)
               (push (list 'sleep seconds) events)))
            ((symbol-function 'set-process-query-on-exit-flag)
             (lambda (process flag)
               (push (list 'query-on-exit process flag) events))))
    (let (observations)
      (dolist (next '(jump direct))
        (setq scenario next
              port nil
              events nil)
        (anaconda-mode-bootstrap-filter
         'server
         "anaconda_mode port 45210\n"
         (lambda () (push '(callback) events)))
        (push (list next
                    port
                    anaconda-mode-ssh-process
                    (nreverse events))
              observations))
      (nreverse observations))))"##;
    let expect = expect![[
        r#"OK ((jump 45210 jump-process ((message "Anaconda Jump Proxy: -J alice@jump:2201") (start "anaconda-ssh" "*anaconda-ssh*" "ssh" "-J alice@jump:2201" "-nNT" "-L" "45210:localhost:45210" "deploy@api.example" "-p" "2222") (sleep 2) (query-on-exit jump-process nil) #1=(callback))) (direct 45210 direct-process ((message "Anaconda Jump Proxy: nil") (start "anaconda-ssh" "*anaconda-ssh*" "ssh" "-nNT" "-L" "45210:localhost:45210" "deploy@api.example" "-p" (number-to-string port)) (sleep 2) (query-on-exit direct-process nil) #1#)))"#
    ]];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn start_reuses_a_bound_server_but_restarts_stale_and_bootstraps_missing_servers() {
    let elisp_form = r##"(let ((scenario 'bound)
      events)
  (cl-letf (((symbol-function 'anaconda-mode-need-restart)
             (lambda () (eq scenario 'stale)))
            ((symbol-function 'anaconda-mode-stop)
             (lambda () (push 'stop events)))
            ((symbol-function 'anaconda-mode-running-p)
             (lambda () (memq scenario '(bound unbound))))
            ((symbol-function 'anaconda-mode-bound-p)
             (lambda () (eq scenario 'bound)))
            ((symbol-function 'anaconda-mode-bootstrap)
             (lambda (callback)
               (push 'bootstrap events)
               (when callback (funcall callback)))))
    (let (results)
      (dolist (next '(bound unbound stale missing))
        (setq scenario next events nil)
        (let ((callback-count 0))
          (anaconda-mode-start
           (lambda ()
             (setq callback-count (1+ callback-count))
             (push 'callback events)))
          (push (list next callback-count (nreverse events)) results)))
      (nreverse results))))"##;
    let expect = expect![
        "OK ((bound 1 (callback)) (unbound 0 nil) (stale 1 (stop bootstrap callback)) (missing 1 (bootstrap callback)))"
    ];
    assert_anaconda_mode_parity(elisp_form, expect);
}

#[test]
fn stop_detaches_filters_and_terminates_every_live_companion_process() {
    let elisp_form = r##"(let ((anaconda-mode-process 'server)
      (anaconda-mode-socat-process 'socat)
      (anaconda-mode-ssh-process 'ssh)
      events)
  (cl-letf (((symbol-function 'process-live-p) (lambda (_process) t))
            ((symbol-function 'set-process-filter)
             (lambda (process filter) (push (list 'filter process filter) events)))
            ((symbol-function 'set-process-sentinel)
             (lambda (process sentinel) (push (list 'sentinel process sentinel) events)))
            ((symbol-function 'kill-process)
             (lambda (process) (push (list 'kill process) events))))
    (anaconda-mode-stop)
    (list
     anaconda-mode-process
     anaconda-mode-socat-process
     anaconda-mode-ssh-process
     (nreverse events))))"##;
    let expect = expect![
        "OK (nil nil nil ((filter server nil) (sentinel server nil) (kill server) (kill socat) (kill ssh)))"
    ];
    assert_anaconda_mode_parity(elisp_form, expect);
}
