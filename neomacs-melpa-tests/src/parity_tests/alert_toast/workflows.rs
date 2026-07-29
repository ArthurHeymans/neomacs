use expect_test::expect;

use super::assert_alert_toast_parity;

/// One notification, end to end: `(alert "..." :style 'toast ...)' and the whole
/// of what reached the notifier's standard input.
///
/// The first thing sent is not the script.  Starting the process costs a
/// one-shot `powershell.exe' asking for the console encoding, then two
/// WindowsRuntime type loads, and only then the toast - the report keeps all
/// three in order, because they are what a user pays on the first notification
/// of a session.  A second alert is sent afterwards with the log truncated in
/// between, and it carries the script alone: the process is persistent, so the
/// start-up cost happens once.
///
/// The script itself is asserted whole rather than in pieces.  It is a format
/// template with four holes - the XML, the priority, the expiry and the fixed
/// Emacs tag and group - and reading it as one string is what shows the XML
/// arriving single-quoted inside `LoadXml' where PowerShell will parse it.
#[test]
fn sending_a_toast_writes_the_whole_powershell_script_to_the_notifier() {
    let elisp_form = r##"(progn
  (alert-toast-test-truncate)
  (alert "Build finished" :title "Emacs" :style 'toast
         :icon alert-toast-test-icon)
  (let ((first-notification (alert-toast-test-sent)))
    (alert-toast-test-truncate)
    (alert "Tests passed" :title "Emacs" :style 'toast
           :icon alert-toast-test-icon)
    (list :first-notification first-notification
          :second-notification (alert-toast-test-sent)
          :process-live (and (process-live-p alert-toast--psprocess) t)
          :process-name (process-name alert-toast--psprocess)
          :style (assq 'toast alert-styles))))"##;
    let expect = expect![[
        r#"OK (:first-notification "[one-shot] [console]::InputEncoding.BodyName\n[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null\n[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml, ContentType=WindowsRuntime] > $null\n$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Emacs</text> <text id=\"2\">Build finished</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(5.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n" :second-notification "$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Emacs</text> <text id=\"2\">Tests passed</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n    $Toast.Tag = \"Emacs\"\n    $Toast.Group = \"Emacs\"\n    $Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default\n    $Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(5.000000)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")\n    $Notifier.Show($Toast);\n" :process-live t :process-name "powershell-toast" :style (toast :title "Windows 10 toast notification" :notifier alert-toast-notify))"#
    ]];

    assert_alert_toast_parity(elisp_form, expect);
}

/// The two numbers in the script that come from the alert rather than from the
/// template: which priority the severity maps to, and how long the toast lives.
///
/// `alert-toast-priorities' maps six severities onto two Windows priorities, and
/// a severity that is not in the list falls back to whatever `normal' maps to -
/// so an unknown severity is quietly treated as normal rather than refused.  All
/// seven cases are sent and the `Priority' line of each script is reported.
///
/// The expiry has a three-way rule, and the middle case is the one worth having:
/// a plain alert expires after `alert-fade-time' seconds, a `:persistent' one
/// after a week, and a `:persistent' one that also carries `:never-persist'
/// falls back to the fade time - the flag that overrides the flag.
#[test]
fn the_severity_picks_the_priority_and_persistence_picks_the_expiry() {
    let elisp_form = r##"(list
  :priorities
  (mapcar (lambda (severity)
            (cons severity
                  (alert-toast-test-lines
                   (alert-toast-test-notify "Body" :title "T" :severity severity
                                            :icon alert-toast-test-icon)
                   "^\\$Toast\\.Priority")))
          '(urgent high moderate normal low trivial nosuchseverity))
  :fade-time alert-fade-time
  :expiry
  (list :plain (alert-toast-test-lines
                (alert-toast-test-notify "Body" :title "T"
                                         :icon alert-toast-test-icon)
                "AddSeconds")
        :persistent (alert-toast-test-lines
                     (alert-toast-test-notify "Body" :title "T" :persistent t
                                              :icon alert-toast-test-icon)
                     "AddSeconds")
        :persistent-but-never-persist
        (alert-toast-test-lines
         (alert-toast-test-notify "Body" :title "T" :persistent t
                                  :never-persist t :icon alert-toast-test-icon)
         "AddSeconds")))"##;
    let expect = expect![[
        r#"OK (:priorities ((urgent "$Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::High") (high "$Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::High") (moderate "$Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default") (normal "$Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default") (low "$Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default") (trivial "$Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default") (nosuchseverity "$Toast.Priority = [Windows.UI.Notifications.ToastNotificationPriority]::Default")) :fade-time 5 :expiry (:plain ("$Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(5.000000)") :persistent ("$Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(604800.000000)") :persistent-but-never-persist ("$Toast.ExpirationTime = [DateTimeOffset]::Now.AddSeconds(5.000000)")))"#
    ]];

    assert_alert_toast_parity(elisp_form, expect);
}

/// The sound ladder, which is where the package makes most of its decisions.
/// Every case is sent through `alert''s `:data' plist, the way the README
/// documents, and the `audio' element and the toast's own attributes are read
/// back out of the XML.
///
/// A toast with no sound options has no `audio' element at all.  A named sound
/// from `alert-toast--sounds' produces one.  A *looping* sound does two further
/// things by itself: it sets `loop="true"' without being asked and it makes the
/// toast long, which is the twenty-second variety.  `:long' alone does the
/// latter without the former.
///
/// The fallback is the case a user is most likely to hit: a sound name that is
/// in neither table does not disable the audio and does not error - it silently
/// becomes the default sound, so a typo is audible as an ordinary notification
/// rather than as nothing.  `:silent' is asserted alongside to show it mutes the
/// element rather than removing it.
#[test]
fn the_audio_options_choose_a_sound_and_a_looping_one_makes_the_toast_long() {
    let elisp_form = r##"(cl-flet ((audio-of
             (data)
             (let ((script (apply #'alert-toast-test-notify
                                  "Body" :title "T"
                                  :icon alert-toast-test-icon
                                  (list :data data))))
               (list :toast (car (alert-toast-test-lines script "^\\$Xml\\.LoadXml"))))))
  (list :no-audio (audio-of nil)
        :named-sound (audio-of '(:audio mail))
        :looping-sound (audio-of '(:audio alarm3))
        :unknown-sound (audio-of '(:audio no-such-sound))
        :silent (audio-of '(:silent t))
        :long-only (audio-of '(:long t))
        :loop-without-a-looping-sound (audio-of '(:audio im :loop t))
        :sound-tables (list :plain (mapcar #'car alert-toast--sounds)
                            :looping (sort (mapcar #'car
                                                   alert-toast--looping-sounds)
                                           #'string<))))"##;
    let expect = expect![[
        r#"OK (:no-audio (:toast "$Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">T</text> <text id=\"2\">Body</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')") :named-sound (:toast "$Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">T</text> <text id=\"2\">Body</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Mail\" silent=\"false\" loop=\"false\"></audio></toast>')") :looping-sound (:toast "$Xml.LoadXml('<toast duration=\"long\"> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">T</text> <text id=\"2\">Body</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Looping.Alarm3\" silent=\"false\" loop=\"true\"></audio></toast>')") :unknown-sound (:toast "$Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">T</text> <text id=\"2\">Body</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Default\" silent=\"false\" loop=\"false\"></audio></toast>')") :silent (:toast "$Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">T</text> <text id=\"2\">Body</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.Default\" silent=\"true\" loop=\"false\"></audio></toast>')") :long-only (:toast "$Xml.LoadXml('<toast duration=\"long\"> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">T</text> <text id=\"2\">Body</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')") :loop-without-a-looping-sound (:toast "$Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">T</text> <text id=\"2\">Body</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual> <audio src=\"ms-winsoundevent:Notification.IM\" silent=\"false\" loop=\"true\"></audio></toast>')") :sound-tables (:plain (default im mail reminder sms) :looping (alarm alarm10 alarm2 alarm3 alarm4 alarm5 alarm6 alarm7 alarm8 alarm9 call call10 call2 call3 call4 call5 call6 call7 call8 call9)))"#
    ]];

    assert_alert_toast_parity(elisp_form, expect);
}

/// The shoulder tap - a notification pinned to a contact - is the package's
/// other template, and reaching it needs *both* halves of the pair.  With
/// `:shoulder-person' and `:shoulder-payload' both present the script changes
/// completely: a different XML template, two bindings rather than one, and a
/// notifier addressed to the People app rather than to Emacs.  With only one of
/// them the package falls back to an ordinary toast without complaint, and the
/// half that was given is simply dropped - so a user who mistypes one keyword
/// gets a plain notification and no indication that the tap did not happen.
///
/// The last two cases are the quoting, and they come out opposite ways.
///
/// Single quotes are handled.  The whole XML is embedded in a single-quoted
/// PowerShell string, so a quote in a title would end that string early;
/// `alert-toast--psquote-replacements' doubles them, and the report shows every
/// quote doubled inside `LoadXml' for a title and a body that would otherwise
/// close the string and start a new command.
///
/// Backslashes are not.  **A lone backslash anywhere in the message body makes
/// the notification fail outright** with `(error "Invalid use of ‘\\’ in
/// replacement text")', so nothing is sent at all - a Windows path in a build
/// message is enough.  Both editors do this identically, so it is the package's
/// defect and is pinned as behaviour rather than filed as a divergence.
///
/// The three neighbours are pinned beside it, because the failure is worthless
/// without them: the same backslash in the *title* is fine, a *doubled*
/// backslash in the body is fine, and apostrophes are fine everywhere.  That
/// places the fault in whatever consumes the body as replacement text rather
/// than in the quoting generally, and it means a reader can see which input is
/// responsible instead of inferring it.
#[test]
fn a_shoulder_tap_needs_both_halves_and_single_quotes_are_doubled() {
    let elisp_form = r##"(list
  :both-halves
  (alert-toast-test-notify "Body" :title "T" :icon alert-toast-test-icon
                           :data '(:shoulder-person "mailto:ada@example.com"
                                   :shoulder-payload "C:\\taps\\wave.gif"))
  :person-only
  (alert-toast-test-lines
   (alert-toast-test-notify "Body" :title "T" :icon alert-toast-test-icon
                            :data '(:shoulder-person "mailto:ada@example.com"))
   "^\\$Xml\\.LoadXml" "CreateToastNotifier")
  :payload-only
  (alert-toast-test-lines
   (alert-toast-test-notify "Body" :title "T" :icon alert-toast-test-icon
                            :data '(:shoulder-payload "C:\\taps\\wave.gif"))
   "^\\$Xml\\.LoadXml" "CreateToastNotifier")
  :quoting
  (list :replacements alert-toast--psquote-replacements
        :script (alert-toast-test-lines
                 (alert-toast-test-notify
                  "it's done'); Remove-Item -Recurse; ('"
                  :title "Ada's build" :icon alert-toast-test-icon)
                 "^\\$Xml\\.LoadXml"))
  :backslash
  (list
   :in-body
   (condition-case error
       (progn (alert-toast-test-notify "Cleaned C:\\ tree"
                                       :title "T" :icon alert-toast-test-icon)
              :sent)
     (error (list :signalled (car error) (cadr error))))
   :in-body-doubled
   (condition-case error
       (alert-toast-test-lines
        (alert-toast-test-notify "Cleaned C:\\\\ tree"
                                 :title "T" :icon alert-toast-test-icon)
        "^\\$Xml\\.LoadXml")
     (error (list :signalled (car error) (cadr error))))
   :in-title
   (condition-case error
       (alert-toast-test-lines
        (alert-toast-test-notify "Body" :title "Cleaned C:\\ tree"
                                 :icon alert-toast-test-icon)
        "^\\$Xml\\.LoadXml")
     (error (list :signalled (car error) (cadr error))))))"##;
    let expect = expect![[
        r#"OK (:both-halves "[one-shot] [console]::InputEncoding.BodyName\n[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null\n[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml, ContentType=WindowsRuntime] > $null\n$Xml = New-Object Windows.Data.Xml.Dom.XmlDocument\n    $Xml.LoadXml('<toast hint-people=\"mailto:ada@example.com\"> <visual> <binding template=\"ToastGeneric\"> <text>T</text> <text>Body</text> <image src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\" hint-crop=\"circle\"></image></binding> <binding template=\"ToastGeneric\" experienceType=\"shoulderTap\"> <image src=\"C:\\taps\\wave.gif\"></image></binding></visual></toast>')\n\n    $Toast = [Windows.UI.Notifications.ToastNotification]::new($Xml)\n\n    $Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Microsoft.People_8wekyb3d8bbwe!x4c7a3b7dy2188y46d4ya362y19ac5a5805e5x')\n    $Notifier.Show($Toast);\n" :person-only ("$Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">T</text> <text id=\"2\">Body</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')" "$Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")") :payload-only ("$Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">T</text> <text id=\"2\">Body</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')" "$Notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier(\"Emacs\")") :quoting (:replacements (("'" . "''")) :script ("$Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Ada''s build</text> <text id=\"2\">it''s done''); Remove-Item -Recurse; (''</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')")) :backslash (:in-body (:signalled error "Invalid use of ‘\\’ in replacement text") :in-body-doubled ("$Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">T</text> <text id=\"2\">Cleaned C:\\\\ tree</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')") :in-title ("$Xml.LoadXml('<toast> <visual> <binding template=\"ToastImageAndText02\"> <text id=\"1\">Cleaned C:\\ tree</text> <text id=\"2\">Body</text> <image id=\"1\" src=\"/home/user/pictures/emacs.png\" placement=\"appLogoOverride\"></image></binding></visual></toast>')")))"#
    ]];

    assert_alert_toast_parity(elisp_form, expect);
}

/// The platform ladder, which is the least-run code in the package: almost
/// nobody is on all three of WSL, Cygwin and plain Windows, so each user
/// exercises one branch and the other two go untested by use.
///
/// `alert-toast--check-wsl' decides the platform from the kernel release, and
/// the rule is a substring match on either "wsl" or "microsoft", case
/// insensitively - so the workflow feeds it four release strings through the
/// stand-in `uname' and reports what each is taken for, including an ordinary
/// Linux one and a capitalised "Microsoft" that a naive match would miss.
///
/// `alert-toast--icon-path' then converts a path per platform, and each branch
/// is reached by binding the flag the package itself computes: under WSL through
/// `wslpath -m', under Cygwin through `cygpath.exe -w', and on plain Linux by
/// returning the path untouched.  The stand-ins echo which converter ran, so the
/// report says not just that the path changed but which tool changed it.
///
/// The default icon is checked here too, and not as a literal: it is built from
/// `data-directory', which belongs to the editor rather than to the package, so
/// pinning the string would compare two installations rather than two
/// behaviours.
#[test]
fn the_platform_ladder_converts_icon_paths_for_wsl_and_for_cygwin() {
    let elisp_form = r##"(list
  :detection
  (mapcar (lambda (release)
            (alert-toast-test-write-program
             "uname" (format "#!/bin/sh\nprintf '%s\\n'\n" release))
            (cons release (and (alert-toast--check-wsl) t)))
          '("6.12.85-neomacs-parity"
            "5.15.90.1-microsoft-standard-WSL2"
            "4.4.0-19041-Microsoft"
            "6.1.0-wsl-custom"))
  :conversion
  (list :plain (let ((alert-toast--wsl nil))
                 (alert-toast--icon-path "/home/user/pictures/emacs.png"))
        :wsl (let ((alert-toast--wsl t))
               (alert-toast--icon-path "/home/user/pictures/emacs.png"))
        :cygwin (let ((alert-toast--wsl nil)
                      (system-type 'cygwin))
                  (alert-toast--icon-path "/home/user/pictures/emacs.png")))
  :default-icon-is-built-from-data-directory
  (equal alert-toast-default-icon
         (concat data-directory "images/icons/hicolor/128x128/apps/emacs.png"))
  :this-session-is-not-wsl alert-toast--wsl)"##;
    let expect = expect![[
        r#"OK (:detection (("6.12.85-neomacs-parity") ("5.15.90.1-microsoft-standard-WSL2" . t) ("4.4.0-19041-Microsoft" . t) ("6.1.0-wsl-custom" . t)) :conversion (:plain "/home/user/pictures/emacs.png" :wsl "C:/from-wslpath/home/user/pictures/emacs.png" :cygwin "C:\\from-cygpath/home/user/pictures/emacs.png") :default-icon-is-built-from-data-directory t :this-session-is-not-wsl nil)"#
    ]];

    assert_alert_toast_parity(elisp_form, expect);
}
