use expect_test::expect;

use super::assert_aqi_parity;

#[test]
fn aqi_cache_clear_all_restores_sentinel_and_single_city_clear_uses_identity_keys() {
    let elisp_form = r##"(let* ((shared-city
                  (copy-sequence
                   "Osaka"))
                 (equal-city
                  (copy-sequence
                   "Osaka"))
                 (aqi-cached-data
                  (list
                   (cons shared-city :old)
                   (cons "Taipei" :other)
                   (cons shared-city :new))))
         (aqi--city-cache-clear
          equal-city)
         (let ((after-equal-key
                (copy-tree
                 aqi-cached-data)))
           (aqi--city-cache-clear
            shared-city)
           (let ((after-identical-key
                  (copy-tree
                   aqi-cached-data)))
             (aqi--city-cache-clear)
             (list
              after-equal-key
              after-identical-key
              aqi-cached-data))))"##;
    let expect = expect![[
        r#"OK ((("Osaka" . :old) ("Taipei" . :other) ("Osaka" . :new)) (("Taipei" . :other)) (("None" . "None")))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_cached_city_predicate_distinguishes_missing_nil_false_and_truthy_cached_values() {
    let elisp_form = r##"(let ((aqi-cached-data
                '(("Nil" . nil)
                  ("False" . nil)
                  ("Zero" . 0)
                  ("Empty" . "")
                  ("Data" . ((aqi . 42))))))
         (mapcar
          (lambda (city)
            (list
             city
             (assoc-default
              city
              aqi-cached-data)
             (aqi--cached-city?
              city)))
          '("Missing"
            "Nil"
            "False"
            "Zero"
            "Empty"
            "Data")))"##;
    let expect = expect![[
        r#"OK (("Missing" nil nil) ("Nil" nil nil) ("False" nil nil) ("Zero" 0 t) ("Empty" "" t) ("Data" ((aqi . 42)) t))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_city_cache_get_hit_returns_current_value_without_network_or_mutation() {
    let elisp_form = r##"(let* ((fixture
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
                  "cache hit requested network"))))
           (let ((value
                  (aqi--city-cache-get
                   "Taipei")))
             (list
              (equal value fixture)
              (assoc-default
               'aqi
               value)
              calls
              (equal
               before
               aqi-cached-data)
              aqi-cached-data))))"##;
    let expect = expect![[
        r#"OK (t 17 nil t (("Taipei" (aqi . 17) (city (name . "Taipei") (geo . [45.274 13.721]) (url . "https://aqicn.example/station")) (dominentpol . "pm25") (time (s . "2023-05-30 12:00:00") (tz . "+02:00")) (iaqi (pm25 (v . 12)) (pm10 (v . 21)) (no2 (v . 7)) (co (v . 3)) (t (v . 24)) (h (v . 61)) (p (v . 1014)) (wg (v . 5))) (attributions . [((name . "World Air Quality Index")) ((name . "Local Sensor Network"))])) ("None" . "None")))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_city_cache_get_miss_updates_once_stores_the_returned_data_and_reuses_it() {
    let elisp_form = r##"(let ((aqi-cached-data
                '(("None" . "None")))
               (request-count
                0))
         (cl-letf
             (((symbol-function
                'aqi-request)
               (lambda (city)
                 (setq
                  request-count
                  (1+
                   request-count))
                 (aqi-test-city-data
                  city
                  33
                  "pm10"))))
           (let ((first
                  (aqi--city-cache-get
                   "Delhi"))
                 (after-first
                  nil))
             (setq after-first
                   (copy-tree
                    aqi-cached-data))
             (let ((second
                    (aqi--city-cache-get
                     "Delhi")))
               (list
                (assoc-default
                 'aqi
                 first)
                (equal first second)
                request-count
                after-first
                aqi-cached-data)))))"##;
    let expect = expect![[
        r#"OK (33 t 1 (("Delhi" (aqi . 33) (city (name . "Delhi") (geo . #1=[45.274 13.721]) (url . "https://aqicn.example/station")) (dominentpol . "pm10") (time (s . "2023-05-30 12:00:00") (tz . "+02:00")) (iaqi (pm25 (v . 12)) (pm10 (v . 21)) (no2 (v . 7)) (co (v . 3)) (t (v . 24)) (h (v . 61)) (p (v . 1014)) (wg (v . 5))) (attributions . #2=[((name . "World Air Quality Index")) ((name . "Local Sensor Network"))])) ("None" . "None")) (("Delhi" (aqi . 33) (city (name . "Delhi") (geo . #1#) (url . "https://aqicn.example/station")) (dominentpol . "pm10") (time (s . "2023-05-30 12:00:00") (tz . "+02:00")) (iaqi (pm25 (v . 12)) (pm10 (v . 21)) (no2 (v . 7)) (co (v . 3)) (t (v . 24)) (h (v . 61)) (p (v . 1014)) (wg (v . 5))) (attributions . #2#)) ("None" . "None")))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_city_cache_update_replaces_all_identity_matching_entries_and_preserves_other_cities() {
    let elisp_form = r##"(let* ((city
                  (copy-sequence
                   "Osaka"))
                 (aqi-cached-data
                  (list
                   (cons city :stale-one)
                   (cons "Taipei" :keep)
                   (cons city :stale-two)))
                 calls)
         (cl-letf
             (((symbol-function
                'aqi-request)
               (lambda (requested-city)
                 (push requested-city calls)
                 (aqi-test-city-data
                  requested-city
                  51
                  "o3"))))
           (let ((result
                  (aqi--city-cache-update
                   city)))
             (list
              result
              calls
              aqi-cached-data
              (length
               (seq-filter
                (lambda (entry)
                  (eq
                   (car entry)
                   city))
                aqi-cached-data))))))"##;
    let expect = expect![[
        r#"OK (#1=(("Osaka" (aqi . 51) (city (name . "Osaka") (geo . [45.274 13.721]) (url . "https://aqicn.example/station")) (dominentpol . "o3") (time (s . "2023-05-30 12:00:00") (tz . "+02:00")) (iaqi (pm25 (v . 12)) (pm10 (v . 21)) (no2 (v . 7)) (co (v . 3)) (t (v . 24)) (h (v . 61)) (p (v . 1014)) (wg (v . 5))) (attributions . [((name . "World Air Quality Index")) ((name . "Local Sensor Network"))])) ("Taipei" . :keep)) ("Osaka") #1# 1)"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_cache_update_with_real_success_callback_exposes_transport_and_data_entry_order() {
    let elisp_form = r##"(let ((aqi-cached-data
                '(("None" . "None")))
               calls)
         (cl-letf
             (((symbol-function
                'request)
               (lambda (url &rest arguments)
                 (setq calls
                       (append
                        calls
                        (list
                         (cons
                          url
                          arguments))))
                 (funcall
                  (plist-get
                   arguments
                   :success)
                  :data
                  `((status . "ok")
                    (data
                     . ,(aqi-test-city-data
                         "Osaka"
                         42
                         "pm25"))))
                 :request-object)))
           (let ((result
                  (aqi--city-cache-update
                   "Osaka")))
             (list
              result
              (assoc-default
               "Osaka"
               aqi-cached-data)
              (mapcar
               (lambda (entry)
                 (list
                  (car entry)
                  (if
                      (listp
                       (cdr entry))
                      (assoc-default
                       'aqi
                       (cdr entry))
                    (cdr entry))))
               aqi-cached-data)
              (length calls)))))"##;
    let expect = expect![[
        r#"OK ((("Osaka" . :request-object) ("Osaka" (aqi . 42) (city (name . "Osaka") (geo . [45.274 13.721]) (url . "https://aqicn.example/station")) (dominentpol . "pm25") (time (s . "2023-05-30 12:00:00") (tz . "+02:00")) (iaqi (pm25 (v . 12)) (pm10 (v . 21)) (no2 (v . 7)) (co (v . 3)) (t (v . 24)) (h (v . 61)) (p (v . 1014)) (wg (v . 5))) (attributions . [((name . "World Air Quality Index")) ((name . "Local Sensor Network"))])) ("None" . "None")) :request-object (("Osaka" :request-object) ("Osaka" 42) ("None" "None")) 1)"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_request_cached_routes_hits_and_misses_and_reuses_newly_cached_success_data() {
    let elisp_form = r##"(let* ((existing
                  (aqi-test-city-data
                   "Existing"
                   11
                   "pm25"))
                 (aqi-cached-data
                  `(("Existing" . ,existing)
                    ("None" . "None")))
                 calls)
         (cl-letf
             (((symbol-function
                'aqi-request)
               (lambda (city)
                 (setq calls
                       (append
                        calls
                        (list city)))
                 (let ((data
                        (aqi-test-city-data
                         city
                         88
                         "o3")))
                   (push
                    (cons city data)
                    aqi-cached-data)
                   :request-object))))
           (let ((hit
                  (aqi-request-cached
                   "Existing"))
                 (miss-result
                  (aqi-request-cached
                   "New"))
                 (new-hit
                  (aqi-request-cached
                   "New")))
             (list
              (assoc-default
               'aqi
               hit)
              miss-result
              (assoc-default
               'aqi
               new-hit)
              calls
              (mapcar
               #'car
               aqi-cached-data)))))"##;
    let expect = expect![[r#"OK (11 :request-object 88 ("New") ("New" "Existing" "None"))"#]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_refresh_period_is_configuration_only_and_does_not_expire_cached_data() {
    let elisp_form = r##"(let* ((aqi-cache-refresh-period
                  1)
                 (aqi-cached-data
                  `(("Old"
                     . ,(aqi-test-city-data
                        "Old"
                        9
                        "pm10"))))
                 (request-count
                  0))
         (cl-letf
             (((symbol-function
                'current-time)
               (lambda ()
                 (seconds-to-time
                  4000000000)))
              ((symbol-function
                'aqi-request)
               (lambda (_city)
                 (setq
                  request-count
                  (1+
                   request-count))
                 :network)))
           (list
            (assoc-default
             'aqi
             (aqi-request-cached
              "Old"))
            request-count
            aqi-cache-refresh-period
            aqi-cached-data)))"##;
    let expect = expect![[
        r#"OK (9 0 1 (("Old" (aqi . 9) (city (name . "Old") (geo . [45.274 13.721]) (url . "https://aqicn.example/station")) (dominentpol . "pm10") (time (s . "2023-05-30 12:00:00") (tz . "+02:00")) (iaqi (pm25 (v . 12)) (pm10 (v . 21)) (no2 (v . 7)) (co (v . 3)) (t (v . 24)) (h (v . 61)) (p (v . 1014)) (wg (v . 5))) (attributions . [((name . "World Air Quality Index")) ((name . "Local Sensor Network"))]))))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}
