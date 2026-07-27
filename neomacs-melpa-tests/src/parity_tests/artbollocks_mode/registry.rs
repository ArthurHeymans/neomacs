use expect_test::expect;

use super::{assert_artbollocks_mode_autoload_parity, assert_artbollocks_mode_parity};

#[test]
fn artbollocks_mode_exact_pin_descriptor_dependency_origin_and_feature_contract_match() {
    let elisp_form = r##"(let ((descriptor
                (cadr
                 (assq
                  'artbollocks-mode
                  package-alist))))
         (list
          (package-desc-name
           descriptor)
          (package-version-join
           (package-desc-version
            descriptor))
          (package-desc-summary
           descriptor)
          (package-desc-kind
           descriptor)
          (package-desc-reqs
           descriptor)
          (package-desc-extras
           descriptor)
          (featurep
           'artbollocks-mode)))"##;
    let expect = expect![[
        r#"OK (artbollocks-mode "20251211.1624" "Improve your writing (especially about art)." nil ((emacs (25 1))) ((:maintainers ("Rob Myers" . "rob@robmyers.org") ("Sacha Chua" . "sacha@sachachua.com")) (:authors ("Rob Myers" . "rob@robmyers.org") ("Sacha Chua" . "sacha@sachachua.com")) (:revdesc . "63d20ed28462") (:commit . "63d20ed2846226f45b35eded69a776143a772ea4") (:url . "https://github.com/sachac/artbollocks-mode")) t)"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_mode_installed_payload_inventory_sizes_and_content_digests_match() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'artbollocks-mode
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor)))
         (mapcar
          (lambda (file)
            (let ((path
                   (expand-file-name
                    file
                    directory)))
              (list
               file
               (file-attribute-size
                (file-attributes
                 path))
               (secure-hash
                'sha256
                path))))
          (sort
           (seq-filter
            (lambda (file)
              (file-regular-p
               (expand-file-name
                file
                directory)))
            (directory-files
             directory
             nil
             "\\`[^.]"))
           #'string<)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" 465 "2f7417084d7573c7c177b75a10cd31659fed6133d33e7fa9aa2d6e3f4ed8419b") ("artbollocks-mode-autoloads.el" 1531 "cde67ff45f636c9c4e92aceb9d5d8815fc1f7ba904199b04465b339fe558cb74") ("artbollocks-mode-pkg.el" 514 "b5a809b40f2f104eea42958a15a33185cafa13d05eabf68581460bd5c42e3363") ("artbollocks-mode.el" 34889 "265a7e0a056b29fd341f2fd458aedff82a93cb785165a55eb52e4c8b2923ad99") ("artbollocks-mode.elc" 15734 "163f7a4d7ff0f13fdd767ada2d1f776f538b4c692e623930468fc73e481e0a0a"))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_mode_complete_callable_macro_alias_command_arglist_doc_and_source_surface_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (fboundp symbol)
            (macrop symbol)
            (commandp symbol)
            (interactive-form
             symbol)
            (help-function-arglist
             symbol
             t)
            (let ((doc
                   (documentation
                    symbol
                    t)))
              (and
               doc
               (secure-hash
                'sha256
                doc)))
            (let ((file
                   (symbol-file
                    symbol
                    'defun)))
              (and
               file
               (file-name-nondirectory
                file)))))
         '(artbollocks-passive-voice-regex
           artbollocks-weasel-words-regex
           artbollocks-jargon-regex
           artbollocks-inside-code-p
           artbollocks-search-for-keyword
           artbollocks-lexical-illusions-search-for-keyword
           artbollocks-passive-voice-search-for-keyword
           artbollocks-weasel-words-search-for-keyword
           artbollocks-search-for-jargon
           artbollocks-add-keywords
           artbollocks-remove-keywords
           interactive-optional-region
           artbollocks-count-letters
           artbollocks-count-syllables
           artbollocks-count-words
           artbollocks-count-sentences
           artbollocks-automated-readability-index
           artbollocks-flesch-reading-ease
           artbollocks-flesch-kinkaid-grade-level
           artbollocks-word-count
           artbollocks-sentence-count
           artbollocks-readability-index
           artbollocks-reading-ease
           artbollocks-grade-level
           artbollocks-mode))"##;
    let expect = expect![[
        r#"OK ((artbollocks-passive-voice-regex t nil nil nil nil "fa67e11f45407a10557ad2ab3ef128b2a7ef886810369919efb655a62d324568" "artbollocks-mode.el") (artbollocks-weasel-words-regex t nil nil nil nil "dc5bc1362813b1f2095632b304870a7d6721a7ca0888ef7c8e361203489d75e7" "artbollocks-mode.el") (artbollocks-jargon-regex t nil nil nil nil "ec0a8217bf2fa874edf8689d6675913af487b2120d578226528d7af168945137" "artbollocks-mode.el") (artbollocks-inside-code-p t nil nil nil (&optional pos) "7409d6fbcf983dd4c74fc74f1f6e90d6b432f676203a0d86c22ee583b883fdb9" "artbollocks-mode.el") (artbollocks-search-for-keyword t nil nil nil (regex limit) "aad1931c3a7c9c34edf740f664f66bc5c11f16a1d18acc88111677c730645b88" "artbollocks-mode.el") (artbollocks-lexical-illusions-search-for-keyword t nil nil nil (limit) nil "artbollocks-mode.el") (artbollocks-passive-voice-search-for-keyword t nil nil nil (limit) nil "artbollocks-mode.el") (artbollocks-weasel-words-search-for-keyword t nil nil nil (limit) nil "artbollocks-mode.el") (artbollocks-search-for-jargon t nil nil nil (limit) nil "artbollocks-mode.el") (artbollocks-add-keywords t nil nil nil nil nil "artbollocks-mode.el") (artbollocks-remove-keywords t nil nil nil nil nil "artbollocks-mode.el") (interactive-optional-region t t nil nil nil "785bf1aed79e9ba9705a598c4939def50dd450a1c6694f974663075d7b240b3f" "artbollocks-mode.el") (artbollocks-count-letters t nil nil nil (&optional start end) nil "artbollocks-mode.el") (artbollocks-count-syllables t nil nil nil (&optional start end) nil "artbollocks-mode.el") (artbollocks-count-words t nil t (interactive #1=(if (use-region-p) (list (region-beginning) (region-end)) (list (point-min) (point-max)))) #2=(&optional start end) "d891c0fce7e38579e52e87012bd519e92131467e5f39f24b8cf7bc652af689cf" "artbollocks-mode.el") (artbollocks-count-sentences t nil t (interactive #1#) #3=(&optional start end) "4f76ac93659a7c352a7374547984e9574991ea8c72702e3a3af76966292a5ab5" "artbollocks-mode.el") (artbollocks-automated-readability-index t nil nil nil (&optional start end) nil "artbollocks-mode.el") (artbollocks-flesch-reading-ease t nil nil nil (&optional start end) nil "artbollocks-mode.el") (artbollocks-flesch-kinkaid-grade-level t nil nil nil (&optional start end) nil "artbollocks-mode.el") (artbollocks-word-count t nil t (interactive #1#) #2# "d891c0fce7e38579e52e87012bd519e92131467e5f39f24b8cf7bc652af689cf" "artbollocks-mode.el") (artbollocks-sentence-count t nil t (interactive #1#) #3# "4f76ac93659a7c352a7374547984e9574991ea8c72702e3a3af76966292a5ab5" "artbollocks-mode.el") (artbollocks-readability-index t nil t (interactive #1#) (&optional start end) "e1d4f8fec84c50ca4014ff9ac4352853df72f1db83c7cd551c368b8c38637228" "artbollocks-mode.el") (artbollocks-reading-ease t nil t (interactive #1#) (&optional start end) "4b208d45879e361ffdfc75785c1670d4a685cb9b67a6951d3d341828d519844b" "artbollocks-mode.el") (artbollocks-grade-level t nil t (interactive #1#) (&optional start end) "245ee9f18cb464f51266f6125f48ebf2ba567c483c9072c0db0a7cc7080da2b9" "artbollocks-mode.el") (artbollocks-mode t nil t (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) (&optional arg) "4f0db634137a1ecb90c5e7349d0404a59aad3a1be98688fc261997251a5f300a" "artbollocks-mode.el"))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_mode_every_custom_variable_value_type_group_standard_and_documentation_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (let ((value
                  (symbol-value
                   symbol)))
             (list
              symbol
              (if
                  (and
                   (listp value)
                   (> (length value) 20))
                  (list
                   :length
                   (length value)
                   :first
                   (car value)
                   :last
                   (car
                    (last value))
                   :sha256
                   (secure-hash
                    'sha256
                    (prin1-to-string
                     value)))
                value)
              (get symbol 'custom-type)
              (get symbol 'custom-group)
              (get symbol 'standard-value)
              (custom-variable-p
               symbol)
              (local-variable-if-set-p
               symbol)
              (let ((doc
                     (documentation-property
                      symbol
                      'variable-documentation
                      t)))
                (and
                 doc
                 (secure-hash
                  'sha256
                  doc))))))
         '(artbollocks-lexical-illusions
           artbollocks-passive-voice
           artbollocks-weasel-words
           artbollocks-jargon
           artbollocks-passive-voice-words
           artbollocks-weasel-words-list
           artbollocks-jargon-words
           artbollocks-mode-hook))"##;
    let expect = expect![[
        r#"OK ((artbollocks-lexical-illusions t (boolean) nil #1=((funcall #'#[nil (t) #2=(t)])) #1# nil "0c1d43faa1f16139d2b81fe65556a711b0035593fa0b22b0943f15b3df38a1ea") (artbollocks-passive-voice t (boolean) nil #3=((funcall #'#[nil (t) #2#])) #3# nil "62a171913ee2dd0743705bd0fac97780e080dd76f05d596c7bf48d9faeb44c13") (artbollocks-weasel-words t (boolean) nil #4=((funcall #'#[nil (t) #2#])) #4# nil "867e7c606efa531275a6d8d792d534f51aa1946c796a060600b518b4ce5a3721") (artbollocks-jargon t (boolean) nil #5=((funcall #'#[nil (t) #2#])) #5# nil "003b7c077f7f5faac9b35e73c576caf2fb55ef3541a7d62ced6510d6019c3fc8") (artbollocks-passive-voice-words (:length 176 :first "\\w+ed" :last "written" :sha256 "a133bd9450d3be12c91de64fd302c6813bf40ba108677442c3f79bdaafa8891b") (repeat string) nil #6=((funcall #'#[nil ('("\\w+ed" "awoken" "been" "born" "beat" "become" "begun" "bent" "beset" "bet" "bid" "bidden" "bound" "bitten" "bled" "blown" "broken" "bred" "brought" "broadcast" "built" "burnt" "burst" "bought" "cast" "caught" "chosen" "clung" "come" "cost" "crept" "cut" "dealt" "dug" "dived" "done" "drawn" "dreamt" "driven" "drunk" "eaten" "fallen" "fed" "felt" "fought" "found" "fit" "fled" "flung" "flown" "forbidden" "forgotten" "foregone" "forgiven" "forsaken" "frozen" "gotten" "given" "gone" "ground" "grown" "hung" "heard" "hidden" "hit" "held" "hurt" "kept" "knelt" "knit" "known" "laid" "led" "leapt" "learnt" "left" "lent" "let" "lain" "lighted" "lost" "made" "meant" "met" "misspelt" "mistaken" "mown" "overcome" "overdone" "overtaken" "overthrown" "paid" "pled" "proven" "put" "quit" "read" "rid" "ridden" "rung" "risen" "run" "sawn" "said" "seen" "sought" "sold" "sent" "set" "sewn" "shaken" "shaven" "shorn" "shed" "shone" "shod" "shot" "shown" "shrunk" "shut" "sung" "sunk" "sat" "slept" "slain" "slid" "slung" "slit" "smitten" "sown" "spoken" "sped" "spent" "spilt" "spun" "spit" "split" "spread" "sprung" "stood" "stolen" "stuck" "stung" "stunk" "stridden" "struck" "strung" "striven" "sworn" "swept" "swollen" "swum" "swung" "taken" "taught" "torn" "told" "thought" "thrived" "thrown" "thrust" "trodden" "understood" "upheld" "upset" "woken" "worn" "woven" "wed" "wept" "wound" "won" "withheld" "withstood" "wrung" "written")) #2#])) #6# nil "d84fe74f2d81933b1ce856ae036a411a819d7f142dfe6eba0cd3a28676f5b3f5") (artbollocks-weasel-words-list (:length 24 :first "\\(\\(are\\|is\\) a number\\)" :last "completely" :sha256 "b84ee709ef422abc989dc066b41c31391768f6798fecb0dfb6087f85e514c412") (repeat string) nil #7=((funcall #'#[nil ('("\\(\\(are\\|is\\) a number\\)" "many" "various" "very" "fairly" "several" "extremely" "exceedingly" "quite" "remarkably" "few" "surprisingly" "mostly" "largely" "huge" "tiny" "excellent" "interestingly" "significantly" "substantially" "clearly" "vast" "relatively" "completely")) #2#])) #7# nil "641558e09a1f06bd78e7ccc6622da7a6ec05d74112617eec69d335b5fdee043b") (artbollocks-jargon-words (:length 214 :first "a priori" :last "zižekian" :sha256 "917423dc28034a9bcb51112dead8f6fcddd24add40039e415c0836ec2e962d87") (repeat string) nil #8=((funcall #'#[nil ('("a priori" "ad hoc" "affirmation" "affirm" "affirms" "alterity" "altermodern" "aporia" "aporetic" "appropriates" "appropriation" "archetypal" "archetypical" "archetype" "archetypes" "autonomous" "autonomy" "baudrillardian" "baudrillarian" "commodification" "committed" "commitment" "commonalities" "contemporaneity" "context" "contexts" "contextual" "contextualise" "contextualises" "contextualisation" "contextialize" "contextializes" "contextualization" "contextuality" "convention" "conventional" "conventions" "coterminous" "critique" "cunning" "cunningly" "death of the author" "debunk" "debunked" "debunking" "debunks" "deconstruct" "deconstruction" "deconstructs" "deleuzian" "desire" "desires" "dialectic" "dialectical" "dialectically" "discourse" "discursive" "disrupt" "disrupts" "engage" "engagement" "engages" "episteme" "epistemic" "ergo" "fetish" "fetishes" "fetishise" "fetishised" "fetishize" "fetishized" "gaze" "gender" "gendered" "historicise" "historicisation" "historicize" "historicization" "hegemonic" "hegemony" "identity" "identity politics" "intensifies" "intensify" "intensifying" "interrogate" "interrogates" "interrogation" "intertextual" "intertextuality" "irony" "ironic" "ironical" "ironically" "ironisation" "ironization" "ironises" "ironizes" "jouissance" "juxtapose" "juxtaposes" "juxtaposition" "lacanian" "lack" "loci" "locus" "locuses" "matrix" "mise en abyme" "mocking" "mockingly" "modalities" "modality" "myth" "mythologies" "mythology" "myths" "narrative" "narrativisation" "narrativization" "narrativity" "nexus" "nodal" "node" "normative" "normativity" "notion" "notions" "objective" "objectivity" "objectivities" "objet petit a" "ontology" "ontological" "operate" "operates" "otherness" "othering" "paradigm" "paradigmatic" "paradigms" "parody" "parodic" "parodies" "physicality" "plenitude" "poetics" "popular notions" "position" "post hoc" "post internet" "post-internet" "postmodernism" "postmodernist" "postmodernity" "postmodern" "practice" "practise" "praxis" "problematic" "problematics" "problematise" "problematize" "proposition" "qua" "reading" "readings" "reification" "relation" "relational" "relationality" "relations" "representation" "representations" "rhizomatic" "rhizome" "simulacra" "simulacral" "simulation" "simulationism" "simulationism" "situate" "situated" "situates" "stereotype" "stereotypes" "strategy" "strategies" "subjective" "subjectivity" "subjectivities" "subvert" "subversion" "subverts" "text" "textual" "textuality" "thinker" "thinkers" "trajectory" "transgress" "transgresses" "transgression" "transgressive" "unfolding" "undermine" "undermining" "undermines" "work" "works" "wry" "wryly" "zizekian" "zižekian")) #2#])) #8# nil "c14bb42c88c0e18aa7d248c068464df3765d7ff0a9400b74c1db02824eaca472") (artbollocks-mode-hook nil hook nil #9=(nil) #9# nil "0d0709e2061a5e2d89015254d923b593f81922584db9b1fd39ce31198080a562"))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_mode_word_dictionaries_preserve_order_duplicates_unicode_and_regex_entries() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (word)
            (list
             word
             (seq-count
              (lambda (candidate)
                (equal
                 candidate
                 word))
              artbollocks-passive-voice-words)
             (member
              word
              artbollocks-passive-voice-words)))
          '("\\w+ed"
            "been"
            "read"
            "written"
            "missing"))
         (mapcar
          (lambda (word)
            (list
             word
             (seq-count
              (lambda (candidate)
                (equal
                 candidate
                 word))
              artbollocks-weasel-words-list)))
          '("\\(\\(are\\|is\\) a number\\)"
            "very"
            "completely"
            "missing"))
         (mapcar
          (lambda (word)
            (list
             word
             (seq-count
              (lambda (candidate)
                (equal
                 candidate
                 word))
              artbollocks-jargon-words)
             (member
              word
              artbollocks-jargon-words)))
          '("a priori"
            "simulationism"
            "mise en abyme"
            "zižekian"
            "work"
            "missing")))"##;
    let expect = expect![[
        r#"OK ((("\\w+ed" 1 ("\\w+ed" "awoken" . #1=("been" "born" "beat" "become" "begun" "bent" "beset" "bet" "bid" "bidden" "bound" "bitten" "bled" "blown" "broken" "bred" "brought" "broadcast" "built" "burnt" "burst" "bought" "cast" "caught" "chosen" "clung" "come" "cost" "crept" "cut" "dealt" "dug" "dived" "done" "drawn" "dreamt" "driven" "drunk" "eaten" "fallen" "fed" "felt" "fought" "found" "fit" "fled" "flung" "flown" "forbidden" "forgotten" "foregone" "forgiven" "forsaken" "frozen" "gotten" "given" "gone" "ground" "grown" "hung" "heard" "hidden" "hit" "held" "hurt" "kept" "knelt" "knit" "known" "laid" "led" "leapt" "learnt" "left" "lent" "let" "lain" "lighted" "lost" "made" "meant" "met" "misspelt" "mistaken" "mown" "overcome" "overdone" "overtaken" "overthrown" "paid" "pled" "proven" "put" "quit" . #2=("read" "rid" "ridden" "rung" "risen" "run" "sawn" "said" "seen" "sought" "sold" "sent" "set" "sewn" "shaken" "shaven" "shorn" "shed" "shone" "shod" "shot" "shown" "shrunk" "shut" "sung" "sunk" "sat" "slept" "slain" "slid" "slung" "slit" "smitten" "sown" "spoken" "sped" "spent" "spilt" "spun" "spit" "split" "spread" "sprung" "stood" "stolen" "stuck" "stung" "stunk" "stridden" "struck" "strung" "striven" "sworn" "swept" "swollen" "swum" "swung" "taken" "taught" "torn" "told" "thought" "thrived" "thrown" "thrust" "trodden" "understood" "upheld" "upset" "woken" "worn" "woven" "wed" "wept" "wound" "won" "withheld" "withstood" "wrung" . #3=("written"))))) ("been" 1 #1#) ("read" 1 #2#) ("written" 1 #3#) ("missing" 0 nil)) (("\\(\\(are\\|is\\) a number\\)" 1) ("very" 1) ("completely" 1) ("missing" 0)) (("a priori" 1 ("a priori" "ad hoc" "affirmation" "affirm" "affirms" "alterity" "altermodern" "aporia" "aporetic" "appropriates" "appropriation" "archetypal" "archetypical" "archetype" "archetypes" "autonomous" "autonomy" "baudrillardian" "baudrillarian" "commodification" "committed" "commitment" "commonalities" "contemporaneity" "context" "contexts" "contextual" "contextualise" "contextualises" "contextualisation" "contextialize" "contextializes" "contextualization" "contextuality" "convention" "conventional" "conventions" "coterminous" "critique" "cunning" "cunningly" "death of the author" "debunk" "debunked" "debunking" "debunks" "deconstruct" "deconstruction" "deconstructs" "deleuzian" "desire" "desires" "dialectic" "dialectical" "dialectically" "discourse" "discursive" "disrupt" "disrupts" "engage" "engagement" "engages" "episteme" "epistemic" "ergo" "fetish" "fetishes" "fetishise" "fetishised" "fetishize" "fetishized" "gaze" "gender" "gendered" "historicise" "historicisation" "historicize" "historicization" "hegemonic" "hegemony" "identity" "identity politics" "intensifies" "intensify" "intensifying" "interrogate" "interrogates" "interrogation" "intertextual" "intertextuality" "irony" "ironic" "ironical" "ironically" "ironisation" "ironization" "ironises" "ironizes" "jouissance" "juxtapose" "juxtaposes" "juxtaposition" "lacanian" "lack" "loci" "locus" "locuses" "matrix" . #5=("mise en abyme" "mocking" "mockingly" "modalities" "modality" "myth" "mythologies" "mythology" "myths" "narrative" "narrativisation" "narrativization" "narrativity" "nexus" "nodal" "node" "normative" "normativity" "notion" "notions" "objective" "objectivity" "objectivities" "objet petit a" "ontology" "ontological" "operate" "operates" "otherness" "othering" "paradigm" "paradigmatic" "paradigms" "parody" "parodic" "parodies" "physicality" "plenitude" "poetics" "popular notions" "position" "post hoc" "post internet" "post-internet" "postmodernism" "postmodernist" "postmodernity" "postmodern" "practice" "practise" "praxis" "problematic" "problematics" "problematise" "problematize" "proposition" "qua" "reading" "readings" "reification" "relation" "relational" "relationality" "relations" "representation" "representations" "rhizomatic" "rhizome" "simulacra" "simulacral" "simulation" . #4=("simulationism" "simulationism" "situate" "situated" "situates" "stereotype" "stereotypes" "strategy" "strategies" "subjective" "subjectivity" "subjectivities" "subvert" "subversion" "subverts" "text" "textual" "textuality" "thinker" "thinkers" "trajectory" "transgress" "transgresses" "transgression" "transgressive" "unfolding" "undermine" "undermining" "undermines" . #7=("work" "works" "wry" "wryly" "zizekian" . #6=("zižekian")))))) ("simulationism" 2 #4#) ("mise en abyme" 1 #5#) ("zižekian" 1 #6#) ("work" 1 #7#) ("missing" 0 nil)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_mode_regex_keyword_constants_faces_and_keymap_bindings_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (symbol-value
              symbol)
             (documentation-property
              symbol
              'variable-documentation
              t)))
          '(artbollocks-lexical-illusions-regex
            artbollocks-lexicalkwlist
            artbollocks-passivekwlist
            artbollocks-weaselkwlist
            artbollocks-kwlist))
         (mapcar
          (lambda (face)
            (list
             face
             (facep face)
             (face-attribute
              face
              :foreground
              nil
              'default)
             (face-attribute
              face
              :background
              nil
              'default)))
          '(artbollocks-lexical-illusions-face
            artbollocks-passive-voice-face
            artbollocks-weasel-words-face
            artbollocks-face))
         (keymapp
          artbollocks-mode-keymap)
         (mapcar
          (lambda (key)
            (list
             key
             (lookup-key
              artbollocks-mode-keymap
              (kbd key))))
          '("C-c ["
            "C-c ]"
            "C-c \\"
            "C-c /"
            "C-c ="
            "C-c x")))"##;
    let expect = expect![[
        r#"OK (((artbollocks-lexical-illusions-regex "\\b\\(\\w+\\)\\W+\\(\\1\\)\\b" nil) (artbollocks-lexicalkwlist ((artbollocks-lexical-illusions-search-for-keyword (2 'artbollocks-lexical-illusions-face t))) nil) (artbollocks-passivekwlist ((artbollocks-passive-voice-search-for-keyword (0 'artbollocks-passive-voice-face t))) nil) (artbollocks-weaselkwlist ((artbollocks-weasel-words-search-for-keyword (0 'artbollocks-weasel-words-face t))) nil) (artbollocks-kwlist ((artbollocks-search-for-jargon (0 'artbollocks-face t))) nil)) ((artbollocks-lexical-illusions-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "black" "magenta") (artbollocks-passive-voice-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "Gray" "White") (artbollocks-weasel-words-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "Brown" "White") (artbollocks-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] "Purple" "White")) t (("C-c [" artbollocks-word-count) ("C-c ]" artbollocks-sentence-count) ("C-c \\" artbollocks-readability-index) ("C-c /" artbollocks-reading-ease) ("C-c =" artbollocks-grade-level) ("C-c x" nil)))"#
    ]];

    assert_artbollocks_mode_parity(elisp_form, expect);
}

#[test]
fn artbollocks_mode_autoload_exposes_minor_mode_contract_without_loading_source() {
    let elisp_form = r##"(let ((function
                (symbol-function
                 'artbollocks-mode)))
         (list
          (featurep
           'artbollocks-mode)
          (fboundp
           'artbollocks-mode)
          (autoloadp function)
          (and
           (autoloadp function)
           (nth 1 function))
          (and
           (autoloadp function)
           (nth 4 function))
          (commandp
           'artbollocks-mode)
          (interactive-form
           'artbollocks-mode)
          (boundp
           'artbollocks-mode)
          (boundp
           'artbollocks-mode-map)
          (boundp
           'artbollocks-mode-keymap)))"##;
    let expect = expect![[
        r#"OK (nil t t "artbollocks-mode" nil t (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) t nil t)"#
    ]];

    assert_artbollocks_mode_autoload_parity(elisp_form, expect);
}
