use expect_test::expect;

use super::assert_attrap_parity;

#[test]
fn attrap_ghc_fixer_exposes_specific_pragma_failures_and_applies_generic_extension_options() {
    let elisp_form = r##"(mapcar
          (lambda (case)
            (pcase-let
              ((`(,message ,contents ,option)
                  case))
              (attrap-test-error-data
               (lambda ()
                 (attrap-test-run-fixer-option
                  'attrap-ghc-fixer
                  message
                  contents
                  option)))))
          '(("Parse error in pattern: pattern"
             "module Demo where\n\n«POINT»pattern Pair x y = (x, y)\n"
             0)
            ("parse error on input ‘case’"
             "module Demo where\n\nrender = «POINT»\\case\n  Just x -> x\n"
             0)
            ("Perhaps you intended to use a language extension to enable explicit-forall syntax"
             "module Demo where\n\n«POINT»identity :: forall a. a -> a\n"
             0)
            ("Illegal constraint: Eq (f a); FlexibleContexts is required"
             "module Demo where\n\n«POINT»render :: Eq (f a) => f a -> String\n"
             0)
            ("The declarations require ViewPatterns and TupleSections"
             "module Demo where\n\n«POINT»pair = (, value)\n"
             1)))"##;
    let expect = expect![[
        r#"OK ((:error wrong-type-argument (listp #[nil ((set-match-data saved-match-data 'evaporate) (goto-char 1) (insert (concat "{-# LANGUAGE " "PatternSynonyms" " #-}\n"))) ((saved-match-data 0 31))])) (:ok (nil "module Demo where\n\nrender = \\case\n  Just x -> x\n" ((:error void-function (nil)) "module Demo where\n\nrender = \\case\n  Just x -> x\n" 29))) (:ok ((((use-extension "ScopedTypeVariables") t)) "module Demo where\n\nidentity :: forall a. a -> a\n" ((:ok nil) "{-# LANGUAGE ScopedTypeVariables #-}\nmodule Demo where\n\nidentity :: forall a. a -> a\n" 38))) (:ok ((((use-extension "FlexibleContexts") t)) "module Demo where\n\nrender :: Eq (f a) => f a -> String\n" ((:ok nil) "{-# LANGUAGE FlexibleContexts #-}\nmodule Demo where\n\nrender :: Eq (f a) => f a -> String\n" 35))) (:ok ((((use-extension "TupleSections") t) ((use-extension "ViewPatterns") t)) "module Demo where\n\npair = (, value)\n" ((:ok nil) "{-# LANGUAGE ViewPatterns #-}\nmodule Demo where\n\npair = (, value)\n" 31))))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_inserts_missing_instance_methods_and_associated_types() {
    let elisp_form = r##"(mapcar
          (lambda (case)
            (apply
             #'attrap-test-run-fixer-option
             'attrap-ghc-fixer
             (append case '(0))))
          '(("No explicit implementation for ‘encode’, ‘decode’\nIn the instance declaration for ‘Codec Demo’"
             "instance Codec Demo where«POINT»\n")
            ("No explicit associated type or default declaration for ‘Item’"
             "instance Collection Demo where«POINT»\n")))"##;
    let expect = expect![[
        r#"OK ((((insert-method t)) "instance Codec Demo where\n" ((:ok nil) "instance Codec Demo where\n  encode = _\n  decode = _\n" 52)) (((insert-type t)) "instance Collection Demo where\n" ((:ok nil) "instance Collection Demo where\n  type Item = _\n" 47)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_replaces_kind_star_and_adds_only_the_missing_data_kind_import() {
    let elisp_form = r##"(mapcar
          (lambda (contents)
            (attrap-test-run-fixer-option
             'attrap-ghc-fixer
             "Using ‘*’ (or its Unicode variant) to mean ‘Data.Kind.Type’ relies on StarIsType"
             contents
             0))
          '("module Demo where\n\nkindOf :: «POINT»* -> Type\n"
            "module Demo where\nimport Data.Kind (Type)\n\nkindOf :: «POINT»* -> Type\n"))"##;
    let expect = expect![[
        r#"OK ((((replace-star-by-Type t)) "module Demo where\n\nkindOf :: * -> Type\n" ((:ok nil) "module Demo where\nimport Data.Kind (Type)\n\nkindOf :: Type -> Type\n" 42)) (((replace-star-by-Type t)) "module Demo where\nimport Data.Kind (Type)\n\nkindOf :: * -> Type\n" ((:ok nil) "module Demo where\nimport Data.Kind (Type)\n\nkindOf :: Type -> Type\n" 19)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_turns_each_valid_hole_fit_into_a_distinct_practical_replacement() {
    let elisp_form = r##"(mapcar
          (lambda (option)
            (attrap-test-run-fixer-option
             'attrap-ghc-fixer
             "Found hole: _result :: Int\nValid hole fits include\n  computedValue :: Int\n  fallbackValue :: Int\n  maxBound :: Int\n"
             "module Demo where\n\nresult = «POINT»_\n"
             option))
          '(0 1 2))"##;
    let expect = expect![[
        r#"OK (((((plug-hole "computedValue") t) ((plug-hole "fallbackValue") t) ((plug-hole "maxBound") t)) "module Demo where\n\nresult = _\n" ((:ok nil) "module Demo where\n\nresult = computedValue\n" 42)) ((((plug-hole "computedValue") t) ((plug-hole "fallbackValue") t) ((plug-hole "maxBound") t)) "module Demo where\n\nresult = _\n" ((:ok nil) "module Demo where\n\nresult = fallbackValue\n" 42)) ((((plug-hole "computedValue") t) ((plug-hole "fallbackValue") t) ((plug-hole "maxBound") t)) "module Demo where\n\nresult = _\n" ((:ok nil) "module Demo where\n\nresult = maxBound\n" 37)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_repairs_redundant_constraints_and_missing_contexts_around_forall() {
    let elisp_form = r##"(list
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Redundant constraint: (Eq a)"
           "module Demo where\n\ndemo :: «POINT»(Eq a, Show a) => a -> String\ndemo = show\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Redundant constraint: (Eq a)"
           "module Demo where\n\nidentity :: «POINT»Eq a => a -> a\nidentity value = value\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Could not deduce (Show a)\nadd (Show a) to the context of\n the type signature for:\n demo ::"
           "module Demo where\n\ndemo :: forall a. a -> String\n«POINT»demo value = show value\n"
           0))"##;
    let expect = expect![[
        r#"OK ((((delete-redundant-constraint t)) "module Demo where\n\ndemo :: (Eq a, Show a) => a -> String\ndemo = show\n" ((:ok nil) "module Demo where\n\ndemo :: ( Show a) => a -> String\ndemo = show\n" 29)) (((delete-redundant-constraint t)) "module Demo where\n\nidentity :: Eq a => a -> a\nidentity value = value\n" ((:ok nil) "module Demo where\n\nidentity ::  a -> a\nidentity value = value\n" 32)) (((add-constraint-to-context t)) "module Demo where\n\ndemo :: forall a. a -> String\ndemo value = show value\n" ((:ok nil) "module Demo where\n\ndemo :: forall a. Show a => a -> String\ndemo value = show value\n" 48)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_adds_bindings_and_signatures_at_the_reported_declaration() {
    let elisp_form = r##"(mapcar
          (lambda (case)
            (apply
             #'attrap-test-run-fixer-option
             'attrap-ghc-fixer
             (append case '(0))))
          '(("The type signature for ‘compute’ lacks an accompanying binding"
             "module Demo where\n\n«POINT»compute :: Int -> Int\nnext = 1\n")
            ("Top-level binding with no type signature:\n compute :: Int -> Int"
             "module Demo where\n\n«POINT»compute value = value + 1\n")
            ("Pattern synonym with no type signature:\n pattern Pair :: a -> b -> (a, b)"
             "module Demo where\n\n«POINT»pattern Pair x y = (x, y)\n")))"##;
    let expect = expect![[
        r#"OK ((((add-binding t)) "module Demo where\n\ncompute :: Int -> Int\nnext = 1\n" ((:ok nil) "module Demo where\n\ncompute :: Int -> Int\ncompute = _\nnext = 1\n" 54)) (((add-signature t)) "module Demo where\n\ncompute value = value + 1\n" ((:ok nil) "module Demo where\n\ncompute :: Int -> Int\ncompute value = value + 1\n" 42)) (((add-signature t)) "module Demo where\n\npattern Pair x y = (x, y)\n" ((:ok nil) "module Demo where\n\npattern Pair :: a -> b -> (a, b)\npattern Pair x y = (x, y)\n" 53)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_ticks_promoted_constructors_and_expands_missing_equation_or_case_patterns() {
    let elisp_form = r##"(list
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Unticked promoted constructor: ‘Just’"
           "type Example = «POINT»Just Int\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Pattern match(es) are non-exhaustive\nIn an equation for ‘render’:\nPatterns not matched:\n  Nothing\n  Just _"
           "render :: Maybe Int -> String\n«POINT»render (Just value) = show value\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Patterns of type ‘Maybe Int’ not matched:\n  Nothing\n  Just _\n"
           "render value = case value of«POINT»\n  Just item -> show item\n"
           0))"##;
    let expect = expect![[
        r#"OK ((((tick-promoted-constructor t)) "type Example = Just Int\n" ((:ok nil) "type Example = 'Just Int\n" 17)) (((add-missing-patterns t)) "render :: Maybe Int -> String\nrender (Just value) = show value\n" ((:ok nil) "render :: Maybe Int -> String\nrender (Just value) = show value\nrender Nothing = _\nrender Just _ = _\n" 100)) (((add-missing-patterns t)) "render value = case value of\n  Just item -> show item\n" ((:ok nil) "render value = case value of\n     Nothing -> _\n     Just _ -> _\n  Just item -> show item\n" 64)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_marks_discarded_do_results_and_unused_names_or_type_variables() {
    let elisp_form = r##"(mapcar
          (lambda (case)
            (apply
             #'attrap-test-run-fixer-option
             'attrap-ghc-fixer
             (append case '(0))))
          '(("A do-notation statement discarded a result of type ‘Int’"
             "main = do\n  «POINT»readValue\n  pure ()\n")
            ("Defined but not used: ‘temporary’"
             "compute input =\n  let «POINT»temporary = expensive input\n  in input\n")
            ("Unused quantified type variable ‘unused’"
             "demo :: forall used «POINT»unused. used -> used\n")))"##;
    let expect = expect![[
        r#"OK ((((explicitly-discard-result t)) "main = do\n  readValue\n  pure ()\n" ((:ok nil) "main = do\n  _ <- readValue\n  pure ()\n" 18)) (((add-underscore t)) "compute input =\n  let temporary = expensive input\n  in input\n" ((:ok nil) "compute input =\n  let _temporary = expensive input\n  in input\n" 24)) (((delete-type-variable t)) "demo :: forall used unused. used -> used\n" ((:ok nil) "demo :: forall used . used -> used\n" 21)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_renames_missing_modules_extensions_and_identifiers_from_real_suggestions() {
    let elisp_form = r##"(list
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Could not find module ‘Data.Mpa’\n  Perhaps you meant\n    Data.Map"
           "module Demo where\nimport «POINT»Data.Mpa\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Unsupported extension: LambdaCases\n  Perhaps you meant ‘LambdaCase’"
           "{-# LANGUAGE «POINT»LambdaCases #-}\nmodule Demo where\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Not in scope: ‘argmax’\nPerhaps you meant one of these: ‘argMax’ (imported from Tensor.Core), ‘maximum’ (line 19)"
           "result = «POINT»argmax values\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Not in scope: data constructor ‘Nothingg’\nPerhaps you meant ‘Nothing’ (imported from Prelude)"
           "result = «POINT»Nothingg\n"
           0))"##;
    let expect = expect![[
        r#"OK ((((rename-module-import t)) "module Demo where\nimport Data.Mpa\n" ((:ok nil) "module Demo where\nimport Data.Map\n" 34)) (((rename-extension t) ((use-extension "LambdaCase") t)) "{-# LANGUAGE LambdaCases #-}\nmodule Demo where\n" ((:ok nil) "{-# LANGUAGE LambdaCase #-}\nmodule Demo where\n" 24)) ((((replace "argmax" by "argMax" from "Tensor.Core") t) ((replace "argmax" by "maximum" from "line 19") t)) "result = argmax values\n" ((:ok nil) "result = argMax values\n" 16)) ((((replace "Nothingg" by "Nothing" from "Prelude") t)) "result = Nothingg\n" ((:ok nil) "result = Nothing\n" 17)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_adds_one_missing_identifier_to_the_exact_import_location() {
    let elisp_form = r##"(attrap-test-run-fixer-option
          'attrap-ghc-fixer
          "‘insert’ to the import list in the import of ‘Data.Map’ (/workspace/Demo.hs:2:1-24)"
          "module Demo where\nimport Data.Map (lookup)«POINT»\nmain = pure ()\n"
          0)"##;
    let expect = expect![[
        r#"OK ((((add-to-import-list "Data.Map") t)) "module Demo where\nimport Data.Map (lookup)\nmain = pure ()\n" ((:ok nil) "module Demo where\nimport Data.Map (lookup,insert)\nmain = pure ()\n" 49))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_offers_every_candidate_import_list_in_message_order() {
    let elisp_form = r##"(list
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Perhaps you want to add ‘lookup’ to one of these import lists:\n‘Data.Map’ (/workspace/Demo.hs:2:1-24)\n‘Data.IntMap’ (/workspace/Demo.hs:3:1-27)"
           "module Demo where\nimport Data.Map (insert)\nimport Data.IntMap (insert)«POINT»\nmain = pure ()\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Perhaps you want to add ‘lookup’ to one of these import lists:\n‘Data.Map’ (/workspace/Demo.hs:2:1-24)\n‘Data.IntMap’ (/workspace/Demo.hs:3:1-27)"
           "module Demo where\nimport Data.Map (insert)\nimport Data.IntMap (insert)«POINT»\nmain = pure ()\n"
           1))"##;
    let expect = expect![[
        r#"OK (((((add-to-import-list "Data.Map") t) ((add-to-import-list "Data.IntMap") t)) "module Demo where\nimport Data.Map (insert)\nimport Data.IntMap (insert)\nmain = pure ()\n" ((:ok nil) "module Demo where\nimport Data.Map (insert,lookup)\nimport Data.IntMap (insert)\nmain = pure ()\n" 49)) ((((add-to-import-list "Data.Map") t) ((add-to-import-list "Data.IntMap") t)) "module Demo where\nimport Data.Map (insert)\nimport Data.IntMap (insert)\nmain = pure ()\n" ((:ok nil) "module Demo where\nimport Data.Map (insert)\nimport Data.IntMap (insert,lookup)\nmain = pure ()\n" 77)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_qualifies_ambiguous_identifiers_using_each_reported_candidate() {
    let elisp_form = r##"(mapcar
          (lambda (option-index)
            (with-temp-buffer
              (pcase-let
                  ((`(,beg ,end)
                    (attrap-test-place-markers
                     "result = «POINT»map«END» transform values\n")))
                (cl-letf
                    (((symbol-function
                       'dante-ident-pos-at-point)
                      (lambda ()
                        (list beg end))))
                  (let* ((options
                          (attrap-ghc-fixer
                           "Ambiguous occurrence ‘map’. It could refer to ‘Data.List.map’, ‘Prelude.map’,"
                           beg
                           end))
                         (shape
                          (attrap-test-option-shape
                           options))
                         (option
                          (nth option-index options)))
                    (list
                     shape
                     (funcall
                      (cdr option))
                     (buffer-string)
                     (point)))))))
          '(0 1))"##;
    let expect = expect![[
        r#"OK (((((rename "Data.List.map") t) ((rename "Prelude.map") t)) nil "result = Data.List.map transform values\n" 23) ((((rename "Data.List.map") t) ((rename "Prelude.map") t)) nil "result = Prelude.map transform values\n" 21))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_adds_missing_module_imports_after_multiline_module_headers() {
    let elisp_form = r##"(attrap-test-run-fixer-option
          'attrap-ghc-fixer
          "No module named ‘Data.Map.Strict’ is imported."
          "module Demo\n  ( runDemo\n  ) where\n\n«POINT»runDemo = Data.Map.Strict.empty\n"
          0)"##;
    let expect = expect![[
        r#"OK ((((add-import "Data.Map.Strict") t)) "module Demo\n  ( runDemo\n  ) where\n\nrunDemo = Data.Map.Strict.empty\n" ((:ok nil) "module Demo\n  ( runDemo\n  ) where\nimport Data.Map.Strict\n\n\nrunDemo = Data.Map.Strict.empty\n" 58))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_deletes_redundant_import_members_and_whole_import_declarations() {
    let elisp_form = r##"(list
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "The import of ‘lookup, insert’ from module ‘Data.Map’ is redundant"
           "module Demo where\n«POINT»import Data.Map (lookup, insert, union)\nrun = union\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Module ‘Control.Applicative’ does not export ‘<*>’"
           "module Demo where\n«POINT»import Control.Applicative ((<*>), pure)\nrun = pure\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "The import of ‘Tree’ from module ‘Data.Tree’ is redundant"
           "module Demo where\n«POINT»import Data.Tree (Tree(..), Forest)\nrun :: Forest Int\nrun = []\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "The qualified import of ‘Data.Map’ is redundant"
           "module Demo where\n«POINT»import qualified Data.Map as Map (lookup)\n\nrun = 1\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "The import of ‘Unused.Module’ is redundant"
           "module Demo where\n«POINT»import Unused.Module hiding (unused)\n\nrun = 1\n"
           0))"##;
    let expect = expect![[
        r#"OK ((((delete-import t)) "module Demo where\nimport Data.Map (lookup, insert, union)\nrun = union\n" ((:ok nil) "module Demo where\nimport Data.Map (  union)\nrun = union\n" 19)) (((delete-import t)) "module Demo where\nimport Control.Applicative ((<*>), pure)\nrun = pure\n" ((:ok nil) "module Demo where\nimport Control.Applicative ( pure)\nrun = pure\n" 19)) (((delete-import t)) "module Demo where\nimport Data.Tree (Tree(..), Forest)\nrun :: Forest Int\nrun = []\n" ((:ok nil) "module Demo where\nimport Data.Tree ( Forest)\nrun :: Forest Int\nrun = []\n" 19)) (((delete-module-import t)) "module Demo where\nimport qualified Data.Map as Map (lookup)\n\nrun = 1\n" ((:ok nil) "module Demo where\nData.Map as Map (lookup)\n\nrun = 1\n" 19)) (((delete-module-import t)) "module Demo where\nimport Unused.Module hiding (unused)\n\nrun = 1\n" ((:ok nil) "module Demo where\nrun = 1\n" 19)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_expands_type_wildcards_and_initializes_each_missing_record_field() {
    let elisp_form = r##"(list
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Found type wildcard ‘_value’ standing for ‘Maybe (Either Text Int)’"
           "demo :: «POINT»_value -> String\ndemo = show\n"
           0)
          (attrap-test-run-fixer-option
           'attrap-ghc-fixer
           "Fields of ‘Config’ not initialised: host, port, secure• In the expression"
           "config = Config «POINT»{ name = \"demo\" }\n"
           0))"##;
    let expect = expect![[
        r#"OK ((((explicit-type-wildcard t)) "demo :: _value -> String\ndemo = show\n" ((:ok nil) "demo :: (Maybe (Either Text Int)) -> String\ndemo = show\n" 34)) (((initialize-fields t)) "config = Config { name = \"demo\" }\n" ((:ok nil) "config = Config {,host = _\n,port = _\n,secure = _\n name = \"demo\" }\n" 50)))"#
    ]];

    assert_attrap_parity(elisp_form, expect);
}

#[test]
fn attrap_ghc_fixer_returns_no_repairs_for_an_unrecognized_diagnostic() {
    let elisp_form = r##"(attrap-test-run-fixer-option
          'attrap-ghc-fixer
          "GHC emitted an entirely unrelated informational message"
          "module Demo where\n\n«POINT»value = 1\n"
          nil)"##;
    let expect = expect![[r#"OK (nil "module Demo where\n\nvalue = 1\n" nil)"#]];

    assert_attrap_parity(elisp_form, expect);
}
