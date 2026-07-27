use expect_test::expect;

use super::assert_align_cljlet_parity;

#[test]
fn align_cljlet_aligns_practical_let_when_if_binding_loop_and_resource_forms() {
    let elisp_form = r##"(prin1-to-string
 (mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char 2)
     (align-cljlet)
     (list (buffer-string) (point) (current-column))))
 '("(let [host \"localhost\"\n port 8080\n timeout-ms 5000]\n  [host port timeout-ms RESULT])"
   "(when-let [user (find-user id)\n permissions (load-permissions user)]\n  [user permissions RESULT])"
   "(if-let [cached (cache-get key)\n calculated (expensive-compute key)]\n  calculated\n  RESULT)"
   "(binding [*out* writer\n *print-length* 100]\n  (prn RESULT))"
   "(loop [remaining jobs\n completed []\n failures {}]\n  RESULT)"
   "(with-open [reader (open-reader path)\n writer (open-writer target)]\n  (copy reader writer)\n  RESULT)")))"##;
    let expect = expect![[
        r#"OK "((\"(let [host       \\\"localhost\\\"\\n      port       8080\\n      timeout-ms 5000]\\n  [host port timeout-ms RESULT])\" 2 1) (\"(when-let [user        (find-user id)\\n           permissions (load-permissions user)]\\n  [user permissions RESULT])\" 2 1) (\"(if-let [cached     (cache-get key)\\n         calculated (expensive-compute key)]\\n  calculated\\n  RESULT)\" 2 1) (\"(binding [*out*          writer\\n          *print-length* 100]\\n  (prn RESULT))\" 2 1) (\"(loop [remaining jobs\\n       completed []\\n       failures  {}]\\n  RESULT)\" 2 1) (\"(with-open [reader (open-reader path)\\n            writer (open-writer target)]\\n  (copy reader writer)\\n  RESULT)\" 2 1))""#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_aligns_metadata_destructuring_namespaced_keys_and_unicode_values() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char 2)
     (align-cljlet)
     (buffer-string)))
 '("(let [^String short \"α\"\n ^java.time.Instant timestamp (now)\n {:keys [host port]} config]\n  [short timestamp host port])"
   "(let [{:user/keys [name email]} account\n [first-item & remaining-items] items\n {:keys [locale] :or {locale \"日本語\"}} preferences]\n  [name email first-item remaining-items locale])"
   "(let [^:dynamic *compact* true\n ^{:tag long :private true} sequence-number 42]\n  [*compact* sequence-number])"))"##;
    let expect = expect![[
        r#"OK ("(let [^String short                \"α\"\n      ^java.time.Instant timestamp (now)\n      {:keys [host port]}          config]\n  [short timestamp host port])" "(let [{:user/keys [name email]}              account\n      [first-item & remaining-items]         items\n      {:keys [locale] :or {locale \"日本語\"}} preferences]\n  [name email first-item remaining-items locale])" "(let [^:dynamic *compact*                        true\n      ^{:tag long :private true} sequence-number 42]\n  [*compact* sequence-number])")"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_aligns_for_bindings_with_filters_while_preserving_body_structure() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char 2)
     (align-cljlet)
     (buffer-string)))
 '("(for [user users\n :let [orders (orders-for user)]\n order orders]\n  (yield user order))"
   "(for [x (range 20)\n :when (odd? x)\n y (range x)\n :while (< y 4)]\n  (yield x y))"
   "(for [[department employees] organization\n employee employees]\n  (yield department employee))"))"##;
    let expect = expect![[
        r#"OK ("(for [user  users\n      :let  [orders (orders-for user)]\n      order orders]\n  (yield user order))" "(for [x      (range 20)\n      :when  (odd? x)\n      y      (range x)\n      :while (< y 4)]\n  (yield x y))" "(for [[department employees] organization\n      employee               employees]\n  (yield department employee))")"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_aligns_literal_maps_with_keywords_tagged_literals_and_nested_values() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char 2)
     (condition-case err
         (progn
           (align-cljlet)
           (list 'aligned (buffer-string)))
       (error
        (list 'error (car err) (error-message-string err)
              (buffer-string))))))
 '("{:id 7\n :display-name \"Ada Lovelace\"\n :roles #{:admin :author}\n :profile {:active true :target 'TARGET}}"
   "{#uuid \"550e8400-e29b-41d4-a716-446655440000\" :identifier\n :short 1\n :configuration (merge defaults overrides)\n :target TARGET}"
   "{[:composite :key] {:nested [1 2 3]}\n :x nil\n :long-keyword-name (fn [value] (* value value))\n :target TARGET}"))"##;
    let expect = expect![[
        r#"OK ((aligned "{:id           7\n :display-name \"Ada Lovelace\"\n :roles        #{:admin :author}\n :profile      {:active true :target 'TARGET}}") (aligned "{#uuid \"550e8400-e29b-41d4-a716-446655440000\" :identifier\n :short                                       1\n :configuration                               (merge defaults overrides)\n :target                                      TARGET}") (aligned "{[:composite :key]  {:nested [1 2 3]}\n :x                 nil\n :long-keyword-name (fn [value] (* value value))\n :target            TARGET}"))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_skips_discarded_reader_forms_when_calculating_binding_widths() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char 2)
     (align-cljlet)
     (buffer-string)))
 '("(let [short #_(slow-database-call) 1\n substantially-longer 2]\n  (+ short substantially-longer))"
   "(let [#_(remove-after-migration) legacy-value 1\n current-value 2\n exceptionally-long-current-value 3]\n  current-value)"
   "(let [#_(old-source) source #_(old-transform) (load-data)\n transformed-result (transform source)]\n  transformed-result)"
   "{#_(obsolete-key) :old #_(obsolete-value) 0\n :current-value 1\n :substantially-longer-key 2}"))"##;
    let expect = expect![[
        r#"OK ("(let [short                #_(slow-database-call) 1\n      substantially-longer 2]\n  (+ short substantially-longer))" "(let [#_(remove-after-migration) legacy-value 1\n      current-value                           2\n      exceptionally-long-current-value        3]\n  current-value)" "(let [#_(old-source) source #_(old-transform) (load-data)\n      transformed-result    (transform source)]\n  transformed-result)" "{#_(obsolete-key) :old     #_(obsolete-value) 0\n :current-value            1\n :substantially-longer-key 2}")"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_aligns_cond_condp_case_and_alt_dispatch_tables() {
    let elisp_form = r##"(prin1-to-string
 (mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char 2)
     (align-cljlet)
     (buffer-string)))
 '("(cond\n (> score 90) :excellent\n (> score 75) :good\n (nil? score) :missing\n :else TARGET)"
   "(condp = status\n :queued (enqueue TARGET)\n :running (monitor TARGET)\n :finished (archive TARGET)\n :else (reject TARGET))"
   "(case event-type\n :created (handle-created TARGET)\n :updated (handle-updated TARGET)\n :deleted (handle-deleted TARGET)\n (handle-unknown TARGET))"
   "(alt!\n request-channel ([request] (serve request TARGET))\n shutdown-channel ([_] (stop TARGET)))")))"##;
    let expect = expect![[
        r#"OK "(\"(cond\\n  (> score 90) :excellent\\n  (> score 75) :good\\n  (nil? score) :missing\\n  :else        TARGET)\" \"(condp = status\\n  :queued   (enqueue TARGET)\\n  :running  (monitor TARGET)\\n  :finished (archive TARGET)\\n  :else     (reject TARGET))\" \"(case event-type\\n  :created (handle-created TARGET)\\n  :updated (handle-updated TARGET)\\n  :deleted (handle-deleted TARGET)\\n  (handle-unknown TARGET))\" \"(alt!\\n  request-channel  ([request] (serve request TARGET))\\n  shutdown-channel ([_] (stop TARGET)))\")""#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_from_nested_body_aligns_only_the_nearest_lexical_form() {
    let elisp_form = r##"(with-temp-buffer
  (clojure-mode)
  (insert "(let [outer 1\n much-longer-outer 2]\n  (when-let [x (find-x)\n              exceptionally-long-inner (find-inner)]\n    {:x x\n     :inner exceptionally-long-inner\n     :marker TARGET}))")
  (goto-char (point-min))
  (search-forward "TARGET")
  (let ((before (buffer-string)))
    (align-cljlet)
    (let ((after-inner (buffer-string)))
      (goto-char (point-min))
      (search-forward "outer")
      (align-cljlet)
      (list before after-inner (buffer-string) (point)))))"##;
    let expect = expect![[
        r#"OK ("(let [outer 1\n much-longer-outer 2]\n  (when-let [x (find-x)\n              exceptionally-long-inner (find-inner)]\n    {:x x\n     :inner exceptionally-long-inner\n     :marker TARGET}))" "(let [outer 1\n much-longer-outer 2]\n  (when-let [x (find-x)\n              exceptionally-long-inner (find-inner)]\n    {:x      x\n     :inner  exceptionally-long-inner\n     :marker TARGET}))" "(let [outer             1\n      much-longer-outer 2]\n  (when-let [x (find-x)\n              exceptionally-long-inner (find-inner)]\n    {:x      x\n     :inner  exceptionally-long-inner\n     :marker TARGET}))" 12)"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_is_idempotent_on_already_aligned_realistic_forms() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (buffer-enable-undo)
     (insert source)
     (setq buffer-undo-list nil)
     (goto-char 2)
     (align-cljlet)
     (let ((once (buffer-string))
           (undo-after-once (length buffer-undo-list)))
       (align-cljlet)
       (list once (buffer-string)
             (equal once (buffer-string))
             undo-after-once
             (length buffer-undo-list)))))
 '("(let [a 1\n substantially-longer-name 2]\n  (+ a substantially-longer-name))"
   "{:a 1\n :substantially-longer-key 2}"
   "(cond\n tiny 1\n substantially-longer-predicate 2\n :else 3)"))"##;
    let expect = expect![[
        r#"OK (("(let [a                         1\n      substantially-longer-name 2]\n  (+ a substantially-longer-name))" "(let [a                         1\n      substantially-longer-name 2]\n  (+ a substantially-longer-name))" t 2 2) ("{:a                        1\n :substantially-longer-key 2}" "{:a                        1\n :substantially-longer-key 2}" t 1 1) ("(cond\n  tiny                           1\n  substantially-longer-predicate 2\n  :else                          3)" "(cond\n  tiny                           1\n  substantially-longer-predicate 2\n  :else                          3)" t 5 5))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_command_preserves_point_and_mark_via_save_excursion_while_modifying_text() {
    let elisp_form = r##"(with-temp-buffer
  (clojure-mode)
  (buffer-enable-undo)
  (insert "(let [short 1\n exceptionally-long-binding 2]\n  (+ short exceptionally-long-binding))")
  (goto-char (point-min))
  (search-forward "exceptionally")
  (set-mark 8)
  (let ((point-before (point))
        (mark-before (mark))
        (modified-before (buffer-modified-p)))
    (align-cljlet)
    (list point-before (point)
          mark-before (mark)
          modified-before (buffer-modified-p)
          (buffer-string)
          (consp buffer-undo-list))))"##;
    let expect = expect![[
        r#"OK (29 55 8 8 t t "(let [short                      1\n      exceptionally-long-binding 2]\n  (+ short exceptionally-long-binding))" t)"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_records_exact_real_buffer_undo_behavior_after_alignment() {
    let elisp_form = r##"(with-temp-buffer
  (clojure-mode)
  (buffer-enable-undo)
  (insert "(let [a 1\n exceptionally-long-name 2\n medium 3]\n  [a exceptionally-long-name medium])")
  (setq buffer-undo-list nil)
  (goto-char 2)
  (let ((original (buffer-string)))
    (align-cljlet)
    (let ((aligned (buffer-string))
          (undo-records (copy-tree buffer-undo-list)))
      (let ((undo-result
             (condition-case err
                 (progn (undo) 'undone)
               (error (list (car err) (error-message-string err))))))
        (list original aligned (buffer-string)
              (equal original (buffer-string))
              undo-records undo-result)))))"##;
    let expect = expect![[
        r#"OK ("(let [a 1\n exceptionally-long-name 2\n medium 3]\n  [a exceptionally-long-name medium])" "(let [a                       1\n      exceptionally-long-name 2\n      medium                  3]\n  [a exceptionally-long-name medium])" "(let [a 1\n exceptionally-long-name 2\n medium 3]\n  [a exceptionally-long-name medium])" t ((66 . 71) (34 . 39) (68 . 85) (9 . 31)) (user-error "No further undo information"))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_handles_multiline_values_strings_comments_and_commas_without_corrupting_data() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char 2)
     (align-cljlet)
     (buffer-string)))
 '("(let [short (-> input\n                 normalize\n                 validate)\n very-long-binding-name \"spaces  inside  string\"]\n  [short very-long-binding-name])"
   "(let [short 1, ; retained explanation\n much-longer-name (+ short 2),\n final-result (* much-longer-name 3)]\n  final-result)"
   "(let [url \"https://example.test/a?x=1  2\"\n request-options {:headers {\"x-name\" \"A  B\"}}\n response (send url request-options)]\n  response)"))"##;
    let expect = expect![[
        r#"OK ("(let [short                  (-> input\n                                 normalize\n                                 validate)\n      very-long-binding-name \"spaces  inside  string\"]\n  [short very-long-binding-name])" "(let [short            1, ; retained explanation\n      much-longer-name (+ short 2),\n      final-result     (* much-longer-name 3)]\n  final-result)" "(let [url             \"https://example.test/a?x=1  2\"\n      request-options {:headers {\"x-name\" \"A  B\"}}\n      response        (send url request-options)]\n  response)")"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_respace_subform_adds_and_removes_exact_columns_across_multiple_fields() {
    let elisp_form = r##"(mapcar
 (lambda (case)
   (with-temp-buffer
     (clojure-mode)
     (insert (car case))
     (goto-char (point-min))
     (down-list)
     (acl-respace-subform (cadr case))
     (list case (buffer-string) (point))))
 '(("(GET \"/\" [] (home))" (8 5 7))
   ("(POST       \"/users\"   [request]      (create request))" (5 10 4))
   ("(alpha     1   extra)" (6 2))
   ("(^String short     \"value\")" (18))))"##;
    let expect = expect![[
        r#"OK ((("(GET \"/\" [] (home))" (8 5 7)) "(GET      \"/\"   []      (home))" 2) (("(POST       \"/users\"   [request]      (create request))" (5 10 4)) "(POST  \"/users\"   [requ(create request))" 2) (("(alpha     1   extra)" (6 2)) "(alpha  1  extra)" 2) (("(^String short     \"value\")" (18)) "(^String short      \"value\")" 2))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}
