use expect_test::expect;

use super::assert_agda_editor_tactics_parity;

#[test]
fn agda_editor_tactics_record_info_parses_the_documented_full_record() {
    let elisp_form = r##"(agda-editor-tactics-record-info
         "record R (X : Set) (x : X) (y : Y x) : Set where

     w : Set
     w = X

     m = w

     field
       a : X

     b : X
     b = X

     field
       c : Y a")"##;
    let expect = expect![[
        r#"OK (:name "R" :level "" :body ((:param "X : Set") (:param "x : X") (:param "y : Y x") (:local "w : Set") (:local "w = X") (:local "m = w") (:field "a : X") (:local "b : X") (:local "b = X") (:field "c : Y a")))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}

#[test]
fn agda_editor_tactics_record_info_preserves_parameters_and_universe_level() {
    let elisp_form = r##"(mapcar
         #'agda-editor-tactics-record-info
         '("record Box (ℓ : Level) (A : Set ℓ) : Set (lsuc ℓ) where
  field
    value : A"
           "record Relation (a b : Level) (A : Set a) : Set (a ⊔ lsuc b) where
    field
      _≈_ : A → A → Set b"))"##;
    let expect = expect![[
        r#"OK ((:name "Box" :level " (lsuc ℓ)" :body ((:param "ℓ : Level") (:param "A : Set ℓ") (:field "value : A"))) (:name "Relation" :level " (a ⊔ lsuc b)" :body ((:param "a b : Level") (:param "A : Set a") (:field "_≈_ : A → A → Set b"))))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}

#[test]
fn agda_editor_tactics_record_info_classifies_preamble_locals_and_fields() {
    let elisp_form = r##"(agda-editor-tactics-record-info
         "record Computed (A : Set) : Set where
  Alias : Set
  Alias = A
  identity : Alias → Alias
  identity x = x
  field
    seed : Alias
    step : Alias → Alias
  twice : Alias → Alias
  twice x = step (step x)")"##;
    let expect = expect![[
        r#"OK (:name "Computed" :level "" :body ((:param "A : Set") (:local "Alias : Set") (:local "Alias = A") (:local "identity : Alias → Alias") (:local "identity x = x") (:field "seed : Alias") (:field "step : Alias → Alias") (:local "twice : Alias → Alias") (:local "twice x = step (step x)")))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}

#[test]
fn agda_editor_tactics_record_info_handles_multiple_field_sections() {
    let elisp_form = r##"(agda-editor-tactics-record-info
         "record SplitFields : Set₁ where
  field
    Carrier : Set
    initial : Carrier
  derived : Carrier
  derived = initial
  field
    operation : Carrier → Carrier
    law : operation initial ≡ initial
  witness : Carrier
  witness = operation initial")"##;
    let expect = expect![[
        r#"OK (:name "SplitFields" :level "₁" :body ((:field "Carrier : Set") (:field "initial : Carrier") (:local "derived : Carrier") (:local "derived = initial") (:field "operation : Carrier → Carrier") (:field "law : operation initial ≡ initial") (:local "witness : Carrier") (:local "witness = operation initial")))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}

#[test]
fn agda_editor_tactics_record_info_omits_blank_lines_but_preserves_declaration_order() {
    let elisp_form = r##"(let* ((parsed
          (agda-editor-tactics-record-info
           "record Sparse (A : Set) : Set where


  before : A → A
  before x = x

  field

    first : A

    second : A

  after : A
  after = first
"))
        (body (plist-get parsed :body)))
         (list parsed (length body) (mapcar #'car body) (mapcar #'cadr body)))"##;
    let expect = expect![[
        r#"OK ((:name "Sparse" :level "" :body ((:param "A : Set") (:local "before : A → A") (:local "before x = x") (:local "first : A") (:local "second : A") (:local "after : A") (:local "after = first"))) 7 (:param :local :local :local :local :local :local) ("A : Set" "before : A → A" "before x = x" "first : A" "second : A" "after : A" "after = first"))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}

#[test]
fn agda_editor_tactics_record_info_uses_each_field_block_indentation() {
    let elisp_form = r##"(agda-editor-tactics-record-info
         "record MixedIndent : Set where
 field
   one : Set
  one-local : Set
  one-local = one
      another-local = one-local
     field
          two : Set
       two-local : Set
       two-local = two")"##;
    let expect = expect![[
        r#"OK (:name "MixedIndent" :level "" :body ((:field "one : Set") (:local "one-local : Set") (:local "one-local = one") (:local "another-local = one-local") (:field "two : Set") (:local "two-local : Set") (:local "two-local = two")))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}

#[test]
fn agda_editor_tactics_record_info_parses_a_parameterless_record() {
    let elisp_form = r##"(list
         (agda-editor-tactics-record-info
          "record UnitLike : Set where
  field
    inhabitant : ⊤")
         (agda-editor-tactics-record-info
          "record Higher : Set (lsuc (lsuc zero)) where
      field
        Carrier : Set₁"))"##;
    let expect = expect![[
        r#"OK ((:name "UnitLike" :level "" :body ((:field "inhabitant : ⊤"))) (:name "Higher" :level " (lsuc (lsuc zero))" :body ((:field "Carrier : Set₁"))))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}

#[test]
fn agda_editor_tactics_record_info_exposes_documented_parser_boundaries() {
    let elisp_form = r##"(mapcar
         (lambda (source)
           (condition-case error
               (list :value (agda-editor-tactics-record-info source))
             (error
              (list
               :error
               (car error)
               (mapcar
                (lambda (item)
                  (if (stringp item)
                      (replace-regexp-in-string
                       (regexp-quote source)
                       "<SOURCE>"
                       item
                       t
                       t)
                    item))
                (cdr error))))))
         '("record NoFields : Set where
  local : Set"
           "private record Hidden : Set where
  field
    value : Set"
           "record Implicit {A : Set} : Set where
  field
    value : A"))"##;
    let expect = expect![[
        r#"OK ((:value (:name "NoFields" :level "" :body ((:local "local : Set")))) (:value (:name "Hidden" :level "" :body ((:field "value : Set")))) (:value (:name "Implicit" :level "" :body ((:field "value : A")))))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}
