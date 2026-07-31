use expect_test::expect;

use super::assert_agda_editor_tactics_batch;

#[test]
fn rendering_public_surface_batch() {
    assert_agda_editor_tactics_batch(&[
        (
            "agda_editor_tactics_rendering_handles_empty_parameterless_records",
            r##"(mapcar
         #'agda-editor-tactics-as-Σ-nested
         '((:name "Empty" :level "" :body nil)
           (:name "Higher" :level "(lsuc ℓ)" :body nil)))"##,
            true,
            expect![[r#"OK ("Empty′ : Set \nEmpty′ = ⊤" "Higher′ : Set (lsuc ℓ)\nHigher′ = ⊤")"#]],
        ),
        (
            "agda_editor_tactics_rendering_builds_nested_fields_and_local_lets",
            r##"(agda-editor-tactics-as-Σ-nested
         '(:name "Monoid"
           :level "ℓ"
           :body
           ((:field "Carrier : Set ℓ")
            (:field "ε : Carrier")
            (:field "_∙_ : Carrier → Carrier → Carrier")
            (:local "left-id : ∀ x → ε ∙ x ≡ x")
            (:local "left-id = proof")
            (:field "right-id : ∀ x → x ∙ ε ≡ x"))))"##,
            true,
            expect![[
        r#"OK "Monoid′ : Set ℓ\nMonoid′ = Σ Carrier ∶ Set ℓ • Σ ε ∶ Carrier • Σ _∙_ ∶ Carrier → Carrier → Carrier • let left-id : ∀ x → ε ∙ x ≡ x ; left-id = proof in Σ right-id ∶ ∀ x → x ∙ ε ≡ x • ⊤""#
    ]],
        ),
        (
            "agda_editor_tactics_rendering_turns_parameters_into_binders_and_lambda_arguments",
            r##"(agda-editor-tactics-as-Σ-nested
         '(:name "Dependent"
           :level "(a ⊔ b)"
           :body
           ((:param "A : Set a")
            (:param "B : A → Set b")
            (:param "x : A")
            (:field "value : B x")
            (:field "proof : value ≡ value"))))"##,
            true,
            expect![[
        r#"OK "Dependent′ : (A : Set a) (B : A → Set b) (x : A) → Set (a ⊔ b)\nDependent′ = λ A B x → Σ value ∶ B x • Σ proof ∶ value ≡ value • ⊤""#
    ]],
        ),
        (
            "agda_editor_tactics_rendering_respects_custom_sigma_naming",
            r##"(let ((agda-editor-tactics-format-Σ-naming "%s-as-nested-Σ"))
         (list
          agda-editor-tactics-format-Σ-naming
          (agda-editor-tactics-as-Σ-nested
           '(:name "Configured"
             :level ""
             :body
             ((:param "A : Set")
              (:field "item : A"))))))"##,
            true,
            expect![[
        r#"OK ("%s-as-nested-Σ" "Configured-as-nested-Σ : (A : Set) → Set \nConfigured-as-nested-Σ = λ A → Σ item ∶ A • ⊤")"#
    ]],
        ),
        (
            "agda_editor_tactics_rendering_normalizes_whitespace_and_let_sequences",
            r##"(agda-editor-tactics-as-Σ-nested
         '(:name "Spacing"
           :level ""
           :body
           ((:local "Alias    :    Set")
            (:local "Alias =    Set")
            (:local "chosen    = Alias")
            (:field "value  :   chosen")
            (:local "proof : value ≡ value")
            (:local "proof = refl"))))"##,
            true,
            expect![[
        r#"OK "Spacing′ : Set \nSpacing′ = let Alias : Set ; Alias = Set ; chosen = Alias in Σ value ∶ chosen • let proof : value ≡ value ; proof = refl in ⊤""#
    ]],
        ),
        (
            "agda_editor_tactics_rendering_replaces_colons_only_for_fields",
            r##"(agda-editor-tactics-as-Σ-nested
         '(:name "Colons"
           :level ""
           :body
           ((:local "Alias : Set")
            (:field "mapping : A :→ B")
            (:local "qualified = Module.value : Alias")
            (:field "proof : mapping x ≡ y"))))"##,
            true,
            expect![[
        r#"OK "Colons′ : Set \nColons′ = let Alias : Set in Σ mapping ∶ A ∶→ B • let qualified = Module.value : Alias in Σ proof ∶ mapping x ≡ y • ⊤""#
    ]],
        ),
        (
            "agda_editor_tactics_rendering_preserves_parameter_order_and_dependencies",
            r##"(let ((record
          '(:name "Category"
            :level "(lsuc (o ⊔ h))"
            :body
            ((:param "o : Level")
             (:param "h : Level")
             (:field "Obj : Set o")
             (:field "Hom : Obj → Obj → Set h")
             (:field "id : ∀ {A} → Hom A A")
             (:field "_∘_ : ∀ {A B C} → Hom B C → Hom A B → Hom A C")))))
         (list
          (agda-editor-tactics-as-Σ-nested record)
          (plist-get record :body)))"##,
            true,
            expect![[
        r#"OK ("Category′ : (o : Level) (h : Level) → Set (lsuc (o ⊔ h))\nCategory′ = λ o h → Σ Obj ∶ Set o • Σ Hom ∶ Obj → Obj → Set h • Σ id ∶ ∀ {A} → Hom A A • Σ _∘_ ∶ ∀ {A B C} → Hom B C → Hom A B → Hom A C • ⊤" ((:param "o : Level") (:param "h : Level") (:field "Obj : Set o") (:field "Hom : Obj → Obj → Set h") (:field "id : ∀ {A} → Hom A A") (:field "_∘_ : ∀ {A B C} → Hom B C → Hom A B → Hom A C")))"#
    ]],
        ),
    ]);
}
