use expect_test::expect;

use super::assert_anki_mode_parity;

#[test]
fn anki_mode_new_card_noninteractive_builds_complete_field_template() {
    let elisp_form = r##"(let ((anki-mode--card-types
               '(("Basic" "Front" "Back" "Source"))))
         (cl-letf (((symbol-function 'find-file)
                    (lambda (file)
                      (set-buffer (get-buffer-create "*anki-mode-card-test*"))
                      (erase-buffer)
                      (setq buffer-file-name file))))
           (unwind-protect
               (progn
                 (anki-mode-new-card-noninteractive "Study" "Basic")
                 (list (string-match-p "anki-card-" (file-name-nondirectory buffer-file-name))
                       major-mode
                       anki-mode--deck
                       anki-mode--card-type
                       (local-variable-p 'anki-mode--deck)
                       (local-variable-p 'anki-mode--card-type)
                       (buffer-string)
                       (point)
                       (thing-at-point 'line t)))
             (kill-buffer "*anki-mode-card-test*"))))"##;
    let expect = expect![[
        r#"OK (0 anki-mode "Study" "Basic" t t "@Front\n\n\n@Back\n\n\n@Source\n\n\n" 8 "\n")"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_new_card_noninteractive_rejects_unknown_model_after_mode_setup() {
    let elisp_form = r##"(let ((anki-mode--card-types
               '(("Basic" "Front" "Back"))))
         (cl-letf (((symbol-function 'find-file)
                    (lambda (file)
                      (set-buffer (get-buffer-create "*anki-mode-unknown-test*"))
                      (erase-buffer)
                      (setq buffer-file-name file))))
           (unwind-protect
               (condition-case err
                   (progn
                     (anki-mode-new-card-noninteractive "Study" "Missing")
                     'unexpected-success)
                 (error
                  (list (car err)
                        (cdr err)
                        major-mode
                        anki-mode--deck
                        anki-mode--card-type
                        (buffer-string))))
             (kill-buffer "*anki-mode-unknown-test*"))))"##;
    let expect = expect![[
        r#"OK (error ("Unrecognised card type: \"Missing\"") anki-mode "Study" "Missing" "")"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_markdown_runs_configured_filter_and_trims_output() {
    let elisp_form = r##"(let ((anki-mode-markdown-command "cat"))
         (list
          (anki-mode--markdown "  **bold**\n\nline two  \n")
          (anki-mode--markdown "")
          (anki-mode--markdown "αβγ & <tag>\n")))"##;
    let expect = expect![[r#"OK ("**bold**\n\nline two" "" "αβγ & <tag>")"#]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_create_card_converts_fields_and_builds_exact_add_notes_payload() {
    let elisp_form = r##"(let (captured saved)
         (cl-letf (((symbol-function 'save-buffer)
                    (lambda (&rest _) (setq saved t)))
                   ((symbol-function 'anki-mode--markdown)
                    (lambda (value) (concat "<html>" value "</html>")))
                   ((symbol-function 'anki-mode-connect)
                    (lambda (&rest args) (setq captured args))))
           (anki-mode-create-card
            "Languages::French"
            "Basic (and reversed card)"
            '(("Front" . "bonjour")
              ("Back" . "hello")
              ("Source" . "**reader**")))
           (let* ((payload (nth 2 captured))
                  (notes (gethash 'notes payload))
                  (note (car notes))
                  (options (cdr (assq 'options note))))
             (list saved
                   (car captured)
                   (cadr captured)
                   (cadddr captured)
                   (hash-table-p payload)
                   (hash-table-count payload)
                   (cdr (assq 'deckName note))
                   (cdr (assq 'modelName note))
                   (cdr (assq 'tags note))
                   (hash-table-p options)
                   (gethash 'allowDuplicate options 'missing)
                   (cdr (assq 'fields note))))))"##;
    let expect = expect![[
        r#"OK (t anki-mode--create-card-cb "addNotes" t t 1 "Languages::French" "Basic (and reversed card)" [] t :json-false (("Front" . "<html>bonjour</html>") ("Back" . "<html>hello</html>") ("Source" . "<html>**reader**</html>")))"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_send_new_card_parses_real_buffer_and_forwards_locals() {
    let elisp_form = r##"(with-temp-buffer
         (setq-local anki-mode--deck "Programming::Lisp")
         (setq-local anki-mode--card-type "Basic")
         (insert "@Front\nWhat does `car` return?\n\n"
                 "@Back\nThe first element of a cons cell.\n"
                 "Preserves the rest unchanged.\n\n"
                 "@Source\nGNU Emacs Lisp Reference Manual")
         (let (captured)
           (cl-letf (((symbol-function 'anki-mode-create-card)
                      (lambda (&rest args) (setq captured args))))
             (anki-mode-send-new-card)
             (list captured
                   anki-mode--deck
                   anki-mode--card-type
                   (buffer-string)))))"##;
    let expect = expect![[
        r#"OK (("Programming::Lisp" "Basic" (("Front" . "What does `car` return?") ("Back" . "The first element of a cons cell.\nPreserves the rest unchanged.") ("Source" . "GNU Emacs Lisp Reference Manual"))) "Programming::Lisp" "Basic" "@Front\nWhat does `car` return?\n\n@Back\nThe first element of a cons cell.\nPreserves the rest unchanged.\n\n@Source\nGNU Emacs Lisp Reference Manual")"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_create_callback_distinguishes_duplicate_and_success() {
    let elisp_form = r##"(let (messages menus)
         (cl-letf (((symbol-function 'message)
                    (lambda (format-string &rest args)
                      (push (apply #'format format-string args) messages)))
                   ((symbol-function 'anki-mode-menu)
                    (lambda () (push 'menu menus))))
           (list
            (anki-mode--create-card-cb [nil])
            (anki-mode--create-card-cb [1234567890])
            (anki-mode--create-card-cb nil)
            (nreverse messages)
            (nreverse menus))))"##;
    let expect = expect![[
        r#"OK (#2=("Card creation returned a null card id, normally this means that the card already exists" "Created card, got back [1234567890]" "Created card, got back nil") #3=(menu . #1=(menu)) #1# #2# #3#)"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}

#[test]
fn anki_mode_end_to_end_send_builds_wire_json_from_real_card_buffer() {
    let elisp_form = r##"(with-temp-buffer
         (setq-local anki-mode--deck "Computer Science")
         (setq-local anki-mode--card-type "Basic")
         (insert "@Front\nWhat is O(log n)?\n@Back\nBinary search\n@Tags\nalgorithms")
         (let ((anki-mode-markdown-command "cat")
               request-arguments)
           (cl-letf (((symbol-function 'save-buffer) #'ignore)
                     ((symbol-function 'request)
                      (lambda (&rest args) (setq request-arguments args))))
             (anki-mode-send-new-card)
             (let* ((json-string (plist-get (cdr request-arguments) :data))
                    (json-object-type 'alist)
                    (json-array-type 'list)
                    (json-key-type 'symbol)
                    (decoded (json-read-from-string json-string))
                    (note (car (cdr (assq 'notes
                                         (cdr (assq 'params decoded)))))))
               (list (cdr (assq 'action decoded))
                     (cdr (assq 'version decoded))
                     (cdr (assq 'deckName note))
                     (cdr (assq 'modelName note))
                     (cdr (assq 'tags note))
                     (cdr (assq 'fields note))
                     (cdr (assq 'allowDuplicate
                                (cdr (assq 'options note)))))))))"##;
    let expect = expect![[
        r#"OK ("addNotes" 6 "Computer Science" "Basic" nil ((Front . "What is O(log n)?") (Back . "Binary search") (Tags . "algorithms")) :json-false)"#
    ]];
    assert_anki_mode_parity(elisp_form, expect);
}
