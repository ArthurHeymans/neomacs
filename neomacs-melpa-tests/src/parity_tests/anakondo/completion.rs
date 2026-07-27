use expect_test::expect;

use super::assert_anakondo_parity;

#[test]
fn completion_symbol_bounds_accept_clojure_symbols_and_reject_non_code_contexts() {
    let elisp_form = r##"(cl-labels
                      ((bounds-at
                         (text &optional position)
                         (with-temp-buffer
                           (set-syntax-table
                            (copy-syntax-table
                             (standard-syntax-table)))
                           (modify-syntax-entry ?\; "<")
                           (modify-syntax-entry ?\n ">")
                           (insert text)
                           (goto-char
                            (or position (point-max)))
                           (anakondo--completion-symbol-bounds))))
                      (list
                       (bounds-at "map")
                       (bounds-at "clojure.string/joi")
                       (bounds-at "'quoted")
                       (bounds-at "@state")
                       (bounds-at "~@pending")
                       (bounds-at "; comment")
                       (bounds-at "\"inside\"" 4)
                       (bounds-at "12345")
                       (bounds-at ":keyword")
                       (bounds-at "^metadata")
                       (bounds-at "#tagged")
                       (bounds-at "\\x")
                       (bounds-at ".method")
                       (bounds-at "")))"##;
    let expect = expect![
        "OK ((1 . 4) (1 . 19) (2 . 8) (2 . 7) (3 . 10) nil nil nil nil nil nil nil nil (1 . 1))"
    ];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn buffer_language_keyword_and_safe_hash_value_helpers_cover_real_boundary_values() {
    let elisp_form = r##"(let ((values
                           (make-hash-table
                            :test 'equal)))
                      (puthash "first" 10 values)
                      (puthash "second" 20 values)
                      (list
                       (mapcar
                        (lambda (filename)
                          (with-temp-buffer
                            (setq buffer-file-name
                                  filename)
                            (anakondo--get-buffer-lang)))
                        '("src/core.clj"
                          "src/core.cljs"
                          "src/core.cljc"
                          "src/core.edn"
                          "README"))
                       (mapcar
                        (lambda (mode)
                          (with-temp-buffer
                            (setq major-mode mode)
                            (anakondo--get-buffer-lang)))
                        '(clojure-mode
                          clojurec-mode
                          clojurescript-mode
                          fundamental-mode))
                       (mapcar
                        #'anakondo--string->keyword
                        '("app.core"
                          ""
                          "名字"
                          nil))
                       (sort
                        (anakondo--safe-hash-table-values
                         values)
                        #'<)
                       (anakondo--safe-hash-table-values
                        nil)))"##;
    let expect = expect![[
        r#"OK (("clj" "cljs" "cljc" "edn" nil) ("clj" "cljc" "cljs" nil) (:app.core : :名字 nil) (10 20) nil)"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn clj_kondo_candidates_merge_current_core_namespaces_and_aliased_dependencies() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "m/")
                      (goto-char (point-max))
                      (cl-labels
                          ((record
                             (&rest pairs)
                             (let ((value
                                    (make-hash-table)))
                               (while pairs
                                 (puthash
                                  (pop pairs)
                                  (pop pairs)
                                  value))
                               value))
                           (var-table
                             (&rest names)
                             (let ((value
                                    (make-hash-table)))
                               (dolist (name names value)
                                 (puthash
                                  (anakondo--string->keyword
                                   name)
                                  (record :name name)
                                  value)))))
                        (let ((var-cache
                               (make-hash-table))
                              (ns-cache
                               (make-hash-table))
                              (usage-cache
                               (make-hash-table)))
                          (puthash
                           :app.core
                           (var-table "run" "shared")
                           var-cache)
                          (puthash
                           :clojure.core
                           (var-table "map" "reduce")
                           var-cache)
                          (puthash
                           :lib.math
                           (var-table "sum" "mean")
                           var-cache)
                          (puthash
                           :lib.io
                           (var-table "read" "write")
                           var-cache)
                          (puthash
                           :app.core
                           (record
                            :name "app.core")
                           ns-cache)
                          (puthash
                           :lib.math
                           (record
                            :name "lib.math")
                           ns-cache)
                          (puthash
                           :app.core
                           (let ((edges
                                  (make-hash-table)))
                             (puthash
                              :lib.math
                              (record
                               :to "lib.math"
                               :alias "m")
                              edges)
                             (puthash
                              :lib.io
                              (record
                               :to "lib.io"
                               :alias nil)
                              edges)
                             edges)
                           usage-cache)
                          (cl-letf
                              (((symbol-function
                                 'anakondo--get-project-var-def-cache)
                                (lambda () var-cache))
                               ((symbol-function
                                 'anakondo--get-project-ns-def-cache)
                                (lambda () ns-cache))
                               ((symbol-function
                                 'anakondo--get-project-ns-usage-cache)
                                (lambda () usage-cache))
                               ((symbol-function
                                 'anakondo--clj-kondo-buffer-analyse-sync)
                                (lambda (&rest _)
                                  :app.core)))
                            (let ((candidates
                                   (anakondo--get-clj-kondo-completion-candidates)))
                              (list
                               (buffer-string)
                               (point)
                               candidates
                               (length candidates)
                               (and
                                (member "m/sum"
                                        candidates)
                                t)
                               (and
                                (member "lib.io/read"
                                        candidates)
                                t)))))))"##;
    let expect = expect![[
        r#"OK ("m/" 3 ("shared" "run" "reduce" "map" "lib.math" "app.core" "lib.io/write" "lib.io/read" "m/mean" "m/sum") 10 t t)"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn local_completion_uses_real_dabbrev_within_current_top_level_form_only() {
    let elisp_form = r##"(with-temp-buffer
                      (emacs-lisp-mode)
                      (insert
                       "(let ((outside-value 9)) outside-value)\n\
(let ((alpha 1)\n\
      (alphabet 2)\n\
      (alpine 3))\n\
  (+ alpha alp")
                      (goto-char (point-max))
                      (let* ((prefix "alp")
                             (prefix-start
                              (- (point)
                                 (length prefix)))
                             (candidates
                              (anakondo--get-local-completion-candidates
                               prefix
                               prefix-start)))
                        (list
                         (sort candidates #'string<)
                         (member
                          "outside-value"
                          candidates)
                         (point)
                         prefix-start)))"##;
    let expect = expect![[r#"OK (("alpha" "alphabet" "alpine") nil 108 105)"#]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn java_candidates_expand_default_and_qualified_classes_and_materialize_lazy_members_once() {
    let elisp_form = r##"(cl-labels
                      ((record
                         (&rest pairs)
                         (let ((value
                                (make-hash-table)))
                           (while pairs
                             (puthash
                              (pop pairs)
                              (pop pairs)
                              value))
                           value))
                       (member-map
                         (name)
                         (record :name name)))
                      (let ((cache
                             (make-hash-table))
                            calls)
                        (puthash
                         :java.lang.Math
                         (anakondo--make-class-map
                          "java.lang.Math" 'lazy)
                         cache)
                        (puthash
                         :com.acme.Tools
                         (anakondo--make-class-map
                          "com.acme.Tools"
                          (list
                           (member-map "build")
                           (member-map "VERSION")))
                         cache)
                        (cl-letf
                            (((symbol-function
                               'anakondo--get-project-java-classes-cache)
                              (lambda () cache))
                             ((symbol-function
                               'anakondo--get-java-analysis-classpath)
                              (lambda (as)
                                (list as "project-cp")))
                             ((symbol-function
                               'anakondo--java-analyze-class-map)
                              (lambda (classpath class)
                                (push
                                 (list classpath class)
                                 calls)
                                (anakondo--make-class-map
                                 class
                                 (list
                                  (member-map "abs")
                                  (member-map "PI"))))))
                          (let ((math
                                 (anakondo--get-java-completion-candidates
                                  "Math/"))
                                (tools
                                 (anakondo--get-java-completion-candidates
                                  "com.acme.Tools/"))
                                (missing
                                 (anakondo--get-java-completion-candidates
                                  "missing.Type/")))
                            (list
                             (seq-filter
                              (lambda (candidate)
                                (string-prefix-p
                                 "Math/" candidate))
                              math)
                             (seq-filter
                              (lambda (candidate)
                                (string-prefix-p
                                 "com.acme.Tools/"
                                 candidate))
                              tools)
                             (length math)
                             (length tools)
                             (length missing)
                             (and
                              (member
                               "java.lang.Math" math)
                              t)
                             (and
                              (member
                               "String" math)
                              t)
                             (mapcar
                              (lambda (member)
                                (gethash :name member))
                              (gethash
                               :methods-and-fields
                               (gethash
                                :java.lang.Math cache)))
                             (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (("Math/abs" "Math/PI") ("com.acme.Tools/build" "com.acme.Tools/VERSION") 100 100 98 t t ("abs" "PI") (((cp "project-cp") "java.lang.Math")))"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}

#[test]
fn completion_at_point_reuses_global_candidates_adds_locals_and_invalidates_on_slash() {
    let elisp_form = r##"(with-temp-buffer
                      (emacs-lisp-mode)
                      (insert "(ma")
                      (goto-char (point-max))
                      (let (global-calls
                            local-calls
                            java-calls)
                        (cl-letf
                            (((symbol-function
                               'anakondo--get-clj-kondo-completion-candidates)
                              (lambda ()
                                (setq global-calls
                                      (1+
                                       (or global-calls 0)))
                                '("map" "mapv")))
                             ((symbol-function
                               'anakondo--get-java-completion-candidates)
                              (lambda (prefix)
                                (push prefix java-calls)
                                '("Math")))
                             ((symbol-function
                               'anakondo--get-local-completion-candidates)
                              (lambda (prefix start)
                                (push
                                 (list prefix start)
                                 local-calls)
                                '("manual")))
                             ((symbol-function
                               'anakondo--get-buffer-lang)
                              (lambda () "clj")))
                          (let* ((capf
                                  (anakondo-completion-at-point))
                                 (table (nth 2 capf))
                                 (first
                                  (all-completions
                                   "ma" table))
                                 (second
                                  (all-completions
                                   "ma" table))
                                 (slash
                                  (all-completions
                                   "Math/" table)))
                            (list
                             (butlast capf)
                             first
                             second
                             slash
                             global-calls
                             (nreverse java-calls)
                             (nreverse local-calls)
                             anakondo--completion-candidates-cache)))))"##;
    let expect = expect![[
        r#"OK ((2 4) ("map" "mapv" "manual") ("map" "mapv" "manual") nil 2 ("ma" "Math/") (("ma" 2) ("ma" 2) ("Math/" 2)) (2 "map" "mapv" "Math"))"#
    ]];
    assert_anakondo_parity(elisp_form, expect);
}
