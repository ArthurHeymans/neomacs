use expect_test::expect;

use super::assert_anaphora_parity;

/// The reason to reach for `aif' at all: look something up once, then use the
/// result without naming it.  The lookup counter is the point -- four forms,
/// four lookups, so no macro evaluates its tested form twice -- and the else
/// branches show what plain `if' cannot give you, `it' bound to the nil the
/// lookup returned.  `awhen' returns nil for a missing project and the value of
/// its last body form for a found one, including when that project is empty.
#[test]
fn looking_a_project_up_binds_it_in_both_branches_and_evaluates_the_lookup_once() {
    let elisp_form = r##"(let ((lookups 0))
  (cl-flet ((project (name)
              (setq lookups (1+ lookups))
              (anaphora-test-project name)))
    (list :found (aif (project "neomacs")
                     (list (plist-get it :name) (length (plist-get it :tasks)))
                   :missing)
          :missing (aif (project "nope")
                       (plist-get it :name)
                     (list :fallback it))
          :when-found (awhen (project "scratch")
                        (list (plist-get it :name) (plist-get it :tasks)))
          :when-missing (awhen (project "nope") :never)
          :lookups lookups)))"##;
    let expect = expect![[
        r#"OK (:found ("neomacs" 3) :missing (:fallback nil) :when-found ("scratch" nil) :when-missing nil :lookups 4)"#
    ]];

    assert_anaphora_parity(elisp_form, expect);
}

/// Nesting is where anaphoric macros could go wrong: each form rebinds `it',
/// and the binding has to be the innermost one inside its body and the outer
/// one again afterwards.  Reading a project, then its owner, then that owner's
/// address gives three nested `it's; the outer project is still `it' on the
/// line after the inner forms, and a closure made inside the outermost body
/// still sees it.
#[test]
fn nested_anaphoric_forms_shadow_it_and_restore_the_outer_binding() {
    let elisp_form = r##"(alet (anaphora-test-project "neomacs")
  (list :outer (plist-get it :name)
        :inner (awhen (plist-get it :owner)
                 (list :login (plist-get it :login)
                       :deeper (aif (plist-get it :email)
                                   (upcase it)
                                 :none)))
        :restored (plist-get it :name)
        :captured (funcall (lambda () (plist-get it :name)))
        :second-project (alet (anaphora-test-project "scratch")
                          (list (plist-get it :name)
                                (awhen (plist-get it :owner)
                                  (aif (plist-get it :email)
                                      (upcase it)
                                    (list :no-email (plist-get it :login))))))
        :restored-again (plist-get it :name)))"##;
    let expect = expect![[
        r#"OK (:outer "neomacs" :inner (:login "eval-exec" :deeper "EXEC@EXAMPLE.COM") :restored "neomacs" :captured "neomacs" :second-project ("scratch" (:no-email nil)) :restored-again "neomacs")"#
    ]];

    assert_anaphora_parity(elisp_form, expect);
}

/// `aand' is the idiom for walking into data that may not be there: each step
/// sees the previous step's value as `it', and the first nil ends the chain.
/// The recorded step labels prove both halves -- the full walk runs all three
/// steps, and the project whose owner has no e-mail stops after the step that
/// returned nil, so `upcase' is never called on nil.
#[test]
fn aand_walks_into_nested_data_and_stops_at_the_first_missing_step() {
    let elisp_form = r##"(let ((steps nil))
  (cl-flet ((note (label value) (push label steps) value))
    (let ((full (aand (anaphora-test-project "neomacs")
                      (note :owner (plist-get it :owner))
                      (note :email (plist-get it :email))
                      (note :upcase (upcase it))))
          (short (aand (anaphora-test-project "scratch")
                       (note :owner (plist-get it :owner))
                       (note :email (plist-get it :email))
                       (note :upcase (upcase it))))
          (none (aand (anaphora-test-project "nope")
                      (note :never (plist-get it :owner)))))
      (list :full full
            :short short
            :none none
            :steps (nreverse steps)))))"##;
    let expect = expect![[
        r#"OK (:full "EXEC@EXAMPLE.COM" :short nil :none nil :steps (:owner :email :upcase :owner :email))"#
    ]];

    assert_anaphora_parity(elisp_form, expect);
}

