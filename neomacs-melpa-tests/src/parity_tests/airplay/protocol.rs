use expect_test::expect;

use super::assert_airplay_parity;

#[test]
fn airplay_query_and_url_builders_encode_real_protocol_inputs() {
    let elisp_form = r##"(let ((airplay->host "living room.local")
                (airplay->port 7000))
         (list
          (airplay/net:--make-query
           '(("position" . "12.5")
             ("title" . "A&B / demo")
             ("empty" . "")))
          (airplay/net:--make-url "play")
          (airplay/net:--make-url "scrub?position=12.5")))"##;
    let expect = expect![[
        r#"OK ("position=12.5&title=A%26B%20%2F%20demo&empty=" "http://living room.local:7000/play" "http://living room.local:7000/scrub?position=12.5")"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_url_builder_discovers_once_and_persists_device_coordinates() {
    let elisp_form = r##"(let ((airplay->host nil)
                (airplay->port 7000)
                calls)
         (cl-letf (((symbol-function 'airplay/device:browse)
                    (lambda ()
                      (push 'browse calls)
                      '("10.0.0.42" . 7100))))
           (list
            (airplay/net:--make-url "photo")
            (airplay/net:--make-url "playback-info")
            airplay->host
            airplay->port
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("http://10.0.0.42:7100/photo" "http://10.0.0.42:7100/playback-info" "10.0.0.42" 7100 (browse))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_http_wrappers_forward_methods_urls_headers_params_and_data_exactly() {
    let elisp_form = r##"(let ((airplay->host "apple-tv.local")
                (airplay->port 7000)
                calls)
         (cl-letf (((symbol-function 'request-deferred)
                    (lambda (url &rest args)
                      (push (list request-backend url args) calls)
                      (list 'request url args))))
           (list
            (airplay/protocol:get
             "scrub" :params '(("position" . "7")))
            (airplay/protocol:post
             "rate" :params '(("value" . "0")))
            (airplay/protocol:put
             "photo"
             :headers '(("X-Apple-Transition" . "Dissolve"))
             :data "JPEG")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ((request "http://apple-tv.local:7000/scrub" #1=(:type "GET" :params (("position" . "7")))) (request "http://apple-tv.local:7000/rate" #2=(:type "POST" :params (("value" . "0")))) (request "http://apple-tv.local:7000/photo" #3=(:type "PUT" :headers (("X-Apple-Transition" . "Dissolve")) :data "JPEG")) ((url-retrieve "http://apple-tv.local:7000/scrub" #1#) (url-retrieve "http://apple-tv.local:7000/rate" #2#) (url-retrieve "http://apple-tv.local:7000/photo" #3#)))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_text_parameter_roundtrip_handles_spacing_order_and_buffer_mutation() {
    let elisp_form = r##"(let ((wire
                (airplay/protocol:make-text-parameters
                 '(("Content-Location" . "http://host/movie.m4v")
                   ("Start-Position" . "0.0")
                   ("X-Token" . "abc")))))
         (with-temp-buffer
           (insert "\n  " wire " \n")
           (let ((before (buffer-string))
                 (parsed (airplay/protocol:parse-text-parameters)))
             (list wire before parsed (buffer-string)
                   (= (point) (point-max))))))"##;
    let expect = expect![[
        r#"OK ("Content-Location: http://host/movie.m4v\nStart-Position: 0.0\nX-Token: abc\n" "\n  Content-Location: http://host/movie.m4v\nStart-Position: 0.0\nX-Token: abc\n \n" (("Start-Position" . "0.0") ("X-Token" . "abc")) "  Content-Location: http://host/movie.m4v\nStart-Position: 0.0\nX-Token: abc\n " t)"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_scrub_parser_converts_fractional_signed_and_missing_fields() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (insert "duration: 83.124794\nposition: 14.467000\n")
           (airplay/protocol:parse-scrub))
         (with-temp-buffer
           (insert "\nposition: -2.5\nduration: 0\nignored line\n")
           (airplay/protocol:parse-scrub))
         (with-temp-buffer
           (insert "position: 7\n")
           (condition-case error
               (airplay/protocol:parse-scrub)
             (error
              (list (car error)
                    (mapcar
                     (lambda (value)
                       (if (stringp value) value (type-of value)))
                     (cdr error)))))))"##;
    let expect = expect![
        "OK ((:position 14.467 :duration 83.124794) (:position -2.5 :duration 0) (wrong-type-argument (symbol symbol)))"
    ];
    assert_airplay_parity(elisp_form, expect);
}

#[test]
fn airplay_plist_xml_parser_decodes_nested_array_dictionary_payloads() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
          "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" "
          "\"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">"
          "<plist version=\"1.0\"><dict>"
          "<key>duration</key><real>90.5</real>"
          "<key>readyToPlay</key><true/>"
          "<key>tracks</key><array><dict>"
          "<key>name</key><string>Main</string>"
          "<key>language</key><string>en</string>"
          "</dict></array>"
          "</dict></plist>")
         (airplay/protocol:parse-plist-xml))"##;
    let expect = expect![[
        r#"OK (("duration" . "90.5") ("readyToPlay") ("tracks" ("name" . "Main") ("language" . "en")))"#
    ]];
    assert_airplay_parity(elisp_form, expect);
}
