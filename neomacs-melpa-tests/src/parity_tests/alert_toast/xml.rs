use expect_test::expect;

use super::assert_alert_toast_parity;

#[test]
fn regular_template_preserves_unicode_and_escapes_text_and_attribute_metacharacters() {
    let elisp_form = r##"
(alert-toast--fill-template
 "Tytuł & <status> \"quoted\" 'single'"
 "Zażółć gęślą jaźń & settle <now> \"safely\" 'once'"
 "C:\\Users\\O'Brien & Sons\\icon<1>.png")
"##;
    let expect = expect![[
        r#"OK "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Tytuł &amp; &lt;status&gt; &quot;quoted&quot; 'single'</text> <text id=\"2\">Zażółć gęślą jaźń &amp; settle &lt;now&gt; &quot;safely&quot; 'once'</text> <image id=\"1\" src=\"C:\\Users\\O'Brien &amp; Sons\\icon&lt;1&gt;.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>""#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn regular_template_audio_silent_long_and_loop_options_cover_every_branch() {
    let elisp_form = r##"
(mapcar
 (lambda (case)
   (cons
    (car case)
    (list
     (apply
      #'alert-toast--fill-template
      "Settlement"
      "Payment accepted"
      "C:\\Icons\\emacs.png"
      (cdr case)))))
 '((plain nil nil nil nil)
   (default default nil nil nil)
   (instant-message im nil nil nil)
   (silent nil t nil nil)
   (explicit-long mail nil t nil)
   (explicit-loop reminder nil nil t)
   (silent-loop sms t nil t)
   (unknown unknown nil nil nil)
   (unknown-loop unknown nil nil t)))
"##;
    let expect = expect![[
        r#"OK ((plain "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Settlement</text> <text id=\"2\">Payment accepted</text> <image id=\"1\" src=\"C:\\Icons\\emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>") (default "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Settlement</text> <text id=\"2\">Payment accepted</text> <image id=\"1\" src=\"C:\\Icons\\emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Default\" silent=\"false\" loop=\"false\"></audio></toast>") (instant-message "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Settlement</text> <text id=\"2\">Payment accepted</text> <image id=\"1\" src=\"C:\\Icons\\emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.IM\" silent=\"false\" loop=\"false\"></audio></toast>") (silent "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Settlement</text> <text id=\"2\">Payment accepted</text> <image id=\"1\" src=\"C:\\Icons\\emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Default\" silent=\"true\" loop=\"false\"></audio></toast>") (explicit-long "<toast duration=\"long\"> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Settlement</text> <text id=\"2\">Payment accepted</text> <image id=\"1\" src=\"C:\\Icons\\emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Mail\" silent=\"false\" loop=\"false\"></audio></toast>") (explicit-loop "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Settlement</text> <text id=\"2\">Payment accepted</text> <image id=\"1\" src=\"C:\\Icons\\emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Reminder\" silent=\"false\" loop=\"true\"></audio></toast>") (silent-loop "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Settlement</text> <text id=\"2\">Payment accepted</text> <image id=\"1\" src=\"C:\\Icons\\emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.SMS\" silent=\"true\" loop=\"true\"></audio></toast>") (unknown "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Settlement</text> <text id=\"2\">Payment accepted</text> <image id=\"1\" src=\"C:\\Icons\\emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Default\" silent=\"false\" loop=\"false\"></audio></toast>") (unknown-loop "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Settlement</text> <text id=\"2\">Payment accepted</text> <image id=\"1\" src=\"C:\\Icons\\emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Default\" silent=\"false\" loop=\"true\"></audio></toast>"))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn every_looping_sound_implies_long_duration_and_looping_audio_source() {
    let elisp_form = r##"
(mapcar
 (lambda (entry)
   (let ((xml
          (alert-toast--fill-template
           "Call" "Incoming" "C:\\icon.png"
           (car entry))))
     (list
      (car entry)
      (cdr entry)
      (and
       (string-match-p
        (regexp-quote (cdr entry)) xml)
       t)
      (and
       (string-match-p
        "duration=\"long\"" xml)
       t)
      (and
       (string-match-p
        "loop=\"true\"" xml)
       t)
      (and
       (string-match-p
        "silent=\"false\"" xml)
       t))))
 alert-toast--looping-sounds)
"##;
    let expect = expect![[
        r#"OK ((alarm10 "ms-winsoundevent:Notification.Looping.Alarm10" t t t t) (call10 "ms-winsoundevent:Notification.Looping.Call10" t t t t) (alarm9 "ms-winsoundevent:Notification.Looping.Alarm9" t t t t) (call9 "ms-winsoundevent:Notification.Looping.Call9" t t t t) (alarm8 "ms-winsoundevent:Notification.Looping.Alarm8" t t t t) (call8 "ms-winsoundevent:Notification.Looping.Call8" t t t t) (alarm7 "ms-winsoundevent:Notification.Looping.Alarm7" t t t t) (call7 "ms-winsoundevent:Notification.Looping.Call7" t t t t) (alarm6 "ms-winsoundevent:Notification.Looping.Alarm6" t t t t) (call6 "ms-winsoundevent:Notification.Looping.Call6" t t t t) (alarm5 "ms-winsoundevent:Notification.Looping.Alarm5" t t t t) (call5 "ms-winsoundevent:Notification.Looping.Call5" t t t t) (alarm4 "ms-winsoundevent:Notification.Looping.Alarm4" t t t t) (call4 "ms-winsoundevent:Notification.Looping.Call4" t t t t) (alarm3 "ms-winsoundevent:Notification.Looping.Alarm3" t t t t) (call3 "ms-winsoundevent:Notification.Looping.Call3" t t t t) (alarm2 "ms-winsoundevent:Notification.Looping.Alarm2" t t t t) (call2 "ms-winsoundevent:Notification.Looping.Call2" t t t t) (call "ms-winsoundevent:Notification.Looping.Call" t t t t) (alarm "ms-winsoundevent:Notification.Looping.Alarm" t t t t))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn shoulder_template_builds_fallback_and_people_bindings_with_exact_escaping() {
    let elisp_form = r##"
(list
 (alert-toast--fill-shoulder
  "Fallback <title> & status"
  "Fallback \"message\" & Zażółć"
  "C:\\Users\\O'Brien\\emacs.png"
  "mailto:o'brien@example.invalid"
  "https://example.invalid/tap.gif?x=1&y=2")
 (alert-toast--fill-shoulder
  "Local payload"
  "Apostrophe's body"
  "C:\\icon.png"
  "mailto:user@example.invalid"
  "C:\\Payloads\\shoulder image.png"))
"##;
    let expect = expect![[
        r#"OK ("<toast hint-people=\"mailto:o'brien@example.invalid\"> <visual> <binding template=\"ToastGeneric\"> <text>Fallback &lt;title&gt; &amp; status</text> <text>Fallback &quot;message&quot; &amp; Zażółć</text> <image src=\"C:\\Users\\O'Brien\\emacs.png\" placement=\"appLogoOverride\" hint-crop=\"circle\"></image></binding> <binding template=\"ToastGeneric\" experienceType=\"shoulderTap\"> <image src=\"https://example.invalid/tap.gif?x=1&amp;y=2\"></image></binding></visual></toast>" "<toast hint-people=\"mailto:user@example.invalid\"> <visual> <binding template=\"ToastGeneric\"> <text>Local payload</text> <text>Apostrophe's body</text> <image src=\"C:\\icon.png\" placement=\"appLogoOverride\" hint-crop=\"circle\"></image></binding> <binding template=\"ToastGeneric\" experienceType=\"shoulderTap\"> <image src=\"C:\\Payloads\\shoulder image.png\"></image></binding></visual></toast>")"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn empty_strings_construct_documents_while_nil_text_and_attributes_surface_exact_signals() {
    let elisp_form = r##"
(mapcar
 (lambda (case)
   (condition-case error-data
       (list
        (car case)
        'value
        (apply (cadr case) (cddr case)))
     (error
      (list
       (car case)
       'signal
       (car error-data)
       (cadr error-data)))))
 '((regular-nil
    alert-toast--fill-template nil nil nil)
   (regular-empty
    alert-toast--fill-template "" "" "" nil nil nil nil)
   (shoulder-nil
    alert-toast--fill-shoulder nil nil nil nil nil)
   (shoulder-empty
    alert-toast--fill-shoulder "" "" "" "" "")))
"##;
    let expect = expect![[
        r#"OK ((regular-nil signal wrong-type-argument arrayp) (regular-empty value "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\"></text> <text id=\"2\"></text> <image id=\"1\" src=\"\" placement=\"appLogoOverride\"></image></binding></visual></toast>") (shoulder-nil signal wrong-type-argument arrayp) (shoulder-empty value "<toast hint-people=\"\"> <visual> <binding template=\"ToastGeneric\"> <text></text> <text></text> <image src=\"\" placement=\"appLogoOverride\" hint-crop=\"circle\"></image></binding> <binding template=\"ToastGeneric\" experienceType=\"shoulderTap\"> <image src=\"\"></image></binding></visual></toast>"))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn long_multiline_message_is_preserved_for_windows_to_truncate_at_display_time() {
    let elisp_form = r##"
(let* ((message
        "line one\nline two\nline three\nline four\nline five\nline six")
       (xml
        (alert-toast--fill-template
         "Audit report"
         message
         "C:\\icon.png"
         nil nil t nil)))
  (list
   xml
   (and (string-match-p "line one" xml) t)
   (and (string-match-p "line six" xml) t)
   (and (string-match-p "duration=\"long\"" xml) t)
   (length xml)))
"##;
    let expect = expect![[
        r#"OK ("<toast duration=\"long\"> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Audit report</text> <text id=\"2\">line one\nline two\nline three\nline four\nline five\nline six</text> <image id=\"1\" src=\"C:\\icon.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>" t t t 280)"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}

#[test]
fn powershell_single_quote_escaping_runs_after_xml_entity_serialization() {
    let elisp_form = r##"
(mapcar
 (lambda (xml)
   (list
    xml
    (s-replace-all
     alert-toast--psquote-replacements
     xml)))
 (list
  (alert-toast--fill-template
   "O'Brien's <invoice>"
   "It's paid & archived"
   "C:\\O'Brien\\icon.png")
  (alert-toast--fill-shoulder
   "O'Brien"
   "It's ready"
   "C:\\O'Brien\\icon.png"
   "mailto:o'brien@example.invalid"
   "https://example.invalid/o'brien.gif")))
"##;
    let expect = expect![[
        r#"OK (("<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">O'Brien's &lt;invoice&gt;</text> <text id=\"2\">It's paid &amp; archived</text> <image id=\"1\" src=\"C:\\O'Brien\\icon.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>" "<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">O''Brien''s &lt;invoice&gt;</text> <text id=\"2\">It''s paid &amp; archived</text> <image id=\"1\" src=\"C:\\O''Brien\\icon.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>") ("<toast hint-people=\"mailto:o'brien@example.invalid\"> <visual> <binding template=\"ToastGeneric\"> <text>O'Brien</text> <text>It's ready</text> <image src=\"C:\\O'Brien\\icon.png\" placement=\"appLogoOverride\" hint-crop=\"circle\"></image></binding> <binding template=\"ToastGeneric\" experienceType=\"shoulderTap\"> <image src=\"https://example.invalid/o'brien.gif\"></image></binding></visual></toast>" "<toast hint-people=\"mailto:o''brien@example.invalid\"> <visual> <binding template=\"ToastGeneric\"> <text>O''Brien</text> <text>It''s ready</text> <image src=\"C:\\O''Brien\\icon.png\" placement=\"appLogoOverride\" hint-crop=\"circle\"></image></binding> <binding template=\"ToastGeneric\" experienceType=\"shoulderTap\"> <image src=\"https://example.invalid/o''brien.gif\"></image></binding></visual></toast>"))"#
    ]];
    assert_alert_toast_parity(elisp_form, expect);
}
