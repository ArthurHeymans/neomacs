use expect_test::expect;

use super::assert_add_hooks_parity;

/// The example in the package's own docstrings, run as written: one function
/// onto two mode hooks, first through `add-hooks-pair' and then through the
/// alist form of `add-hooks'.  Both hook variables are read back and both hooks
/// are then run, so the report says the function was added *and* that it fires.
///
/// The `-hook' suffix is what makes the example read the way it does: the
/// caller writes `css-mode' and the function goes onto `css-mode-hook'.  A name
/// that already ends in `-hook' is left alone, so both spellings can be mixed in
/// one call, and a hook variable nobody has defined yet is simply created by
/// `add-hook' - which is how this works for modes that are not loaded.
#[test]
fn the_documented_example_puts_one_function_on_several_mode_hooks() {
    let elisp_form = r##"(progn
  (add-hooks-test-reset 'css-mode-hook 'sgml-mode-hook 'add-hooks-test-plain-hook)
  (add-hooks-pair '(css-mode sgml-mode) 'add-hooks-test-emmet-mode)
  (let ((by-pair (list :css css-mode-hook
                       :sgml sgml-mode-hook
                       :css-fires (add-hooks-test-fire 'css-mode-hook)
                       :sgml-fires (add-hooks-test-fire 'sgml-mode-hook))))
    (add-hooks-test-reset 'css-mode-hook 'sgml-mode-hook)
    (add-hooks '(((css-mode sgml-mode) . add-hooks-test-emmet-mode)))
    (let ((by-alist (list :css css-mode-hook :sgml sgml-mode-hook)))
      (add-hooks-test-reset 'css-mode-hook 'add-hooks-test-plain-hook)
      (add-hooks-pair '(css-mode add-hooks-test-plain-hook)
                      'add-hooks-test-emmet-mode)
      (list :through-a-pair by-pair
            :through-the-alist by-alist
            :both-forms-agree (equal (plist-get by-pair :css)
                                     (plist-get by-alist :css))
            :suffix-implied (add-hooks-normalize-hook 'css-mode)
            :suffix-kept (add-hooks-normalize-hook 'add-hooks-test-plain-hook)
            :mixed-spellings (list css-mode-hook add-hooks-test-plain-hook)
            :undefined-hook-before (boundp 'add-hooks-test-unheard-of-mode-hook)
            :undefined-hook-added (add-hooks-pair 'add-hooks-test-unheard-of-mode
                                                  'add-hooks-test-emmet-mode)
            :undefined-hook-after (and (boundp 'add-hooks-test-unheard-of-mode-hook)
                                       add-hooks-test-unheard-of-mode-hook)))))"##;
    let expect = expect![
        "OK (:through-a-pair (:css (add-hooks-test-emmet-mode) :sgml (add-hooks-test-emmet-mode) :css-fires (emmet-mode) :sgml-fires (emmet-mode)) :through-the-alist (:css (add-hooks-test-emmet-mode) :sgml (add-hooks-test-emmet-mode)) :both-forms-agree t :suffix-implied css-mode-hook :suffix-kept add-hooks-test-plain-hook :mixed-spellings ((add-hooks-test-emmet-mode) (add-hooks-test-emmet-mode)) :undefined-hook-before nil :undefined-hook-added nil :undefined-hook-after (add-hooks-test-emmet-mode))"
    ];

    assert_add_hooks_parity(elisp_form, expect);
}

/// The reason the package exists: one call instead of six.  Three hooks and two
/// functions produce every combination, and each hook is run so the report shows
/// the order the functions fire in as well as the order they are stored in.
///
/// That order is `add-hook''s doing rather than the package's - each call
/// prepends, so the functions come out in the reverse of the order they were
/// written in the call, and a user reading their own `add-hooks' form cannot
/// assume the first function listed runs first.  Calling the same form twice is
/// asserted to change nothing, since `add-hook' does not add a function that is
/// already there, which is what makes the package safe to put in a config file
/// that gets re-evaluated.  The two readings are asserted to be `eq', not merely
/// `equal': the second call left the identical cons in place rather than
/// rebuilding an equal one, which is why the snapshot shows them as `#1=' and
/// `#1#' back references.
#[test]
fn one_pair_covers_every_hook_crossed_with_every_function() {
    let elisp_form = r##"(progn
  (add-hooks-test-reset 'css-mode-hook 'sgml-mode-hook 'text-mode-hook)
  (add-hooks-pair '(css-mode sgml-mode text-mode)
                  '(add-hooks-test-emmet-mode add-hooks-test-rainbow-mode))
  (let ((once (list :css css-mode-hook
                    :sgml sgml-mode-hook
                    :text text-mode-hook
                    :css-fires (add-hooks-test-fire 'css-mode-hook))))
    (add-hooks-pair '(css-mode sgml-mode text-mode)
                    '(add-hooks-test-emmet-mode add-hooks-test-rainbow-mode))
    (let ((twice (list :css css-mode-hook
                       :sgml sgml-mode-hook
                       :text text-mode-hook)))
      (add-hooks-test-reset 'css-mode-hook 'sgml-mode-hook 'text-mode-hook)
      (add-hooks '((css-mode . add-hooks-test-emmet-mode)
                   ((sgml-mode text-mode) . (add-hooks-test-emmet-mode
                                             add-hooks-test-rainbow-mode))))
      (list :after-one-call once
            :after-the-same-call-again twice
            :unchanged (equal (plist-get once :css) (plist-get twice :css))
            :the-very-same-list (eq (plist-get once :css) (plist-get twice :css))
            :from-an-alist-of-two-pairs
            (list :css css-mode-hook :sgml sgml-mode-hook :text text-mode-hook)
            :and-they-fire (list (add-hooks-test-fire 'css-mode-hook)
                                 (add-hooks-test-fire 'sgml-mode-hook)
                                 (add-hooks-test-fire 'text-mode-hook))))))"##;
    let expect = expect![
        "OK (:after-one-call (:css #1=(add-hooks-test-rainbow-mode add-hooks-test-emmet-mode) :sgml #2=(add-hooks-test-rainbow-mode add-hooks-test-emmet-mode) :text #3=(add-hooks-test-rainbow-mode add-hooks-test-emmet-mode) :css-fires (rainbow-mode emmet-mode)) :after-the-same-call-again (:css #1# :sgml #2# :text #3#) :unchanged t :the-very-same-list t :from-an-alist-of-two-pairs (:css (add-hooks-test-emmet-mode) :sgml (add-hooks-test-rainbow-mode add-hooks-test-emmet-mode) :text (add-hooks-test-rainbow-mode add-hooks-test-emmet-mode)) :and-they-fire ((emmet-mode) (rainbow-mode emmet-mode) (rainbow-mode emmet-mode)))"
    ];

    assert_add_hooks_parity(elisp_form, expect);
}

/// The one piece of judgement in the package.  `add-hooks-listify' has to decide
/// whether what it was handed is a list of functions to iterate or a single
/// function to wrap, and a lambda is both a list and a function - so the test is
/// `(and (listp object) (not (functionp object)))', function-ness winning.
///
/// The three cases are asserted by running the hook rather than by printing what
/// is in it, because a closure's printed form is an implementation detail while
/// the number of times something fires is not.  A single lambda fires once; a
/// list of two lambdas fires twice, in the reverse of the order they were
/// written, as prepending implies; a bare symbol fires once.
///
/// `nil' is the quiet case: it is a list and is not a function, so it iterates
/// over nothing and the call adds nothing at all.  A user whose variable of
/// functions happens to be empty gets silence rather than an error.
#[test]
fn a_lambda_counts_as_one_function_and_a_list_as_several() {
    let elisp_form = r##"(let ((hook 'add-hooks-test-plain-hook))
  (add-hooks-test-reset hook)
  (add-hooks-pair 'add-hooks-test-plain (add-hooks-test-recorder 'single))
  (let ((single (list :entries (length add-hooks-test-plain-hook)
                      :fires (add-hooks-test-fire hook))))
    (add-hooks-test-reset hook)
    (add-hooks-pair 'add-hooks-test-plain
                    (list (add-hooks-test-recorder 'first)
                          (add-hooks-test-recorder 'second)))
    (let ((several (list :entries (length add-hooks-test-plain-hook)
                         :fires (add-hooks-test-fire hook))))
      (add-hooks-test-reset hook)
      (add-hooks-pair 'add-hooks-test-plain 'add-hooks-test-emmet-mode)
      (let ((symbol (list :entries (length add-hooks-test-plain-hook)
                          :fires (add-hooks-test-fire hook))))
        (add-hooks-test-reset hook)
        (add-hooks-pair 'add-hooks-test-plain nil)
        (list :one-lambda single
              :two-lambdas several
              :one-symbol symbol
              :nil-functions (list :entries (length add-hooks-test-plain-hook)
                                   :value add-hooks-test-plain-hook)
              :listify (list :lambda (length (add-hooks-listify
                                              (add-hooks-test-recorder 'x)))
                             :list-of-two (length (add-hooks-listify '(a b)))
                             :symbol (add-hooks-listify 'add-hooks-test-emmet-mode)
                             :nil (add-hooks-listify nil)))))))"##;
    let expect = expect![
        "OK (:one-lambda (:entries 1 :fires (single)) :two-lambdas (:entries 2 :fires (second first)) :one-symbol (:entries 1 :fires (emmet-mode)) :nil-functions (:entries 0 :value nil) :listify (:lambda 1 :list-of-two 2 :symbol (add-hooks-test-emmet-mode) :nil nil))"
    ];

    assert_add_hooks_parity(elisp_form, expect);
}

/// Two ways of getting it wrong, which fail at opposite ends.
///
/// A hook named with a string is rejected immediately:
/// `add-hooks-normalize-hook' only rewrites symbols, so the string reaches
/// `add-hook' unchanged and signals `wrong-type-argument symbolp' before
/// anything is added.  The mistake is reported where it was made.
///
/// A list of ordinary symbols that are not functions is accepted without a
/// murmur.  It is a list and is not a function, so every element is added as
/// though it were a hook function, and the hook variable afterwards looks
/// perfectly ordinary - the mistake only surfaces later, when something runs the
/// hook and Emacs signals `void-function' for a symbol the user never meant as a
/// function.  That is the same shape as passing an unevaluated form where a
/// function was wanted, which is the likeliest way to reach it.
#[test]
fn a_string_hook_is_refused_at_once_and_a_list_of_non_functions_is_not() {
    let elisp_form = r##"(progn
  (add-hooks-test-reset 'css-mode-hook 'add-hooks-test-plain-hook)
  (list
   :string-hook
   (list :signal (condition-case error
                     (add-hooks-pair "css-mode" 'add-hooks-test-emmet-mode)
                   (error (list (car error) (cadr error))))
         :hook-untouched css-mode-hook)
   :list-of-non-functions
   (progn
     (add-hooks-pair 'add-hooks-test-plain '(alpha beta))
     (list :stored add-hooks-test-plain-hook
           :looks-normal (listp add-hooks-test-plain-hook)
           :signal-when-run (condition-case error
                                (add-hooks-test-fire 'add-hooks-test-plain-hook)
                              (error (list (car error) (cadr error))))))
   :unevaluated-form
   (progn
     (add-hooks-test-reset 'add-hooks-test-plain-hook)
     (add-hooks-pair 'add-hooks-test-plain '(setq add-hooks-test-fired 'oops))
     (list :stored add-hooks-test-plain-hook
           :signal-when-run (condition-case error
                                (add-hooks-test-fire 'add-hooks-test-plain-hook)
                              (error (list (car error) (cadr error))))))))"##;
    let expect = expect![
        "OK (:string-hook (:signal (wrong-type-argument symbolp) :hook-untouched nil) :list-of-non-functions (:stored (beta alpha) :looks-normal t :signal-when-run (void-function beta)) :unevaluated-form (:stored (#1='oops add-hooks-test-fired setq) :signal-when-run (invalid-function #1#)))"
    ];

    assert_add_hooks_parity(elisp_form, expect);
}
