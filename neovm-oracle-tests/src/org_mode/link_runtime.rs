use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_custom_link_follow_export_store_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (let (follow-calls)
    (org-link-set-parameters
     "probe"
     :follow (lambda (path arg)
               (push (list path arg) follow-calls))
     :export (lambda (path desc backend _info)
               (format "[%s:%s:%s]" backend path (or desc "")))
     :store (lambda ()
              (org-link-store-props
               :type "probe"
               :link "probe:stored"
               :description "Stored Probe")
              t))
    (with-temp-buffer
      (org-mode)
      (insert "[[probe:abc%20def][Desc]]\n")
      (goto-char (point-min))
      (let ((link (org-element-context)))
        (org-link-open link '(4))
        (let ((html (org-export-string-as
                     "[[probe:abc][Desc]]" 'html t))
              (ascii (org-export-string-as
                      "[[probe:abc]]" 'ascii t))
              (org-stored-links nil)
              (org-store-link-plist nil))
          (org-store-link nil nil)
          (list (nreverse follow-calls)
                html
                ascii
                org-store-link-plist
                org-stored-links))))))"##,
    );
}

#[test]
fn org_custom_link_activation_completion_export_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (let ((follow-calls nil)
        (store-calls nil)
        (complete-calls nil)
        (activate-calls nil)
        (org-stored-links nil)
        (org-store-link-plist nil))
    (org-link-set-parameters
     "combo"
     :follow (lambda (path arg)
               (push (list path arg) follow-calls))
     :export (lambda (path desc backend info)
               (format "{%s|%s|%s|toc=%S}"
                       backend path (or desc "")
                       (plist-get info :with-toc)))
     :store (lambda ()
              (push 'store store-calls)
              (org-link-store-props
               :type "combo"
               :link "combo:stored/value"
               :description "Stored combo")
              t)
     :complete (lambda (&optional arg)
                 (push arg complete-calls)
                 "combo:completed/path")
     :face (lambda (path) (if (string-match-p "warn" path)
                              'org-warning
                            'org-link))
     :display "COMBO"
     :activate-func (lambda (start end path bracketp)
                      (push (list (- start (point-min))
                                  (- end (point-min))
                                  path bracketp)
                            activate-calls)
                      (put-text-property start end 'combo-path path)))
    (with-temp-buffer
      (org-mode)
      (insert "#+OPTIONS: toc:nil\n")
      (insert "[[combo:ok/path][Okay]] [[combo:warn/path]]\n")
      (font-lock-ensure (point-min) (point-max))
      (let ((props
             (mapcar
              (lambda (needle)
                (save-excursion
                  (goto-char (point-min))
                  (search-forward needle)
                  (list needle
                        (get-text-property (match-beginning 0) 'face)
                        (get-text-property (match-beginning 0) 'display)
                        (get-text-property (match-beginning 0) 'combo-path)
                        (get-text-property (match-beginning 0)
                                           'mouse-face)
                        (keymapp (get-text-property
                                  (match-beginning 0) 'keymap)))))
              '("Okay" "combo:warn"))))
        (goto-char (point-min))
        (search-forward "Okay")
        (org-open-at-point '(4))
        (let ((completion (org-link-complete-file '(4)))
              (html (org-export-string-as
                     "[[combo:ok/path][Okay]]" 'html t
                     '(:with-toc nil)))
              (ascii (org-export-string-as
                      "[[combo:warn/path]]" 'ascii t
                      '(:with-toc nil)))
              (org-out (org-export-string-as
                        "[[combo:ok/path][Okay]]" 'org t
                        '(:with-toc nil))))
          (org-store-link nil nil)
          (list props
                (nreverse activate-calls)
                (nreverse follow-calls)
                completion
                (nreverse complete-calls)
                html
                ascii
                org-out
                (nreverse store-calls)
                org-store-link-plist
                org-stored-links
                (buffer-substring-no-properties
                 (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_link_escape_decode_make_string_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ol)
  (let* ((raw "file:has space/ümlaut?#x")
         (escaped (org-link-escape raw))
         (unescaped (org-link-unescape escaped))
         (encoded (org-link-encode "a b/ç" '(?\s ?/ ?ç)))
         (decoded (org-link-decode encoded)))
    (list escaped
          unescaped
          encoded
          decoded
          (org-link-make-string "https://example.org/a b" "Example")
          (org-link-make-string "https://example.org/a b"))))"##,
    );
}

#[test]
fn org_link_store_props_mail_date_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ol)
  (let ((org-link-from-user-regexp "me@example\\.org")
        org-store-link-plist)
    (org-link-store-props
     :type "mail"
     :from "Me <me@example.org>"
     :to "Ada <ada@example.org>"
     :date "Wed, 27 May 2026 09:30:00 +0000"
     :subject "Hello")
    (org-link-add-props :link "mailto:ada@example.org" :description "Hello")
    (list (plist-get org-store-link-plist :fromname)
          (plist-get org-store-link-plist :fromaddress)
          (plist-get org-store-link-plist :toname)
          (plist-get org-store-link-plist :toaddress)
          (plist-get org-store-link-plist :fromto)
          (plist-get org-store-link-plist :date-timestamp)
          (plist-get org-store-link-plist :date-timestamp-inactive)
          (plist-get org-store-link-plist :link)
          (plist-get org-store-link-plist :description))))"##,
    );
}

#[test]
fn org_link_navigation_toggle_context_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (with-temp-buffer
    (org-mode)
    (insert "* Links\n")
    (insert "[[https://example.org/a][Alpha]] plain https://example.org/b\n")
    (insert "[[file:/tmp/demo.txt::12][File line]] and [[#target][Target]]\n")
    (insert "* Target\n:PROPERTIES:\n:CUSTOM_ID: target\n:END:\n")
    (font-lock-ensure (point-min) (point-max))
    (let ((snap
           (lambda (label)
             (let ((context (org-element-context)))
               (list label
                     (point)
                     (org-element-type context)
                     (org-element-property :type context)
                     (org-element-property :path context)
                     (org-element-property :raw-link context)
                     org-link-descriptive
                     (get-text-property (point) 'invisible)
                     (get-text-property (point) 'display))))))
      (goto-char (point-min))
      (org-next-link)
      (let ((first (funcall snap 'first)))
        (org-next-link)
        (let ((second (funcall snap 'second)))
          (org-next-link)
          (let ((third (funcall snap 'third)))
            (org-previous-link)
            (let ((back (funcall snap 'back)))
              (org-toggle-link-display)
              (font-lock-ensure (point-min) (point-max))
              (let ((after-toggle (funcall snap 'toggle)))
                (list first
                      second
                      third
                      back
                      after-toggle
                      (buffer-substring-no-properties
                       (point-min) (point-max))))))))))"##,
    );
}

#[test]
fn org_link_abbrev_expand_open_from_string_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (let ((org-link-abbrev-alist-local
         '(("bug" . "https://bugs.example/%s")
           ("hex" . "https://hex.example/%h")))
        (org-link-abbrev-alist
         '(("doc" . "https://docs.example/%s")))
        calls)
    (org-link-set-parameters
     "probe-open"
     :follow (lambda (path arg) (push (list path arg) calls)))
    (with-temp-buffer
      (org-mode)
      (insert "* H\n")
      (insert "See <<radio target>> and [[probe-open:value%20x][open me]].\n")
      (insert "* Destination\n:PROPERTIES:\n:CUSTOM_ID: dest\n:END:\n")
      (goto-char (point-min))
      (search-forward "probe-open")
      (org-link-open-from-string "[[probe-open:from-string][Open]]" '(4))
      (let ((custom (save-excursion
                      (org-link-open-from-string "[[#dest]]")
                      (org-get-heading t t t t)))
            (radio (save-excursion
                     (org-link-open-from-string "[[radio target]]")
                     (buffer-substring-no-properties
                      (point) (line-end-position)))))
        (list (mapcar #'org-link-expand-abbrev
                      '("bug:123" "doc:topic" "hex:a b/c" "plain:x"))
              (nreverse calls)
              custom
              radio
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_link_store_region_file_context_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (let ((file (make-temp-file "org-link-store" nil ".org")))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* Alpha\nBody one\n** Beta\nBody two\n"))
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (setq org-stored-links nil
                  org-store-link-plist nil)
            (goto-char (point-min))
            (search-forward "Beta")
            (let ((at-heading (org-store-link nil nil)))
              (push-mark (line-beginning-position) t t)
              (end-of-line)
              (let ((from-region (org-store-link nil nil))
                    (plist org-store-link-plist)
                    (stored org-stored-links))
                (list (replace-regexp-in-string
                       (regexp-quote file) "<file>" at-heading)
                      (replace-regexp-in-string
                       (regexp-quote file) "<file>" from-region)
                      (plist-get plist :description)
                      (mapcar (lambda (entry)
                                (list (replace-regexp-in-string
                                       (regexp-quote file) "<file>"
                                       (car entry))
                                      (cdr entry)))
                              stored)
                      (buffer-substring-no-properties
                       (point-min) (point-max)))))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_link_precise_target_region_named_heading_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (with-temp-buffer
    (org-mode)
    (insert "#+NAME: tbl\n")
    (insert "| a | b |\n| 1 | 2 |\n")
    (insert "* TODO [#A] Target :tag:\n")
    (insert ":PROPERTIES:\n:CUSTOM_ID: custom-target\n:END:\n")
    (insert "Body region words here.\n")
    (let ((offset (lambda (row)
                    (and row
                         (list (nth 0 row)
                               (nth 1 row)
                               (- (nth 2 row) (point-min)))))))
      (goto-char (point-min))
      (search-forward "| a |")
      (let ((named (funcall offset (org-link-precise-link-target))))
        (search-forward "Target")
        (let ((heading (funcall offset (org-link-precise-link-target))))
          (search-forward "region words")
          (push-mark (match-beginning 0) t t)
          (goto-char (match-end 0))
          (let ((region (funcall offset (org-link-precise-link-target))))
            (list named
                  heading
                  region
                  (org-link-heading-search-string)
                  (org-link-heading-search-string
                   "TODO [#B] Other [33%] :x:y:")
                  (buffer-substring-no-properties
                   (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_link_search_targets_names_coderef_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (with-temp-buffer
    (org-mode)
    (insert "* Alpha\n")
    (insert "<<radio target>> text.\n")
    (insert "#+NAME: named-table\n")
    (insert "| a | b |\n")
    (insert "* Code\n")
    (insert "#+begin_src emacs-lisp -n -r\n")
    (insert "(message \"hi\") ;; (call)\n")
    (insert "#+end_src\n")
    (insert "* Multi   Word\nbody\n")
    (let ((probe
           (lambda (search)
             (goto-char (point-min))
             (let ((kind (org-link-search search nil t)))
               (list search
                     kind
                     (buffer-substring-no-properties
                      (line-beginning-position)
                      (line-end-position))
                     (- (point) (point-min)))))))
      (list (funcall probe "radio target")
            (funcall probe "named-table")
            (funcall probe "(call)")
            (funcall probe "*Multi Word")
            (funcall probe "body")
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_link_open_file_search_targets_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (let ((file (make-temp-file "org-link-open-file" nil ".org")))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* Alpha\n")
            (insert ":PROPERTIES:\n:CUSTOM_ID: alpha-id\n:END:\n")
            (insert "Alpha body.\n")
            (insert "* Beta\n")
            (insert "<<radio-file-target>>\n")
            (insert "#+NAME: named-block\n")
            (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n"))
          (let ((link-to (lambda (search)
                           (concat "file:" file "::" search))))
            (let (out)
              (dolist (search '("#alpha-id" "*Beta" "radio-file-target"
                                "named-block"))
                (org-link-open-from-string
                 (org-link-make-string (funcall link-to search)) '(16))
                (push (list search
                            "<file>"
                            (buffer-substring-no-properties
                             (line-beginning-position)
                             (line-end-position))
                            (- (point) (point-min)))
                      out))
              (nreverse out))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_export_resolve_links_reference_matrix_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ox)
  (with-temp-buffer
    (org-mode)
    (insert "#+NAME: tbl\n")
    (insert "#+CAPTION: Table caption\n")
    (insert "| a | b |\n| 1 | 2 |\n\n")
    (insert "* Target heading\n")
    (insert ":PROPERTIES:\n:CUSTOM_ID: custom-target\n:END:\n")
    (insert "<<radio target>> body.\n")
    (insert "#+begin_src emacs-lisp -n -r\n")
    (insert "(message \"hi\") ;; (call)\n")
    (insert "#+end_src\n")
    (insert "Links: [[tbl]] [[*Target heading]] [[#custom-target]] ")
    (insert "[[radio target]] [[(call)]].\n")
    (let* ((tree (org-element-parse-buffer))
           (info (org-export-get-environment 'html nil nil))
           (links (org-element-map tree 'link #'identity))
           (table (org-element-map tree 'table #'identity nil t))
           (src (org-element-map tree 'src-block #'identity nil t))
           (headline (org-element-map tree 'headline #'identity nil t))
           (resolve
            (lambda (link)
              (let ((raw (org-element-property :raw-link link)))
                (list raw
                      (org-element-type
                       (org-export-resolve-link link info))
                      (org-export-get-reference
                       (org-export-resolve-link link info) info))))))
      (list (mapcar resolve links)
            (org-export-get-caption table)
            (org-export-get-reference table info)
            (org-export-get-reference src info)
            (org-export-get-reference headline info)
            (org-export-get-ordinal table info)
            (org-export-resolve-coderef "call" info)
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_insert_link_region_file_stored_edit_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (let* ((root (make-temp-file "org-link-insert" t))
         (org-file (expand-file-name "notes.org" root))
         (other-file (expand-file-name "docs/read me.txt" root)))
    (unwind-protect
        (progn
          (make-directory (file-name-directory other-file) t)
          (with-temp-file other-file
            (insert "external\n"))
          (with-temp-file org-file
            (insert "* Source\n")
            (insert "Selected Words\n")
            (insert "* Target\nBody\n"))
          (org-link-set-parameters
           "descprobe"
           :insert-description
           (lambda (link desc)
             (format "auto:%s:%s" link (or desc "none"))))
          (with-current-buffer (find-file-noselect org-file)
            (org-mode)
            (let ((default-directory root)
                  (org-link-file-path-type 'relative)
                  (org-link-keep-stored-after-insertion nil)
                  (org-stored-links
                   '(("https://example.org/one" "One")
                     ("https://example.org/two" "Two"))))
              (goto-char (point-min))
              (search-forward "Selected")
              (push-mark (match-beginning 0) t t)
              (search-forward "Words")
              (org-insert-link nil "descprobe:path value" nil)
              (let ((after-region
                     (buffer-substring-no-properties
                      (point-min) (point-max)))
                    (stored-after-region org-stored-links))
                (goto-char (point-min))
                (search-forward "Body")
                (end-of-line)
                (insert "\n")
                (org-insert-link nil (concat "file:" other-file "::7")
                                 "External doc")
                (insert "\n")
                (org-insert-link nil (concat "file:" org-file "::*Target")
                                 "Same file target")
                (insert "\n")
                (org-insert-link nil "descprobe:auto-only" nil)
                (let ((after-file-auto
                       (buffer-substring-no-properties
                        (point-min) (point-max))))
                  (goto-char (point-min))
                  (search-forward "Selected Words")
                  (cl-letf (((symbol-function 'read-string)
                             (lambda (prompt &optional initial &rest _)
                               (list prompt initial)
                               "https://edited.example/a b")))
                    (org-insert-link nil nil "Edited Desc"))
                  (let ((after-edit
                         (buffer-substring-no-properties
                          (point-min) (point-max))))
                    (goto-char (point-max))
                    (insert "\n")
                    (org-insert-all-links nil "- " "\n")
                    (let ((after-insert-all
                           (buffer-substring-no-properties
                            (point-min) (point-max)))
                          (stored-after-delete org-stored-links))
                      (setq org-stored-links
                            '(("https://example.org/keep" "Keep")
                              ("https://example.org/also" "Also")))
                      (org-insert-all-links '(4) "+ " "\n")
                      (list after-region
                            stored-after-region
                            after-file-auto
                            after-edit
                            after-insert-all
                            stored-after-delete
                            org-stored-links
                            (mapcar #'org-link-display-format
                                    '("[[x:y][Shown]]" "[[x:y]]" "plain"))
                            (mapcar #'org-link-add-angle-brackets
                                    '("a" "<b" "c>" "<d>"))
                            (replace-regexp-in-string
                             (regexp-quote root)
                             "<root>"
                             (buffer-substring-no-properties
                              (point-min) (point-max)))))))))))
      (when (get-file-buffer org-file) (kill-buffer (get-file-buffer org-file)))
      (delete-directory root t))))"##,
    );
}