/// `acond' binds each clause's own test result in that clause's body, which is
/// what makes it worth using over `cond': the classifier below reads
/// `(plist-get task :points)' once and then uses the number.  A clause with no
/// body returns the tested value itself, and a record matching nothing falls
/// through to the final clause.
#[test]
fn acond_classifies_each_record_with_the_value_its_own_clause_tested() {
    let elisp_form = r##"(let ((tests 0))
  (cl-flet ((points (task) (setq tests (1+ tests)) (plist-get task :points)))
    (list :classified
          (mapcar (lambda (task)
                    (acond
                     ((eq (plist-get task :state) 'done) (list :done it))
                     ((points task) (list :estimated it (* it 2)))
                     ((plist-get task :title))
                     (t :unknown)))
                  (anaphora-test-tasks "neomacs"))
          :tests tests
          :no-clause-matches (acond (nil :never) ((cdr nil) :never-either))
          :bare-clause-value (acond ((plist-get (car (anaphora-test-tasks "neomacs")) :title))))))"##;
    let expect = expect![[
        r#"OK (:classified ((:done t) (:estimated 8 16) "write docs") :tests 2 :no-clause-matches nil :bare-clause-value "port isearch")"#
    ]];

    assert_anaphora_parity(elisp_form, expect);
}

/// A small report built the way the library intends: `alambda' gives the tree
/// walker a name for itself (`self') without a `letrec', `awhile' rebinds `it'
/// to each item as a queue drains, and the arithmetic macros thread the running
/// value through as `it'.  The last entry pins a sharp edge of that threading:
/// in `a-' the binding starts at the first *subtrahend*, so `it' in the
/// dividend position is unbound.
#[test]
fn a_recursive_walk_a_work_queue_and_the_arithmetic_macros_build_a_report() {
    let elisp_form = r##"(let* ((points (mapcar (lambda (task) (or (plist-get task :points) 0))
                       (anaphora-test-tasks "neomacs")))
       (tree (list 1 (list 2 (list 3 4)) 5))
       (total (funcall (alambda (node)
                         (cond ((null node) 0)
                               ((numberp node) node)
                               (t (+ (self (car node)) (self (cdr node))))))
                       tree))
       (queue (mapcar (lambda (task) (plist-get task :title))
                      (anaphora-test-tasks "neomacs")))
       (seen nil))
  (awhile (pop queue)
    (push (list it (length it)) seen))
  (list :tree-total total
        :titles (nreverse seen)
        :points points
        :sum (a+ 2 (* it 3) (- it 1))
        :product (a* 3 (+ it 1) it)
        :difference (a- 10 (+ 2 2) it)
        :quotient (a/ 100 5 it)
        :no-it-in-the-dividend (condition-case error (a- 10 (/ it 2) it) (error error))
        :empty-sums (list (a+) (a*) (a- 4))))"##;
    let expect = expect![[
        r#"OK (:tree-total 15 :titles (("port isearch" 12) ("fix the collector" 17) ("write docs" 10)) :points (5 8 0) :sum 13 :product 48 :difference 2 :quotient 4 :no-it-in-the-dividend (void-variable it) :empty-sums (0 1 -4))"#
    ]];

    assert_anaphora_parity(elisp_form, expect);
}

