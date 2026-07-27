use expect_test::expect;

use super::assert_adafruit_wisdom_parity;

#[test]
fn adafruit_wisdom_select_uses_item_count_as_random_limit_and_returns_each_title_exactly() {
    let elisp_form = r##"(let ((feed
                '((rss nil
                       (channel nil
                                (item nil
                                      (title nil
                                             "first & literal"))
                                (item nil
                                      (title nil
                                             "middle"))
                                (item nil
                                      (title nil
                                             "last"))))))
               random-limits)
         (cl-letf
             (((symbol-function
                'adafruit-wisdom-cached-get)
               (lambda ()
                 feed)))
           (mapcar
            (lambda (index)
              (cl-letf
                  (((symbol-function
                     'random)
                    (lambda (limit)
                      (push
                       limit
                       random-limits)
                      index)))
                (list
                 index
                 (adafruit-wisdom-select)
                 (car
                  random-limits))))
            '(0 1 2))))"##;
    let expect = expect![[r#"OK ((0 "first & literal" 3) (1 "middle" 3) (2 "last" 3))"#]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_select_empty_feed_signals_the_exact_random_limit_error() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'adafruit-wisdom-cached-get)
           (lambda ()
             '((rss nil
                    (channel nil))))))
       (condition-case error-data
           (list
            'ok
            (adafruit-wisdom-select))
         (error
          (list
           'error
           (car
            error-data)
           (error-message-string
            error-data)
           (cdr
            error-data)))))"##;
    let expect = expect![[r#"OK (error args-out-of-range "Args out of range: 0" (0))"#]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}

#[test]
fn adafruit_wisdom_select_item_without_title_returns_empty_string_not_nil() {
    let elisp_form = r##"(cl-letf
         (((symbol-function
            'adafruit-wisdom-cached-get)
           (lambda ()
             '((rss nil
                    (channel nil
                             (item nil
                                   (description nil
                                                "missing")))))))
          ((symbol-function
            'random)
           (lambda (limit)
             (list
              'limit
              limit)
             0)))
       (list
        (adafruit-wisdom-select)
        (null
         (adafruit-wisdom-select))))"##;
    let expect = expect![[r#"OK ("" nil)"#]];
    assert_adafruit_wisdom_parity(elisp_form, expect);
}
