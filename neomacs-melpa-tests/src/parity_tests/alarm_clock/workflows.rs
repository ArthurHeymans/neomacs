use expect_test::expect;

use super::assert_alarm_clock_parity;

/// The whole point of the package in one call: `M-x alarm-clock-set', a
/// relative time and a message.  A timer is scheduled two minutes out, the
/// message is trimmed, the alarm is rendered in `*alarm clock*' with its own
/// record attached to the first character of the line, and -- because
/// `alarm-clock-auto-save' is on -- the cache file is written.  The delay is
/// asserted as a delta in tenths of a second from a timestamp taken
/// immediately before the call; the rendered clock reading is compared against
/// the alarm's own time rather than against a literal.
#[test]
fn setting_an_alarm_schedules_its_timer_and_lists_it() {
    let elisp_form = r##"(progn
  (alarm-clock-test-setup)
  (let ((scheduled (alarm-clock-test-set "2 minutes" "  Stand up  ")))
    (list :scheduled scheduled
          :lines (alarm-clock-test-lines)
          :header (alarm-clock-test-header)
          :cache (alarm-clock-test-cache)
          :cache-records-the-alarms (alarm-clock-test-cache-matches-alarms)
          :files (alarm-clock-test-files))))"##;
    let expect = expect![[
        r#"OK (:scheduled ("Stand up" 120 t 2) :lines ((:time-matches-the-alarm t :remaining-hour-minute "00:01" :message "Stand up" :property-message "Stand up" :property-only-on-first-character t)) :header ("Time                 Remaining      Message" alarm-clock-mode t) :cache ";; Auto-generated file; don't edit\n((:time \"<ISO>\" :message \"Stand up\"))\n" :cache-records-the-alarms (("Stand up" . t)) :files ("." ".." "alarm-clock.cache"))"#
    ]];

    assert_alarm_clock_parity(elisp_form, expect);
}

/// The time specifications the commentary advertises: plain English, and the
/// abbreviations `45s', `3m', `1h30m' that the package rewrites into it.  Each
/// one is set through the public command and the resulting delay is read back
/// from the alarm record, so what is pinned is the schedule a user gets rather
/// than the rewriting rule in isolation.  A specification nothing can parse is
/// an error and leaves no alarm behind.
#[test]
fn the_documented_time_specifications_all_schedule_the_delay_they_name() {
    let elisp_form = r##"(progn
  (alarm-clock-test-setup)
  (let ((scheduled (mapcar (lambda (spec)
                             (alarm-clock-test-set (car spec) (cdr spec)))
                           '(("45s" . "forty five seconds")
                             ("3m" . "three minutes")
                             ("1h30m" . "ninety minutes")
                             ("30 seconds" . "half a minute")))))
    (list :scheduled scheduled
          :rejected (condition-case error
                        (alarm-clock-set "not a time at all" "never")
                      (error error))
          :alarms-after-rejection (length alarm-clock--alist))))"##;
    let expect = expect![[
        r#"OK (:scheduled (("forty five seconds" 45 t 2) ("three minutes" 180 t 1) ("ninety minutes" 5400 t 1) ("half a minute" 30 t 1)) :rejected (error "Invalid time specification") :alarms-after-rejection 4)"#
    ]];

    assert_alarm_clock_parity(elisp_form, expect);
}

/// The listing is sorted by time regardless of the order the alarms were set
/// in, each line carries its own alarm record as a text property on its first
/// character only, and the countdown column counts down from the alarm's own
/// time.  `alarm-clock-list-view' re-renders and shows the buffer; the mode is
/// read-only, which is what makes the single-key bindings safe.
#[test]
fn the_listing_is_sorted_and_carries_each_alarm_as_a_text_property() {
    let elisp_form = r##"(progn
  (alarm-clock-test-setup)
  (let ((scheduled (mapcar (lambda (spec)
                             (alarm-clock-test-set (car spec) (cdr spec)))
                           '(("2 hours" . "last")
                             ("10 minutes" . "first")
                             ("1 hour" . "middle")))))
    (alarm-clock-list-view)
    (list :lines (alarm-clock-test-lines)
          :header (alarm-clock-test-header)
          :scheduled scheduled
          :state (alarm-clock-test-state)
          :displayed (buffer-name (window-buffer (selected-window))))))"##;
    let expect = expect![[
        r#"OK (:lines ((:time-matches-the-alarm t :remaining-hour-minute "00:09" :message "first" :property-message "first" :property-only-on-first-character t) (:time-matches-the-alarm t :remaining-hour-minute "00:59" :message "middle" :property-message "middle" :property-only-on-first-character t) (:time-matches-the-alarm t :remaining-hour-minute "01:59" :message "last" :property-message "last" :property-only-on-first-character t)) :header ("Time                 Remaining      Message" alarm-clock-mode t) :scheduled (("last" 7200 t 2) ("first" 600 t 1) ("middle" 3600 t 1)) :state (("last" t) ("middle" t) ("first" t)) :displayed "*alarm clock*")"#
    ]];

    assert_alarm_clock_parity(elisp_form, expect);
}

