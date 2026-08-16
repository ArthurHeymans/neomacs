use expect_test::expect;

use super::ParityBatchCase;

/// Loading the package registers the backend command and its documented
/// configuration surface: the five defcustoms with defaults and types, the
/// customization group, and the backend's elisp entry points.
fn loading_registers_the_backend_and_its_configuration() -> ParityBatchCase {
    ParityBatchCase::value(
        "loading_registers_the_backend_and_its_configuration",
        r####"(list
 :source (cgo319-test-source-state)
 :entry-points
 (list :backend-command (commandp 'company-go)
       :prefix (fboundp 'company-go--prefix)
       :candidates (fboundp 'company-go--candidates)
       :annotation (fboundp 'company-go--annotation)
       :meta (fboundp 'company-go--meta)
       :location (fboundp 'company-go--location)
       :doc-buffer (fboundp 'company-go--doc-buffer))
 :options
 (mapcar
  (lambda (option)
    (list :option option
          :custom-variable-p (and (custom-variable-p option) t)
          :standard (eval (car (get option 'standard-value)))
          :type (get option 'custom-type)))
  '(company-go-show-annotation
    company-go-begin-after-member-access
    company-go-insert-arguments
    company-go-gocode-command
    company-go-gocode-args)))"####,
        expect![[r##"TODO"##]],
    )
}

/// The pure candidate pipeline: `company-go--format-meta' strips the func
/// marker and keeps other type prefixes, and `company-go--get-candidates'
/// propertizes each CSV row with its meta and package.
fn the_csv_candidate_pipeline_formats_meta_and_packages() -> ParityBatchCase {
    ParityBatchCase::value(
        "the_csv_candidate_pipeline_formats_meta_and_packages",
        r####"(let ((rows '(("func" "Println" "func(a ...interface{})" "fmt")
                     ("func" "Sprint" "func(a ...interface{}) string" "fmt")
                     ("package" "fmt" "package" "")
                     ("const" "Pi" "float64" "math"))))
  (list
   :format-meta
   (mapcar (lambda (row)
             (list :raw (nth 2 row)
                   :formatted (company-go--format-meta row)))
           rows)
   :candidates
   (mapcar (lambda (cand)
             (list :text (substring-no-properties cand)
                   :meta (get-text-property 0 'meta cand)
                   :package (get-text-property 0 'package cand)))
           (company-go--get-candidates
            '("func,,Println,,func(a ...interface{}),,fmt"
              "const,,Pi,,float64,,math"
              "package,,fmt,,package,,")))))"####,
        expect![[r##"TODO"##]],
    )
}

/// The invocation contract through a fake gocode: the real arg assembly
/// passes the extra args, the csv-with-package formatter, the buffer file
/// name, and the c<offset> cursor position, and the canned CSV answer
/// flows through `company-go--candidates' as propertized candidates.
fn the_invocation_contract_through_a_fake_gocode() -> ParityBatchCase {
    ParityBatchCase::value(
        "the_invocation_contract_through_a_fake_gocode",
        r####"(unwind-protect
    (progn
      (cgo319-test-reset)
      (let* ((root (cgo319-test-root))
             (file (expand-file-name "main.go" root))
             (script (cgo319-test-fake-gocode
                      root
                      "func,,Println,,func(a ...interface{}),,fmt")))
        (let ((coding-system-for-write 'utf-8-unix))
          (with-temp-file file (insert "package main\n\nfunc main() {\n\tfmt.P\n}\n")))
        (let ((buffer (find-file-noselect file)))
          (with-current-buffer buffer
            (goto-char (point-min))
            (search-forward "fmt.P")
            (setq company-go-gocode-command script
                  company-go-gocode-args '("-s"))
            (let ((candidates (company-go--candidates)))
              (list
               :argv
               (with-temp-buffer
                 (insert-file-contents (expand-file-name "argv.txt" root))
                 (buffer-substring-no-properties (point-min) (point-max)))
               :candidates
               (mapcar (lambda (cand)
                         (list :text (substring-no-properties cand)
                               :meta (get-text-property 0 'meta cand)
                               :package (get-text-property 0 'package cand)))
                       candidates)
               :offset-arg-passed
               (string-match-p "c[0-9]+"
                               (with-temp-buffer
                                 (insert-file-contents
                                  (expand-file-name "argv.txt" root))
                                 (buffer-string)))))))))
  (cgo319-test-reset))"####,
        expect![[r##"TODO"##]],
    )
}

/// The prefix contract: with `company-go-begin-after-member-access' the
/// prefix after a member dot is grabbed (returning the symbol and the
/// trailing dot marker), and with it off the plain symbol is grabbed.
fn the_prefix_contract_at_member_access_dots() -> ParityBatchCase {
    ParityBatchCase::value(
        "the_prefix_contract_at_member_access_dots",
        r####"(unwind-protect
    (progn
      (cgo319-test-reset)
      (with-temp-buffer
        (goto-char (point-min))
        (insert "package main\n\nfunc main() {\n\tfmt.Print\n}\n")
        (goto-char (point-min))
        (search-forward "fmt.P")
        (let ((at-symbol (company-go--prefix)))
          (goto-char (point-min))
          (search-forward "fmt.")
          (let ((at-dot-begin (company-go--prefix)))
            (setq company-go-begin-after-member-access nil)
            (goto-char (point-min))
            (search-forward "fmt.P")
            (let ((at-symbol-plain (company-go--prefix)))
              (list :at-symbol at-symbol
                    :at-dot-begin at-dot-begin
                    :at-symbol-plain at-symbol-plain))))))
  (cgo319-test-reset))"####,
        expect![[r##"TODO"##]],
    )
}

pub(super) fn workflows_public_surface_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        loading_registers_the_backend_and_its_configuration(),
        the_csv_candidate_pipeline_formats_meta_and_packages(),
        the_invocation_contract_through_a_fake_gocode(),
        the_prefix_contract_at_member_access_dots(),
    ]
}
