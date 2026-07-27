use expect_test::expect;

use super::assert_annotate_parity;

#[test]
fn annotate_serialized_record_accessors_cover_root_and_reply_fields() {
    let elisp_form = r##"(let* ((root '(3 8 "review" "alpha" 2 :by-length "root-id" nil))
               (reply '(0 0 "from: dev\nagreed" "" 1 :new-line "reply-id" "root-id"))
               (record (annotate-make-record "/work/demo.rs" (list root reply) "checksum")))
         (list record
               (annotate-filename-from-dump record)
               (annotate-annotations-from-dump record)
               (annotate-checksum-from-dump record)
               (mapcar
                (lambda (annotation)
                  (list
                   (annotate-beginning-of-annotation annotation)
                   (annotate-ending-of-annotation annotation)
                   (annotate-annotation-interval annotation)
                   (annotate-annotation-string annotation)
                   (annotate-annotated-text annotation)
                   (annotate-color-index-from-dump annotation)
                   (annotate-placement-policy-from-dump annotation)
                   (annotate-id-from-dump annotation)
                   (annotate-reply-to-from-dump annotation)
                   (annotate-annotation-root-p annotation)
                   (annotate-annotation-leaf-p annotation (list record))))
                (list root reply))))"##;
    let expect = expect![[
        r#"OK (("/work/demo.rs" #1=((3 8 "review" "alpha" 2 :by-length "root-id" nil) (0 0 "from: dev\nagreed" "" 1 :new-line "reply-id" "root-id")) "checksum") "/work/demo.rs" #1# "checksum" ((3 8 (3 7) "review" "alpha" 2 :by-length "root-id" nil t nil) (0 0 (0 -1) "from: dev\nagreed" "" 1 :new-line "reply-id" "root-id" nil t)))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_serialized_mutators_update_text_and_reply_target_in_place() {
    let elisp_form = r##"(let ((annotation
               '(3 8 "old" "alpha" 0 :by-length "child" "parent-a")))
         (let ((text-result
                (annotate-annotation-replace-annotation-text annotation "new"))
               (reply-result
                (annotate-annotation-replace-reply-to annotation "parent-b")))
           (list annotation
                 text-result
                 reply-result
                 (eq annotation text-result)
                 (eq annotation reply-result))))"##;
    let expect =
        expect![[r#"OK (#1=(3 8 "new" "alpha" 0 :by-length "child" "parent-b") #1# #1# t t)"#]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_database_remove_and_replace_target_only_matching_root() {
    let elisp_form = r##"(let* ((a '(1 4 "first" "abc" 0 :by-length "a" nil))
               (reply '(0 0 "reply" "" 0 :by-length "r" "a"))
               (b '(8 12 "second" "def" 1 :new-line "b" nil))
               (db (list
                    (list "/one.txt" (list a reply b) "sum-one")
                    (list "/two.txt" (list '(2 5 "other" "xyz")) "sum-two"))))
         (list
          (annotate-db-remove-annotation db "/one.txt" 1 4)
          (annotate-db-remove-annotation db "/missing.txt" 1 4)
          (annotate-db-replace-annotation db "/one.txt" 8 12 "updated")
          (annotate-db-replace-annotation db "/one.txt" 99 100 "missing")))"##;
    let expect = expect![[
        r#"OK ((("/one.txt" #1=(#5=(0 0 "reply" "" 0 :by-length "r" "a") #3=(8 12 "updated" "def" 1 :new-line "b" nil)) "sum-one") . #2=(("/two.txt" ((2 5 "other" "xyz")) "sum-two"))) #6=(("/one.txt" (#4=(1 4 "first" "abc" 0 :by-length "a" nil) . #1#) "sum-one") . #2#) (("/one.txt" (#3# #4# #5#) "sum-one") . #2#) #6#)"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_remove_stray_replies_recursively_prunes_orphan_threads() {
    let elisp_form = r##"(let* ((root '(1 5 "root" "text" 0 :by-length "root" nil))
               (child '(0 0 "child" "" 0 :by-length "child" "root"))
               (grandchild '(0 0 "grandchild" "" 0 :by-length "grand" "child"))
               (orphan '(0 0 "orphan" "" 0 :by-length "orphan" "missing"))
               (orphan-child '(0 0 "orphan child" "" 0 :by-length "orphan-child" "orphan"))
               (db (list (list "/a" (list grandchild orphan-child child orphan root) "sum"))))
         (annotate-remove-stray-replies db))"##;
    let expect = expect![[
        r#"OK (("/a" ((0 0 "grandchild" "" 0 :by-length "grand" "child") (0 0 "child" "" 0 :by-length "child" "root") (1 5 "root" "text" 0 :by-length "root" nil)) "sum"))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_database_purge_removes_empty_annotations_and_records() {
    let elisp_form = r##"(let ((db
               '(("/a" ((1 2 "kept" "a") (3 4 "" "b") (5 6 nil "c")) "sum-a")
                 ("/b" nil "sum-b")
                 ("/c" ((7 8 "also kept" "d")) "sum-c"))))
         (list
          (annotate-db-purge* db)
          (annotate--db-empty-p db)
          (annotate--db-empty-p '(("/empty" nil "sum")))
          (annotate-string-empty-p nil)
          (annotate-string-empty-p "")
          (annotate-string-empty-p "note")))"##;
    let expect = expect![[
        r#"OK ((("/a" ((1 2 "kept" "a") (3 4 "" "b") (5 6 nil "c")) "sum-a") ("/c" ((7 8 "also kept" "d")) "sum-c")) nil t t t nil)"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_database_round_trip_writes_and_reads_deterministic_local_file() {
    let elisp_form = r##"(let* ((root (expand-file-name "annotate-db-test" (getenv "TMPDIR")))
               (annotate-file (expand-file-name "annotations.el" root))
               (annotate-database-confirm-deletion nil)
               (db '(("/project/a.rs"
                      ((1 4 "review" "let" 0 :by-length "id-a" nil))
                      "sum-a")
                     ("/project/b.org"
                      ((8 12 "todo" "task" 2 :new-line "id-b" nil))
                      "sum-b"))))
         (make-directory root t)
         (annotate-dump-annotation-data db)
         (let ((bytes (with-temp-buffer
                        (insert-file-contents-literally annotate-file)
                        (buffer-string)))
               (loaded (annotate-load-annotation-data)))
           (annotate-dump-annotation-data nil)
           (list bytes
                 loaded
                 (file-exists-p annotate-file))))"##;
    let expect = expect![[
        r#"OK ("((\"/project/a.rs\" ((1 4 \"review\" \"let\" 0 :by-length \"id-a\" nil)) \"sum-a\") (\"/project/b.org\" ((8 12 \"todo\" \"task\" 2 :new-line \"id-b\" nil)) \"sum-b\"))" (("/project/a.rs" ((1 4 "review" "let" 0 :by-length "id-a" nil)) "sum-a") ("/project/b.org" ((8 12 "todo" "task" 2 :new-line "id-b" nil)) "sum-b")) nil)"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_interval_overlap_and_merge_cover_touching_disjoint_and_reply_cases() {
    let elisp_form = r##"(let ((a '(2 6 "A" "cdef" 0 :by-length "a" nil))
               (touching '(6 9 "B" "ghi" 1 :by-length "b" nil))
               (overlap '(5 10 "C" "fghij" 2 :new-line "c" nil))
               (disjoint '(12 15 "D" "mno" 0 :by-length "d" nil))
               (reply '(0 0 "reply" "" 0 :by-length "r" "a")))
         (list
          (annotate--merge-interval '(2 5) '(4 9))
          (annotate--db-annotations-overlaps-p a touching)
          (annotate--db-annotations-overlaps-p a overlap)
          (annotate--db-annotations-overlaps-p a disjoint)
          (annotate--db-annotations-overlaps-p a reply)
          (with-temp-buffer
            (insert "0123456789abcdefghij")
            (annotate--db-merge-annotations a overlap))))"##;
    let expect =
        expect![[r#"OK ((2 9) nil t nil nil (2 10 "A C" "12345678" 0 :by-length "a" nil))"#]];
    assert_annotate_parity(elisp_form, expect);
}

#[test]
fn annotate_find_children_root_and_leaf_traverse_practical_thread() {
    let elisp_form = r##"(let* ((root '(1 5 "root" "text" 0 :by-length "root" nil))
               (child-a '(0 0 "child a" "" 0 :by-length "a" "root"))
               (child-b '(0 0 "child b" "" 0 :by-length "b" "root"))
               (grandchild '(0 0 "grandchild" "" 0 :by-length "g" "a"))
               (record (list "/file" (list grandchild child-b root child-a) "sum"))
               (db (list record)))
         (list
          (annotate-get-annotation-children db root)
          (annotate-get-annotation-children db "a")
          (annotate-annotation-find-root db grandchild)
          (annotate-annotation-find-root db root)
          (mapcar
           (lambda (annotation)
             (list (annotate-annotation-id annotation)
                   (annotate-annotation-leaf-p annotation db)))
           (list root child-a child-b grandchild))))"##;
    let expect = expect![[
        r#"OK (((0 0 "child a" "" 0 :by-length "a" "root") (0 0 "child b" "" 0 :by-length "b" "root")) ((0 0 "grandchild" "" 0 :by-length "g" "a")) #1=(1 5 "root" "text" 0 :by-length "root" nil) #1# (("root" nil) ("a" nil) ("b" t) ("g" t)))"#
    ]];
    assert_annotate_parity(elisp_form, expect);
}
