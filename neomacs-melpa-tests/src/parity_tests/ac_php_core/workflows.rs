use expect_test::expect;

use super::assert_ac_php_core_parity;

/// Indexing, which everything else depends on.
///
/// `ac-php-remake-tags` resolves the project root, writes the configuration
/// file when it finds none -- the fixture deliberately does not write one, so
/// what is asserted is the package's own -- runs the indexer, and leaves an
/// index where the package looks for it.  The loaded result is then asserted
/// in full: the classes, the inheritance edges read from `extends`, the
/// namespaced function, and the file list the index's positions refer to.
#[test]
fn indexing_a_project_writes_its_configuration_and_loads_every_class() {
    let elisp_form = r##"
(let ((root (ac-php-test-make-project))
      (program (ac-php-test-install-php)))
  (ac-php-test-in-php-buffer
   "src/Service/Cart.php"
   (call-interactively 'ac-php-remake-tags)
   (let* ((finished (ac-php-test-wait-for-index))
          (tags-data (ac-php-get-tags-data))
          (cache (expand-file-name "cache" ac-php-test-root)))
     (list :indexer-finished finished
           :calls (mapcar (lambda (call)
                            (mapcar #'file-name-nondirectory call))
                          (ac-php-test-php-calls))
           :config (ac-php-test-read
                    (expand-file-name ".ac-php-conf.json" ac-php-test-project))
           :index-files (sort (mapcar #'file-name-nondirectory
                                      (directory-files-recursively cache ""))
                              #'string<)
           :progress ac-php-phptags-index-progress
           :classes (let (keys)
                      (maphash (lambda (key _value) (push key keys))
                               (ac-php-g--class-map tags-data))
                      (sort keys #'string<))
           :inheritance (let (edges)
                          (maphash (lambda (key value)
                                     (push (cons key (append value nil)) edges))
                                   (ac-php-g--inherit-map tags-data))
                          (sort edges (lambda (a b) (string< (car a) (car b)))))
           :functions (let (keys)
                        (maphash (lambda (key _value) (push key keys))
                                 (ac-php-g--function-map tags-data))
                        (sort keys #'string<))
           :indexed-files (mapcar (lambda (file) (file-relative-name file root))
                                  (append (ac-php-g--file-list tags-data) nil))))))
"##;

    let expect = expect![[
        r##"OK (:indexer-finished t :calls (("phpctags" ".ac-php-conf.json" "cache" "--rebuild=no" "--realpath_flag=yes")) :config "{\n  \"use-cscope\": null,\n  \"tag-dir\": null,\n  \"filter\": {\n    \"php-file-ext-list\": [\n      \"php\"\n    ],\n    \"php-path-list\": [\n      \".\"\n    ],\n    \"ignore-ruleset\": [\n      \"# like .gitignore file \",\n      \"/vendor/**/[tT]ests/**/*.php\",\n      \"/vendor/**/[Ee]xamples/**/*.php\",\n      \"/vendor/composer/*.php\",\n      \"/vendor/*.php\",\n      \"# not need php_codesniffer\",\n      \"/vendor/squizlabs/php_codesniffer/**/*.php\",\n      \"#  -- end -- \"\n    ]\n  }\n}" :index-files ("tags-vendor.el" "tags.el") :progress 83 :classes ("\\Shop\\Model\\Product" "\\Shop\\Service\\BaseCart" "\\Shop\\Service\\Cart") :inheritance (("\\Shop\\Service\\Cart" "\\Shop\\Service\\BaseCart")) :functions ("\\Shop\\Model\\Product" "\\Shop\\Model\\Product(" "\\Shop\\Model\\formatMoney(" "\\Shop\\Service\\BaseCart" "\\Shop\\Service\\BaseCart(" "\\Shop\\Service\\Cart" "\\Shop\\Service\\Cart(") :indexed-files ("src/Model/Product.php" "src/Service/BaseCart.php" "src/Service/Cart.php"))"##
    ]];
    assert_ac_php_core_parity(elisp_form, expect);
}

/// Finding the project root, which decides what gets indexed and where the
/// index is written.
///
/// The package walks up from the visited file looking for any of three
/// markers, and each one is given its own tree here so that the resolved root
/// is visible in the configuration path the indexer was handed.  The fourth
/// case has no marker at all: the walk reaches the filesystem root, the
/// command reports that it cannot resolve a project, and then signals rather
/// than returning -- the nil root is used as a string a moment later.
///
/// The argument log is shared across the cases, so each one reads the last
/// invocation rather than the first.
#[test]
fn the_project_root_is_found_by_any_marker_and_the_command_fails_without_one() {
    let elisp_form = r##"
(let ((base (expand-file-name "markers" ac-php-test-root))
      (program (ac-php-test-install-php)))
  (mapcar
   (lambda (case)
     (let* ((name (car case))
            (marker (cdr case))
            (directory (expand-file-name name base))
            (file (expand-file-name "src/deep/Thing.php" directory)))
       (make-directory (file-name-directory file) t)
       (ac-php-test-write file "<?php\nclass Thing {}\n")
       (when marker
         (ac-php-test-write (expand-file-name marker directory) ""))
       (let ((buffer (find-file-noselect file))
             (ac-php-gen-tags-flag nil)
             ;; The stand-in appends to one log across the whole form, so a
             ;; case reports only the invocations it added.
             (before (let ((calls (ac-php-test-php-calls)))
                       (if (listp calls) (length calls) 0))))
         (unwind-protect
             (with-current-buffer buffer
               (php-mode)
               (let ((outcome (ac-php-test-outcome
                               (progn (call-interactively 'ac-php-remake-tags) t))))
                 (ac-php-test-wait-for-index)
                 (list name
                       :outcome outcome
                       :indexed-root
                       (let* ((calls (ac-php-test-php-calls))
                              (added (nthcdr before (if (listp calls) calls nil))))
                         (mapcar (lambda (call)
                                   (let ((config (nth 1 call)))
                                     (and (stringp config)
                                          (string-match "markers/\\([^/]+\\)/" config)
                                          (match-string 1 config))))
                                 added)))))
           (kill-buffer buffer)))))
   '(("projectile" . ".projectile")
     ("conf" . ".ac-php-conf.json")
     ("composer" . "vendor/autoload.php")
     ("nothing" . nil))))
"##;

    let expect = expect![[
        r#"OK (("projectile" :outcome (:ok t) :indexed-root ("projectile")) ("conf" :outcome (:ok t) :indexed-root ("conf")) ("composer" :outcome (:ok t) :indexed-root ("composer")) ("nothing" :outcome (:error wrong-type-argument (stringp nil)) :indexed-root nil))"#
    ]];
    assert_ac_php_core_parity(elisp_form, expect);
}

/// Jumping from a use of a class to its declaration and back, which is the
/// package's `M-.` and `M-,`.
///
/// The jump is asserted to land on the declaring line of the other file, and
/// `ac-php-location-stack-back` to return to the exact line it started from.
///
/// It is asserted twice, because in a session that has not loaded `xref` it
/// does not work at all.  `ac-php--location-stack-push` prefers
/// `xref-push-marker-stack` and falls back to `find-tag-marker-ring` when that
/// is not a function -- but `xref-push-marker-stack` is not autoloaded, and
/// the fallback names a variable that only exists once `etags` is loaded, so
/// the command signals `void-variable` before going anywhere.  Requiring
/// `xref`, which any session that has used `M-.` for anything else has already
/// done, makes the same jump work.
#[test]
fn jumping_to_a_definition_needs_xref_and_then_the_location_stack_returns() {
    let elisp_form = r##"
(let ((root (ac-php-test-make-project))
      (program (ac-php-test-install-php)))
  (ac-php-test-in-php-buffer
   "src/Service/Cart.php"
   (call-interactively 'ac-php-remake-tags)
   (ac-php-test-wait-for-index)
   (cl-flet ((at-the-use-of-product ()
               (goto-char (point-min))
               (search-forward "new Product")
               (backward-char 3)))
     (at-the-use-of-product)
     (let ((without-xref (ac-php-test-outcome
                          (progn (call-interactively 'ac-php-find-symbol-at-point) t))))
       (require 'xref)
       (at-the-use-of-product)
       (let ((started (list (file-name-nondirectory (buffer-file-name))
                            (line-number-at-pos))))
         (call-interactively 'ac-php-find-symbol-at-point)
         (let ((arrived (list (file-name-nondirectory (buffer-file-name))
                              (line-number-at-pos)
                              (buffer-substring-no-properties
                               (line-beginning-position) (line-end-position)))))
           (call-interactively 'ac-php-location-stack-back)
           (list :without-xref without-xref
                 :started started
                 :arrived arrived
                 :returned (list (file-name-nondirectory (buffer-file-name))
                                 (line-number-at-pos))
                 :stack-depth (length ac-php-location-stack))))))))
"##;

    let expect = expect![[
        r#"OK (:without-xref (:error void-variable (find-tag-marker-ring)) :started ("Cart.php" 10) :arrived ("Product.php" 7 "class Product") :returned ("Cart.php" 10) :stack-depth 1)"#
    ]];
    assert_ac_php_core_parity(elisp_form, expect);
}

/// Working out what an expression at point refers to, which is the step
/// between "the user typed `->`" and "these are the candidates".
///
/// Five contexts in one method of one file, each resolved differently:
/// `$this` from the enclosing class, `$product` from the `new Product(...)`
/// that assigned it, `Product::` from the `use` clause at the top of a file in
/// a different namespace, `self::` from the enclosing class again, and
/// `parent::` from the `extends` clause, which resolves to a placeholder
/// rather than to the parent's name.
///
/// The results are stripped of text properties before being asserted: the
/// parent case comes back propertized with buffer positions, which would
/// otherwise be encoded into the expectation.
#[test]
fn the_type_at_point_is_resolved_from_the_buffer_and_the_index() {
    let elisp_form = r##"
(let ((root (ac-php-test-make-project))
      (program (ac-php-test-install-php)))
  (ac-php-test-in-php-buffer
   "src/Service/Cart.php"
   (call-interactively 'ac-php-remake-tags)
   (ac-php-test-wait-for-index)
   (mapcar
    (lambda (text)
      (goto-char (point-min))
      (search-forward "return $product;")
      (beginning-of-line)
      (let ((start (point)) resolved)
        (insert text)
        (setq resolved (ac-php-get-class-at-point (ac-php-get-tags-data)))
        (delete-region start (point))
        (list text (if (stringp resolved) (substring-no-properties resolved) resolved))))
    '("$this->" "$product->" "Product::" "self::" "parent::"))))
"##;

    let expect = expect![[
        r#"OK (("$this->" "\\Shop\\Service\\Cart.") ("$product->" "\\Shop\\Model\\Product.") ("Product::" "\\Shop\\Model\\Product.") ("self::" "\\Shop\\Service\\Cart.") ("parent::" "\\Shop\\Service\\Cart.__parent__."))"#
    ]];
    assert_ac_php_core_parity(elisp_form, expect);
}

/// What happens when half the index goes missing, which is not a hypothetical
/// -- the vendor index is a separate file written by the same indexer run, and
/// anything that clears a cache directory selectively can remove it.
///
/// `ac-php-load-data` signals `(wrong-type-argument hash-table-p nil)` instead
/// of reporting a missing file. The consequence one level up is worse and is
/// asserted too: `ac-php-get-tags-data` treats the missing file as "no index
/// yet", starts a rebuild, and returns the rebuild's value -- the symbol
/// `ac-php-phptags-index-process-filter` -- so every caller that expects tags
/// data gets a process filter instead and signals `wrong-type-argument listp`
/// somewhere further away from the cause.
#[test]
fn a_missing_vendor_index_turns_every_lookup_into_a_type_error() {
    let elisp_form = r##"
(let ((root (ac-php-test-make-project))
      (program (ac-php-test-install-php)))
  (ac-php-test-in-php-buffer
   "src/Service/Cart.php"
   (call-interactively 'ac-php-remake-tags)
   (ac-php-test-wait-for-index)
   (let* ((cache (expand-file-name "cache" ac-php-test-root))
          (directory (car (directory-files cache t "^tags-")))
          (tags-file (expand-file-name "tags.el" directory))
          (vendor-file (expand-file-name "tags-vendor.el" directory)))
     (list :both-present (list (file-exists-p tags-file) (file-exists-p vendor-file))
           :loads-with-vendor (and (ac-php-get-tags-data) t)
           :after-deleting-the-vendor-index
           (progn
             (delete-file vendor-file)
             (setq ac-php-tag-last-data-list nil)
             (list :load-data
                   (ac-php-test-outcome
                    (and (ac-php-load-data tags-file vendor-file
                                           (directory-file-name ac-php-test-project))
                         t))
                   :tags-data-returns (ac-php-get-tags-data)
                   :class-at-point
                   (ac-php-test-outcome
                    (ac-php-get-class-at-point (ac-php-get-tags-data)))))))))
"##;

    let expect = expect![
        "OK (:both-present (t t) :loads-with-vendor t :after-deleting-the-vendor-index (:load-data (:error wrong-type-argument (hash-table-p nil)) :tags-data-returns ac-php-phptags-index-process-filter :class-at-point (:ok nil)))"
    ];
    assert_ac_php_core_parity(elisp_form, expect);
}
