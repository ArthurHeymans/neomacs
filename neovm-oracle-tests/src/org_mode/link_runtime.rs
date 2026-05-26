use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_custom_link_follow_export_store_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ol)
  (let (follow-calls)
    (org-link-set-parameters
     "probe"
     :follow (lambda (path arg)
               (push (list path arg) follow-calls))
     :export (lambda (path desc backend _info)
               (format "[%s:%s:%s]" backend path (or desc "")))
     :store (lambda ()
              (org-link-store-props
               :type "probe"
               :link "probe:stored"
               :description "Stored Probe")
              t))
    (with-temp-buffer
      (org-mode)
      (insert "[[probe:abc%20def][Desc]]\n")
      (goto-char (point-min))
      (let ((link (org-element-context)))
        (org-link-open link '(4))
        (let ((html (org-export-string-as
                     "[[probe:abc][Desc]]" 'html t))
              (ascii (org-export-string-as
                      "[[probe:abc]]" 'ascii t))
              (org-stored-links nil)
              (org-store-link-plist nil))
          (org-store-link nil nil)
          (list (nreverse follow-calls)
                html
                ascii
                org-store-link-plist
                org-stored-links))))))"##,
    );
}

#[test]
fn org_link_escape_decode_make_string_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ol)
  (let* ((raw "file:has space/ümlaut?#x")
         (escaped (org-link-escape raw))
         (unescaped (org-link-unescape escaped))
         (encoded (org-link-encode "a b/ç" '(?\s ?/ ?ç)))
         (decoded (org-link-decode encoded)))
    (list escaped
          unescaped
          encoded
          decoded
          (org-link-make-string "https://example.org/a b" "Example")
          (org-link-make-string "https://example.org/a b"))))"##,
    );
}

#[test]
fn org_link_store_props_mail_date_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ol)
  (let ((org-link-from-user-regexp "me@example\\.org")
        org-store-link-plist)
    (org-link-store-props
     :type "mail"
     :from "Me <me@example.org>"
     :to "Ada <ada@example.org>"
     :date "Wed, 27 May 2026 09:30:00 +0000"
     :subject "Hello")
    (org-link-add-props :link "mailto:ada@example.org" :description "Hello")
    (list (plist-get org-store-link-plist :fromname)
          (plist-get org-store-link-plist :fromaddress)
          (plist-get org-store-link-plist :toname)
          (plist-get org-store-link-plist :toaddress)
          (plist-get org-store-link-plist :fromto)
          (plist-get org-store-link-plist :date-timestamp)
          (plist-get org-store-link-plist :date-timestamp-inactive)
          (plist-get org-store-link-plist :link)
          (plist-get org-store-link-plist :description))))"##,
    );
}
