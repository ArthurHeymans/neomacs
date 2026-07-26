use expect_test::expect;

use super::assert_academic_phrases_parity;

#[test]
fn academic_phrases_all_57_categories_have_exact_titles_counts_and_id_boundaries() {
    let elisp_form = r##"(mapcar
              (lambda (cat)
                (let* ((entry
                        (ht-get
                         academic-phrases--all-phrases
                         cat))
                       (items
                        (ht-get
                         entry
                         :items)))
                  (list
                   cat
                   (ht-get
                    entry
                    :title)
                   (length
                    items)
                   (ht-get
                    (car
                     items)
                    :id)
                   (ht-get
                    (car
                     (last
                      items))
                    :id))))
              (academic-phrases--gen-cats-keywords
               1
               57))"##;
    let expect = expect![[
        r#"OK ((:cat1 "Establishing why your topic X is important" 12 1 12) (:cat2 "Outlining the past-present history of the study of X" 11 13 23) (:cat3 "Outlining the possible future of X" 5 24 28) (:cat4 "Indicating the gap in knowledge and possible limitations" 28 29 56) (:cat5 "Stating the aim of your paper and its contribution" 14 57 70) (:cat6 "Explaining the key terminology in your field" 15 71 85) (:cat7 "Explaining how you will use terminology and acronyms in your paper" 11 86 96) (:cat8 "Giving the structure of paper - what is and is not included" 12 97 108) (:cat9 "Giving general panorama of past-to-present literature" 14 109 122) (:cat10 "Reviewing past literature" 7 123 129) (:cat11 "Reviewing subsequent and more recent literature" 11 130 140) (:cat12 "Reporting what specific authors have said" 13 141 153) (:cat13 "Mentioning positive aspects of others’ work" 4 154 157) (:cat14 "Highlighting limitations of previous studies - authors not mentioned by name" 14 158 171) (:cat15 "Highlighting limitations of previous studies - authors mentioned by name" 18 172 189) (:cat16 "Using the opinions of others to justify your criticism of someone’s work" 9 190 198) (:cat17 "Describing purpose of testing / methods used" 7 199 205) (:cat18 "Outlining similarities with other authors’ models, systems etc." 10 206 215) (:cat19 "Describing the apparatus and materials used and their source" 9 216 224) (:cat20 "Reporting software used" 7 225 231) (:cat21 "Reporting customizations performed" 6 232 237) (:cat22 "Formulating equations, theories and theorems" 15 238 252) (:cat23 "Explaining why you chose your specific method, model, equipment, sample etc." 9 253 261) (:cat24 "Explaining the preparation of samples, solutions etc." 8 262 269) (:cat25 "Outlining selection procedure for samples, surveys etc." 10 270 279) (:cat26 "Indicating the time frame (past tenses)" 11 280 290) (:cat27 "Indicating the time frame in a general process (present tenses)" 12 291 302) (:cat28 "Indicating that care must be taken" 5 303 307) (:cat29 "Describing benefits of your method, equipment etc." 10 308 317) (:cat30 "Outlining alternative approaches " 4 318 321) (:cat31 "Explaining how you got your results" 5 322 326) (:cat32 "Reporting results from questionnaires and interviews" 11 327 337) (:cat33 "Stating what you found" 8 338 345) (:cat34 "Stating what you did not find" 5 346 350) (:cat35 "Highlighting significant results and achievements" 18 351 368) (:cat36 "Stating that your results confirm previous evidence" 10 369 378) (:cat37 "Stating that your results are in contrast with previous evidence" 12 379 390) (:cat38 "Stating and justifying the acceptability of your results" 9 391 399) (:cat39 "Expressing caution regarding the interpretation of results" 6 400 405) (:cat40 "Outlining undesired or unexpected results" 10 406 415) (:cat41 "Admitting limitations" 12 416 427) (:cat42 "Explaining and justifying undesired or unexpected results" 19 428 446) (:cat43 "Minimizing undesired or unexpected results" 14 447 460) (:cat44 "Expressing opinions and probabilities" 14 461 474) (:cat45 "Announcing your conclusions and summarizing content" 5 475 479) (:cat46 "Restating the results (Conclusions section)" 4 480 483) (:cat47 "Highlighting achievements (Conclusions section)" 14 484 497) (:cat48 "Highlighting limitations (Conclusions section)" 10 498 507) (:cat49 "Outlining possible applications and implications of your work" 21 508 528) (:cat50 "Future work already underway or planned by the authors" 7 529 535) (:cat51 "Future work proposed for third parties to carry out" 16 536 551) (:cat52 "Acknowledgements" 10 552 561) (:cat53 "Referring to tables and figures, and to their implications" 13 562 574) (:cat54 "Making transitions, focusing on a new topic" 3 575 577) (:cat55 "Referring backwards and forwards in the paper" 7 578 584) (:cat56 "Referring back to your research aim" 5 585 589) (:cat57 "Referring outside the paper" 3 590 592))"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_entire_data_table_has_contiguous_ids_exact_schema_and_canonical_digest() {
    let elisp_form = r##"(let ((categories
                    (academic-phrases--gen-cats-keywords
                     1
                     57))
                   ids
                   (schema-valid
                    t)
                   (types-valid
                    t)
                   canonical)
               (dolist (cat categories)
                 (let* ((entry
                         (ht-get
                          academic-phrases--all-phrases
                          cat))
                        (items
                         (ht-get
                          entry
                          :items))
                        canonical-items)
                   (unless
                       (and
                        (hash-table-p
                         entry)
                        (stringp
                         (ht-get
                          entry
                          :title))
                        (not
                         (string-empty-p
                          (ht-get
                           entry
                           :title)))
                        (equal
                         (sort
                          (mapcar
                           #'symbol-name
                           (ht-keys
                            entry))
                          #'string<)
                         '(":items"
                           ":title"))
                        (consp
                         items))
                     (setq
                      schema-valid
                      nil))
                   (dolist (item items)
                     (let ((id
                            (ht-get
                             item
                             :id))
                           (template
                            (ht-get
                             item
                             :template))
                           (choices
                            (ht-get
                             item
                             :choices)))
                       (push
                        id
                        ids)
                       (unless
                           (and
                            (hash-table-p
                             item)
                            (equal
                             (sort
                              (mapcar
                               #'symbol-name
                               (ht-keys
                                item))
                              #'string<)
                             '(":choices"
                               ":id"
                               ":template")))
                         (setq
                          schema-valid
                          nil))
                       (unless
                           (and
                            (integerp
                             id)
                            (stringp
                             template)
                            (not
                             (string-empty-p
                              template))
                            (listp
                             choices)
                            (cl-every
                             #'listp
                             choices)
                            (cl-every
                             (lambda (group)
                               (cl-every
                                #'stringp
                                group))
                             choices))
                         (setq
                          types-valid
                          nil))
                       (push
                        (list
                         id
                         template
                         choices)
                        canonical-items)))
                   (push
                    (list
                     cat
                     (ht-get
                      entry
                      :title)
                     (nreverse
                      canonical-items))
                    canonical)))
               (setq
                ids
                (nreverse
                 ids))
               (setq
                canonical
                (nreverse
                 canonical))
               (let ((printed
                      (prin1-to-string
                       canonical)))
                 (list
                  (length
                   categories)
                  (length
                   ids)
                  (equal
                   ids
                   (number-sequence
                    1
                    592))
                  schema-valid
                  types-valid
                  (length
                   printed)
                  (secure-hash
                   'sha256
                   printed))))"##;
    let expect = expect![[
        r#"OK (57 592 t t t 65415 "8a2274956cbb5dfb5475e38757f54bc1a323c44b3f803305d0c8384e051159be")"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_all_prompt_items_expand_every_placeholder_and_preserve_ids() {
    let elisp_form = r##"(let (canonical
                   (all-expanded
                    t)
                   (ids-preserved
                    t))
               (dolist (cat
                        (academic-phrases--gen-cats-keywords
                         1
                         57))
                 (let* ((items
                         (academic-phrases--get-items
                          cat))
                        (prompts
                         (academic-phrases--prompt-items
                          cat))
                        (ids
                         (mapcar
                          (lambda (item)
                            (ht-get
                             item
                             :id))
                          items)))
                   (unless
                       (equal
                        (mapcar
                         #'cdr
                         prompts)
                        ids)
                     (setq
                      ids-preserved
                      nil))
                   (unless
                       (cl-every
                        (lambda (prompt)
                          (and
                           (not
                            (s-contains?
                             "{1}"
                             (car
                              prompt)))
                           (not
                            (s-contains?
                             "{2}"
                             (car
                              prompt)))
                           (not
                            (s-contains?
                             "{3}"
                             (car
                              prompt)))))
                        prompts)
                     (setq
                      all-expanded
                      nil))
                   (push
                    (list
                     cat
                     prompts)
                    canonical)))
               (setq
                canonical
                (nreverse
                 canonical))
               (let ((printed
                      (prin1-to-string
                       canonical)))
                 (list
                  all-expanded
                  ids-preserved
                  (length
                   printed)
                  (secure-hash
                   'sha256
                   printed)
                  (car
                   (cadr
                    (car
                     canonical)))
                  (car
                   (last
                    (cadr
                     (car
                      (last
                       canonical))))))))"##;
    let expect = expect![[
        r#"OK (t t 54668 "8c933873407e5f1b406dbdbacb75a7d7cbca09cd1f15c0a54ee78eae52179490" ("X is the [main/leading/primary/major] cause of ..." . 1) ("More details on this topic can be found in [Ref]." . 592))"#
    ]];

    assert_academic_phrases_parity(elisp_form, expect);
}

#[test]
fn academic_phrases_data_variable_metadata_and_default_identity_match() {
    let elisp_form = r##"(list
               (get
                'academic-phrases--all-phrases
                'variable-documentation)
               (default-boundp
                'academic-phrases--all-phrases)
               (eq
                academic-phrases--all-phrases
                (default-value
                 'academic-phrases--all-phrases))
               (local-variable-if-set-p
                'academic-phrases--all-phrases)
               (hash-table-test
                academic-phrases--all-phrases)
               (hash-table-size
                academic-phrases--all-phrases)
               (hash-table-count
                academic-phrases--all-phrases))"##;
    let expect = expect!["OK (nil t t nil equal 96 57)"];

    assert_academic_phrases_parity(elisp_form, expect);
}