/// Both spellings are the same macro: the traditional `aif' is a `defalias' to
/// `anaphoric-if', and the alias carries the indentation and edebug metadata
/// the long name has, which is what makes it behave in the editor.  Passing a
/// negative argument to the installer removes exactly the aliases it created
/// -- the long names keep working throughout -- and reinstalling restores them.
#[test]
fn the_long_names_are_the_same_macros_and_the_short_aliases_carry_their_metadata() {
    let elisp_form = r##"(let ((installed
       (list :same-result
             (list (anaphoric-if (anaphora-test-project "neomacs") (plist-get it :name) :none)
                   (aif (anaphora-test-project "neomacs") (plist-get it :name) :none))
             :alias (symbol-function 'aif)
             :indent (list (get 'aif 'lisp-indent-function)
                           (get 'anaphoric-if 'lisp-indent-function)
                           (get 'awhen 'lisp-indent-function))
             :edebug (list (get 'aif 'edebug-form-spec)
                           (get 'acond 'edebug-form-spec)
                           (get 'alambda 'edebug-form-spec))
             :long-names-only anaphora-use-long-names-only)))
  (anaphora--install-traditional-aliases -1)
  (let ((removed (list :short (list (fboundp 'aif) (fboundp 'acond) (fboundp 'a+))
                       :long (list (fboundp 'anaphoric-if) (fboundp 'anaphoric-cond))
                       :long-still-works
                       (anaphoric-when (anaphora-test-project "scratch")
                         (plist-get it :name)))))
    (anaphora--install-traditional-aliases)
    (list :installed installed
          :after-removal removed
          :after-reinstall (list (fboundp 'aif)
                                 (symbol-function 'a+)
                                 (aif (anaphora-test-project "scratch")
                                     (plist-get it :name)
                                   :none)))))"##;
    let expect = expect![[
        r#"OK (:installed (:same-result ("neomacs" "neomacs") :alias anaphoric-if :indent (2 2 1) :edebug (t cond lambda) :long-names-only nil) :after-removal (:short (nil nil nil) :long (t t) :long-still-works "scratch") :after-reinstall (t anaphoric-+ "scratch"))"#
    ]];

    assert_anaphora_parity(elisp_form, expect);
}

/// The macros have to mean the same thing in compiled code, which is where
/// most of this library's users meet it.  The same function -- an `aand' walk,
/// an `acond' classifier, an `alambda' recursion and an `awhile' loop -- is
/// evaluated interpreted and byte-compiled, and the two must agree on every
/// answer while only one of them is a byte-code object.
#[test]
fn byte_compiling_the_same_anaphoric_code_gives_the_same_answers() {
    let elisp_form = r##"(let* ((source '(lambda (projects)
                    (list :login (aand (car projects)
                                       (plist-get it :owner)
                                       (plist-get it :login)
                                       (upcase it))
                          :states (mapcar (lambda (task)
                                            (acond
                                             ((eq (plist-get task :state) 'done) (list :done it))
                                             ((plist-get task :points) (* it 10))
                                             (t :none)))
                                          (plist-get (car projects) :tasks))
                          :depth (funcall (alambda (node)
                                            (if (consp node)
                                                (1+ (self (car node)))
                                              0))
                                          '(((1))))
                          :drained (let ((queue (list 3 2 1)) (seen nil))
                                     (awhile (pop queue)
                                       (push (* it it) seen))
                                     seen))))
       (interpreted (eval source t))
       (compiled (byte-compile (eval source t)))
       (interpreted-result (funcall interpreted anaphora-test-projects))
       (compiled-result (funcall compiled anaphora-test-projects)))
  (list :interpreted interpreted-result
        :agree (equal interpreted-result compiled-result)
        :compiled-is-byte-code (byte-code-function-p compiled)
        :interpreted-is-byte-code (byte-code-function-p interpreted)))"##;
    let expect = expect![[
        r#"OK (:interpreted (:login "EVAL-EXEC" :states ((:done t) 80 :none) :depth 3 :drained (1 4 9)) :agree t :compiled-is-byte-code t :interpreted-is-byte-code nil)"#
    ]];

    assert_anaphora_parity(elisp_form, expect);
}
