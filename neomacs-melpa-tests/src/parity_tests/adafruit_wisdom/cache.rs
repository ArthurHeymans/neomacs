use expect_test::expect;

use super::assert_adafruit_wisdom_parity;

#[test]
fn adafruit_wisdom_fresh_cache_reads_parses_and_preserves_bytes_without_requesting() {
    let elisp_form = r##"(progn
         (make-directory
          (file-name-directory
           adafruit-wisdom-cache-file)
          t)
         (with-temp-file
             adafruit-wisdom-cache-file
           (set-buffer-file-coding-system
            'utf-8-unix)
           (insert
            "<?xml version=\"1.0\"?><rss><channel><item><title>cached &amp; wise</title></item><item><title>second</title></item></channel></rss>"))
         (let ((before
                (with-temp-buffer
                  (insert-file-contents-literally
                   adafruit-wisdom-cache-file)
                  (buffer-string)))
               request-calls)
           (cl-letf
               (((symbol-function
                  'request)
                 (lambda (&rest arguments)
                   (push
                    arguments
                    request-calls)
                   (error
                    "fresh cache must not request"))))
             (let* ((xml
                     (adafruit-wisdom-cached-get))
                    (items
                     (dom-by-tag
                      xml
                      'item))
                    (after
                     (with-temp-buffer
                       (insert-file-contents-literally
                        adafruit-wisdom-cache-file)
                       (buffer-string))))
               (list
                (caar
                 xml)
                (mapcar
                 (lambda (item)
                   (dom-text
                    (car
                     (dom-by-tag
                      item
                      'title))))
                 items)
                request-calls
                (equal
                 before
                 after)
                after
                (file-exists-p
                 adafruit-wisdom-cache-file))))))"##;
    let expect = expect![[
        r#"OK (rss ("cached & wise" "second") nil t "<?xml version=\"1.0\"?><rss><channel><item><title>cached &amp; wise</title></item><item><title>second</title></item></channel></rss>" t)"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_stale_cache_requests_exactly_once_overwrites_and_parses_response() {
    let elisp_form = r##"(progn
         (make-directory
          (file-name-directory
           adafruit-wisdom-cache-file)
          t)
         (with-temp-file
             adafruit-wisdom-cache-file
           (insert
            "<rss><channel><item><title>stale</title></item></channel></rss>"))
         (set-file-times
          adafruit-wisdom-cache-file
          (seconds-to-time
           0))
         (let ((payload
                "<rss><channel><item><title>fetched</title></item><item><title>new</title></item></channel></rss>")
               request-calls)
           (cl-letf
               (((symbol-function
                  'request)
                 (lambda (url &rest arguments)
                   (push
                    (cons
                     url
                     arguments)
                    request-calls)
                   (make-request-response
                    :data
                    payload))))
             (let* ((xml
                     (adafruit-wisdom-cached-get))
                    (items
                     (dom-by-tag
                      xml
                      'item)))
               (list
                (mapcar
                 (lambda (item)
                   (dom-text
                    (car
                     (dom-by-tag
                      item
                      'title))))
                 items)
                (nreverse
                 request-calls)
                (with-temp-buffer
                  (insert-file-contents-literally
                   adafruit-wisdom-cache-file)
                  (buffer-string)))))))"##;
    let expect = expect![[
        r#"OK (("fetched" "new") (("https://www.adafruit.com/feed/quotes.xml" :type "GET" :sync t :timeout 15 :parser buffer-string)) "<rss><channel><item><title>fetched</title></item><item><title>new</title></item></channel></rss>")"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_missing_cache_creates_file_and_uses_exact_request_contract() {
    let elisp_form = r##"(progn
         (when
             (file-exists-p
              adafruit-wisdom-cache-file)
           (delete-file
            adafruit-wisdom-cache-file))
         (let ((payload
                "<rss><channel><item><title>downloaded</title></item></channel></rss>")
               request-calls)
           (cl-letf
               (((symbol-function
                  'request)
                 (lambda (url &rest arguments)
                   (push
                    (cons
                     url
                     arguments)
                    request-calls)
                   (make-request-response
                    :data
                    payload))))
             (let ((xml
                    (adafruit-wisdom-cached-get)))
               (list
                (mapcar
                 (lambda (item)
                   (dom-text
                    (car
                     (dom-by-tag
                      item
                      'title))))
                 (dom-by-tag
                  xml
                  'item))
                (nreverse
                 request-calls)
                (file-exists-p
                 adafruit-wisdom-cache-file)
                (file-attribute-size
                 (file-attributes
                  adafruit-wisdom-cache-file))
                (with-temp-buffer
                  (insert-file-contents-literally
                   adafruit-wisdom-cache-file)
                  (buffer-string)))))))"##;
    let expect = expect![[
        r#"OK (("downloaded") (("https://www.adafruit.com/feed/quotes.xml" :type "GET" :sync t :timeout 15 :parser buffer-string)) t 68 "<rss><channel><item><title>downloaded</title></item></channel></rss>")"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_cache_ttl_uses_strict_less_than_for_fresh_and_stale_paths() {
    let elisp_form = r##"(progn
         (make-directory
          (file-name-directory
           adafruit-wisdom-cache-file)
          t)
         (with-temp-file
             adafruit-wisdom-cache-file
           (insert
            "<rss><channel><item><title>cache</title></item></channel></rss>"))
         (let ((request-count
                0)
               (fixed-time
                (seconds-to-time
                 2000000000)))
           (cl-letf
               (((symbol-function
                  'request)
                 (lambda (&rest _)
                   (setq
                    request-count
                    (1+
                     request-count))
                   (make-request-response
                   :data
                    "<rss><channel><item><title>network</title></item></channel></rss>")))
                ((symbol-function
                  'current-time)
                 (lambda ()
                   fixed-time)))
             (set-file-times
              adafruit-wisdom-cache-file
              (time-subtract
               fixed-time
               (seconds-to-time
                (-
                 adafruit-wisdom-cache-ttl
                 1))))
             (let ((fresh-title
                    (dom-text
                     (car
                      (dom-by-tag
                       (car
                        (dom-by-tag
                         (adafruit-wisdom-cached-get)
                         'item))
                       'title)))))
               (set-file-times
                adafruit-wisdom-cache-file
                (time-subtract
                 fixed-time
                 (seconds-to-time
                  adafruit-wisdom-cache-ttl)))
               (let ((stale-title
                      (dom-text
                       (car
                        (dom-by-tag
                         (car
                          (dom-by-tag
                           (adafruit-wisdom-cached-get)
                           'item))
                         'title)))))
                 (list
                  fresh-title
                  stale-title
                  request-count))))))"##;
    let expect = expect![[r#"OK ("cache" "network" 1)"#]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_malformed_fresh_cache_signals_xml_error_without_network_fallback() {
    let elisp_form = r##"(progn
         (make-directory
          (file-name-directory
           adafruit-wisdom-cache-file)
          t)
         (with-temp-file
             adafruit-wisdom-cache-file
           (insert
            "<rss><channel><item>"))
         (let ((request-count
                0))
           (cl-letf
               (((symbol-function
                  'request)
                 (lambda (&rest _)
                   (setq
                    request-count
                    (1+
                     request-count))
                   (error
                    "unexpected request"))))
             (condition-case error-data
                 (list
                  'ok
                  (adafruit-wisdom-cached-get))
               (error
                (list
                 'error
                 (car
                  error-data)
                 (error-message-string
                  error-data)
                 request-count
                 (file-exists-p
                  adafruit-wisdom-cache-file)))))))"##;
    let expect = expect![[
        r#"OK (error error "XML: (Not Well-Formed) End of document while reading element ‘item’" 0 t)"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_nil_response_data_signals_insert_error_without_publishing_cache_file() {
    let elisp_form = r##"(progn
         (when
             (file-exists-p
              adafruit-wisdom-cache-file)
           (delete-file
            adafruit-wisdom-cache-file))
         (cl-letf
             (((symbol-function
                'request)
               (lambda (&rest _)
                 (make-request-response
                  :data
                  nil))))
           (condition-case error-data
               (list
                'ok
                (adafruit-wisdom-cached-get))
             (error
              (list
               'error
               (car
                error-data)
               (error-message-string
                error-data)
               (file-exists-p
                adafruit-wisdom-cache-file)
               (and
                (file-exists-p
                 adafruit-wisdom-cache-file)
                (file-attribute-size
                 (file-attributes
                  adafruit-wisdom-cache-file))))))))"##;
    let expect = expect![[
        r#"OK (error wrong-type-argument "Wrong type argument: char-or-string-p, nil" nil nil)"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_request_error_propagates_without_publishing_cache_file() {
    let elisp_form = r##"(progn
         (when
             (file-exists-p
              adafruit-wisdom-cache-file)
           (delete-file
            adafruit-wisdom-cache-file))
         (let ((request-count
                0))
           (cl-letf
               (((symbol-function
                  'request)
                 (lambda (&rest _)
                   (setq
                    request-count
                    (1+
                     request-count))
                   (error
                    "fixture request failure"))))
             (condition-case error-data
                 (list
                  'ok
                  (adafruit-wisdom-cached-get))
               (error
                (list
                 'error
                 (car
                  error-data)
                 (error-message-string
                  error-data)
                 request-count
                 (file-exists-p
                  adafruit-wisdom-cache-file)))))))"##;
    let expect = expect![[r#"OK (error error "fixture request failure" 1 nil)"#]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_malformed_download_is_not_published_and_is_requested_again() {
    let elisp_form = r##"(progn
         (when
             (file-exists-p
              adafruit-wisdom-cache-file)
           (delete-file
            adafruit-wisdom-cache-file))
         (let ((request-count
                0)
               outcomes)
           (cl-letf
               (((symbol-function
                  'request)
                 (lambda (&rest _)
                   (setq
                    request-count
                    (1+
                     request-count))
                   (make-request-response
                    :data
                    "<rss><channel><item>"))))
             (dotimes
                 (_
                  2)
               (push
                (condition-case error-data
                    (list
                     'ok
                     (adafruit-wisdom-cached-get))
                  (error
                   (list
                    'error
                    (car
                     error-data)
                    (error-message-string
                     error-data))))
                outcomes))
             (list
              (nreverse
               outcomes)
              request-count
              (file-exists-p
               adafruit-wisdom-cache-file)
              (and
               (file-exists-p
                adafruit-wisdom-cache-file)
               (with-temp-buffer
                 (insert-file-contents-literally
                  adafruit-wisdom-cache-file)
                 (buffer-string)))))))"##;
    let expect = expect![[
        r#"OK (((error error "XML: (Not Well-Formed) End of document while reading element ‘item’") (error error "XML: (Not Well-Formed) End of document while reading element ‘item’")) 2 nil nil)"#
    ]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}
