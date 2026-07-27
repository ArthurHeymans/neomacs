use expect_test::expect;

use super::assert_aqi_parity;

#[test]
fn aqi_accessor_macros_expand_to_exact_request_cache_and_let_alist_contracts() {
    let elisp_form = r##"(mapcar
         (lambda (form)
           (list
            form
            (macroexpand-1
             form)))
         '((aqi--make-city-raw-accessor
            'aqi-test-pm25
            .iaqi.pm25.v)
           (aqi--make-city-format-accessor
            'aqi-test-summary
            (format
             "%s/%s"
             .city.name
             .aqi))))"##;
    let expect = expect![[
        r#"OK (((aqi--make-city-raw-accessor #1='aqi-test-pm25 .iaqi.pm25.v) (fset #1# (lambda (city) (aqi-request city) (let-alist (assoc-default city aqi-cached-data) .iaqi.pm25.v)))) ((aqi--make-city-format-accessor #2='aqi-test-summary #3=(format "%s/%s" .city.name .aqi)) (fset #2# (lambda (city) (aqi-request city) (format "%s" (let-alist (assoc-default city aqi-cached-data) #3#))))))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_generated_numeric_accessor_performs_real_request_cache_lookup_on_every_call() {
    let elisp_form = r##"(let ((aqi-cached-data
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
                    (if
                        (equal city "Osaka")
                        42
                      17)
                    "pm25"))
                  aqi-cached-data)
                 :request-result)))
           (list
            (aqi-city-aqi
             "Osaka")
            (aqi-city-aqi
             "Taipei")
            (aqi-city-aqi
             "Osaka")
            calls
            (mapcar
             #'car
             aqi-cached-data))))"##;
    let expect =
        expect![[r#"OK (42 17 42 ("Osaka" "Taipei" "Osaka") ("Osaka" "Taipei" "Osaka" "None"))"#]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_generated_longitude_latitude_accessor_formats_integer_float_and_negative_coordinates() {
    let elisp_form = r##"(let ((aqi-cached-data
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
                 (let ((geo
                        (pcase city
                          ("Integer"
                           [1 2])
                          ("Float"
                           [45.274 13.721])
                          ("Negative"
                           [-33.8688 151.2093]))))
                   (push
                    (cons
                     city
                     `((city
                        (name . ,city)
                        (geo . ,geo))))
                    aqi-cached-data))
                 :request-result)))
           (list
            (aqi-city-lonlat
             "Integer")
            (aqi-city-lonlat
             "Float")
            (aqi-city-lonlat
             "Negative")
            calls)))"##;
    let expect = expect![[
        r#"OK ("1, 2" "45.274, 13.721" "-33.8688, 151.2093" ("Integer" "Float" "Negative"))"#
    ]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_macros_create_practical_nested_raw_and_formatted_accessors_at_runtime() {
    let elisp_form = r##"(let ((aqi-cached-data
                '(("None" . "None")))
               calls)
         (unwind-protect
             (progn
               (aqi--make-city-raw-accessor
                'aqi-test-pm25
                .iaqi.pm25.v)
               (aqi--make-city-format-accessor
                'aqi-test-summary
                (format
                 "%s: AQI %s (%s)"
                 .city.name
                 .aqi
                 .dominentpol))
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
                          "o3"))
                        aqi-cached-data)
                       :request-result)))
                 (list
                  (aqi-test-pm25
                   "Višnjan")
                  (aqi-test-summary
                   "Višnjan")
                  (help-function-arglist
                   'aqi-test-pm25
                   t)
                  (help-function-arglist
                   'aqi-test-summary
                   t)
                  calls)))
           (fmakunbound
            'aqi-test-pm25)
           (fmakunbound
            'aqi-test-summary)))"##;
    let expect = expect![[r#"OK (12 "Višnjan: AQI 73 (o3)" (city) (city) ("Višnjan" "Višnjan"))"#]];

    assert_aqi_parity(elisp_form, expect);
}

#[test]
fn aqi_accessors_expose_missing_fields_nil_formatting_and_coordinate_errors_exactly() {
    let elisp_form = r##"(let ((aqi-cached-data
                '(("None" . "None"))))
         (cl-letf
             (((symbol-function
                'aqi-request)
               (lambda (city)
                 (push
                  (cons
                   city
                   `((city
                      (name . ,city))))
                  aqi-cached-data)
                 :request-result)))
           (list
            (aqi-city-aqi
             "Sparse")
            (condition-case error-data
                (list
                 :ok
                 (aqi-city-lonlat
                  "Sparse"))
              (error
               (list
                :error
                (car error-data)
                (cdr error-data))))
            (assoc-default
             "Sparse"
             aqi-cached-data))))"##;
    let expect = expect![[r#"OK (nil (:ok "nil, nil") ((city (name . "Sparse"))))"#]];

    assert_aqi_parity(elisp_form, expect);
}
