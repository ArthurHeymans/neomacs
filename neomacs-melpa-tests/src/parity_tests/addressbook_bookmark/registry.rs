use expect_test::expect;

use super::{assert_addressbook_bookmark_autoload_parity, assert_addressbook_bookmark_parity};

#[test]
fn addressbook_bookmark_exact_pin_metadata_header_feature_and_complete_prefix_surface_match() {
    let elisp_form = r##"(progn
         (require
          'lisp-mnt)
         (let ((descriptor
                (cadr
                 (assq
                  'addressbook-bookmark
                  package-alist)))
               callables)
           (mapatoms
            (lambda (symbol)
              (when
                  (and
                   (string-prefix-p
                    "addressbook-"
                    (symbol-name
                     symbol))
                   (fboundp
                    symbol))
                (push
                 symbol
                 callables))))
           (list
            (package-desc-name
             descriptor)
            (package-version-join
             (package-desc-version
              descriptor))
            (package-desc-summary
             descriptor)
            (package-desc-kind
             descriptor)
            (package-desc-reqs
             descriptor)
            (package-desc-extras
             descriptor)
            (featurep
             'addressbook-bookmark)
            (with-temp-buffer
              (insert-file-contents
               (getenv
                "NEOMACS_PACKAGE_SOURCE"))
              (list
               (lm-header
                "version")
               (lm-header
                "x-url")))
            (sort
             callables
             (lambda (left right)
               (string-lessp
                (symbol-name
                 left)
                (symbol-name
                 right)))))))"##;
    let expect = expect![[
        r#"OK (addressbook-bookmark "20260105.453" "An address book based on Standard Emacs bookmarks." nil ((emacs (24))) ((:maintainers ("Thierry Volpiatto" . "thievol@posteo.net")) (:authors ("Thierry Volpiatto" . "thievol@posteo.net")) (:revdesc . "469faa4206e6") (:commit . "469faa4206e6503d9c045fecb2ec0af8e1dcf504") (:url . "https://github.com/thierryvolpiatto/addressbook-bookmark")) t (nil "https://github.com/thierryvolpiatto/addressbook-bookmark") (addressbook--bookmark-from-mail addressbook--goto-name addressbook--insert-header addressbook-alist-only addressbook-bmenu-edit addressbook-bookmark-addressbook-p addressbook-bookmark-edit addressbook-bookmark-filter-setup-alist addressbook-bookmark-jump addressbook-bookmark-make-entry addressbook-bookmark-p addressbook-bookmark-set addressbook-bookmark-set-1 addressbook-complete-multiple addressbook-edit addressbook-get-contact-data addressbook-get-mu4e-from-field addressbook-gnus-sum-bookmark addressbook-goto-map addressbook-jump addressbook-maybe-save-bookmark addressbook-message-complete addressbook-mode addressbook-mode-revert addressbook-mu4e-bookmark addressbook-pp-info addressbook-quit addressbook-read-name addressbook-set-mail-buffer addressbook-set-mail-buffer-1 addressbook-set-mail-buffer-and-cc addressbook-set-mail-buffer-for-all addressbook-turn-on-mail-completion))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_complete_callable_contract_surface_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist
             symbol
             t)
            (commandp
             symbol)
            (interactive-form
             symbol)
            (documentation
             symbol
             t)
            (file-name-nondirectory
             (symbol-file
              symbol
              'defun))))
         '(addressbook-mode
           addressbook-mode-revert
           addressbook-quit
           addressbook-set-mail-buffer-1
           addressbook-set-mail-buffer
           addressbook-set-mail-buffer-and-cc
           addressbook-set-mail-buffer-for-all
           addressbook-turn-on-mail-completion
           addressbook-bookmark-addressbook-p
           addressbook-alist-only
           addressbook-message-complete
           addressbook-bookmark-make-entry
           addressbook-read-name
           addressbook-bookmark-set-1
           addressbook-bookmark-set
           addressbook-maybe-save-bookmark
           addressbook--bookmark-from-mail
           addressbook-gnus-sum-bookmark
           addressbook-get-mu4e-from-field
           addressbook-mu4e-bookmark
           addressbook-bookmark-edit
           addressbook-edit
           addressbook-bmenu-edit
           addressbook--insert-header
           addressbook-pp-info
           addressbook--goto-name
           addressbook-get-contact-data
           addressbook-goto-map
           addressbook-bookmark-jump
           addressbook-bookmark-p
           addressbook-bookmark-filter-setup-alist
           addressbook-complete-multiple
           addressbook-jump))"##;
    let expect = expect![[
        r#"OK ((addressbook-mode nil t (interactive nil) "Interface for addressbook.\n\nSpecial commands:\n\\{addressbook-mode-map}\n\nIn addition to any hooks its parent mode `special-mode' might have\nrun, this mode runs the hook `addressbook-mode-hook', as the final or\npenultimate step during initialization." "addressbook-bookmark.el") (addressbook-mode-revert (&optional _revert-auto _no-confirm) t (interactive nil) nil "addressbook-bookmark.el") (addressbook-quit nil t (interactive nil) "Quit addressbook buffer." "addressbook-bookmark.el") (addressbook-set-mail-buffer-1 (&optional bookmark-name append cc) nil nil "Setup a mail buffer with BOOKMARK-NAME email using `message-mode'." "addressbook-bookmark.el") (addressbook-set-mail-buffer (append) t (interactive "P") "Prepare email buffer with `message-mode' from addressbook buffer." "addressbook-bookmark.el") (addressbook-set-mail-buffer-and-cc (append) t (interactive "P") "Add a cc field to a mail buffer for this bookmark." "addressbook-bookmark.el") (addressbook-set-mail-buffer-for-all nil t (interactive nil) nil "addressbook-bookmark.el") (addressbook-turn-on-mail-completion nil nil nil nil "addressbook-bookmark.el") (addressbook-bookmark-addressbook-p (bookmark) nil nil nil "addressbook-bookmark.el") (addressbook-alist-only nil nil nil nil "addressbook-bookmark.el") (addressbook-message-complete nil nil nil nil "addressbook-bookmark.el") (addressbook-bookmark-make-entry (name group email phone web street city state zipcode country note image-path) nil nil "Build an addressbook bookmark entry." "addressbook-bookmark.el") (addressbook-read-name (prompt) nil nil "Prompt as many time PROMPT is not empty." "addressbook-bookmark.el") (addressbook-bookmark-set-1 (&optional contact) nil nil "Add contact repetitively until user say no.\n\nWhen CONTACT arg is provided add only contact CONTACT and exit." "addressbook-bookmark.el") (addressbook-bookmark-set nil t (interactive nil) "Record addressbook bookmark entries interactively." "addressbook-bookmark.el") (addressbook-maybe-save-bookmark nil nil nil "Increment save counter and maybe save `bookmark-alist'." "addressbook-bookmark.el") (addressbook--bookmark-from-mail (data) nil nil "Record an addressbook bookmark from a mail buffer." "addressbook-bookmark.el") (addressbook-gnus-sum-bookmark nil t (interactive nil) "Record an addressbook bookmark from a gnus summary buffer." "addressbook-bookmark.el") (addressbook-get-mu4e-from-field nil nil nil "Return from field contents from a mu4e buffer." "addressbook-bookmark.el") (addressbook-mu4e-bookmark nil t (interactive nil) "Record an addressbook bookmark from a mu4e view buffer." "addressbook-bookmark.el") (addressbook-bookmark-edit (bookmark) nil nil "Edit an addressbook bookmark entry." "addressbook-bookmark.el") (addressbook-edit nil t (interactive nil) "Edit contact from addressbook buffer." "addressbook-bookmark.el") (addressbook-bmenu-edit nil t (interactive nil) "Edit an addresbook bookmark entry from bmenu list." "addressbook-bookmark.el") (addressbook--insert-header nil nil nil nil "addressbook-bookmark.el") (addressbook-pp-info (name &optional append) nil nil "Print addressbook entries to an addressbook buffer." "addressbook-bookmark.el") (addressbook--goto-name nil nil nil nil "addressbook-bookmark.el") (addressbook-get-contact-data nil nil nil "Get bookmark entry of contact at point in addressbook buffer." "addressbook-bookmark.el") (addressbook-goto-map (&optional bookmark) t (interactive nil) "Show an open street map for this address.\nNeeds `osm' package as dependency." "addressbook-bookmark.el") (addressbook-bookmark-jump (bookmark) nil nil "Default handler to jump to an addressbook bookmark." "addressbook-bookmark.el") (addressbook-bookmark-p (bookmark) nil nil "Return non--nil if BOOKMARK is a contact recorded with addressbook-bookmark.\nBOOKMARK is a bookmark name or a bookmark record." "addressbook-bookmark.el") (addressbook-bookmark-filter-setup-alist nil nil nil "Return a filtered `bookmark-alist' sorted alphabetically." "addressbook-bookmark.el") (addressbook-complete-multiple (prompt collection &optional predicate require-match initial-input hist) nil nil "Returns a list of candidates.\nUse either `completing-read' or `completing-read-multiple'.\n`completing-read' is used when helm-mode is available, this allows\nusing marked candidates." "addressbook-bookmark.el") (addressbook-jump (bmks) t (interactive (list (let* ((bookmark-alist (addressbook-bookmark-filter-setup-alist))) (addressbook-complete-multiple "Jump to contact: " bookmark-alist nil t)))) "Jump to bookmarks BMKS, a list of bookmarks.\nWith a prefix arg append to addressbook buffer." "addressbook-bookmark.el"))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_custom_face_mode_map_and_mode_contract_match() {
    let elisp_form = r##"(list
         addressbook-buffer-name
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (default-value
              symbol)
             (custom-variable-p
              symbol)
             (get
              symbol
              'standard-value)
             (get
              symbol
              'custom-type)
             (get
              symbol
              'custom-group)
             (documentation-property
              symbol
              'variable-documentation
              t)))
          '(addressbook-separator
            addressbook-align-image))
         (facep
          'abook-separator)
         (get
          'abook-separator
          'face-defface-spec)
         (face-documentation
          'abook-separator)
         (mapcar
          (lambda (key)
            (list
             key
             (lookup-key
              addressbook-mode-map
              (kbd
               key))))
          '("q"
            "m"
            "M"
            "e"
            "C-c C-c"
            "C-c f c"
            "r"
            "s"
            "C-c m"))
         (with-temp-buffer
           (addressbook-mode)
           (list
            major-mode
            mode-name
            buffer-read-only
            (eq
             revert-buffer-function
             #'addressbook-mode-revert)
            (local-variable-p
             'revert-buffer-function)
            (eq
             (current-local-map)
             addressbook-mode-map))))"##;
    let expect = expect![[
        r#"OK ("*addressbook*" ((addressbook-separator #("---------------------------------------------" 0 45 (face abook-separator)) #1=((funcall #'#[nil ((propertize (make-string 45 45) 'face 'abook-separator)) #2=(helm-comp-read-use-marked gnus-article-current osm-server t)])) #1# string nil "*String used to separate contacts in addressbook buffer.") (addressbook-align-image nil #3=((funcall #'#[nil (nil) #2#])) #3# boolean nil "If true, images will be padded to the margin")) [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:foreground "red"))) "*Face used for lines separating addressbook entries." (("q" addressbook-quit) ("m" addressbook-set-mail-buffer) ("M" addressbook-set-mail-buffer-for-all) ("e" addressbook-edit) ("C-c C-c" addressbook-set-mail-buffer) ("C-c f c" addressbook-set-mail-buffer-and-cc) ("r" addressbook-bookmark-set) ("s" bookmark-save) ("C-c m" addressbook-goto-map)) (addressbook-mode "addressbook" t t t t))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_installed_inventory_and_source_sha256_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'addressbook-bookmark
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor)))
         (mapcar
          (lambda (file)
            (let ((path
                   (expand-file-name
                    file
                    directory)))
              (if
                  (string-suffix-p
                   ".elc"
                   file)
                  (list
                   file
                   'generated-bytecode)
                (list
                 file
                 (file-attribute-size
                  (file-attributes
                   path))
                 (with-temp-buffer
                   (set-buffer-multibyte
                    nil)
                   (insert-file-contents-literally
                    path)
                   (secure-hash
                    'sha256
                    (current-buffer)))))))
          (sort
           (seq-filter
            (lambda (file)
              (file-regular-p
               (expand-file-name
                file
                directory)))
            (directory-files
             directory
             nil
             "\\`[^.]"))
           #'string-lessp)))"##;
    let expect = expect![[
        r#"OK (("addressbook-bookmark-autoloads.el" 1666 "29507771c7517e0bf890a8633fb2006370d05229a15db3690794663140e0739f") ("addressbook-bookmark-pkg.el" 447 "a0a97ea215d3d6ca2d8cc8388669f959d7c221fe3b01e56a362be619501579e6") ("addressbook-bookmark.el" 25439 "f7e1222ab267597c3258df8dce362bbef4eb9b1629c38b3b7070bb749a26ec88") ("addressbook-bookmark.elc" generated-bytecode))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_generated_autoload_surface_matches_exactly() {
    let elisp_form = r##"(list
         (featurep
          'addressbook-bookmark-autoloads)
         (featurep
          'addressbook-bookmark)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp
              symbol)
             (and
              (fboundp
               symbol)
              (autoloadp
               (symbol-function
                symbol)))
             (commandp
              symbol)
             (and
              (fboundp
               symbol)
              (help-function-arglist
               symbol
               t))))
          '(addressbook-turn-on-mail-completion
            addressbook-bookmark-set-1
            addressbook-gnus-sum-bookmark
            addressbook-mu4e-bookmark
            addressbook-bmenu-edit
            addressbook-bookmark-jump
            addressbook-jump
            addressbook-mode
            addressbook-bookmark-p)))"##;
    let expect = expect![[
        r#"OK (t nil ((addressbook-turn-on-mail-completion t t nil "[Arg list not available until function definition is loaded.]") (addressbook-bookmark-set-1 t t nil "[Arg list not available until function definition is loaded.]") (addressbook-gnus-sum-bookmark t t t "[Arg list not available until function definition is loaded.]") (addressbook-mu4e-bookmark t t t "[Arg list not available until function definition is loaded.]") (addressbook-bmenu-edit t t t "[Arg list not available until function definition is loaded.]") (addressbook-bookmark-jump t t nil "[Arg list not available until function definition is loaded.]") (addressbook-jump t t t "[Arg list not available until function definition is loaded.]") (addressbook-mode nil nil nil nil) (addressbook-bookmark-p nil nil nil nil)))"#
    ]];
    assert_addressbook_bookmark_autoload_parity(elisp_form, expect);
}