/// Persistence, both directions.  Saving writes a self-describing file whose
/// layout is pinned with the timestamps masked, and whose timestamps are
/// checked separately by parsing them back to the delays they were set with.
/// Restoring in a state that has no alarms brings them back as live timers and
/// re-renders the listing.  The directory listing is pinned too: the save
/// leaves the cache file and nothing else -- no lock file, and no backup
/// despite the package's comment saying `save-buffer' would make one.
#[test]
fn saving_writes_the_alarms_and_restoring_brings_them_back_as_timers() {
    let elisp_form = r##"(progn
  (alarm-clock-test-setup)
  (let ((start (float-time))
        (scheduled (list (alarm-clock-test-set "1 hour" "water the plants")
                         (alarm-clock-test-set "2 hours" "call the bank"))))
    (alarm-clock-save)
    (let ((saved (list :cache (alarm-clock-test-cache)
                       :cache-records-the-alarms (alarm-clock-test-cache-matches-alarms)
                       :cache-minutes (alarm-clock-test-cache-minutes start)
                       :files (alarm-clock-test-files))))
      (alarm-clock-save)
      (let ((twice (alarm-clock-test-files))
            (baseline (progn (alarm-clock--kill-all)
                             (copy-sequence timer-list)))
            (restore-start (float-time)))
        (alarm-clock-restore)
        (list :scheduled scheduled
              :saved saved
              :files-after-second-save twice
              :emptied (null alarm-clock--alist)
              :restored (alarm-clock-test-alarms restore-start 'minutes)
              :new-timers (length (alarm-clock-test-new-timers baseline))
              :lines (alarm-clock-test-lines))))))"##;
    let expect = expect![[
        r#"OK (:scheduled (("water the plants" 3600 t 2) ("call the bank" 7200 t 1)) :saved (:cache ";; Auto-generated file; don't edit\n((:time \"<ISO>\" :message \"call the bank\")\n (:time \"<ISO>\" :message \"water the plants\"))\n" :cache-records-the-alarms (("call the bank" . t) ("water the plants" . t)) :cache-minutes (("call the bank" . 120) ("water the plants" . 60)) :files ("." ".." "alarm-clock.cache" "alarm-clock.cache~")) :files-after-second-save ("." ".." "alarm-clock.cache" "alarm-clock.cache~") :emptied nil :restored (("call the bank" 120 t) ("water the plants" 60 t)) :new-timers 2 :lines ((:time-matches-the-alarm t :remaining-hour-minute "00:59" :message "water the plants" :property-message "water the plants" :property-only-on-first-character t) (:time-matches-the-alarm t :remaining-hour-minute "01:59" :message "call the bank" :property-message "call the bank" :property-only-on-first-character t)))"#
    ]];

    assert_alarm_clock_parity(elisp_form, expect);
}

