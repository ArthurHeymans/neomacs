use expect_test::expect;

use super::assert_aqi_parity;

#[test]
fn aqi_brief_report_without_cache_requests_city_and_formats_real_summary() {
    let elisp_form = r##"(let ((aqi-use-cache
                nil)
               (aqi-cached-data
                '(("None" . "None")))
               calls)
         (cl-letf
             (((symbol-function
                'aqi-request)
               (lambda (city)
                 (setq calls
                       (append
                        calls
                        (list city)))
                 (push
                  (cons
                   city
                   (aqi-test-city-data
                    "Višnjan"
                    40
                    "o3"))
                  aqi-cached-data)
                 :request-object)))
           (list
            (aqi-report-brief
             "Višnjan")
            calls
            (assoc-default
             'aqi
             (assoc-default
              "Višnjan"
              aqi-cached-data))
            (mapcar
             #'car
             aqi-cached-data))))"##;
    let expect = expect![[
        r#"OK ("Air Quality Index in Višnjan is 40 and the dominant pollutant is o3" ("Višnjan") 40 ("Višnjan" "None"))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_brief_report_cache_hit_avoids_request_and_marks_output_cached() {
    let elisp_form = r##"(let* ((aqi-use-cache
                  t)
                 (fixture
                  (aqi-test-city-data
                   "Taipei"
                   17
                   "pm25"))
                 (aqi-cached-data
                  `(("Taipei" . ,fixture)
                    ("None" . "None")))
                 (before
                  (copy-tree
                   aqi-cached-data))
                 calls)
         (cl-letf
             (((symbol-function
                'aqi-request)
               (lambda (city)
                 (push city calls)
                 (error
                  "cached report requested network"))))
           (list
            (aqi-report-brief
             "Taipei")
            calls
            (equal
             before
             aqi-cached-data)
            aqi-cached-data)))"##;
    let expect = expect![[
        r#"OK ("Air Quality Index in Taipei is 17 and the dominant pollutant is pm25 (cached)" nil t (("Taipei" (aqi . 17) (city (name . "Taipei") (geo . [45.274 13.721]) (url . "https://aqicn.example/station")) (dominentpol . "pm25") (time (s . "2023-05-30 12:00:00") (tz . "+02:00")) (iaqi (pm25 (v . 12)) (pm10 (v . 21)) (no2 (v . 7)) (co (v . 3)) (t (v . 24)) (h (v . 61)) (p (v . 1014)) (wg (v . 5))) (attributions . [((name . "World Air Quality Index")) ((name . "Local Sensor Network"))])) ("None" . "None")))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_brief_report_cache_miss_requests_once_populates_cache_and_marks_output_cached() {
    let elisp_form = r##"(let ((aqi-use-cache
                t)
               (aqi-cached-data
                '(("None" . "None")))
               calls)
         (cl-letf
             (((symbol-function
                'aqi-request)
               (lambda (city)
                 (setq calls
                       (append
                        calls
                        (list city)))
                 (push
                  (cons
                   city
                   (aqi-test-city-data
                    city
                    73
                    "pm10"))
                  aqi-cached-data)
                 :request-object)))
           (list
            (aqi-report-brief
             "Delhi")
            calls
            (aqi--cached-city?
             "Delhi")
            (assoc-default
             'aqi
             (aqi--city-cache-get
              "Delhi")))))"##;
    let expect = expect![[
        r#"OK ("Air Quality Index in Delhi is 73 and the dominant pollutant is pm10 (cached)" ("Delhi") t 73)"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_brief_report_place_normalization_exposes_nil_empty_string_name_and_non_string_boundaries() {
    let elisp_form = r##"(let ((aqi-use-cache
                nil)
               (aqi-cached-data
                '(("None" . "None")))
               calls)
         (cl-letf
             (((symbol-function
                'aqi-request)
               (lambda (city)
                 (setq calls
                       (append
                        calls
                        (list city)))
                 (push
                  (cons
                   city
                   (aqi-test-city-data
                    city
                    25
                    "pm25"))
                  aqi-cached-data)
                 :request-object)))
           (list
            (mapcar
             (lambda (place)
               (condition-case error-data
                   (list
                    place
                    :ok
                    (aqi-report-brief
                     place))
                 (error
                  (list
                   place
                   :error
                   (car error-data)
                   (cdr error-data)))))
             (list
              nil
              ""
              "Osaka"
              42))
            calls)))"##;
    let expect = expect![[
        r#"OK (((nil :ok "Air Quality Index in here is 25 and the dominant pollutant is pm25") ("" :ok "Air Quality Index in here is 25 and the dominant pollutant is pm25") ("Osaka" :ok "Air Quality Index in Osaka is 25 and the dominant pollutant is pm25") (42 :error wrong-type-argument (stringp 42))) ("here" "here" "Osaka"))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_full_report_formats_complete_practical_org_table_with_all_measurements_and_attributions() {
    let elisp_form = r##"(let ((aqi-use-cache
                nil)
               (aqi-cached-data
                '(("None" . "None")))
               calls)
         (cl-letf
             (((symbol-function
                'org-mode)
               (lambda ()
                 :org-fixture))
              ((symbol-function
                'aqi-request)
               (lambda (city)
                 (push city calls)
                 (push
                  (cons
                   city
                   (aqi-test-city-data
                    city
                    40
                    "o3"))
                  aqi-cached-data)
                 :request-object)))
           (list
            (aqi-report-full
             "Višnjan")
            (nreverse
             calls)
            (length
             (split-string
              (aqi-report-full
               "Višnjan")
              "\n"
              nil)))))"##;
    let expect = expect![[
        r#"OK ("* Air Quality index in Višnjan is 40\n\nMost recent report at 2023-05-30 12:00:00 (UTC+02:00).\n\n| Dominant pollutant                   |   o3 |\n| PM2.5 (fine particulate matter)      |   12 |\n| PM10 (respirable particulate matter) |   21 |\n| NO2 (Nitrogen Dioxide)               |   7 |\n| CO (Carbon Monoxide)                 |   3 |\n|                                      |    |\n| Temperature (Celsius)                |   24 |\n| Humidity                             |   61 |\n| Air pressure                         |   1014 |\n| Wind                                 |   5 |\n\nFurther details can be found at [[https://aqicn.example/station][aqicn]].\n\nData provided by World Air Quality Index and Local Sensor Network" ("Višnjan") 18)"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_full_report_formats_complete_plain_text_when_org_mode_is_unavailable() {
    let elisp_form = r##"(let ((aqi-use-cache
                nil)
               (aqi-cached-data
                '(("None" . "None")))
               (had-org-mode
                (fboundp
                 'org-mode))
               (saved-org-mode
                (and
                 (fboundp
                  'org-mode)
                 (symbol-function
                  'org-mode))))
         (unwind-protect
             (progn
               (when
                   had-org-mode
                 (fmakunbound
                  'org-mode))
               (cl-letf
                   (((symbol-function
                      'aqi-request)
                     (lambda (city)
                       (push
                        (cons
                         city
                         (aqi-test-city-data
                          city
                          51
                          "pm25"))
                        aqi-cached-data)
                       :request-object)))
                 (list
                  (fboundp
                   'org-mode)
                  (aqi-report-full
                   "Sydney"))))
           (when
               had-org-mode
             (fset
              'org-mode
              saved-org-mode))))"##;
    let expect = expect![[
        r#"OK (nil "Air Quality index in Sydney is 51 as of 2023-05-30 12:00:00 (UTC+02:00).\n\nDominant pollutant is pm25\nPM2.5 (fine particulate matter): 12\nPM10 (respirable particulate matter): 21\nNO2 (Nitrogen Dioxide): 7\nCO (Carbon Monoxide): 3\n\nTemperature (Celsius): 24\nHumidity: 61\nAir pressure: 1014\nWind: 5\n\nFurther details can be found at [[https://aqicn.example/station][aqicn]].\n\nData provided by World Air Quality Index and Local Sensor Network")"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_full_report_returns_cached_semantic_error_with_city_context() {
    let elisp_form = r##"(let ((aqi-use-cache
                t)
               (aqi-cached-data
                '(("Missing"
                   . "Request error: Unknown station")
                  ("None" . "None")))
               calls)
         (cl-letf
             (((symbol-function
                'aqi-request)
               (lambda (city)
                 (push city calls)
                 (error
                  "semantic cache entry must avoid network"))))
           (list
            (aqi-report-full
             "Missing")
            calls
            aqi-cached-data)))"##;
    let expect = expect![[
        r#"OK ("Request error: Unknown station (Missing)" nil (("Missing" . "Request error: Unknown station") ("None" . "None")))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_full_report_exposes_missing_optional_measurements_as_nil_without_losing_structure() {
    let elisp_form = r##"(let ((aqi-use-cache
                nil)
               (aqi-cached-data
                '(("None" . "None"))))
         (cl-letf
             (((symbol-function
                'org-mode)
               (lambda ()
                 :org-fixture))
              ((symbol-function
                'aqi-request)
               (lambda (city)
                 (push
                  (cons
                   city
                   `((aqi . 5)
                     (city
                      (name . ,city)
                      (url . "https://aqicn.example/sparse"))
                     (dominentpol . "unknown")
                     (time
                      (s . "unknown")
                      (tz . "+00:00"))
                     (attributions
                      . [((name . "Primary"))
                         ((name . "Secondary"))])))
                  aqi-cached-data)
                 :request-object)))
           (aqi-report-full
            "Sparse")))"##;
    let expect = expect![[
        r#"OK "* Air Quality index in Sparse is 5\n\nMost recent report at unknown (UTC+00:00).\n\n| Dominant pollutant                   |   unknown |\n| PM2.5 (fine particulate matter)      |   nil |\n| PM10 (respirable particulate matter) |   nil |\n| NO2 (Nitrogen Dioxide)               |   nil |\n| CO (Carbon Monoxide)                 |   nil |\n|                                      |    |\n| Temperature (Celsius)                |   nil |\n| Humidity                             |   nil |\n| Air pressure                         |   nil |\n| Wind                                 |   nil |\n\nFurther details can be found at [[https://aqicn.example/sparse][aqicn]].\n\nData provided by Primary and Secondary""#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_report_brief_command_builds_org_buffer_displays_it_and_returns_true() {
    let elisp_form = r##"(let (events)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'aqi-report-brief)
                   (lambda (city)
                     (push
                      (list
                       :brief
                       city)
                      events)
                     "Air Quality Index in Osaka is 42 and the dominant pollutant is pm25"))
                  ((symbol-function
                    'org-mode)
                   (lambda ()
                     (setq
                      major-mode
                      'org-mode
                      mode-name
                      "Org Fixture")
                     (push
                      (list
                       :mode
                       (buffer-name))
                      events)))
                  ((symbol-function
                    'display-buffer)
                   (lambda (buffer &rest _)
                     (push
                      (list
                       :display
                       (buffer-name buffer)
                       (with-current-buffer buffer
                         (buffer-string))
                       (with-current-buffer buffer
                         major-mode))
                      events)
                     :displayed)))
               (let ((result
                      (aqi-report
                       "Osaka"
                       'brief))
                     (buffer
                      (get-buffer
                       "*Air Quality - Osaka*")))
                 (list
                  result
                  (nreverse
                   events)
                  (and
                   buffer
                   (with-current-buffer buffer
                     (list
                      (buffer-string)
                      major-mode
                      mode-name
                      (point)))))))
           (aqi-test-kill-report-buffers)))"##;
    let expect = expect![[
        r#"OK (t ((:brief "Osaka") (:mode "*Air Quality - Osaka*") (:display "*Air Quality - Osaka*" "Air Quality Index in Osaka is 42 and the dominant pollutant is pm25" org-mode)) ("Air Quality Index in Osaka is 42 and the dominant pollutant is pm25" org-mode "Org Fixture" 68))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_report_default_full_reuses_and_erases_city_buffer_before_each_render() {
    let elisp_form = r##"(let ((reports
                '("first detailed report"
                  "second detailed report"))
               displayed)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'aqi-report-full)
                   (lambda (_city)
                     (pop reports)))
                  ((symbol-function
                    'org-mode)
                   (lambda ()
                     (setq
                      major-mode
                      'org-mode)))
                  ((symbol-function
                    'display-buffer)
                   (lambda (buffer &rest _)
                     (setq displayed
                           (append
                            displayed
                            (list
                             (with-current-buffer buffer
                               (buffer-string)))))
                     :displayed)))
               (let ((first
                      (aqi-report
                       "Ulaanbaatar"))
                     (first-buffer
                      (get-buffer
                       "*Air Quality - Ulaanbaatar*"))
                     second
                     second-buffer)
                 (setq second
                       (aqi-report
                        "Ulaanbaatar"
                        'full)
                       second-buffer
                       (get-buffer
                        "*Air Quality - Ulaanbaatar*"))
                 (list
                  first
                  second
                  (eq
                   first-buffer
                   second-buffer)
                  displayed
                  (with-current-buffer
                      second-buffer
                    (buffer-string))
                  reports)))
           (aqi-test-kill-report-buffers)))"##;
    let expect = expect![[
        r#"OK (t t t ("first detailed report" "second detailed report") "second detailed report" nil)"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_report_unknown_type_warns_leaves_empty_org_buffer_and_still_displays_it() {
    let elisp_form = r##"(let (warnings displays)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'warn)
                   (lambda (format-string &rest arguments)
                     (let ((text
                            (apply
                             #'format
                             format-string
                             arguments)))
                       (push text warnings)
                       text)))
                  ((symbol-function
                    'org-mode)
                   (lambda ()
                     (setq
                      major-mode
                      'org-mode)))
                  ((symbol-function
                    'display-buffer)
                   (lambda (buffer &rest _)
                     (push
                      (list
                       (buffer-name buffer)
                       (with-current-buffer buffer
                         (buffer-string))
                       (with-current-buffer buffer
                         major-mode))
                      displays)
                     :displayed)))
               (list
                (aqi-report
                 "Osaka"
                 'compact)
                (nreverse
                 warnings)
                (nreverse
                 displays)))
           (aqi-test-kill-report-buffers)))"##;
    let expect = expect![[
        r#"OK (t ("Unknown report type: 'compact. Try using 'full or 'brief") (("*Air Quality - Osaka*" "" org-mode)))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_report_keeps_independent_buffers_and_contents_for_multiple_real_city_names() {
    let elisp_form = r##"(let (displayed)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'aqi-report-brief)
                   (lambda (city)
                     (format
                      "brief:%s"
                      city)))
                  ((symbol-function
                    'org-mode)
                   (lambda ()
                     (setq
                      major-mode
                      'org-mode)))
                  ((symbol-function
                    'display-buffer)
                   (lambda (buffer &rest _)
                     (push
                      (buffer-name buffer)
                      displayed)
                     :displayed)))
               (mapcar
                (lambda (city)
                  (aqi-report
                   city
                   'brief))
                '("New Delhi"
                  "Kraków"
                  "Višnjan"))
               (list
                (nreverse
                 displayed)
                (mapcar
                 (lambda (city)
                   (let ((buffer
                          (get-buffer
                           (format
                            "*Air Quality - %s*"
                            city))))
                     (list
                      city
                      (and
                       buffer
                       (with-current-buffer buffer
                         (buffer-string)))
                      (and
                       buffer
                       (with-current-buffer buffer
                         major-mode)))))
                 '("New Delhi"
                   "Kraków"
                   "Višnjan"))))
           (aqi-test-kill-report-buffers)))"##;
    let expect = expect![[
        r#"OK (("*Air Quality - New Delhi*" "*Air Quality - Kraków*" "*Air Quality - Višnjan*") (("New Delhi" "brief:New Delhi" org-mode) ("Kraków" "brief:Kraków" org-mode) ("Višnjan" "brief:Višnjan" org-mode)))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}
