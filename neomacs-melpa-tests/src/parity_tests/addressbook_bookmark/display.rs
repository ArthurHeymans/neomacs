use expect_test::expect;

use super::assert_addressbook_bookmark_parity;

#[test]
fn addressbook_bookmark_insert_header_uses_login_separator_and_exact_properties() {
    let elisp_form = r##"(let ((user-login-name
                "ada")
               (addressbook-separator
                "---"))
         (with-temp-buffer
           (let ((return
                  (addressbook--insert-header)))
             (list
              return
              (buffer-string)
              (buffer-substring
               (point-min)
               (line-end-position))
              (get-text-property
               (point-min)
               'face)))))"##;
    let expect = expect![[
        r#"OK (nil #("Addressbook Ada\n\n---\n" 0 15 (face #1=((:foreground "green" :underline t)))) #("Addressbook Ada\n\n---\n" 0 15 (face #1#)) ((:foreground "green" :underline t)))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_pp_info_renders_every_text_field_mode_and_name_property() {
    let elisp_form = r##"(let* ((addressbook-buffer-name
                  "*addressbook-parity-full*")
                 (addressbook-separator
                  "---")
                 (addressbook-align-image
                  nil)
                 (bookmark-alist
                  '(("Ada"
                     (type . "addressbook")
                     (group . "math")
                     (email . "ada@example.test")
                     (phone . "+44")
                     (web . "https://example.test")
                     (street . "1 Engine Way")
                     (city . "London")
                     (state . "London")
                     (zipcode . "SW1")
                     (country . "UK")
                     (note . "programmer")
                     (image . ""))))
                 result)
         (unwind-protect
             (cl-letf (((symbol-function
                         'bookmark-maybe-load-default-file)
                        (lambda ()
                          'loaded)))
               (addressbook-pp-info
                "Ada")
               (with-current-buffer
                   addressbook-buffer-name
                 (setq
                  result
                  (list
                   (buffer-string)
                   major-mode
                   mode-name
                   buffer-read-only
                   show-trailing-whitespace
                   (get-text-property
                    (point-min)
                    'face)
                   (progn
                     (goto-char
                      (point-min))
                     (search-forward
                      "Name:")
                     (list
                      (get-text-property
                       (line-beginning-position)
                       'name)
                      (get-text-property
                       (1+
                        (line-beginning-position))
                       'name)))))))
           (when
               (get-buffer
                addressbook-buffer-name)
             (kill-buffer
              addressbook-buffer-name)))
         result)"##;
    let expect = expect![[
        r#"OK (#("Addressbook Melpa-Test\n\n---\nName:    Ada\nGroup:   math\nMail:    ada@example.test\nPhone:   +44\nWeb:     https://example.test\nStreet:  1 Engine Way\nCity:    London\nState:   London\nZipcode: SW1\nCountry: UK\nNote:    programmer\n---\n" 0 22 (face ((:foreground "green" :underline t))) 28 33 (name "Ada" face ((:underline t))) 41 47 (face ((:underline t))) 55 60 (face ((:underline t))) 81 87 (face ((:underline t))) 94 98 (face ((:underline t))) 124 131 (face ((:underline t))) 146 151 (face ((:underline t))) 162 168 (face ((:underline t))) 178 186 (face ((:underline t))) 191 199 (face ((:underline t))) 203 208 (face ((:underline t)))) addressbook-mode "addressbook" t nil ((:foreground "green" :underline t)) ("Ada" "Ada"))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_pp_info_omits_empty_fields_and_prevents_duplicate_append() {
    let elisp_form = r##"(let* ((addressbook-buffer-name
                  "*addressbook-parity-append*")
                 (addressbook-separator
                  "===")
                 (bookmark-alist
                  '(("Ada"
                     (type . "addressbook")
                     (group . "")
                     (email . "ada@example.test")
                     (phone . "")
                     (web . "")
                     (street . "")
                     (city . "")
                     (state . "")
                     (zipcode . "")
                     (country . "")
                     (note . "")
                     (image . ""))
                    ("Bob"
                     (type . "addressbook")
                     (group . "")
                     (email . "bob@example.test")
                     (phone . "")
                     (web . "")
                     (street . "")
                     (city . "")
                     (state . "")
                     (zipcode . "")
                     (country . "")
                     (note . "")
                     (image . ""))))
                 result)
         (unwind-protect
             (cl-letf (((symbol-function
                         'bookmark-maybe-load-default-file)
                        (lambda ()
                          nil)))
               (addressbook-pp-info
                "Ada")
               (addressbook-pp-info
                "Ada"
                t)
               (addressbook-pp-info
                "Bob"
                t)
               (with-current-buffer
                   addressbook-buffer-name
                 (setq
                  result
                  (list
                   (buffer-string)
                   (save-excursion
                     (goto-char
                      (point-min))
                     (how-many
                      "^Name:"))
                   (save-excursion
                     (goto-char
                      (point-min))
                     (how-many
                      "^Group:"))
                   (save-excursion
                     (goto-char
                      (point-min))
                     (how-many
                      "^Mail:"))))))
           (when
               (get-buffer
                addressbook-buffer-name)
             (kill-buffer
              addressbook-buffer-name)))
         result)"##;
    let expect = expect![[
        r#"OK (#("Addressbook Melpa-Test\n\n===\nName:    Ada\nMail:    ada@example.test\n===\nName:    Bob\nMail:    bob@example.test\n===\n" 0 22 (face ((:foreground "green" :underline t))) 28 33 (name "Ada" face #1=((:underline t))) 41 46 (face #2=((:underline t))) 71 76 (name "Bob" face #1#) 84 89 (face #2#)) 2 0 2)"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_pp_info_image_alignment_observes_frame_and_image_width() {
    let elisp_form = r##"(let* ((addressbook-buffer-name
                  "*addressbook-parity-image*")
                 (addressbook-separator
                  "-")
                 (addressbook-align-image
                  t)
                 (bookmark-alist
                  '(("Ada"
                     (type . "addressbook")
                     (group . "")
                     (email . "")
                     (phone . "")
                     (web . "")
                     (street . "")
                     (city . "")
                     (state . "")
                     (zipcode . "")
                     (country . "")
                     (note . "")
                     (image . "portrait.png"))))
                 calls
                 result)
         (unwind-protect
             (cl-letf (((symbol-function
                         'bookmark-maybe-load-default-file)
                        (lambda ()
                          nil))
                       ((symbol-function
                         'file-exists-p)
                        (lambda (path)
                          (push
                           (list
                            'exists
                            path)
                           calls)
                          t))
                       ((symbol-function
                         'create-image)
                        (lambda (path &rest arguments)
                          (push
                           (list
                            'create
                            path
                            arguments)
                           calls)
                          'fake-image))
                       ((symbol-function
                         'image-size)
                        (lambda (image &rest arguments)
                          (push
                           (list
                            'size
                            image
                            arguments)
                           calls)
                          '(7.2 . 3.0)))
                       ((symbol-function
                         'frame-width)
                        (lambda (&rest arguments)
                          (push
                           (list
                            'frame
                            arguments)
                           calls)
                          30))
                       ((symbol-function
                         'insert-image)
                        (lambda (image &rest arguments)
                          (push
                           (list
                            'insert
                            image
                            arguments)
                           calls)
                          (insert
                           "<IMAGE>"))))
               (addressbook-pp-info
                "Ada")
               (with-current-buffer
                   addressbook-buffer-name
                 (setq
                  result
                  (list
                   (buffer-string)
                   (nreverse
                    calls)))))
           (when
               (get-buffer
                addressbook-buffer-name)
             (kill-buffer
              addressbook-buffer-name)))
         result)"##;
    let expect = expect![[
        r#"OK (#("Addressbook Melpa-Test\n\n-\nName:    Ada          <IMAGE>\n-\n" 0 22 (face ((:foreground "green" :underline t))) 26 31 (name "Ada" face ((:underline t)))) ((exists "portrait.png") (create "portrait.png" nil) (size fake-image nil) (frame nil) (insert fake-image nil)))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_goto_name_and_contact_data_follow_separator_regions() {
    let elisp_form = r##"(let ((addressbook-separator
                "---")
               (bookmark-alist
                '(("Ada" (type . "addressbook") (email . "ada@example.test"))
                  ("Bob" (type . "addressbook") (email . "bob@example.test")))))
         (with-temp-buffer
           (insert
            "---\nName:    Ada\nMail:    ada@example.test\n---\nName:    Bob\nMail:    bob@example.test\n---\n")
           (goto-char
            (point-min))
           (search-forward
            "Name:    Ada")
           (put-text-property
            (line-beginning-position)
            (+
             (line-beginning-position)
             5)
            'name
            "Ada")
           (search-forward
            "Name:    Bob")
           (put-text-property
            (line-beginning-position)
            (+
             (line-beginning-position)
             5)
            'name
           "Bob")
           (goto-char
            (point-max))
           (search-backward
            "Mail:    bob@example.test")
           (let ((return
                  (addressbook--goto-name)))
             (list
              return
              (point)
              (buffer-substring-no-properties
               (line-beginning-position)
               (line-end-position))
              (addressbook-get-contact-data)))))"##;
    let expect = expect![[
        r#"OK (0 48 "Name:    Bob" ("Bob" (type . "addressbook") (email . "bob@example.test")))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_goto_map_covers_available_missing_city_and_missing_osm() {
    let elisp_form = r##"(let ((contact
                '("Ada"
                  (street . "1 Engine Way")
                  (city . "London")
                  (state . "London")
                  (zipcode . "SW1")
                  (country . "UK")))
               require-results
               searches
               messages)
         (cl-letf (((symbol-function
                     'require)
                    (lambda (feature &rest _arguments)
                      (if
                          (eq
                           feature
                           'osm)
                          (pop
                           require-results)
                        feature)))
                   ((symbol-function
                     'osm-search)
                    (lambda (query)
                      (push
                       (list
                        query
                        osm-server)
                       searches)
                      'searched))
                   ((symbol-function
                     'message)
                    (lambda (format-string &rest arguments)
                      (push
                       (apply
                        #'format
                        format-string
                        arguments)
                       messages))))
           (setq
            require-results
            '(t
              t
              nil))
           (let ((found
                  (addressbook-goto-map
                   contact))
                 (no-city
                  (addressbook-goto-map
                   '("No City"
                     (street . "Nowhere")
                     (city . "")
                     (state . "")
                     (zipcode . "")
                     (country . ""))))
                 (no-osm
                  (addressbook-goto-map
                   contact)))
             (list
              found
              no-city
              no-osm
              (nreverse
               searches)
              (nreverse
               messages)))))"##;
    let expect = expect![[
        r#"OK (searched #2=("No address known for this contact" . #1=("Osm maps not available.")) #1# (("1 Engine Way London London SW1 UK" default)) #2#)"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}

#[test]
fn addressbook_bookmark_jump_builds_default_handler_record_from_rendered_buffer() {
    let elisp_form = r##"(let ((bookmark
                '("Ada"
                  (filename)
                  (type . "addressbook")
                  (email . "ada@example.test")
                  (handler . addressbook-bookmark-jump)))
               pp-calls
               handler-calls)
         (with-temp-buffer
           (let ((origin
                  (current-buffer)))
             (cl-letf (((symbol-function
                         'addressbook-pp-info)
                        (lambda (name &optional append)
                          (push
                           (list
                            name
                            append
                            current-prefix-arg)
                           pp-calls)
                          (set-buffer
                           origin)
                          'rendered))
                       ((symbol-function
                         'bookmark-default-handler)
                        (lambda (record)
                          (push
                           record
                           handler-calls)
                          'handled)))
               (let ((without-prefix
                      (let ((current-prefix-arg
                             nil))
                        (addressbook-bookmark-jump
                         bookmark)))
                     (with-prefix
                      (let ((current-prefix-arg
                             '(4)))
                        (addressbook-bookmark-jump
                         bookmark))))
                 (list
                  without-prefix
                  with-prefix
                  (nreverse
                   pp-calls)
                  (mapcar
                   (lambda (record)
                     (list
                      (car
                       record)
                      (eq
                       (cdr
                        (assq
                         'buffer
                         record))
                       origin)
                      (assoc-default
                       'type
                       record)
                      (assoc-default
                       'email
                       record)))
                   (nreverse
                    handler-calls))))))))"##;
    let expect = expect![[
        r#"OK (handled handled (("Ada" nil nil) ("Ada" append (4))) (("" t "addressbook" "ada@example.test") ("" t "addressbook" "ada@example.test")))"#
    ]];
    assert_addressbook_bookmark_parity(elisp_form, expect);
}