/// Cancelling from the listing, with the key the mode documents: `C-k' on the
/// line removes that line, cancels that alarm's timer -- it leaves
/// `timer-list', so it can never fire -- drops it from the alarm list, and
/// rewrites the cache file without it.  The other alarm is untouched, and
/// `C-k' on a line with no alarm is a plain user error.
#[test]
fn killing_an_alarm_cancels_its_timer_and_rewrites_the_saved_file() {
    let elisp_form = r##"(progn
  (alarm-clock-test-setup)
  (let ((scheduled (list (alarm-clock-test-set "1 hour" "doomed")
                         (alarm-clock-test-set "2 hours" "survivor"))))
    (alarm-clock-list-view)
    (with-current-buffer "*alarm clock*"
      (set-window-buffer (selected-window) (current-buffer))
      (goto-char (point-min))
      (let* ((doomed (plist-get (get-text-property (point-min) 'alarm-clock) :timer))
             (before (list :state (alarm-clock-test-state)
                           :scheduled (and (memq doomed timer-list) t)
                           :cache-records-the-alarms (alarm-clock-test-cache-matches-alarms))))
        (execute-kbd-macro (kbd "C-k"))
        (list :scheduled scheduled
              :before before
              :after-state (alarm-clock-test-state)
              :killed-timer-still-scheduled (and (memq doomed timer-list) t)
              :lines (alarm-clock-test-lines)
              :cache (alarm-clock-test-cache)
              :cache-records-the-alarms (alarm-clock-test-cache-matches-alarms)
              :on-an-empty-line
              (progn (goto-char (point-max))
                     (condition-case error (alarm-clock-kill) (error error))))))))"##;
    let expect = expect![[
        r#"OK (:scheduled (("doomed" 3600 t 2) ("survivor" 7200 t 1)) :before (:state (("survivor" t) ("doomed" t)) :scheduled t :cache-records-the-alarms (("survivor" . t) ("doomed" . t))) :after-state (("survivor" t)) :killed-timer-still-scheduled nil :lines ((:time-matches-the-alarm t :remaining-hour-minute "01:59" :message "survivor" :property-message "survivor" :property-only-on-first-character t)) :cache ";; Auto-generated file; don't edit\n((:time \"<ISO>\" :message \"survivor\"))\n" :cache-records-the-alarms (("survivor" . t)) :on-an-empty-line (user-error "No alarm clock on the current line"))"#
    ]];

    assert_alarm_clock_parity(elisp_form, expect);
}

/// What a user actually gets when an alarm fires here.  The package tries a
/// desktop notifier, a sound player and the optional `alert' package before
/// falling back to the echo area; none of those exist on this host, which the
/// workflow records rather than assumes, so the message is the whole of the
/// notification.  Firing starts no subprocess, and the alarm's own timer is
/// spent afterwards.
#[test]
fn a_firing_alarm_reports_itself_in_the_echo_area_when_no_notifier_exists() {
    let elisp_form = r##"(progn
  (alarm-clock-test-setup)
  (let* ((baseline (copy-sequence timer-list))
         (scheduled (alarm-clock-test-set "10 seconds" "Tea is ready")))
    (let* ((fired (plist-get (car alarm-clock--alist) :timer))
           (mark (alarm-clock-test-message-mark))
           (processes-before (length (process-list))))
      (timer-event-handler fired)
      (list :notifiers (alarm-clock-test-notifiers)
            :alert-available (fboundp 'alert)
            :sound-file-exists (file-exists-p (expand-file-name alarm-clock-sound-file))
            :messages (alarm-clock-test-messages-since mark)
            :processes-started (- (length (process-list)) processes-before)
            :timer-still-scheduled (and (memq fired timer-list) t)
            :new-timers-from-firing (length (alarm-clock-test-new-timers baseline))
            :stopped alarm-clock--stopped
            :scheduled scheduled
            :state (alarm-clock-test-state)))))"##;
    let expect = expect![[
        r#"OK (:notifiers (("notify-send") ("terminal-notifier") ("mpg123") ("afplay")) :alert-available nil :sound-file-exists t :messages ("[Alarm Clock] - Tea is ready") :processes-started 0 :timer-still-scheduled nil :new-timers-from-firing 1 :stopped nil :scheduled ("Tea is ready" 10 t 2) :state (("Tea is ready" nil)))"#
    ]];

    assert_alarm_clock_parity(elisp_form, expect);
}

/// The saved file is user-editable, so restoring has to cope with what it
/// finds.  An alarm whose time has passed is dropped from the list and from
/// the listing -- but a timer is still scheduled for it first, so it fires at
/// once, which is worth knowing before hand-editing the file.  A missing file
/// and an empty file both restore nothing without complaint, and a timestamp
/// that is not a timestamp is an error naming the type it wanted.
#[test]
fn expired_and_malformed_saved_alarms_are_handled_when_restoring() {
    let elisp_form = r##"(progn
  (alarm-clock-test-setup)
  (alarm-clock-test-write-cache '((-3600 . "already rang") (3600 . "still pending")))
  (let* ((baseline (copy-sequence timer-list))
         (start (float-time)))
    (alarm-clock-restore)
    (let ((expired (list :alarms (alarm-clock-test-alarms start 'minutes)
                         :new-timers (length (alarm-clock-test-new-timers baseline))
                         :lines (alarm-clock-test-lines)
                         :file-still-has-both (alarm-clock-test-cache-minutes start))))
      (delete-file alarm-clock-cache-file)
      (let ((missing (progn (alarm-clock-restore) (length alarm-clock--alist))))
        (write-region "" nil alarm-clock-cache-file nil 'silent)
        (let ((empty (progn (alarm-clock-restore) (length alarm-clock--alist))))
          (write-region "((:time \"not-a-timestamp\" :message \"broken\"))\n"
                        nil alarm-clock-cache-file nil 'silent)
          (list :expired expired
                :missing-file missing
                :empty-file empty
                :malformed (condition-case error (alarm-clock-restore) (error error))
                :alarms-after-malformed (length alarm-clock--alist)))))))"##;
    let expect = expect![[
        r#"OK (:expired (:alarms (("still pending" 60 t)) :new-timers 2 :lines ((:time-matches-the-alarm t :remaining-hour-minute "00:59" :message "still pending" :property-message "still pending" :property-only-on-first-character t)) :file-still-has-both (("already rang" . -60) ("still pending" . 60))) :missing-file 0 :empty-file 0 :malformed (wrong-type-argument fixnump nil) :alarms-after-malformed 0)"#
    ]];

    assert_alarm_clock_parity(elisp_form, expect);
}
