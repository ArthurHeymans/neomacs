use super::{assert_achievements_parity, assert_advanced_achievements_parity};
use expect_test::expect;

#[test]
fn achievements_basic_catalog_exact_records_and_predicate_forms_match() {
    let elisp_form = r##"(cl-labels
         ((normalize-function
           (function)
           (cond
            ((byte-code-function-p
              function)
             'compiled)
            ((eq
              (type-of function)
              'interpreted-function)
             'interpreted)
            ((eq
              (car-safe function)
              'closure)
             (cons
              'lambda
              (cddr function)))
            (t function))))
         (mapcar
          (lambda (achievement)
            (list
            (emacs-achievement-name
             achievement)
            (emacs-achievement-description
             achievement)
            (emacs-achievement-points
             achievement)
            (emacs-achievement-transient
             achievement)
            (emacs-achievement-min-score
             achievement)
            (emacs-achievement-unlocks
             achievement)
            (let ((predicate
                   (emacs-achievement-predicate
                    achievement)))
              (cond
               ((eq predicate t)
                t)
               ((null predicate)
                nil)
               ((functionp predicate)
                (normalize-function
                 predicate))
               (t predicate)))
            (let ((post-command
                   (emacs-achievement-post-command
                    achievement)))
              (cond
               ((null post-command)
                nil)
               ((functionp
                 post-command)
                (normalize-function
                 post-command))
               (t post-command)))))
          achievements-list))"##;
    let expect = expect![[
        r#"OK (("Achiever" "You used the achievements package." 5 nil 0 nil (lambda nil (and t)) nil) ("Not All There" "You have a fractional achievement score." 0.5 nil 0 nil (lambda nil (and (/= achievements-score (round achievements-score)))) nil) ("Unlocker" "You have earned over 50 points in Emacs achievements.  Not bad." 5 nil 0 advanced-achievements (lambda nil (and (>= achievements-score 50))) nil) ("Over Achiever" "You have earned 500 points in Emacs achievements.  Don't you have some real work to do?" 5 nil 0 nil (lambda nil (and (>= achievements-score 500))) nil) ("Cheater" "You have earned all Emacs achievements.  Actually that's impossible." 5 nil 0 nil (lambda nil (and (every #'achievements-earned-p achievements-list))) nil) ("Free Software Zealot" "You've read the sales pitch." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '(about-emacs describe-copying describe-distribution describe-gnu-project describe-no-warranty)))) nil) ("First things first" "You learned new things by using `help-for-help'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'help-for-help))) nil) ("Show me the way" "You learned new things by using `help-with-tutorial'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'help-with-tutorial))) nil) ("RTFM" "You learned new things by using `info-emacs-manual'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'info-emacs-manual))) nil) ("Log Auditor" "You learned new things by using `view-echo-area-messages'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'view-echo-area-messages))) nil) ("FAQ" "You learned new things by using `view-emacs-FAQ'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'view-emacs-FAQ))) nil) ("What's new?" "You learned new things by using `view-emacs-news'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'view-emacs-news))) nil) ("Am I the only one?" "You learned new things by using `view-emacs-problems'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'view-emacs-problems))) nil) ("Entomologist" "You learned new things by using `view-emacs-debugging'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'view-emacs-debugging))) nil) ("Joining the cause" "You learned new things by using `view-emacs-todo'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'view-emacs-todo))) nil) ("Where else can I look?" "You learned new things by using `view-external-packages'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'view-external-packages))) nil) ("World Traveler" "You learned new things by using `(view-hello-file describe-language-environment describe-input-method describe-coding-system)'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '(view-hello-file describe-language-environment describe-input-method describe-coding-system)))) nil) ("Package Hunter" "You learned new things by using `(finder-by-keyword describe-package)'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '(finder-by-keyword describe-package)))) nil) ("I know I read it somewhere" "You answered a question by using `apropos-documentation'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'apropos-documentation))) nil) ("Apropos of Nothing" "You answered a question by using `apropos'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'apropos))) nil) ("Answer in search of a question" "You answered a question by using `apropos-value'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'apropos-value))) nil) ("What to type?" "You answered a question by using `describe-bindings'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'describe-bindings))) nil) ("What does this do?" "You answered a question by using `(describe-function Info-goto-emacs-command-node info-lookup-symbol describe-variable describe-mode)'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '(describe-function Info-goto-emacs-command-node info-lookup-symbol describe-variable describe-mode)))) nil) ("What happens when I do this?" "You answered a question by using `(describe-key describe-key-briefly Info-goto-emacs-key-command-node where-is)'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '(describe-key describe-key-briefly Info-goto-emacs-key-command-node where-is)))) nil) ("When is a word not a word?" "You answered a question by using `describe-syntax'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'describe-syntax))) nil) ("What did I just do?" "You answered a question by using `(command-history view-lossage)'." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '(command-history view-lossage)))) nil) ("Shortcut genius" "You don't need to learn new shortcuts anymore." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(suggest-key-bindings nil)))) nil) ("Streamlined" "Your .emacs took less that 1 second to load." 5 t 0 nil (lambda nil (and (< (float-time (time-subtract after-init-time before-init-time)) 1))) nil) ("Oops" "Your .emacs file had an error." 5 nil 0 nil (lambda nil (and init-file-had-error)) nil) ("Traditionalist" "You use .emacs instead of .emacs.d/init.el." 5 nil 0 nil (lambda nil (and (string-match "/\\.emacs\\'" user-init-file))) nil) ("Modernist" "You use .emacs.d/init.el instead of .emacs." 5 nil 0 nil (lambda nil (and (string-match "/init\\.el\\'" user-init-file))) nil) ("Post Modernist" "You don't use .emacs.d/init.el or .emacs." 5 nil 0 nil (lambda nil (and (not (or (string-match "/init\\.el\\'" user-init-file) (string-match "/\\.emacs\\'" user-init-file))))) nil) ("Need for Speed" "Your .emacs is byte-compiled." 5 nil 0 nil (lambda nil (and (or (file-exists-p (concat user-init-file "c")) (file-exists-p (concat user-init-file ".elc"))))) nil) ("Last Year's Model" "Your byte-compiled .emacs is out of date." 5 nil 0 nil (lambda nil (and (cond ((file-exists-p (concat user-init-file "c")) (file-newer-than-file-p user-init-file (concat user-init-file "c"))) ((file-exists-p (concat user-init-file ".elc")) (file-newer-than-file-p user-init-file (concat user-init-file ".elc")))))) nil) ("Purest Vanilla" "You have no .emacs file.  How is that even possible?" 5 nil 0 nil (lambda nil (and (not (file-exists-p user-init-file)))) nil) ("Twenty Five" "You have enjoyed `5x5'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run '5x5))) nil) ("The Future of Pixar" "You have enjoyed `((animate-string animate-sequence animate-birthday-present))'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run '((animate-string animate-sequence animate-birthday-present))))) nil) ("Van Gogh" "You have enjoyed `artist-mode'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'artist-mode))) nil) ("Hide and Seek" "You have enjoyed `blackbox'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'blackbox))) nil) ("Blubb blubb" "You have enjoyed `bubbles'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'bubbles))) nil) ("Change the world!" "You have enjoyed `butterfly'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'butterfly))) nil) ("Spy vs Spy" "You have enjoyed `decipher'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'decipher))) nil) ("Tabloids" "You have enjoyed `dissociated-press'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'dissociated-press))) nil) ("I <3 Eliza" "You have enjoyed `doctor'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'doctor))) nil) ("Adventure!" "You have enjoyed `dunnet'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'dunnet))) nil) ("Connect 5" "You have enjoyed `gomoku'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'gomoku))) nil) ("Penmanship" "You have enjoyed `handwrite'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'handwrite))) nil) ("Saigon" "You have enjoyed `hanoi'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'hanoi))) nil) ("It's Alive!" "You have enjoyed `life'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'life))) nil) ("Telegraph Operator" "You have enjoyed `morse-region'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'morse-region))) nil) ("Arithmetician" "You have enjoyed `mpuz'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'mpuz))) nil) ("Ping" "You have enjoyed `pong'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'pong))) nil) ("Chase your tail" "You have enjoyed `snake'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'snake))) nil) ("All alone?" "You have enjoyed `solitaire'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'solitaire))) nil) ("Tessellator" "You have enjoyed `tetris'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'tetris))) nil) ("Yow!" "You have enjoyed `yow'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'yow))) nil) ("The Matrix" "You have enjoyed `zone'." 5 nil 50 nil (lambda nil (and (achievements-command-was-run 'zone))) nil) ("Um, Star?" "You have installed and enjoyed `emstar'." 5 nil 100 nil (lambda nil (and (achievements-command-was-run 'emstar))) nil) ("Can you read upside down?" "You have installed and enjoyed `fliptext-flip-region'." 5 nil 100 nil (lambda nil (and (achievements-command-was-run 'fliptext-flip-region))) nil) ("Hack, hack, hack" "You have installed and enjoyed `nethack'." 5 nil 100 nil (lambda nil (and (achievements-command-was-run 'nethack))) nil) ("Nyan, Nyan, Nyan" "You have installed and enjoyed `nyan-mode'." 5 nil 100 nil (lambda nil (and (achievements-command-was-run 'nyan-mode))) nil) ("Pretty Stable" "You have an uptime of over 1 day." 5 nil 0 nil (lambda nil (and (featurep 'uptimes) (> (let* ((uptime (car uptimes-top-n)) (seconds (- (cddr uptime) (cadr uptime)))) seconds) 86400))) nil) ("It keeps going and going..." "You have an uptime of over 1 week." 5 nil 0 nil (lambda nil (and (featurep 'uptimes) (> (let* ((uptime (car uptimes-top-n)) (seconds (- (cddr uptime) (cadr uptime)))) seconds) 604800))) nil) ("Marathon Hacker" "You have an uptime of over 30 days." 5 nil 0 nil (lambda nil (and (featurep 'uptimes) (> (let* ((uptime (car uptimes-top-n)) (seconds (- (cddr uptime) (cadr uptime)))) seconds) 2592000))) nil) ("Methuselah" "You have an uptime of over 1 year!?" 5 nil 0 nil (lambda nil (and (featurep 'uptimes) (> (let* ((uptime (car uptimes-top-n)) (seconds (- (cddr uptime) (cadr uptime)))) seconds) 31536000))) nil) ("Short Story" "You've written the equivalent of a short story." 5 nil 0 nil (lambda nil (and (> (achievements-num-times-commands-were-run '(self-insert-command org-self-insert-command)) 12000))) nil) ("Nanowrimo" "You could have finished Nanowrimo by now." 5 nil 0 nil (lambda nil (and (> (achievements-num-times-commands-were-run '(self-insert-command org-self-insert-command)) 300000))) nil) ("War and Peace" "You've written the equivalent of War and Peace." 5 nil 0 nil (lambda nil (and (> (achievements-num-times-commands-were-run '(self-insert-command org-self-insert-command)) 3523722))) nil) ("Proust" "You could have beaten Proust for longest novel." 5 nil 0 nil (lambda nil (and (> (achievements-num-times-commands-were-run '(self-insert-command org-self-insert-command)) 7200000))) nil) ("Loyalist" "You use GNU Emacs" 5 nil 0 nil (lambda nil (and (not (string-match "XEmacs\\|Lucid" emacs-version)))) nil) ("Patriot or Rebel?" "You use XEmacs" 5 nil 0 nil (lambda nil (and (string-match "XEmacs\\|Lucid" emacs-version))) nil) ("Green Glowing faces" "You have used the console version Emacs." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(window-system nil)))) nil) ("X marks the spot" "You have used the x version Emacs." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(window-system x)))) nil) ("MacPort or Aquamacs" "You have used the mac version Emacs." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(window-system mac)))) nil) ("GNUStep or Cocoa" "You have used the nextstep version Emacs." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(window-system ns)))) nil) ("Windows" "You have used the windows version Emacs." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(window-system w32)))) nil) ("DOS?" "You have used the DOS version Emacs." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(window-system pc)))) nil) ("Following the Hurd" "You have used Emacs on a gnu system." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(system-type gnu)))) nil) ("Tux's Friend" "You have used Emacs on a gnu/linux system." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(system-type gnu/linux)))) nil) ("Beastie's Pal" "You have used Emacs on a gnu/kfreebsd system." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(system-type gnu/kfreebsd)))) nil) ("Friends with Hexley" "You have used Emacs on a darwin system." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(system-type darwin)))) nil) ("DOS Box" "You have used Emacs on a ms-dos system." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(system-type ms-dos)))) nil) ("Windows Machine" "You have used Emacs on a windows-nt system." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(system-type windows-nt)))) nil) ("The Swan" "You have used Emacs on a cygwin system." 5 nil 0 nil (lambda nil (and (achievements-variable-was-set '(system-type cygwin)))) nil) ("Anonymous" "You have `user-mail-address' set to nil." 5 nil 0 nil (lambda nil (and (equal user-mail-address nil))) nil) ("The One and Only" "You are Richard Stallman." 50 nil 0 nil (lambda nil (and (equal user-mail-address "rms@gnu.org"))) nil) ("A Well Oiled Machine" "You have helped maintain Emacs." 50 nil 0 nil (lambda nil (and (member user-mail-address '("rms@gnu.org" "cyd@gnu.org" "monnier@iro.umontreal.ca" "eliz@gnu.org" "johnw@gnu.org" "larsi@gnus.org" "acorallo@gnu.org")))) nil) ("Tainted Love" "You have enabled non-GNU package repositories." 5 nil 0 nil (lambda nil (and (featurep 'package) (some (lambda (repo) (not (string-match "elpa\\.gnu\\.org" (cdr repo)))) package-archives))) nil) ("Vanilla" "You have no installed packages." 5 nil 0 nil (lambda nil (and (= (length package-alist) 0))) nil) ("Package Neophyte" "You have installed at least 1 package." 5 nil 0 nil (lambda nil (and (>= (length package-alist) 1))) nil) ("Package Apprentice" "You have installed over 10 packages." 5 nil 0 nil (lambda nil (and (>= (length package-alist) 10))) nil) ("Package Journeyman" "You have installed over 100 packages." 5 nil 0 nil (lambda nil (and (>= (length package-alist) 100))) nil) ("Clean Desk" "You have less than 10 buffers open." 5 nil 0 nil (lambda nil (and (<= (length (buffer-list)) 10))) nil) ("Messy Desk" "You have over 100 buffers open." 5 nil 0 nil (lambda nil (and (>= (length (buffer-list)) 100))) nil) ("Papers to the ceiling" "You have over 1000 buffers open." 5 nil 0 nil (lambda nil (and (>= (length (buffer-list)) 1000))) nil) ("The Ol' Switcheroo" "You've switched to another buffer" 5 nil 0 nil (lambda nil (and (achievements-command-was-run '((switch-to-buffer ido-switch-buffer))))) nil) ("Buffer, buffers, everywhere" "You've seen all the buffers that can be seen." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '((list-buffers ibuffer))))) nil) ("Top o' the morning" "You've used Emacs as a replacement for top." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'proced))) nil) ("Archer" "You use the arrow keys a lot." 5 nil 0 nil (lambda nil (and (let #1=((in-a-row 0) (success nil)) (mapc (lambda #2=(e) (if (memq e '(right left up down)) (if (>= #3=(incf in-a-row) 5) . #4=((setq success t))) . #5=((setq in-a-row 0)))) . #6=((recent-keys))) . #7=(success)))) nil) ("William Tell" "You use the arrow keys for almost everything." 5 nil 0 nil (lambda nil (and (let #1# (mapc (lambda #2# (if (memq e '(right left up down)) (if (>= #3# 20) . #4#) . #5#)) . #6#) . #7#))) nil) ("No arrows" "You know the replacements for the arrow keys." 5 nil 0 nil (lambda nil (and nil)) interpreted))"#
    ]];
    assert_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_advanced_catalog_exact_records_and_predicate_forms_match() {
    let elisp_form = r##"(cl-labels
         ((normalize-function
           (function)
           (cond
            ((byte-code-function-p
              function)
             'compiled)
            ((eq
              (type-of function)
              'interpreted-function)
             'interpreted)
            ((eq
              (car-safe function)
              'closure)
             (cons
              'lambda
              (cddr function)))
            (t function))))
         (mapcar
          (lambda (achievement)
            (list
            (emacs-achievement-name
             achievement)
            (emacs-achievement-description
             achievement)
            (emacs-achievement-points
             achievement)
            (emacs-achievement-transient
             achievement)
            (emacs-achievement-min-score
             achievement)
            (emacs-achievement-unlocks
             achievement)
            (let ((predicate
                   (emacs-achievement-predicate
                    achievement)))
              (cond
               ((eq predicate t)
                t)
               ((null predicate)
                nil)
               ((functionp predicate)
                (normalize-function
                 predicate))
               (t predicate)))))
          achievements-list))"##;
    let expect = expect![[
        r#"OK (("Inception" "You have used recursive editing and exited succesfully." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '(recursive-edit exit-recursive-edit))))) ("Narrow minded" "You have used narrowing." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '((narrow-to-region narrow-to-page narrow-to-defun)))))) ("Forbidden Fruits" "You have used all disabled commands." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '(erase-buffer dired-find-alternate-file list-timers list-threads narrow-to-page downcase-region upcase-region set-goal-column scroll-left narrow-to-region))))) ("Enabler" "You have enabled all commands." 5 nil 0 nil (lambda nil (and (= 0 (length (cl-loop for s being the symbols when (get s 'disabled) collect s)))))) ("Case Changer" "You have changed the case of a few words." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '(upcase-word downcase-word capitalize-word))))) ("CASE CHANGER" "You have changed the case of vast amounts of text." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '(upcase-region downcase-region))))) ("The Great Destroyer" "You have laid waste to an entire buffer in one go." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'erase-buffer)))) ("Goal Setter" "You have set the goal column." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'set-goal-column)))) ("Wide Load" "You have scrolled to see an extra wide buffer." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'scroll-left)))) ("Dired reuse" "You have reused a dired buffer to look at another file/directory." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'dired-find-alternate-file)))) ("Yes Man" "You can't say no." 5 nil 0 nil (lambda nil (and (and (>= (length yes-or-no-p-history) 10) (every (lambda (x) (equal "yes" x)) yes-or-no-p-history))))) ("Leaving Home" "You have edited files outside your home directory." 5 nil 0 nil (lambda nil (and (and (getenv "HOME") (not (every (lambda (x) (or (string-match (concat "^" (regexp-quote (getenv "HOME"))) x) (string-match (concat "^[~]/") x))) file-name-history)))))) ("The Examined Life" "You have command logging enabled." 5 nil 0 nil (lambda nil (and (achievements-command-was-run 'keyfreq-show)))) ("Playing it Safe" "Your .emacs is under version control." 5 nil 0 nil (lambda nil (and (and (require 'vc nil t) (vc-backend user-init-file))))) ("Arbitrator" "You have used smerge-mode to resolve conflicts." 5 nil 0 nil (lambda nil (and (achievements-command-was-run '((smerge-keep-all smerge-keep-base smerge-keep-current smerge-keep-mine smerge-keep-other)))))) ("Surfs up" "You use Emacs for surfing the web" 5 nil 0 nil (lambda nil (and (achievements-command-was-run '((eww eww-browse-url eww-open-file)))))) ("Polyglot" "You have used over 20 major-modes at once." 5 nil 0 nil (lambda nil (and (<= 20 (length (let ((modes nil)) (cl-loop for buf in (buffer-list) do (add-to-list 'modes (buffer-local-value 'major-mode buf))) modes)))))) ("Org-anizer" "You have used `org-mode'." 5 nil 0 org-achievements (lambda nil (and (achievements-command-was-run 'org-mode)))))"#
    ]];
    assert_advanced_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_input_in_a_row_builds_exact_form_and_detects_only_consecutive_events() {
    let elisp_form = r##"(let ((form
              (input-in-a-row
               '(left right)
               3)))
         (mapcar
          (lambda (events)
            (cl-letf
                (((symbol-function
                   'recent-keys)
                  (lambda ()
                    events)))
              (list
               events
               (eval form t))))
          '((left right left)
            (left other
                  right left)
            (up left right
                left down)
            (left right))))"##;
    let expect = expect![
        "OK (((left right left) t) ((left other right left) nil) ((up left right left down) t) ((left right) nil))"
    ];
    assert_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_no_arrows_post_command_predicate_removes_replacements_only_on_valid_keys() {
    let elisp_form = r##"(let* ((achievement
                  (achievements-get-achievements-by-name
                   "No arrows"))
                 (predicate
                  (emacs-achievement-post-command
                   achievement))
                 (achievements--arrow-keys-needing-replacements
                  '(right left up down))
                 (achievements--arrow-key-replacement-commands
                  '(forward-char
                    backward-char)))
         (mapcar
          (lambda (fixture)
            (let ((this-command
                   (car fixture))
                  (last-input-event
                   (cadr fixture)))
              (list
               fixture
               (funcall predicate)
               achievements--arrow-key-replacement-commands)))
          '((unrelated 120)
            (forward-char right)
            (forward-char 6)
            (backward-char 2))))"##;
    let expect = expect![
        "OK (((unrelated 120) nil #1=(forward-char . #2=(backward-char))) ((forward-char right) nil #1#) ((forward-char 6) nil #2#) ((backward-char 2) t nil))"
    ];
    assert_achievements_parity(elisp_form, expect);
}

#[test]
fn achievements_no_arrows_uses_default_replacement_commands_in_exact_order() {
    let elisp_form = r##"(let* ((achievement
                  (achievements-get-achievements-by-name
                   "No arrows"))
                 (predicate
                  (emacs-achievement-post-command
                   achievement))
                 (original
                  (copy-sequence
                   achievements--arrow-key-replacement-commands))
                 (last-input-event
                  'fixture-non-arrow))
         (list
          original
          (mapcar
           (lambda (command)
             (let ((this-command
                    command))
               (list
                command
                (funcall predicate)
                (copy-sequence
                 achievements--arrow-key-replacement-commands))))
           original)))"##;
    let expect = expect![
        "OK ((right-char left-char previous-line next-line) ((right-char nil (left-char previous-line next-line)) (left-char nil (previous-line next-line)) (previous-line nil (next-line)) (next-line t nil)))"
    ];
    assert_achievements_parity(elisp_form, expect);
}
