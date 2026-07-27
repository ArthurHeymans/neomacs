use expect_test::expect;

use super::assert_agent_shell_parity;

#[test]
fn composes_environment_from_manual_dotenv_and_inherited_sources() {
    let elisp_form = r##"
(let* ((root (expand-file-name "agent-shell-env"
                                (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
       (first (expand-file-name "base.env" root))
       (second (expand-file-name "local.env" root))
       (process-environment '("INHERITED=yes" "SHARED=from-process")))
  (make-directory root t)
  (with-temp-file first
    (insert "# project defaults\nAPI_HOST=https://example.test\nSHARED=from-base\n\n"))
  (with-temp-file second
    (insert "TOKEN=secret\nSHARED=from-local\n"))
  (agent-shell-make-environment-variables
   "MODE" "parity"
   "EMPTY" ""
   :load-env (list first second)
   :inherit-env t))
"##;
    let expect = expect![[
        r#"OK ("MODE=parity" "EMPTY=" "API_HOST=https://example.test" "SHARED=from-base" "TOKEN=secret" "SHARED=from-local" "INHERITED=yes" "SHARED=from-process")"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn parses_quoted_unquoted_and_punctuated_file_mentions_from_a_prompt() {
    let elisp_form = r##"
(mapcar
 (lambda (prompt)
   (list prompt (agent-shell--parse-file-mentions prompt)))
 '("Compare @src/lib.rs with @\"docs/design notes.org\", then update @Cargo.toml."
   "At start: @README.md"
   "Email dev@example.com and mention @nested/path/file-name_test.el"
   "No attachment in this prompt"
   "Duplicate @src/lib.rs and @src/lib.rs"))
"##;
    let expect = expect![[
        r#"OK (("Compare @src/lib.rs with @\"docs/design notes.org\", then update @Cargo.toml." (((:start . 7) (:end . 19) (:path . "src/lib.rs")) ((:start . 24) (:end . 49) (:path . "docs/design notes.org")) ((:start . 62) (:end . 75) (:path . "Cargo.toml.")))) ("At start: @README.md" (((:start . 9) (:end . 20) (:path . "README.md")))) ("Email dev@example.com and mention @nested/path/file-name_test.el" (((:start . 33) (:end . 64) (:path . "nested/path/file-name_test.el")))) ("No attachment in this prompt" nil) ("Duplicate @src/lib.rs and @src/lib.rs" (((:start . 9) (:end . 21) (:path . "src/lib.rs")) ((:start . 25) (:end . 37) (:path . "src/lib.rs")))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn builds_embedded_resource_blocks_from_real_project_files() {
    let elisp_form = r##"
(let* ((root (file-name-as-directory
              (expand-file-name "agent-shell-content"
                                (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
       (source (expand-file-name "src/example.el" root))
       (notes (expand-file-name "design notes.md" root))
       (agent-shell--state
        '((:prompt-capabilities . ((:embedded-context . t))))))
  (make-directory (file-name-directory source) t)
  (with-temp-file source
    (insert "(defun example ()\n  \"A useful function.\"\n  42)\n"))
  (with-temp-file notes
    (insert "# Decision\n\nUse deterministic paths.\n"))
  (cl-letf (((symbol-function 'agent-shell-cwd) (lambda () root)))
    (agent-shell--build-content-blocks
     "Review @src/example.el against @\"design notes.md\" and explain risks")))
"##;
    let expect = expect![[
        r##"OK ((#1=(type . "text") (text . "Review")) (#2=(type . "resource") (resource (uri . "file://[ORACLE-SANDBOX]/agent-shell-content/src/example.el") (text . "(defun example ()\n  \"A useful function.\"\n  42)\n") (mimeType . "application/emacs-lisp"))) (#1# (text . " against")) (#2# (resource (uri . "file://[ORACLE-SANDBOX]/agent-shell-content/design notes.md") (text . "# Decision\n\nUse deterministic paths.\n") (mimeType . "text/plain"))) ((type . "text") (text . " and explain risks")))"##
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn falls_back_to_resource_links_when_embedding_is_unavailable_or_too_large() {
    let elisp_form = r##"
(let* ((root (file-name-as-directory
              (expand-file-name "agent-shell-links"
                                (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
       (file (expand-file-name "large.txt" root)))
  (make-directory root t)
  (with-temp-file file
    (insert "0123456789abcdefghijklmnopqrstuvwxyz"))
  (cl-letf (((symbol-function 'agent-shell-cwd) (lambda () root)))
    (list
     (let ((agent-shell--state '((:prompt-capabilities . nil))))
       (agent-shell--build-content-blocks "Inspect @large.txt"))
     (let ((agent-shell--state
            '((:prompt-capabilities . ((:embedded-context . t)))))
           (agent-shell-embed-file-size-limit 8))
       (agent-shell--build-content-blocks "Inspect @large.txt")))))
"##;
    let expect = expect![[
        r#"OK (((#1=(type . "text") (text . "Inspect")) (#2=(type . "resource_link") (uri . "file://[ORACLE-SANDBOX]/agent-shell-links/large.txt") (name . "large.txt") (mimeType . "text/plain") (size . 36))) ((#1# (text . "Inspect")) (#2# (uri . "file://[ORACLE-SANDBOX]/agent-shell-links/large.txt") (name . "large.txt") (mimeType . "text/plain") (size . 36))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn encodes_a_real_binary_image_only_when_the_agent_accepts_images() {
    let elisp_form = r##"
(let* ((root (file-name-as-directory
              (expand-file-name "agent-shell-image"
                                (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
       (file (expand-file-name "pixel.png" root))
       (png (unibyte-string
             #x89 #x50 #x4e #x47 #x0d #x0a #x1a #x0a
             #x00 #x00 #x00 #x0d #x49 #x48 #x44 #x52
             #x00 #x00 #x00 #x01 #x00 #x00 #x00 #x01)))
  (make-directory root t)
  (let ((coding-system-for-write 'binary))
    (with-temp-file file
      (set-buffer-multibyte nil)
      (insert png)))
  (cl-letf (((symbol-function 'agent-shell-cwd) (lambda () root)))
    (let* ((agent-shell--state
            '((:prompt-capabilities . ((:image . t)
                                       (:embedded-context . t)))))
           (image-blocks
            (agent-shell--build-content-blocks "Analyze @pixel.png"))
           (agent-shell--state '((:prompt-capabilities . nil)))
           (link-blocks
            (agent-shell--build-content-blocks "Analyze @pixel.png"))
           (image (cadr image-blocks)))
      (list
       (map-elt image 'type)
       (map-elt image 'mimeType)
       (equal (base64-decode-string (map-elt image 'data)) png)
       (length (map-elt image 'data))
       link-blocks))))
"##;
    let expect = expect![[
        r#"OK ("image" "image/png" t 32 (((type . "text") (text . "Analyze")) ((type . "resource_link") (uri . "file://[ORACLE-SANDBOX]/agent-shell-image/pixel.png") (name . "pixel.png") (mimeType . "image/png") (size . 24))))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn converts_agent_content_blocks_into_copyable_markdown_without_drops() {
    let elisp_form = r##"
(mapcar
 #'agent-shell--content-block-to-markdown
 '(((type . "text") (text . "Completed the review."))
   ((type . "image") (mimeType . "image/png")
    (name . "coverage chart") (uri . "file:///workspace/chart.png"))
   ((type . "image") (mimeType . "image/svg+xml")
    (uri . "https://example.test/diagram.svg"))
   ((type . "image") (mimeType . "image/png")
    (data . "iVBORw0KGgo="))
   ((type . "resource_link") (uri . "file:///workspace/report.txt")
    (name . "report.txt"))))
"##;
    let expect = expect![[
        r#"OK ("Completed the review." "\n\n![coverage chart](file:///workspace/chart.png)\n\n" "\n\n![image](https://example.test/diagram.svg)\n\n" "\n\n![image]([ORACLE-XDG-CACHE]/agent-shell/content/03690ab2ccd21a7441feadb0a5f629f0.png)\n\n" "\n\n[report.txt](file:///workspace/report.txt)\n\n")"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn numbers_selected_source_context_and_preserves_source_faces_only() {
    let elisp_form = r##"
(with-temp-buffer
  (insert "alpha\nbeta\ngamma\ndelta\n")
  (goto-char (point-min))
  (font-lock-mode 1)
  (put-text-property (line-beginning-position 2) (line-end-position 2)
                     'face 'font-lock-keyword-face)
  (put-text-property (line-beginning-position 3) (line-end-position 3)
                     'mouse-face 'highlight)
  (let ((numbered
         (agent-shell--get-numbered-region
          :buffer (current-buffer)
          :from (line-beginning-position 2)
          :to (line-end-position 4))))
    (list (substring-no-properties numbered)
          (get-text-property
           (string-match "beta" numbered) 'face numbered)
          (get-text-property
           (string-match "gamma" numbered) 'mouse-face numbered))))
"##;
    let expect =
        expect![[r#"OK ("   2: beta\n   3: gamma\n   4: delta" font-lock-keyword-face nil)"#]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn shortens_only_paths_inside_the_active_agent_workspace() {
    let elisp_form = r##"
(cl-letf (((symbol-function 'agent-shell-cwd)
           (lambda () "/work/neomacs/")))
  (mapcar
   #'agent-shell--shorten-paths
   '("/work/neomacs/src/lib.rs"
     "Review /work/neomacs/src/lib.rs lines 20-40 and /work/neomacs/Cargo.toml"
     "/work/neomacs-other/src/lib.rs"
     "/external/reference.txt"
     ""
     nil)))
"##;
    let expect = expect![[
        r#"OK ("src/lib.rs" "Review src/lib.rs lines 20-40 and Cargo.toml" "/work/neomacs-other/src/lib.rs" "/external/reference.txt" "" nil)"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}

#[test]
fn formats_a_mixed_execution_plan_for_terminal_and_graphical_surfaces() {
    let elisp_form = r##"
(mapcar
 (lambda (graphic)
   (cl-letf (((symbol-function 'display-graphic-p)
              (lambda (&optional _) graphic)))
     (let ((plan
            (agent-shell--format-plan
             [((content . "Inspect pinned source") (status . "completed"))
              ((content . "Run GNU oracle") (status . "in_progress"))
              ((content . "Compare Neomacs result") (status . "pending"))
              ((content . "Document divergence") (status . "failed"))])))
       (list graphic
             (substring-no-properties plan)
             (get-text-property 0 'font-lock-face plan)))))
 '(nil t))
"##;
    let expect = expect![[
        r#"OK ((nil "[✓] Inspect pinned source\n[…] Run GNU oracle\n[…] Compare Neomacs result\n[✗] Document divergence" ((agent-shell-success #1=(:inverse-video t)) default)) (t " ✓  Inspect pinned source\n …  Run GNU oracle\n …  Compare Neomacs result\n ✗  Document divergence" ((agent-shell-success #1#) default)))"#
    ]];
    assert_agent_shell_parity(elisp_form, expect);
}
