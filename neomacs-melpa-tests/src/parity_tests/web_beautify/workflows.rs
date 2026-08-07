use expect_test::expect;

use super::ParityBatchCase;

fn programs_commands_and_args_are_registered() -> ParityBatchCase {
    ParityBatchCase::value(
        "programs_commands_and_args_are_registered",
        r####"
(list :html-program web-beautify-html-program
      :css-program web-beautify-css-program
      :js-program web-beautify-js-program
      :args web-beautify-args
      :html (commandp 'web-beautify-html)
      :css (commandp 'web-beautify-css)
      :js (commandp 'web-beautify-js)
      :html-buffer (fboundp 'web-beautify-html-buffer)
      :css-buffer (fboundp 'web-beautify-css-buffer)
      :js-buffer (fboundp 'web-beautify-js-buffer)
      :format-region (fboundp 'web-beautify-format-region)
      :format-buffer (fboundp 'web-beautify-format-buffer)
      :feature (featurep 'web-beautify))
"####,
        expect![[
            r#"OK (:html-program "html-beautify" :css-program "css-beautify" :js-program "js-beautify" :args ("-f" "-") :html t :css t :js t :html-buffer t :css-buffer t :js-buffer t :format-region t :format-buffer t :feature t)"#
        ]],
    )
}

fn shell_command_and_messages_are_deterministic() -> ParityBatchCase {
    ParityBatchCase::value(
        "shell_command_and_messages_are_deterministic",
        r####"
(list :shell (web-beautify-get-shell-command "js-beautify")
      :shell-custom
      (let ((web-beautify-args '("-s" "2" "-f" "-")))
        (web-beautify-get-shell-command "html-beautify"))
      :missing (web-beautify-command-not-found-message "js-beautify")
      :error (web-beautify-format-error-message "*Web Beautify Errors*"))
"####,
        expect![[
            r#"OK (:shell "js-beautify -f -" :shell-custom "html-beautify -s 2 -f -" :missing "js-beautify not found. Install it with `npm -g install js-beautify`." :error "Could not apply web-beautify. See *Web Beautify Errors* to for details.")"#
        ]],
    )
}

fn missing_program_path_messages_without_mutating_buffer() -> ParityBatchCase {
    ParityBatchCase::value(
        "missing_program_path_messages_without_mutating_buffer",
        r####"
(with-temp-buffer
  (insert "function x(){return 1}")
  (let ((before (buffer-string))
        (messages nil)
        (web-beautify-js-program "neomacs-web-beautify-missing-binary"))
    (cl-letf (((symbol-function 'message)
               (lambda (fmt &rest args)
                 (push (apply #'format fmt args) messages)
                 nil))
              ((symbol-function 'executable-find)
               (lambda (_program) nil)))
      (web-beautify-js-buffer)
      (list :buffer (buffer-string)
            :unchanged (equal before (buffer-string))
            :messages (nreverse messages)))))
"####,
        expect![[
            r#"OK (:buffer "function x(){return 1}" :unchanged t :messages ("neomacs-web-beautify-missing-binary not found. Install it with `npm -g install js-beautify`."))"#
        ]],
    )
}

fn region_command_dispatches_to_region_or_buffer() -> ParityBatchCase {
    ParityBatchCase::value(
        "region_command_dispatches_to_region_or_buffer",
        r####"
(let (calls)
  (cl-letf (((symbol-function 'web-beautify-format-region)
             (lambda (program beg end)
               (push (list :region program beg end) calls)))
            ((symbol-function 'web-beautify-format-buffer)
             (lambda (program)
               (push (list :buffer program) calls)))
            ((symbol-function 'use-region-p)
             (lambda () nil)))
    (web-beautify-js)
    (web-beautify-html)
    (web-beautify-css)
    (cl-letf (((symbol-function 'use-region-p) (lambda () t))
              ((symbol-function 'region-beginning) (lambda () 2))
              ((symbol-function 'region-end) (lambda () 9)))
      (web-beautify-js)
      (web-beautify-html)
      (web-beautify-css)))
  (nreverse calls))
"####,
        expect![[
            r#"OK ((:buffer "js-beautify") (:buffer "html-beautify") (:buffer "css-beautify") (:region "js-beautify" 2 9) (:region "html-beautify" 2 9) (:region "css-beautify" 2 9))"#
        ]],
    )
}

fn buffer_helpers_forward_configured_programs() -> ParityBatchCase {
    ParityBatchCase::value(
        "buffer_helpers_forward_configured_programs",
        r####"
(let (calls
      (web-beautify-js-program "custom-js")
      (web-beautify-html-program "custom-html")
      (web-beautify-css-program "custom-css"))
  (cl-letf (((symbol-function 'web-beautify-format-buffer)
             (lambda (program)
               (push program calls))))
    (web-beautify-js-buffer)
    (web-beautify-html-buffer)
    (web-beautify-css-buffer)
    (nreverse calls)))
"####,
        expect![[r#"OK ("custom-js" "custom-html" "custom-css")"#]],
    )
}

pub(super) fn workflow_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        programs_commands_and_args_are_registered(),
        shell_command_and_messages_are_deterministic(),
        missing_program_path_messages_without_mutating_buffer(),
        region_command_dispatches_to_region_or_buffer(),
        buffer_helpers_forward_configured_programs(),
    ]
}
