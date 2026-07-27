use expect_test::expect;

use super::assert_arview_parity;

#[test]
fn arview_copy_remote_file_leaves_local_to_local_paths_untouched() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'copy-file)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       :copied)))
                 (list
                  (arview-copy-remote-file
                   "/work/archive.tar"
                   "/work/extract/")
                  calls)))"##;
    let expect = expect![[r#"OK ("/work/archive.tar" nil)"#]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_copy_remote_file_leaves_same_host_remote_paths_untouched() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'copy-file)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       :copied)))
                 (list
                  (arview-copy-remote-file
                   "/ssh:alice@build.example:/src/archive.tar"
                   "/ssh:bob@build.example:/scratch/")
                  calls)))"##;
    let expect = expect![[r#"OK ("/ssh:alice@build.example:/src/archive.tar" nil)"#]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_copy_remote_file_copies_remote_archive_to_local_directory() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'copy-file)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       :copied)))
                 (list
                  (arview-copy-remote-file
                   "/ssh:alice@build.example:/src/archive space.tar"
                   "/local/cache/")
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("/local/cache/archive space.tar" (("/ssh:alice@build.example:/src/archive space.tar" "/local/cache/")))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_copy_remote_file_copies_local_archive_to_remote_directory() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'copy-file)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       :copied)))
                 (list
                  (arview-copy-remote-file
                   "/local/配布 archive.zip"
                   "/ssh:alice@build.example:/scratch/")
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("/ssh:alice@build.example:/scratch/配布 archive.zip" (("/local/配布 archive.zip" "/ssh:alice@build.example:/scratch/")))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_copy_remote_file_copies_between_different_remote_hosts() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'copy-file)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       :copied)))
                 (list
                  (arview-copy-remote-file
                   "/ssh:alice@source.example:/src/資料.tar"
                   "/ssh:bob@target.example:/scratch/")
                  (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("/ssh:bob@target.example:/scratch/資料.tar" (("/ssh:alice@source.example:/src/資料.tar" "/ssh:bob@target.example:/scratch/")))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}

#[test]
fn arview_copy_remote_file_concatenates_destination_exactly_without_normalizing_separator() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'copy-file)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       :copied)))
                 (mapcar
                  (lambda (destination)
                    (list
                     destination
                     (arview-copy-remote-file
                      "/ssh:host:/src/archive.tar"
                      destination)))
                  '("/cache"
                    "/cache/"
                    "/cache//"))))"##;
    let expect = expect![[
        r#"OK (("/cache" "/cachearchive.tar") ("/cache/" "/cache/archive.tar") ("/cache//" "/cache//archive.tar"))"#
    ]];
    assert_arview_parity(elisp_form, expect);
}
