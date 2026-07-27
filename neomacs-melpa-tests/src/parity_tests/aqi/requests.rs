use expect_test::expect;

use super::assert_aqi_parity;

#[test]
fn aqi_request_success_uses_exact_url_token_sync_parser_and_caches_response_data() {
    let elisp_form = r##"(let ((aqi-api-key
                "fixture-token")
               (aqi-cached-data
                '(("None" . "None")))
               captured)
         (cl-letf
             (((symbol-function
                'request)
               (lambda (url &rest arguments)
                 (setq captured
                       (list
                        url
                        (plist-get
                         arguments
                         :sync)
                        (plist-get
                         arguments
                         :params)
                        (plist-get
                         arguments
                         :parser)
                        (functionp
                         (plist-get
                          arguments
                          :success))
                        (functionp
                         (plist-get
                          arguments
                          :error))))
                 (funcall
                  (plist-get
                   arguments
                   :success)
                  :data
                  `((status . "ok")
                    (data
                     . ,(aqi-test-city-data
                         "Višnjan"
                         40
                         "o3"))))
                 :request-object)))
           (let ((result
                  (aqi-request
                   "Višnjan")))
             (list
              result
              captured
              (assoc-default
               'aqi
               (assoc-default
                "Višnjan"
                aqi-cached-data))
              (mapcar
               #'car
               aqi-cached-data)))))"##;
    let expect = expect![[
        r#"OK (:request-object ("https://api.waqi.info/feed/Višnjan/" t (("token" . "fixture-token")) json-read t t) 40 ("Višnjan" "None"))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_request_api_error_caches_human_readable_error_and_returns_transport_object() {
    let elisp_form = r##"(let ((aqi-cached-data
                '(("None" . "None")))
               captured)
         (cl-letf
             (((symbol-function
                'request)
               (lambda (url &rest arguments)
                 (setq captured
                       (list
                        url
                        (plist-get
                         arguments
                         :params)))
                 (funcall
                  (plist-get
                   arguments
                   :success)
                  :data
                  '((status . "error")
                    (data . "Unknown station")))
                 :request-object)))
           (list
            (aqi-request
             "@missing")
            captured
            aqi-cached-data
            (aqi--cached-city?
             "@missing")
            (aqi--city-cache-get
             "@missing"))))"##;
    let expect = expect![[
        r#"OK (:request-object ("https://api.waqi.info/feed/@missing/" (("token" . "demo"))) (("@missing" . "Request error: Unknown station") ("None" . "None")) t "Request error: Unknown station")"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_request_transport_error_reports_exact_message_without_mutating_cache() {
    let elisp_form = r##"(let ((aqi-cached-data
                '(("None" . "None")))
               messages)
         (cl-letf
             (((symbol-function
                'message)
               (lambda (format-string &rest arguments)
                 (let ((text
                        (apply
                         #'format
                         format-string
                         arguments)))
                   (setq messages
                         (append
                          messages
                          (list text)))
                   text)))
              ((symbol-function
                'request)
               (lambda (_url &rest arguments)
                 (funcall
                  (plist-get
                   arguments
                   :error)
                  :error-thrown
                  "connection refused"
                  :response
                  :fixture-response)
                 :request-object)))
           (list
            (aqi-request
             "Offline")
            messages
            aqi-cached-data
            (aqi--cached-city?
             "Offline"))))"##;
    let expect = expect![[
        r#"OK (:request-object ("WAQI error: connection refused") (("None" . "None")) nil)"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_repeated_success_requests_preserve_duplicate_history_and_newest_value_wins() {
    let elisp_form = r##"(let ((aqi-cached-data
                '(("None" . "None")))
               (scores
                '(10 20 30)))
         (cl-letf
             (((symbol-function
                'request)
               (lambda (_url &rest arguments)
                 (let ((score
                        (pop scores)))
                   (funcall
                    (plist-get
                     arguments
                     :success)
                    :data
                    `((status . "ok")
                      (data
                       . ,(aqi-test-city-data
                           "Osaka"
                           score
                           "pm25")))))
                 :request-object)))
           (dotimes
               (_
                3)
             (aqi-request
              "Osaka"))
           (list
            (mapcar
             (lambda (entry)
               (list
                (car entry)
                (and
                 (listp
                  (cdr entry))
                 (assoc-default
                  'aqi
                  (cdr entry)))))
             aqi-cached-data)
            (assoc-default
             'aqi
             (aqi--city-cache-get
              "Osaka"))
            scores)))"##;
    let expect = expect![[r#"OK ((("Osaka" 30) ("Osaka" 20) ("Osaka" 10) ("None" nil)) 30 nil)"#]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_geo_request_formats_coordinates_forwards_token_and_reports_success_payload() {
    let elisp_form = r##"(let ((aqi-api-key
                "geo-token")
               captured
               messages)
         (cl-letf
             (((symbol-function
                'message)
               (lambda (format-string &rest arguments)
                 (let ((text
                        (apply
                         #'format
                         format-string
                         arguments)))
                   (setq messages
                         (append
                          messages
                          (list text)))
                   text)))
              ((symbol-function
                'request)
               (lambda (url &rest arguments)
                 (setq captured
                       (list
                        url
                        (plist-get
                         arguments
                         :sync)
                        (plist-get
                         arguments
                         :params)
                        (plist-get
                         arguments
                         :parser)))
                 (funcall
                  (plist-get
                   arguments
                   :success)
                  :data
                  '((status . "ok")
                    (data
                     (aqi . 51))))
                 :geo-request)))
           (list
            (aqi-request-geo
             -33.8688
             151.2093)
            captured
            messages)))"##;
    let expect = expect![[
        r#"OK (:geo-request ("https://api.waqi.info/feed/geo:-33.8688;151.2093/" t (("token" . "geo-token")) json-read) ("200: ((status . ok) (data (aqi . 51)))"))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_geo_request_transport_error_uses_shared_error_message_contract() {
    let elisp_form = r##"(let (captured messages)
         (cl-letf
             (((symbol-function
                'message)
               (lambda (format-string &rest arguments)
                 (let ((text
                        (apply
                         #'format
                         format-string
                         arguments)))
                   (push text messages)
                   text)))
              ((symbol-function
                'request)
               (lambda (url &rest arguments)
                 (setq captured url)
                 (funcall
                  (plist-get
                   arguments
                   :error)
                  :error-thrown
                  '(timeout . 30))
                 :geo-request)))
           (list
            (aqi-request-geo
             0
             0)
            captured
            (nreverse
             messages))))"##;
    let expect = expect![[
        r#"OK (:geo-request "https://api.waqi.info/feed/geo:0;0/" ("WAQI error: (timeout . 30)"))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_search_reports_ok_and_api_error_payloads_with_exact_unescaped_query_urls() {
    let elisp_form = r##"(let (calls messages)
         (cl-letf
             (((symbol-function
                'message)
               (lambda (format-string &rest arguments)
                 (let ((text
                        (apply
                         #'format
                         format-string
                         arguments)))
                   (setq messages
                         (append
                          messages
                          (list text)))
                   text)))
              ((symbol-function
                'request)
               (lambda (url &rest arguments)
                 (setq calls
                       (append
                        calls
                        (list
                         (list
                          url
                          (plist-get
                           arguments
                           :sync)
                          (plist-get
                           arguments
                           :params)
                          (plist-get
                           arguments
                           :parser)))))
                 (funcall
                  (plist-get
                   arguments
                   :success)
                  :data
                  (if
                      (string-match-p
                       "New Delhi"
                       url)
                      '((status . "ok")
                        (data
                         . [((station
                              (name . "Delhi")))]))
                    '((status . "error")
                      (data . "No stations"))))
                 :search-request)))
           (list
            (aqi-search
             "New Delhi")
            (aqi-search
             "Øresund")
            calls
            messages)))"##;
    let expect = expect![[
        r#"OK (:search-request :search-request (("https://api.waqi.info/search/?keyword=New Delhi&" t (("token" . "demo")) json-read) ("https://api.waqi.info/search/?keyword=Øresund&" t (("token" . "demo")) json-read)) ("Search: [((station (name . Delhi)))]" "Search error: No stations"))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_search_transport_error_reports_failure_and_preserves_exact_unicode_query() {
    let elisp_form = r##"(let (captured messages)
         (cl-letf
             (((symbol-function
                'message)
               (lambda (format-string &rest arguments)
                 (let ((text
                        (apply
                         #'format
                         format-string
                         arguments)))
                   (push text messages)
                   text)))
              ((symbol-function
                'request)
               (lambda (url &rest arguments)
                 (setq captured
                       (list
                        url
                        (plist-get
                         arguments
                         :params)))
                 (funcall
                  (plist-get
                   arguments
                   :error)
                  :error-thrown
                  "TLS failure")
                 :search-request)))
           (list
            (aqi-search
             "Kraków center")
            captured
            (nreverse
             messages))))"##;
    let expect = expect![[
        r#"OK (:search-request ("https://api.waqi.info/search/?keyword=Kraków center&" (("token" . "demo"))) ("WAQI error: TLS failure"))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}
