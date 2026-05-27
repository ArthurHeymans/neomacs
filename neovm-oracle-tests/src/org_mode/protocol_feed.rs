use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_protocol_parse_store_open_source_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-protocol)
  (let* ((root (make-temp-file "org-protocol" t))
         (work (expand-file-name "site" root))
         (article (expand-file-name "posts/article.org" work))
         (index (expand-file-name "index.org" work))
         (org-stored-links nil)
         (kill-ring nil)
         (org-protocol-project-alist
          `(("site"
             :base-url "https://example.org/"
             :working-directory ,(file-name-as-directory work)
             :online-suffix ".html"
             :working-suffix ".org"
             :rewrites (("https://example\\.org/?$" . "index.org"))))))
    (unwind-protect
        (progn
          (make-directory (file-name-directory article) t)
          (with-temp-file article (insert "* Article\n"))
          (with-temp-file index (insert "* Index\n"))
          (let* ((query "url=https%3A%2F%2Fexample.org%2Fposts%2Farticle.html&title=Hello+World&body=A%2FB")
                 (plist (org-protocol-parse-parameters query t))
                 (old (org-protocol-parse-parameters
                       "https:%2F%2Fexample.org%2Fold/Old%20Title/body"
                       nil '(:url :title :body)))
                 (split (org-protocol-split-data "a%2Fb/c+d" t))
                 (flat (org-protocol-flatten-greedy
                        '("/tmp/org-protocol:/greedy:/one" ("two" (3 . 4)))
                        t "<cwd>/"))
                 (store (org-protocol-store-link plist))
                 (opened (org-protocol-open-source
                          '(:url "https://example.org/posts/article.html?utm=1")))
                 (rewritten (org-protocol-open-source
                             '(:url "https://example.org/"))))
            (list plist
                  old
                  split
                  flat
                  store
                  org-stored-links
                  kill-ring
                  (file-relative-name opened root)
                  (file-relative-name rewritten root))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_feed_parse_format_status_add_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-feed)
  (let* ((root (make-temp-file "org-feed" t))
         (file (expand-file-name "feeds.org" root))
         (rss (get-buffer-create " *rss-feed*")))
    (unwind-protect
        (progn
          (with-current-buffer rss
            (erase-buffer)
            (insert "<?xml version=\"1.0\"?><rss><channel>\n")
            (insert "<item><guid isPermaLink=\"false\">g-1</guid><title>One &amp; Two</title><link>https://example.org/1</link><description>Line 1\nLine 2</description><pubDate>2026-05-27</pubDate></item>\n")
            (insert "<item><guid>https://example.org/2</guid><title>Second</title><link>https://example.org/2</link><description>Desc</description></item>\n")
            (insert "</channel></rss>"))
          (with-temp-file file (insert "* Inbox\nExisting\n"))
          (let* ((raw (org-feed-parse-rss-feed rss))
                 (parsed (mapcar #'org-feed-parse-rss-entry raw))
                 (formatted
                  (mapcar (lambda (entry)
                            (org-feed-format-entry
                             entry
                             "\n* TODO %h\n  %u\n  %description\n  %a"
                             nil))
                          parsed))
                 (pos (org-feed-goto-inbox-internal file "Inbox"))
                 (status '(("old" t "abc"))))
            (org-feed-add-items pos formatted)
            (org-feed-write-status
             pos "FEEDSTATUS"
             (append status
                     (mapcar (lambda (entry)
                               (list (plist-get entry :guid)
                                     t
                                     (sha1 (plist-get entry :item-full-text))))
                             parsed)))
            (list (mapcar (lambda (entry)
                            (list (plist-get entry :guid)
                                  (plist-get entry :title)
                                  (plist-get entry :guid-permalink)
                                  (plist-get entry :link)))
                          parsed)
                  (org-feed-read-previous-status pos "FEEDSTATUS")
                  (with-current-buffer (find-file-noselect file)
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))))
      (when (get-buffer rss) (kill-buffer rss))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_feed_update_with_custom_retriever_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-feed)
  (let* ((root (make-temp-file "org-feed-update" t))
         (file (expand-file-name "feeds.org" root))
         (org-feed-save-after-adding nil)
         (org-feed-retrieve-method
          (lambda (_url)
            (let ((buf (get-buffer-create " *mock-feed*")))
              (with-current-buffer buf
                (erase-buffer)
                (insert "<?xml version=\"1.0\"?><rss><channel>")
                (insert "<item><guid>https://example.org/new</guid><title>New Item</title><link>https://example.org/new</link><description>New desc</description></item>")
                (insert "</channel></rss>"))
              buf)))
         (feed (list "Mock" "mock://feed" file "Inbox"
                     :drawer "MOCKSTATUS"
                     :filter (lambda (entry)
                               (and (string-match-p "New" (plist-get entry :title))
                                    entry)))))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* Inbox\n")
            (insert ":MOCKSTATUS:\n((\"https://example.org/old\" t \"oldsha\"))\n:END:\n"))
          (let ((first (org-feed-update feed))
                (second (org-feed-update feed)))
            (with-current-buffer (find-file-noselect file)
              (list first
                    second
                    (org-feed-read-previous-status (point-min) "MOCKSTATUS")
                    (buffer-substring-no-properties
                     (point-min) (point-max))))))
      (when (get-buffer " *mock-feed*") (kill-buffer " *mock-feed*"))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-directory root t))))"##,
    );
}
