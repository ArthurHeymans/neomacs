//! Package ecosystem compatibility harness for Neomacs.
//!
//! A scenario installs packages into an isolated, workspace-local sandbox,
//! exits the editor, and launches a fresh process to probe the installed
//! packages. The same scenario can run against Neomacs or GNU Emacs and
//! against either revision-pinned package source or a local fixture archive.

use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use neomacs_test_oracle::{
    BatchProbe, EvalOutcome, ExpectedOutcome, extract_marked_batch_protocol,
    extract_marked_outcome, wrap_elisp_batch_outcomes, wrap_elisp_outcome,
};
use wait_timeout::ChildExt;

mod source_lock;

pub use source_lock::{
    LockedPackageSource, SourceBuild, locked_melpa_install_plan, locked_melpa_source,
    locked_melpa_sources, preflight_locked_melpa_packages, prepare_cached_locked_melpa_package,
};

const RESULT_MARKER: &str = "NEOMACS-MELPA-RESULT:";
const OUTCOME_MARKER: &str = "NEOMACS-MELPA-OUTCOME:";
const BATCH_BEGIN_MARKER: &str = "NEOMACS-MELPA-BEGIN:";
const BATCH_COMPLETE_MARKER: &str = "NEOMACS-MELPA-COMPLETE:";
const TRANSPORTED_FORM_FUNCTION: &str = "neomacs--melpa-oracle-transported-form";
const INSTALLED_MARKER: &str = "NEOMACS-MELPA-INSTALLED:";
const DEFAULT_PROCESS_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Copy)]
struct PackageArchiveSpec {
    cache_directory: &'static str,
    label: &'static str,
    name: &'static str,
    url: &'static str,
}

const GNU_ELPA_ARCHIVE: PackageArchiveSpec = PackageArchiveSpec {
    cache_directory: "package-cache-gnu-elpa",
    label: "GNU ELPA",
    name: "gnu",
    url: "https://elpa.gnu.org/packages/",
};

/// The exact Async release selected from GNU ELPA by the comprehensive API
/// parity corpus.
pub const ASYNC_GNU_ELPA_PIN: (&str, &str) = ("async", "1.9.9");

/// The exact current MELPA Async package selected by the comprehensive
/// serialization, process, byte-compilation, Dired, package, and mail workflow
/// parity corpus. This remains distinct from `ASYNC_GNU_ELPA_PIN`.
/// MELPA built this archive from upstream commit
/// `5faab28916603bb324d9faba057021ce028ca847`.
pub const ASYNC_MELPA_PIN: (&str, &str) = ("async", "20260318.1803");

/// The exact async1 package selected by the comprehensive callback-chain,
/// scheduler, parallel aggregation, and timer parity corpus. MELPA built this
/// archive from upstream commit
/// `88cccffe14bdd0a61dbb2e33edf8c335706f24dc`.
pub const ASYNC1_MELPA_PIN: (&str, &str) = ("async1", "20260421.2116");

/// The exact asyncloop package selected by the comprehensive non-blocking
/// series, cancellation, timer-ordering, recovery, and lifecycle parity
/// corpus. MELPA built this archive from upstream commit
/// `7d60950d160098a879293e049b9863bc955f8666`.
pub const ASYNCLOOP_MELPA_PIN: (&str, &str) = ("asyncloop", "20240818.1247");

/// The exact atom-dark-theme package selected by the comprehensive theme
/// registration, face, customization, remapping, and lifecycle parity corpus.
/// MELPA built this archive from upstream commit
/// `2b3c7ad42bbcab3214a131f8957b92e717b36ad3`.
pub const ATOM_DARK_THEME_MELPA_PIN: (&str, &str) = ("atom-dark-theme", "20220114.1902");

/// The exact atom-one-dark-theme package selected by the comprehensive
/// palette, face, variable, remapping, and lifecycle parity corpus. MELPA
/// built this archive from upstream commit
/// `bba02fb2672a4c439d71920d8e068a3ff2ed463e`.
pub const ATOM_ONE_DARK_THEME_MELPA_PIN: (&str, &str) = ("atom-one-dark-theme", "20260119.1824");

/// The exact auto-complete package selected by the comprehensive source,
/// candidate, completion, dictionary, history, configuration, and lifecycle
/// parity corpus. MELPA built this archive from upstream commit
/// `07f9915e08342410b933145d7934998709753a29`.
pub const AUTO_COMPLETE_MELPA_PIN: (&str, &str) = ("auto-complete", "20251231.1622");

/// The exact auto-complete-auctex package selected by the comprehensive
/// argument-expansion, candidate, action, source, setup, and real LaTeX
/// workflow parity corpus. MELPA built this archive from upstream commit
/// `855633f668bcc4b9408396742a7cb84e0c4a2f77`.
pub const AUTO_COMPLETE_AUCTEX_MELPA_PIN: (&str, &str) = ("auto-complete-auctex", "20140223.1758");

/// The exact AUCTeX release selected from GNU ELPA for the
/// auto-complete-auctex integration parity corpus.
pub const AUCTEX_GNU_ELPA_PIN: (&str, &str) = ("auctex", "14.1.2");

/// The exact auto-complete-c-headers package selected by the comprehensive
/// include-path, filesystem-cache, documentation, candidate-source, and
/// completion workflow parity corpus. MELPA built this archive from upstream
/// commit `52fef720c6f274ad8de52bef39a343421006c511`.
pub const AUTO_COMPLETE_C_HEADERS_MELPA_PIN: (&str, &str) =
    ("auto-complete-c-headers", "20150912.323");

/// The exact auto-complete-chunk package selected by the comprehensive
/// chunk-boundary, candidate, source, dictionary, and practical completion
/// workflow parity corpus. MELPA built this archive from upstream commit
/// `a9aa77ffb84a1037984a7ce4dda25074272f13fe`.
pub const AUTO_COMPLETE_CHUNK_MELPA_PIN: (&str, &str) = ("auto-complete-chunk", "20140225.946");

/// The exact auto-complete-clang package selected by the comprehensive output
/// parsing, compiler invocation, language/argument, documentation, template,
/// and completion workflow parity corpus. MELPA built this archive from
/// upstream commit `a195db1d0593b4fb97efe50885e12aa6764d998c`.
pub const AUTO_COMPLETE_CLANG_MELPA_PIN: (&str, &str) = ("auto-complete-clang", "20140409.752");

/// The exact auto-complete-clang-async package selected by the comprehensive
/// completion parsing, template, client/server protocol, asynchronous process,
/// syntax-check, and C/C++ workflow parity corpus. MELPA built this archive
/// from upstream commit `a5114e3477793ccb9420acc5cd6a1cb26be65964`.
pub const AUTO_COMPLETE_CLANG_ASYNC_MELPA_PIN: (&str, &str) =
    ("auto-complete-clang-async", "20130526.1527");

/// The exact auto-complete-distel package selected by the comprehensive
/// prefix, source, Distel bridge, documentation, and practical Erlang
/// completion workflow parity corpus. MELPA built this archive from upstream
/// commit `acc4c0a5521904203d797fe96b08e5fae4233c7e`.
pub const AUTO_COMPLETE_DISTEL_MELPA_PIN: (&str, &str) = ("auto-complete-distel", "20180827.1344");

/// The exact companion Distel completion library required by
/// `AUTO_COMPLETE_DISTEL_MELPA_PIN`. MELPA built both archives from the same
/// upstream commit `acc4c0a5521904203d797fe96b08e5fae4233c7e`.
/// The exact Distel Completion Lib package selected for the practical Erlang
/// source indexing and completion workflow corpus, and as auto-complete-distel's
/// completion-library dependency. MELPA built this archive from upstream
/// commit `acc4c0a5521904203d797fe96b08e5fae4233c7e`.
pub const DISTEL_COMPLETION_LIB_MELPA_PIN: (&str, &str) =
    ("distel-completion-lib", "20180827.1344");

/// The exact auto-complete-exuberant-ctags package selected by the
/// comprehensive tag discovery, index parsing, candidate, hook, and practical
/// project workflow parity corpus. MELPA built this archive from upstream
/// commit `ff6121ff8b71beb5aa606d28fd389c484ed49765`.
pub const AUTO_COMPLETE_EXUBERANT_CTAGS_MELPA_PIN: (&str, &str) =
    ("auto-complete-exuberant-ctags", "20140320.724");

/// The exact auto-complete-nxml package selected by the comprehensive context,
/// candidate, documentation, namespace, action, and practical nXML workflow
/// parity corpus. MELPA built this archive from upstream commit
/// `ac7b09a23e45f9bd02affb31847263de4180163a`.
pub const AUTO_COMPLETE_NXML_MELPA_PIN: (&str, &str) = ("auto-complete-nxml", "20140221.458");

/// The exact auto-complete-pcmp package selected by the comprehensive
/// programmable-completion capture, action, advice, error, and practical
/// command workflow parity corpus. MELPA built this archive from upstream
/// commit `2595d3dab1ef3549271ca922f212928e9d830eec`.
pub const AUTO_COMPLETE_PCMP_MELPA_PIN: (&str, &str) = ("auto-complete-pcmp", "20140303.255");

/// The exact log4e dependency selected for the auto-complete-pcmp corpus and
/// by the practical logger lifecycle, formatting, messaging, navigation, and
/// source-instrumentation parity corpus. MELPA built this archive from
/// upstream commit `6d71462df9bf595d3861bfb328377346aceed422`.
pub const LOG4E_MELPA_PIN: (&str, &str) = ("log4e", "20240123.1313");

/// The exact yaxception dependency selected for the auto-complete-pcmp corpus.
pub const YAXCEPTION_MELPA_PIN: (&str, &str) = ("yaxception", "20240107.504");

/// The exact auto-complete-rst package selected by the comprehensive source
/// generation, directive/option parsing, command, setup, and practical
/// reStructuredText workflow parity corpus. MELPA built this archive from
/// upstream commit `4803ce41a96224e6fa54e6741a5b5f40ebed7351`.
pub const AUTO_COMPLETE_RST_MELPA_PIN: (&str, &str) = ("auto-complete-rst", "20140225.944");

/// The exact auto-complete-sage package selected by the comprehensive
/// documentation-cache, REPL, edit-buffer, source, setup, and practical Sage
/// completion workflow parity corpus. MELPA built this archive from upstream
/// commit `51b8e3905196d266e1f8aa47881189833151b398`.
pub const AUTO_COMPLETE_SAGE_MELPA_PIN: (&str, &str) = ("auto-complete-sage", "20160514.751");

/// The exact current sage-shell-mode dependency selected for the
/// auto-complete-sage integration corpus. MELPA built this archive from
/// upstream commit `bb59cd559a9d7639d9ef16addbb0809ea4790392`.
pub const SAGE_SHELL_MODE_MELPA_PIN: (&str, &str) = ("sage-shell-mode", "20260523.1504");

/// The exact Deferred package selected for the practical asynchronous order,
/// recovery, ledger, parallel aggregation, and subprocess parity corpus, and
/// as the sage-shell-mode package graph dependency. MELPA built this archive
/// from upstream commit `2239671d94b38d92e9b28d4e12fd79814cfb9c16`.
pub const DEFERRED_MELPA_PIN: (&str, &str) = ("deferred", "20170901.1330");

/// The exact Elixir Mode package selected by the practical indentation,
/// fontification, navigation, documentation, and formatter workflow corpus.
/// MELPA built this archive from upstream commit
/// `00d6580a040a750e019218f9392cf9a4c2dac23a`.
pub const ELIXIR_MODE_MELPA_PIN: (&str, &str) = ("elixir-mode", "20230626.1738");

/// The exact Emmet Mode package selected by the practical HTML, CSS, JSX,
/// preview, wrapping, and edit-point workflow corpus. MELPA built this archive
/// from upstream commit `322d3bb112fced57d63b44863357f7a0b7eee1e3`.
pub const EMMET_MODE_MELPA_PIN: (&str, &str) = ("emmet-mode", "20240617.45");

/// The exact EPL package selected by the practical package metadata,
/// descriptor, database, installation, deletion, and built-in discovery
/// workflow corpus. MELPA built this archive from upstream commit
/// `78ab7a85c08222cd15582a298a364774e3282ce6`.
pub const EPL_MELPA_PIN: (&str, &str) = ("epl", "20180205.2049");

/// The exact Erlang Mode package selected by the practical OTP module
/// editing, semantic fontification, navigation, EDoc, skeleton, identifier,
/// and compile-option workflow corpus. MELPA built this archive from upstream
/// OTP commit `1259612946cb36a8bf9614b289090bb32fbcbeb2`.
pub const ERLANG_MELPA_PIN: (&str, &str) = ("erlang", "20260724.1508");

/// The exact GNU ELPA let-alist dependency selected for the sage-shell-mode
/// package graph.
pub const LET_ALIST_GNU_ELPA_PIN: (&str, &str) = ("let-alist", "1.0.6");

/// The exact audio-notes-mode package selected by the comprehensive
/// filesystem, playback, process-control, mode-line, advice, and global-mode
/// lifecycle parity corpus. MELPA built this archive from upstream commit
/// `fa38350829c7e97257efc746a010471d33748a68`.
pub const AUDIO_NOTES_MODE_MELPA_PIN: (&str, &str) = ("audio-notes-mode", "20170611.2159");

/// The exact australia-holidays package selected by the comprehensive
/// national, state, territory, customization, date-calculation, and calendar
/// integration parity corpus. MELPA built this archive from upstream commit
/// `a73bbc940bc953164b8ed77e61e65a7a3aff4da5`.
pub const AUSTRALIA_HOLIDAYS_MELPA_PIN: (&str, &str) = ("australia-holidays", "20250706.1213");

/// The exact auth-source-kwallet package selected by the comprehensive
/// backend, process, cache, customization, and authentication-workflow parity
/// corpus. MELPA built this archive from upstream commit
/// `1e1bff2403966c3a0683ee65fb28cb8d8ff2c389`.
pub const AUTH_SOURCE_KWALLET_MELPA_PIN: (&str, &str) = ("auth-source-kwallet", "20250419.1330");

/// The exact popup dependency selected for the auto-complete parity corpus.
/// MELPA built this archive from upstream commit
/// `45a0b759076ce4139aba36dde0a2904136282e73`.
pub const POPUP_MELPA_PIN: (&str, &str) = ("popup", "20251231.1622");

/// The exact 0blayout package selected by the comprehensive API parity corpus.
pub const ZERO_B_LAYOUT_MELPA_PIN: (&str, &str) = ("0blayout", "20190703.527");

/// The exact 0x0 package selected by the comprehensive API parity corpus.
pub const ZERO_X_ZERO_MELPA_PIN: (&str, &str) = ("0x0", "20230823.2214");

/// The exact 0xc package selected by the comprehensive API parity corpus.
pub const ZERO_X_C_MELPA_PIN: (&str, &str) = ("0xc", "20201025.2105");

/// The exact 2048-game package selected by the comprehensive API parity corpus.
pub const GAME_2048_MELPA_PIN: (&str, &str) = ("2048-game", "20230809.356");

/// The exact 2bit package selected by the comprehensive API parity corpus.
pub const TWO_BIT_MELPA_PIN: (&str, &str) = ("2bit", "20200926.1418");

/// The exact 750words package selected by the comprehensive API parity corpus.
pub const SEVEN_FIFTY_WORDS_MELPA_PIN: (&str, &str) = ("750words", "20220625.1407");

/// The exact @ package selected by the comprehensive API parity corpus.
pub const AT_MELPA_PIN: (&str, &str) = ("@", "20240923.1318");

/// The exact a package selected by the comprehensive API parity corpus.
pub const A_MELPA_PIN: (&str, &str) = ("a", "20210929.1510");

/// The exact aa-edit-mode package selected by the comprehensive API parity
/// corpus.
pub const AA_EDIT_MODE_MELPA_PIN: (&str, &str) = ("aa-edit-mode", "20170119.320");

/// The exact Aangit package selected by the comprehensive API parity corpus.
pub const AANGIT_MELPA_PIN: (&str, &str) = ("aangit", "20231106.2115");

/// The exact AAS package selected by the comprehensive API parity corpus.
pub const AAS_MELPA_PIN: (&str, &str) = ("aas", "20230303.2214");

/// The exact abc-mode package selected by the comprehensive API parity corpus.
pub const ABC_MODE_MELPA_PIN: (&str, &str) = ("abc-mode", "20220713.1359");

/// The exact Abgaben package selected by the comprehensive API parity corpus.
pub const ABGABEN_MELPA_PIN: (&str, &str) = ("abgaben", "20171119.646");

/// The exact abl-mode package selected by the comprehensive API parity corpus.
pub const ABL_MODE_MELPA_PIN: (&str, &str) = ("abl-mode", "20240423.1214");

/// The exact abridge-diff package selected by the comprehensive API parity
/// corpus.
pub const ABRIDGE_DIFF_MELPA_PIN: (&str, &str) = ("abridge-diff", "20230307.2159");

/// The exact abs-mode package selected by the comprehensive API parity corpus.
pub const ABS_MODE_MELPA_PIN: (&str, &str) = ("abs-mode", "20260415.813");

/// The exact abyss-theme package selected by the comprehensive API parity
/// corpus.
pub const ABYSS_THEME_MELPA_PIN: (&str, &str) = ("abyss-theme", "20260125.1959");

/// The exact ac-alchemist package selected by the comprehensive API parity
/// corpus.
pub const AC_ALCHEMIST_MELPA_PIN: (&str, &str) = ("ac-alchemist", "20150908.656");

/// The exact ac-c-headers package selected by the comprehensive API parity
/// corpus.
pub const AC_C_HEADERS_MELPA_PIN: (&str, &str) = ("ac-c-headers", "20200816.1007");

/// The exact ac-capf package selected by the comprehensive API parity corpus.
pub const AC_CAPF_MELPA_PIN: (&str, &str) = ("ac-capf", "20151101.217");

/// The exact ac-clang package selected by the comprehensive API parity corpus.
pub const AC_CLANG_MELPA_PIN: (&str, &str) = ("ac-clang", "20180710.546");

/// The exact ac-dcd package selected by the comprehensive API parity corpus.
pub const AC_DCD_MELPA_PIN: (&str, &str) = ("ac-dcd", "20250925.946");

/// The exact ac-emmet package selected by the comprehensive API parity corpus.
pub const AC_EMMET_MELPA_PIN: (&str, &str) = ("ac-emmet", "20131015.1558");

/// The exact ac-emoji package selected by the comprehensive API parity corpus.
pub const AC_EMOJI_MELPA_PIN: (&str, &str) = ("ac-emoji", "20150823.711");

/// The exact ac-etags package selected by the comprehensive API parity corpus.
pub const AC_ETAGS_MELPA_PIN: (&str, &str) = ("ac-etags", "20161001.1507");

/// The exact ac-geiser package selected by the comprehensive API parity corpus.
pub const AC_GEISER_MELPA_PIN: (&str, &str) = ("ac-geiser", "20200318.824");

/// The exact ac-haskell-process package selected by the comprehensive API
/// parity corpus.
pub const AC_HASKELL_PROCESS_MELPA_PIN: (&str, &str) = ("ac-haskell-process", "20150423.1402");

/// The exact ac-helm package selected by the comprehensive API parity corpus.
pub const AC_HELM_MELPA_PIN: (&str, &str) = ("ac-helm", "20160319.233");

/// The exact ac-html package selected by the comprehensive API parity corpus.
pub const AC_HTML_MELPA_PIN: (&str, &str) = ("ac-html", "20151005.731");

/// The exact ac-html-angular package selected by the comprehensive API parity
/// corpus.
pub const AC_HTML_ANGULAR_MELPA_PIN: (&str, &str) = ("ac-html-angular", "20151225.719");

/// The exact ac-html-bootstrap package selected by the comprehensive API
/// parity corpus.
pub const AC_HTML_BOOTSTRAP_MELPA_PIN: (&str, &str) = ("ac-html-bootstrap", "20160302.1701");

/// The exact ac-html-csswatcher package selected by the comprehensive API
/// parity corpus.
pub const AC_HTML_CSSWATCHER_MELPA_PIN: (&str, &str) = ("ac-html-csswatcher", "20151208.2113");

/// The exact ac-inf-ruby package selected by the comprehensive API parity
/// corpus.
pub const AC_INF_RUBY_MELPA_PIN: (&str, &str) = ("ac-inf-ruby", "20131115.1150");

/// The exact ac-ispell package selected by the comprehensive API parity
/// corpus.
pub const AC_ISPELL_MELPA_PIN: (&str, &str) = ("ac-ispell", "20151101.226");

/// The exact ac-js2 package selected by the comprehensive API parity corpus.
pub const AC_JS2_MELPA_PIN: (&str, &str) = ("ac-js2", "20190101.933");

/// The exact ac-math package selected by the comprehensive API parity corpus.
pub const AC_MATH_MELPA_PIN: (&str, &str) = ("ac-math", "20141116.2127");

/// The exact ac-mozc package selected by the comprehensive API parity corpus.
pub const AC_MOZC_MELPA_PIN: (&str, &str) = ("ac-mozc", "20150227.1619");

/// The exact ac-octave package selected by the comprehensive API parity corpus.
pub const AC_OCTAVE_MELPA_PIN: (&str, &str) = ("ac-octave", "20180406.334");

/// The exact ac-php package selected by the comprehensive API parity corpus.
pub const AC_PHP_MELPA_PIN: (&str, &str) = ("ac-php", "20240328.1036");

/// The exact ac-php-core package selected by the comprehensive API parity
/// corpus.
pub const AC_PHP_CORE_MELPA_PIN: (&str, &str) = ("ac-php-core", "20260210.846");

/// The exact ac-racer package selected by the comprehensive API parity corpus.
pub const AC_RACER_MELPA_PIN: (&str, &str) = ("ac-racer", "20170114.809");

/// The exact ac-rtags package selected by the comprehensive API parity corpus.
pub const AC_RTAGS_MELPA_PIN: (&str, &str) = ("ac-rtags", "20191222.920");

/// The exact ac-skk package selected by the comprehensive API parity corpus.
pub const AC_SKK_MELPA_PIN: (&str, &str) = ("ac-skk", "20141230.119");

/// The exact ac-slime package selected by the comprehensive API parity corpus.
pub const AC_SLIME_MELPA_PIN: (&str, &str) = ("ac-slime", "20171027.2100");

/// The exact ac-sly package selected by the comprehensive API parity corpus.
pub const AC_SLY_MELPA_PIN: (&str, &str) = ("ac-sly", "20170728.1027");

/// The exact Academic Phrases package selected by the comprehensive API parity
/// corpus.
pub const ACADEMIC_PHRASES_MELPA_PIN: (&str, &str) = ("academic-phrases", "20180723.1021");

/// The exact Accent package selected by the comprehensive API parity corpus.
pub const ACCENT_MELPA_PIN: (&str, &str) = ("accent", "20250210.906");

/// The exact Ace Flyspell package selected by the comprehensive API parity
/// corpus.
pub const ACE_FLYSPELL_MELPA_PIN: (&str, &str) = ("ace-flyspell", "20170309.509");

/// The exact Ace Isearch package selected by the comprehensive API parity
/// corpus.
pub const ACE_ISEARCH_MELPA_PIN: (&str, &str) = ("ace-isearch", "20220809.1748");

/// The exact Ace Jump Buffer package selected by the comprehensive API parity
/// corpus.
pub const ACE_JUMP_BUFFER_MELPA_PIN: (&str, &str) = ("ace-jump-buffer", "20171031.1550");

/// The exact Ace Jump Helm Line package selected by the comprehensive API
/// parity corpus.
pub const ACE_JUMP_HELM_LINE_MELPA_PIN: (&str, &str) = ("ace-jump-helm-line", "20160918.1836");

/// The exact ace-jump-mode package selected by the comprehensive API parity
/// corpus.
pub const ACE_JUMP_MODE_MELPA_PIN: (&str, &str) = ("ace-jump-mode", "20140616.815");

/// The exact ace-jump-zap package selected by the comprehensive API parity
/// corpus.
pub const ACE_JUMP_ZAP_MELPA_PIN: (&str, &str) = ("ace-jump-zap", "20170717.1849");

/// The exact ace-link package selected by the comprehensive API parity corpus.
pub const ACE_LINK_MELPA_PIN: (&str, &str) = ("ace-link", "20241101.1344");

/// The exact ace-mc package selected by the comprehensive API parity corpus.
pub const ACE_MC_MELPA_PIN: (&str, &str) = ("ace-mc", "20190206.749");

/// The exact ace-pinyin package selected by the comprehensive API parity
/// corpus.
pub const ACE_PINYIN_MELPA_PIN: (&str, &str) = ("ace-pinyin", "20210827.355");

/// The exact ace-popup-menu package selected by the comprehensive API parity
/// corpus.
pub const ACE_POPUP_MENU_MELPA_PIN: (&str, &str) = ("ace-popup-menu", "20230606.1445");

/// The exact ace-window package selected by the comprehensive API parity
/// corpus.
pub const ACE_WINDOW_MELPA_PIN: (&str, &str) = ("ace-window", "20220911.358");

/// The exact achievements package selected by the comprehensive API parity
/// corpus.
pub const ACHIEVEMENTS_MELPA_PIN: (&str, &str) = ("achievements", "20240703.318");

/// The exact ack-menu package selected by the comprehensive API parity corpus.
pub const ACK_MENU_MELPA_PIN: (&str, &str) = ("ack-menu", "20150504.2022");

/// The exact acme-theme package selected by the comprehensive API parity
/// corpus.
pub const ACME_THEME_MELPA_PIN: (&str, &str) = ("acme-theme", "20210430.302");

/// The exact acp package selected by the comprehensive API parity corpus.
pub const ACP_MELPA_PIN: (&str, &str) = ("acp", "20260719.342");

/// The exact act-mode package selected by the comprehensive API parity corpus.
pub const ACT_MODE_MELPA_PIN: (&str, &str) = ("act-mode", "20240718.39");

/// The exact actionscript-mode package selected by the comprehensive API parity
/// corpus.
pub const ACTIONSCRIPT_MODE_MELPA_PIN: (&str, &str) = ("actionscript-mode", "20180527.1701");

/// The exact activity-watch-mode package selected by the comprehensive API
/// parity corpus.
pub const ACTIVITY_WATCH_MODE_MELPA_PIN: (&str, &str) = ("activity-watch-mode", "20260311.835");

/// The exact acton-mode package selected by the comprehensive API parity corpus.
pub const ACTON_MODE_MELPA_PIN: (&str, &str) = ("acton-mode", "20250113.1059");

/// The exact ada-ts-mode package selected by the comprehensive API parity
/// corpus.
pub const ADA_TS_MODE_MELPA_PIN: (&str, &str) = ("ada-ts-mode", "20260627.1553");

/// The exact adafruit-wisdom package selected by the comprehensive API parity
/// corpus.
pub const ADAFRUIT_WISDOM_MELPA_PIN: (&str, &str) = ("adafruit-wisdom", "20200217.306");

/// The exact add-hooks package selected by the comprehensive API parity corpus.
pub const ADD_HOOKS_MELPA_PIN: (&str, &str) = ("add-hooks", "20171217.123");

/// The exact add-node-modules-path package selected by the comprehensive API
/// parity corpus.
pub const ADD_NODE_MODULES_PATH_MELPA_PIN: (&str, &str) = ("add-node-modules-path", "20230307.655");

/// The exact addressbook-bookmark package selected by the comprehensive API
/// parity corpus.
pub const ADDRESSBOOK_BOOKMARK_MELPA_PIN: (&str, &str) = ("addressbook-bookmark", "20260105.453");

/// The exact ado-mode package selected by the comprehensive API parity corpus.
pub const ADO_MODE_MELPA_PIN: (&str, &str) = ("ado-mode", "20260210.1431");

/// The exact adoc-mode package selected by the comprehensive API parity corpus.
pub const ADOC_MODE_MELPA_PIN: (&str, &str) = ("adoc-mode", "20260612.638");

/// The exact advent-mode package selected by the comprehensive API parity
/// corpus.
pub const ADVENT_MODE_MELPA_PIN: (&str, &str) = ("advent-mode", "20260209.1903");

/// The exact adwaita-dark-theme package selected by the comprehensive API
/// parity corpus.
pub const ADWAITA_DARK_THEME_MELPA_PIN: (&str, &str) = ("adwaita-dark-theme", "20231209.1033");

/// The exact AES package selected by the comprehensive API parity corpus.
pub const AES_MELPA_PIN: (&str, &str) = ("aes", "20211204.2348");

/// The exact Affe package selected by the comprehensive API parity corpus.
pub const AFFE_MELPA_PIN: (&str, &str) = ("affe", "20260519.1026");

/// The exact Afterglow package selected by the comprehensive API parity corpus.
pub const AFTERGLOW_MELPA_PIN: (&str, &str) = ("afterglow", "20240312.953");

/// The exact afternoon-theme package selected by the comprehensive API parity
/// corpus.
pub const AFTERNOON_THEME_MELPA_PIN: (&str, &str) = ("afternoon-theme", "20140104.1859");

/// The exact ag package selected by the comprehensive API parity corpus.
pub const AG_MELPA_PIN: (&str, &str) = ("ag", "20201031.2202");

/// The exact agda-editor-tactics package selected by the comprehensive API
/// parity corpus.
pub const AGDA_EDITOR_TACTICS_MELPA_PIN: (&str, &str) = ("agda-editor-tactics", "20211024.2357");

/// The exact agda-lib-mode package selected by the comprehensive API parity
/// corpus.
pub const AGDA_LIB_MODE_MELPA_PIN: (&str, &str) = ("agda-lib-mode", "20251013.2307");

/// The exact age package selected by the comprehensive API parity corpus.
pub const AGE_MELPA_PIN: (&str, &str) = ("age", "20250806.1723");

/// The exact agenix package selected by the comprehensive API parity corpus.
pub const AGENIX_MELPA_PIN: (&str, &str) = ("agenix", "20250209.551");

/// The exact agent-recall package selected by the comprehensive API parity
/// corpus.
pub const AGENT_RECALL_MELPA_PIN: (&str, &str) = ("agent-recall", "20260710.1707");

/// The exact agent-shell package selected by the comprehensive API parity
/// corpus.
pub const AGENT_SHELL_MELPA_PIN: (&str, &str) = ("agent-shell", "20260728.953");

/// The exact aggressive-fill-paragraph package selected by the comprehensive
/// API parity corpus.
pub const AGGRESSIVE_FILL_PARAGRAPH_MELPA_PIN: (&str, &str) =
    ("aggressive-fill-paragraph", "20240213.2320");

/// The exact aggressive-indent package selected by the comprehensive API
/// parity corpus.
pub const AGGRESSIVE_INDENT_MELPA_PIN: (&str, &str) = ("aggressive-indent", "20230112.1300");

/// The exact agitjo package selected by the comprehensive API parity corpus.
pub const AGITJO_MELPA_PIN: (&str, &str) = ("agitjo", "20260523.2048");

/// The exact agtags package selected by the comprehensive API parity corpus.
pub const AGTAGS_MELPA_PIN: (&str, &str) = ("agtags", "20250523.1654");

/// The exact ah package selected by the comprehensive API parity corpus.
pub const AH_MELPA_PIN: (&str, &str) = ("ah", "20220730.1058");

/// The exact aHg package selected by the comprehensive API parity corpus.
pub const AHG_MELPA_PIN: (&str, &str) = ("ahg", "20241113.748");

/// The exact ahk-mode package selected by the comprehensive API parity corpus.
pub const AHK_MODE_MELPA_PIN: (&str, &str) = ("ahk-mode", "20200412.1832");

/// The exact ahungry-theme package selected by the comprehensive API parity
/// corpus.
pub const AHUNGRY_THEME_MELPA_PIN: (&str, &str) = ("ahungry-theme", "20180131.328");

/// The exact ai-code package selected by the comprehensive API parity corpus.
pub const AI_CODE_MELPA_PIN: (&str, &str) = ("ai-code", "20260727.2322");

/// The exact aider package selected by the comprehensive API parity corpus.
pub const AIDER_MELPA_PIN: (&str, &str) = ("aider", "20251201.133");

/// The exact Aidermacs package selected by the comprehensive API parity corpus.
pub const AIDERMACS_MELPA_PIN: (&str, &str) = ("aidermacs", "20260726.839");

/// The exact aidev-mode package selected by the comprehensive API parity
/// corpus.
pub const AIDEV_MODE_MELPA_PIN: (&str, &str) = ("aidev-mode", "20250318.2144");

/// The exact aiken-mode package selected by the comprehensive API parity
/// corpus.
pub const AIKEN_MODE_MELPA_PIN: (&str, &str) = ("aiken-mode", "20230920.1210");

/// The exact aio package selected by the comprehensive API parity corpus.
pub const AIO_MELPA_PIN: (&str, &str) = ("aio", "20260214.1529");

/// The exact airline-themes package selected by the comprehensive API parity
/// corpus.
pub const AIRLINE_THEMES_MELPA_PIN: (&str, &str) = ("airline-themes", "20250502.1915");

/// The exact airplay package selected by the comprehensive API parity corpus.
pub const AIRPLAY_MELPA_PIN: (&str, &str) = ("airplay", "20130212.1226");

/// The exact alabaster-themes package selected by the comprehensive API parity
/// corpus.
pub const ALABASTER_THEMES_MELPA_PIN: (&str, &str) = ("alabaster-themes", "20260113.657");

/// The exact alan-mode package selected by the comprehensive API parity corpus.
pub const ALAN_MODE_MELPA_PIN: (&str, &str) = ("alan-mode", "20260523.1330");

/// The exact alarm-clock package selected by the comprehensive API parity
/// corpus.
pub const ALARM_CLOCK_MELPA_PIN: (&str, &str) = ("alarm-clock", "20250123.556");

/// The exact Alchemist package selected by the comprehensive API parity corpus.
pub const ALCHEMIST_MELPA_PIN: (&str, &str) = ("alchemist", "20180312.1304");

/// The exact alda-mode package selected by the comprehensive API parity corpus.
pub const ALDA_MODE_MELPA_PIN: (&str, &str) = ("alda-mode", "20251223.6");

/// The exact alect-themes package selected by the comprehensive API parity
/// corpus.
pub const ALECT_THEMES_MELPA_PIN: (&str, &str) = ("alect-themes", "20251205.1503");

/// The exact Alectryon package selected by the comprehensive API parity
/// corpus.
pub const ALECTRYON_MELPA_PIN: (&str, &str) = ("alectryon", "20260525.2000");

/// The exact alert package selected by the comprehensive API parity corpus.
pub const ALERT_MELPA_PIN: (&str, &str) = ("alert", "20260316.2025");

/// The exact alert-termux package selected by the comprehensive API parity
/// corpus.
pub const ALERT_TERMUX_MELPA_PIN: (&str, &str) = ("alert-termux", "20181119.951");

/// The exact alert-toast package selected by the comprehensive API parity
/// corpus.
pub const ALERT_TOAST_MELPA_PIN: (&str, &str) = ("alert-toast", "20220312.229");

/// The exact align-cljlet package selected by the comprehensive API parity
/// corpus.
pub const ALIGN_CLJLET_MELPA_PIN: (&str, &str) = ("align-cljlet", "20160112.2101");

/// The exact all-ext package selected by the comprehensive API parity corpus.
/// MELPA built this archive from upstream commit
/// `c865c62506af2c9edc7705a7c24dc8b70d5d4de2`.
pub const ALL_EXT_MELPA_PIN: (&str, &str) = ("all-ext", "20200315.1443");

/// The exact all-the-icons package selected by the comprehensive API parity
/// corpus.
pub const ALL_THE_ICONS_MELPA_PIN: (&str, &str) = ("all-the-icons", "20250527.927");

/// The exact all-the-icons-completion package selected by the comprehensive
/// API parity corpus.
pub const ALL_THE_ICONS_COMPLETION_MELPA_PIN: (&str, &str) =
    ("all-the-icons-completion", "20240128.2048");

/// The exact all-the-icons-dired package selected by the comprehensive API
/// parity corpus.
pub const ALL_THE_ICONS_DIRED_MELPA_PIN: (&str, &str) = ("all-the-icons-dired", "20231207.1324");

/// The exact all-the-icons-gnus package selected by the comprehensive API
/// parity corpus.
pub const ALL_THE_ICONS_GNUS_MELPA_PIN: (&str, &str) = ("all-the-icons-gnus", "20180511.654");

/// The exact all-the-icons-ibuffer package selected by the comprehensive API
/// parity corpus.
pub const ALL_THE_ICONS_IBUFFER_MELPA_PIN: (&str, &str) =
    ("all-the-icons-ibuffer", "20230503.1625");

/// The exact all-the-icons-ivy package selected by the comprehensive API
/// parity corpus.
pub const ALL_THE_ICONS_IVY_MELPA_PIN: (&str, &str) = ("all-the-icons-ivy", "20190508.1803");

/// The exact all-the-icons-ivy-rich package selected by the comprehensive API
/// parity corpus. MELPA built this archive from upstream commit
/// `c098cc85123a401b0ab8f2afd3a25853e61d7d28`.
pub const ALL_THE_ICONS_IVY_RICH_MELPA_PIN: (&str, &str) =
    ("all-the-icons-ivy-rich", "20230420.1234");

/// The exact all-the-icons-nerd-fonts package selected by the comprehensive
/// API parity corpus.
pub const ALL_THE_ICONS_NERD_FONTS_MELPA_PIN: (&str, &str) =
    ("all-the-icons-nerd-fonts", "20260614.1246");

/// The exact almost-mono-themes package selected by the comprehensive API
/// parity corpus.
pub const ALMOST_MONO_THEMES_MELPA_PIN: (&str, &str) = ("almost-mono-themes", "20250722.1957");

/// The exact alsamixer package selected by the comprehensive API parity
/// corpus.
pub const ALSAMIXER_MELPA_PIN: (&str, &str) = ("alsamixer", "20250106.1025");

/// The exact alt-codes package selected by the comprehensive API parity
/// corpus.
pub const ALT_CODES_MELPA_PIN: (&str, &str) = ("alt-codes", "20260101.557");

/// The exact Amaranth Dark theme selected by the comprehensive API parity
/// corpus. MELPA built this archive from upstream commit
/// `624e0b5ef632b3adfdc03e44dce7a98cd48d47ed`.
pub const AMARANTH_DARK_THEME_MELPA_PIN: (&str, &str) = ("amaranth-dark-theme", "20251228.1916");

/// The exact amber-glow-theme package selected by the comprehensive API
/// parity corpus.
pub const AMBER_GLOW_THEME_MELPA_PIN: (&str, &str) = ("amber-glow-theme", "20250305.936");

/// The exact amd-mode package selected by the comprehensive API parity corpus.
pub const AMD_MODE_MELPA_PIN: (&str, &str) = ("amd-mode", "20180111.1402");

/// The exact Ameba package selected by the comprehensive API parity corpus.
/// MELPA built this archive from upstream commit
/// `0c4925ae0e998818326adcb47ed27ddf9761c7dc`.
pub const AMEBA_MELPA_PIN: (&str, &str) = ("ameba", "20200103.1454");

/// The exact ample-regexps package selected by the comprehensive API parity
/// corpus.
pub const AMPLE_REGEXPS_MELPA_PIN: (&str, &str) = ("ample-regexps", "20200508.1021");

/// The exact ample-theme package selected by the comprehensive API parity
/// corpus.
pub const AMPLE_THEME_MELPA_PIN: (&str, &str) = ("ample-theme", "20260611.1532");

/// The exact ample-zen-theme package selected by the comprehensive API parity
/// corpus.
pub const AMPLE_ZEN_THEME_MELPA_PIN: (&str, &str) = ("ample-zen-theme", "20150119.2154");

/// The exact amread-mode package selected by the comprehensive API parity
/// corpus. MELPA built this archive from upstream commit
/// `bf06b05c6322fe74f0e5ac2436cad46f66f673c6`.
pub const AMREAD_MODE_MELPA_PIN: (&str, &str) = ("amread-mode", "20240903.1534");

/// The exact amsreftex package selected by the comprehensive API parity corpus.
pub const AMSREFTEX_MELPA_PIN: (&str, &str) = ("amsreftex", "20240512.1746");

/// The exact amx package selected by the comprehensive API parity corpus.
pub const AMX_MELPA_PIN: (&str, &str) = ("amx", "20230413.1210");

/// The exact anaconda-mode package selected by the comprehensive API parity
/// corpus.
pub const ANACONDA_MODE_MELPA_PIN: (&str, &str) = ("anaconda-mode", "20250430.227");

/// The exact anakondo package selected by the comprehensive API parity corpus.
/// MELPA built this archive from upstream commit
/// `16b0ba14d94a5d7e55655efc9e1d6d069a9306f2`.
pub const ANAKONDO_MELPA_PIN: (&str, &str) = ("anakondo", "20210221.1727");

/// The exact anaphora package selected by the comprehensive API parity corpus.
pub const ANAPHORA_MELPA_PIN: (&str, &str) = ("anaphora", "20260720.903");

/// The exact ancient-one-dark-theme package selected by the comprehensive
/// theme parity corpus.
pub const ANCIENT_ONE_DARK_THEME_MELPA_PIN: (&str, &str) =
    ("ancient-one-dark-theme", "20211030.1358");

/// The exact ancient-theme package selected by the comprehensive API parity
/// corpus.
pub const ANCIENT_THEME_MELPA_PIN: (&str, &str) = ("ancient-theme", "20260322.1856");

/// The exact android-env package selected by the comprehensive API parity
/// corpus.
pub const ANDROID_ENV_MELPA_PIN: (&str, &str) = ("android-env", "20220810.1449");

/// The exact android-mode package selected by the comprehensive API parity
/// corpus. MELPA built this archive from upstream commit
/// `67f7c0d7d37605efc7f055b76d731556861c3eb9`.
pub const ANDROID_MODE_MELPA_PIN: (&str, &str) = ("android-mode", "20250106.1022");

/// The exact angry-police-captain package selected by the comprehensive API
/// parity corpus.
pub const ANGRY_POLICE_CAPTAIN_MELPA_PIN: (&str, &str) = ("angry-police-captain", "20120829.1252");

/// The exact angular-mode package selected by the comprehensive API parity
/// corpus.
pub const ANGULAR_MODE_MELPA_PIN: (&str, &str) = ("angular-mode", "20151201.2127");

/// The exact angular-snippets package selected by the comprehensive API
/// parity corpus.
pub const ANGULAR_SNIPPETS_MELPA_PIN: (&str, &str) = ("angular-snippets", "20140514.523");

/// The exact Anju package selected by the comprehensive mouse UI parity
/// corpus.
pub const ANJU_MELPA_PIN: (&str, &str) = ("anju", "20260701.2139");

/// The exact anki-connect package selected by the comprehensive API parity
/// corpus. MELPA built this archive from upstream commit
/// `e32e611d54a3819f88c5ff58009df70c9ae01934`.
pub const ANKI_CONNECT_MELPA_PIN: (&str, &str) = ("anki-connect", "20250414.1301");

/// The exact anki-editor package selected by the comprehensive API parity
/// corpus. MELPA built this archive from upstream commit
/// `4a55c3f937b176d31e36d484c196682cae9f9104`.
pub const ANKI_EDITOR_MELPA_PIN: (&str, &str) = ("anki-editor", "20260714.1156");

/// The exact anki-editor-view package selected by the comprehensive API parity
/// corpus.
pub const ANKI_EDITOR_VIEW_MELPA_PIN: (&str, &str) = ("anki-editor-view", "20230807.806");

/// The exact anki-mode package selected by the comprehensive API parity corpus.
pub const ANKI_MODE_MELPA_PIN: (&str, &str) = ("anki-mode", "20201223.719");

/// The exact anki-vocabulary package selected by the comprehensive API parity
/// corpus.
pub const ANKI_VOCABULARY_MELPA_PIN: (&str, &str) = ("anki-vocabulary", "20200103.325");

/// The exact Annalist package selected by the comprehensive recording and Org
/// rendering parity corpus.
pub const ANNALIST_MELPA_PIN: (&str, &str) = ("annalist", "20260531.1558");

/// The exact Annotate package selected by the comprehensive API parity corpus.
pub const ANNOTATE_MELPA_PIN: (&str, &str) = ("annotate", "20260514.1320");

/// The exact annotate-depth package selected by the comprehensive API parity
/// corpus.
pub const ANNOTATE_DEPTH_MELPA_PIN: (&str, &str) = ("annotate-depth", "20160520.2040");

/// The exact annotation package selected by the comprehensive API parity
/// corpus. MELPA built this archive from upstream commit
/// `213db6e50bb89c1b0b2832eab4c6caafb137eb6d`.
pub const ANNOTATION_MELPA_PIN: (&str, &str) = ("annotation", "20250805.1029");

/// The exact annoying-arrows-mode package selected by the comprehensive API
/// parity corpus.
pub const ANNOYING_ARROWS_MODE_MELPA_PIN: (&str, &str) = ("annoying-arrows-mode", "20161024.646");

/// The exact ansi package selected by the practical terminal-rendering parity
/// corpus. MELPA built this archive from upstream commit
/// `a3aa9daa37a75fec22186399014a790a6c554311`.
pub const ANSI_MELPA_PIN: (&str, &str) = ("ansi", "20251118.230");

/// The exact Ansible package selected by the comprehensive playbook editing
/// and vault workflow parity corpus.
pub const ANSIBLE_MELPA_PIN: (&str, &str) = ("ansible", "20260607.1852");

/// The exact ansible-doc package selected by the comprehensive documentation
/// workflow parity corpus.
pub const ANSIBLE_DOC_MELPA_PIN: (&str, &str) = ("ansible-doc", "20160924.824");

/// The exact ansible-vault package selected by the comprehensive API parity
/// corpus. MELPA built this archive from upstream commit
/// `74f96ce226f51bec203af343f73182ea132749a6`.
pub const ANSIBLE_VAULT_MELPA_PIN: (&str, &str) = ("ansible-vault", "20251029.2146");

/// The exact Ansilove package selected by the practical ANSI-art conversion
/// and viewing parity corpus. MELPA built this archive from upstream commit
/// `a75eb6c89a1d96e1b4fa028ecca9be8b13c95230`.
pub const ANSILOVE_MELPA_PIN: (&str, &str) = ("ansilove", "20250105.1853");

/// The exact ant package selected by the comprehensive build-workflow parity
/// corpus.
pub const ANT_MELPA_PIN: (&str, &str) = ("ant", "20160211.1543");

/// The exact Anti-Zenburn Theme package selected by the practical editing,
/// review, and build-output parity corpus. MELPA built this archive from
/// upstream commit `dbafbaa86be67c1d409873f57a5c0bbe1e7ca158`.
pub const ANTI_ZENBURN_THEME_MELPA_PIN: (&str, &str) = ("anti-zenburn-theme", "20180712.1838");

/// The exact anx-api package selected by the comprehensive API workflow parity
/// corpus.
pub const ANX_API_MELPA_PIN: (&str, &str) = ("anx-api", "20140208.1514");

/// The exact AnyBar package selected by the practical indicator lifecycle,
/// custom-image, and multi-instance parity corpus. MELPA built this archive
/// from upstream commit
/// `7a0743e0d31bcb36ab1bb2e351f3e7139c422ac5`.
pub const ANYBAR_MELPA_PIN: (&str, &str) = ("anybar", "20160816.1421");

/// The exact Anyins package selected by the comprehensive API parity corpus.
pub const ANYINS_MELPA_PIN: (&str, &str) = ("anyins", "20131229.1041");

/// The exact Anzu package selected by the practical incremental-search,
/// scoped-rename, selective-replacement, and global-mode parity corpus. MELPA
/// built this archive from upstream commit
/// `bc3a0032bb6aa7f5886f10460cd53eb7b8b020af`.
pub const ANZU_MELPA_PIN: (&str, &str) = ("anzu", "20240929.201");

/// The exact aozora-view package selected by the practical reading, bookmark,
/// redraw, and cache-resume parity corpus. MELPA built this archive from
/// upstream commit
/// `b0390616d19e45f15f9a2f5d5688274831e721fd`.
pub const AOZORA_VIEW_MELPA_PIN: (&str, &str) = ("aozora-view", "20140310.1317");

/// The exact Apache Mode package selected by the comprehensive editing parity
/// corpus.
pub const APACHE_MODE_MELPA_PIN: (&str, &str) = ("apache-mode", "20210519.1931");

/// The exact APDL Mode package selected by the practical authoring, inspection,
/// help, solver, artifact, and license-operation parity corpus. MELPA built
/// this archive from upstream commit
/// `4883ab085811b85cc75c44b5af478ab8f7e98386`.
pub const APDL_MODE_MELPA_PIN: (&str, &str) = ("apdl-mode", "20250508.908");

/// The exact APEL package selected by the practical legacy-package,
/// message-routing, product, MIME, richtext, filesystem, and CCL parity
/// corpus. MELPA built this archive from upstream commit
/// `1b043cfea58ea146356c237a5286ead69e97417b`.
pub const APEL_MELPA_PIN: (&str, &str) = ("apel", "20250608.1806");

/// The exact Apheleia package selected by the practical formatter, point
/// preservation, project configuration, save-mode, concurrency, and
/// diagnostic parity corpus. MELPA built this archive from upstream commit
/// `14a0bb4454fb2cc3b5b377619288b742ce117da5`.
pub const APHELEIA_MELPA_PIN: (&str, &str) = ("apheleia", "20260619.1935");

/// The exact APIB Mode package selected by the comprehensive API parity corpus.
pub const APIB_MODE_MELPA_PIN: (&str, &str) = ("apib-mode", "20200101.1017");

/// The exact apiwrap package selected by the practical generated-client
/// lifecycle, policy, error-recovery, and discovery parity corpus. MELPA built
/// this archive from upstream commit
/// `e4c9c57d6620a788ec8a715ff1bb50542edea3a6`.
pub const APIWRAP_MELPA_PIN: (&str, &str) = ("apiwrap", "20180602.2231");

/// The exact app-monochrome-themes package selected by the practical code,
/// writing, Dired, and theme-lifecycle parity corpus. MELPA built this archive
/// from upstream commit
/// `bd8bfee0b64bf10543f4cefaf40bb5dcd4cf123b`.
pub const APP_MONOCHROME_THEMES_MELPA_PIN: (&str, &str) =
    ("app-monochrome-themes", "20250710.2315");

/// The exact apparmor-mode package selected by the practical policy authoring,
/// fontification, completion, and live-diagnostics parity corpus. MELPA built
/// this archive from upstream commit
/// `b0e4bbcd30aafd71f484c74164351af40ef885bf`.
pub const APPARMOR_MODE_MELPA_PIN: (&str, &str) = ("apparmor-mode", "20260515.454");

/// The exact Apple Container TRAMP package selected by the practical
/// interactive completion, optional-user remote editing, and cleanup lifecycle
/// parity corpus. MELPA built this archive from upstream commit
/// `f47d58d029c594f4c9e9b1cfff79630de68a9cb5`.
pub const APPLE_CONTAINER_TRAMP_MELPA_PIN: (&str, &str) =
    ("apple-container-tramp", "20260504.1350");

/// The exact apples-mode package exercised by practical authoring, installed
/// snippet, execution, toolchain, error-recovery, and scratch persistence
/// workflows. MELPA built this archive from upstream commit
/// `83a9ab0d6ba82496e2f7df386909b1a55701fccb`.
pub const APPLES_MODE_MELPA_PIN: (&str, &str) = ("apples-mode", "20110121.418");

/// The exact AppleScript Mode package selected by the practical authoring,
/// outline-navigation, file-saving, macOS execution-boundary, failure-state,
/// one-off command, and structured-result parity corpus. MELPA built this
/// archive from upstream commit
/// `00c141bbff46c89a96598b605dee05dd1d89f624`.
pub const APPLESCRIPT_MODE_MELPA_PIN: (&str, &str) = ("applescript-mode", "20210802.1715");

/// The exact apropospriate-theme package selected by the practical code,
/// diff, Org, ANSI output, customization, and dark/light lifecycle parity
/// corpus. MELPA built this archive from upstream commit
/// `2b26eed7e2063ca93998a6807f5a4e602483a23d`.
pub const APROPOSPRIATE_THEME_MELPA_PIN: (&str, &str) = ("apropospriate-theme", "20251010.121");

/// The exact apt-sources-list package selected by the practical repository
/// authoring, interactive editing, suite migration, navigation, validation,
/// fontification, and file-persistence parity corpus. MELPA built this archive
/// from upstream commit `44112833b3fa7f4d7e43708e5996782e22bb2fa3`.
pub const APT_SOURCES_LIST_MELPA_PIN: (&str, &str) = ("apt-sources-list", "20180527.1241");

/// The exact AQI package selected by the comprehensive data, cache, request,
/// and reporting parity corpus.
pub const AQI_MELPA_PIN: (&str, &str) = ("aqi", "20230530.1204");

/// The exact arch-packer package selected by the practical package listing,
/// detail, repository refresh, search, install, marking, upgrade, and removal
/// parity corpus. MELPA built this archive from upstream commit
/// `940e96f7d357c6570b675a0f942181c787f1bfd7`.
pub const ARCH_PACKER_MELPA_PIN: (&str, &str) = ("arch-packer", "20170730.1321");

/// The exact arduino-mode package selected by the comprehensive API parity
/// corpus. MELPA built this archive from upstream commit
/// `b2ffd8441851659cb1cc844156073967729585e5`.
pub const ARDUINO_MODE_MELPA_PIN: (&str, &str) = ("arduino-mode", "20240527.1603");

/// The exact Flycheck package selected by its direct diagnostics parity
/// corpus and by arduino-mode's optional integration coverage.
pub const FLYCHECK_MELPA_PIN: (&str, &str) = ("flycheck", "20260728.931");

/// The exact flycheck-dmd-dub package selected by the practical DUB project
/// discovery, metadata, subprocess, cache, and buffer-local flag parity corpus.
/// MELPA built this archive from upstream commit
/// c1bf54b7eca8951a38ce9f6ae12e07a011f03eb5.
pub const FLYCHECK_DMD_DUB_MELPA_PIN: (&str, &str) = ("flycheck-dmd-dub", "20250304.1432");

/// The exact Geiser package selected by the practical Scheme editing,
/// implementation, completion, evaluation-protocol, and source-navigation
/// parity corpus. MELPA built this archive from upstream commit
/// 3e506d06b34ccda8a50ac3e43c90d722c00065fe.
pub const GEISER_MELPA_PIN: (&str, &str) = ("geiser", "20260718.8");

/// The exact gntp package selected by the practical Growl registration,
/// notification-wire, file-icon, network-send, and reply-handling parity
/// corpus. MELPA built this archive from upstream commit
/// 767571135e2c0985944017dc59b0be79af222ef5.
pub const GNTP_MELPA_PIN: (&str, &str) = ("gntp", "20141025.250");

/// The exact haskell-mode package selected by the practical source-editing,
/// fontification, declaration indexing, import formatting, layout indentation,
/// navigation, folding, and SCC annotation parity corpus. MELPA built this
/// archive from upstream commit 2dd755a5fa11577a9388af88f385d2a8e18f7a8d.
pub const HASKELL_MODE_MELPA_PIN: (&str, &str) = ("haskell-mode", "20260206.1050");

/// The exact arscript-mode package selected by the comprehensive mode,
/// font-lock, indentation, and editing parity corpus. MELPA built this archive
/// from upstream commit `797e1d0ef1312e8ff846abd0c6853358041f7691`.
pub const ARSCRIPT_MODE_MELPA_PIN: (&str, &str) = ("arscript-mode", "20240819.1927");

/// The exact arxiv-citation package selected by the comprehensive parsing,
/// citation, dependency, editing, and download-workflow parity corpus. MELPA
/// built this archive from upstream commit
/// `04de0dae1121fb92c30b393449c6f8d6d940dbed`.
pub const ARXIV_CITATION_MELPA_PIN: (&str, &str) = ("arxiv-citation", "20230713.627");

/// The exact arxiv-mode package selected by the comprehensive query, rendering,
/// bibliography, navigation, and command-workflow parity corpus. MELPA built
/// this archive from upstream commit
/// `f629ec64f8bbac0cadb472c6741f8f33d49e9160`.
pub const ARXIV_MODE_MELPA_PIN: (&str, &str) = ("arxiv-mode", "20240111.2203");

/// The exact asciidoc-mode package selected by the comprehensive Tree-sitter,
/// editing, navigation, completion, and diagnostics parity corpus. MELPA built
/// this archive from upstream commit
/// `8914fad451f9c7f9c2286cf18db5edaa51a92cd7`.
pub const ASCIIDOC_MODE_MELPA_PIN: (&str, &str) = ("asciidoc-mode", "20260612.645");

/// The exact asdf-vm package selected by the comprehensive tool-version,
/// process, environment, installer, plugin, and menu workflow parity corpus.
/// MELPA built this archive from upstream commit
/// `f6dbb4b6560cd7e5bb05006e9fc416c5c323b567`.
pub const ASDF_VM_MELPA_PIN: (&str, &str) = ("asdf-vm", "20250710.1053");

/// The exact ast-grep package selected by the comprehensive command, stream,
/// rewrite, completion-backend, and outline workflow parity corpus. MELPA
/// built this archive from upstream commit
/// `28bc6e9ac21acf1d1ef58b962b6acd670c27e80f`.
pub const AST_GREP_MELPA_PIN: (&str, &str) = ("ast-grep", "20260702.238");

/// The exact archive-phar package selected by the comprehensive archive
/// browsing and extraction parity corpus.
pub const ARCHIVE_PHAR_MELPA_PIN: (&str, &str) = ("archive-phar", "20221009.2129");

/// The exact Archive Region package selected by the comprehensive editing and
/// filesystem workflow parity corpus.
pub const ARCHIVE_REGION_MELPA_PIN: (&str, &str) = ("archive-region", "20200316.1425");

/// The exact archive-rpm package selected by the practical archive browsing,
/// extraction, metadata, compression, and binary-fidelity workflow parity
/// corpus. MELPA built this archive from upstream commit
/// `cb48fee04cb0cbb26f760a3b95649f7dac78c6ec`.
pub const ARCHIVE_RPM_MELPA_PIN: (&str, &str) = ("archive-rpm", "20220527.632");

/// The exact arduino-cli-mode package selected by the practical sketch,
/// compilation, upload, dependency, menu, and serial-monitor workflow parity
/// corpus. MELPA built this archive from upstream commit
/// `d5614acdca80871cf4db65843227223b5a0e3a2c`.
pub const ARDUINO_CLI_MODE_MELPA_PIN: (&str, &str) = ("arduino-cli-mode", "20260628.2219");

/// The exact aria2 package selected by the practical downloads-dashboard,
/// transfer-control, URL-dialog, and torrent-import workflow parity corpus.
/// MELPA built this archive from upstream commit
/// `1f2cbe624f3a4e0109b5dc123bb4bbed496b15a7`.
pub const ARIA2_MELPA_PIN: (&str, &str) = ("aria2", "20230314.2131");

/// The exact Arjen Grey Theme package selected by the practical editor,
/// installed-loading, Helm selection, stacking, and restoration workflow
/// parity corpus. MELPA built this archive from upstream commit
/// `4cd0be72b65d42390e2105cfdaa408a1ead8d8d1`.
pub const ARJEN_GREY_THEME_MELPA_PIN: (&str, &str) = ("arjen-grey-theme", "20170522.2047");

/// The exact Ariadne package selected by the practical key-bound definition,
/// live BERT-RPC stream, navigation, reply, and offline workflow parity
/// corpus. MELPA built this archive from upstream commit
/// `6fe401c7f996bcbc2f685e7971324c6f5e5eaf15`.
pub const ARIADNE_MELPA_PIN: (&str, &str) = ("ariadne", "20131117.1711");

/// The exact Art Bollocks Mode package selected by the practical documented
/// text-editing, comment/docstring review, customized editorial-policy, and
/// readability-metrics workflow parity corpus. MELPA built this archive from
/// upstream commit `63d20ed2846226f45b35eded69a776143a772ea4`.
pub const ARTBOLLOCKS_MODE_MELPA_PIN: (&str, &str) = ("artbollocks-mode", "20251211.1624");

/// The exact arview package selected by the comprehensive archive detection,
/// extraction, Dired lifecycle, process, and cleanup parity corpus.
pub const ARVIEW_MELPA_PIN: (&str, &str) = ("arview", "20160419.2109");

/// The exact ASCII Table package selected by the comprehensive formatting,
/// rendering, navigation, and command-workflow parity corpus.
pub const ASCII_TABLE_MELPA_PIN: (&str, &str) = ("ascii-table", "20231215.1527");

/// The exact Asilea package selected by the comprehensive annealing,
/// compiler-option, process, and callback parity corpus.
pub const ASILEA_MELPA_PIN: (&str, &str) = ("asilea", "20150105.1525");

/// The exact asm-blox package selected by the comprehensive parser, virtual
/// machine, gameboard, editor, persistence, and puzzle parity corpus.
pub const ASM_BLOX_MELPA_PIN: (&str, &str) = ("asm-blox", "20240106.1930");

/// The exact asn1-mode package selected by the comprehensive lexical,
/// indentation, font-lock, outline, and editing-workflow parity corpus. MELPA
/// built this archive from upstream commit
/// `d5d4a8259daf708411699bcea85d322f18beb972`.
pub const ASN1_MODE_MELPA_PIN: (&str, &str) = ("asn1-mode", "20170729.226");

/// The exact Assess package selected by the comprehensive buffer, filesystem,
/// indentation, fontification, discovery, and call-capture parity corpus.
/// MELPA built this archive from upstream commit
/// `cadeb24a5d8261fad4bdfdc09e7d571cc395a6ca`.
pub const ASSESS_MELPA_PIN: (&str, &str) = ("assess", "20240303.1454");

/// The exact astro-ts-mode package selected by the comprehensive mixed-language
/// Tree-sitter, indentation, font-lock, and editing-workflow parity corpus.
/// MELPA built this archive from upstream commit
/// `1d24c9d399dee4cfea6ed9b49d8e08891665e16c`.
pub const ASTRO_TS_MODE_MELPA_PIN: (&str, &str) = ("astro-ts-mode", "20260417.101");

/// The exact Astute package selected by the comprehensive typography,
/// font-lock, customization, and minor-mode lifecycle parity corpus.
pub const ASTUTE_MELPA_PIN: (&str, &str) = ("astute", "20241015.444");

/// The exact Astyle package selected by the comprehensive argument selection,
/// formatter-command, region, buffer, failure, and on-save parity corpus.
/// MELPA built this archive from upstream commit
/// `04ff2941f08c4b731fe6a18ee1697436d1ca1cc0`.
pub const ASTYLE_MELPA_PIN: (&str, &str) = ("astyle", "20200328.616");

/// The exact ASX package selected by the comprehensive search, DOM,
/// request, navigation, and Org rendering parity corpus.
pub const ASX_MELPA_PIN: (&str, &str) = ("asx", "20191024.1100");

/// The exact async-await package selected by the comprehensive Promise,
/// generator, macro-expansion, and asynchronous-workflow parity corpus. MELPA
/// built this archive from upstream commit
/// `e0d15e8057ed7520100bc50c5552278292ebcb07`.
pub const ASYNC_AWAIT_MELPA_PIN: (&str, &str) = ("async-await", "20220827.437");

/// The exact async-backup package selected by the comprehensive path,
/// predicate, process, save-hook, and backup-lifecycle parity corpus. MELPA
/// built this archive from upstream commit
/// `d07a7bd4a5c3332a8a585680d67925385c595927`.
pub const ASYNC_BACKUP_MELPA_PIN: (&str, &str) = ("async-backup", "20230412.1534");

/// The exact async-http-queue package selected by the comprehensive state,
/// scheduling, response, callback, and lifecycle parity corpus. MELPA built
/// this archive from upstream commit
/// `bd37342372a0b24ce0d54e9dad8070af997b0a0b`.
pub const ASYNC_HTTP_QUEUE_MELPA_PIN: (&str, &str) = ("async-http-queue", "20260316.755");

/// The exact async-job-queue package selected by the comprehensive fixed-slot
/// dispatch, FIFO, callback, saturation, and lifecycle parity corpus.
/// MELPA built this archive from upstream commit
/// `eeafcce7f960305666b2a51aec55cc6333f6af1b`.
pub const ASYNC_JOB_QUEUE_MELPA_PIN: (&str, &str) = ("async-job-queue", "20230427.2122");

/// The exact async-status package selected by the comprehensive filesystem,
/// indicator-item, rendering, and progress-lifecycle parity corpus. MELPA
/// built this archive from upstream commit
/// `d2f5becc9850c26aa71fb581f9fc389eac740f52`.
pub const ASYNC_STATUS_MELPA_PIN: (&str, &str) = ("async-status", "20230821.204");

/// The exact atcoder-tools package selected by the comprehensive run
/// configuration, command construction, metadata, filesystem, and contest
/// workflow parity corpus. MELPA built this archive from upstream commit
/// `cfe61ed18ea9b3b1bfb6f9e7d80a47599680cd1f`.
pub const ATCODER_TOOLS_MELPA_PIN: (&str, &str) = ("atcoder-tools", "20200109.1236");

/// The exact attrap package selected by the comprehensive option, diagnostic
/// dispatch, Elisp, GHC, HLint, LaTeX, and repair-workflow parity corpus.
/// MELPA built this archive from upstream commit
/// `ad1d9443fcd93e32f2aefadc5af2646701664581`.
pub const ATTRAP_MELPA_PIN: (&str, &str) = ("attrap", "20260304.1504");

/// The exact atl-long-lines package selected by the comprehensive mode,
/// line-measurement, timer, toggle, and end-to-end workflow parity corpus.
/// MELPA built this archive from upstream commit
/// `82cdd4edefba2d5b1d491bf3fcc487385819d713`.
pub const ATL_LONG_LINES_MELPA_PIN: (&str, &str) = ("atl-long-lines", "20240101.929");

/// The exact atl-markup package selected by the comprehensive cursor
/// classification, truncation, timer, and minor-mode workflow parity corpus.
/// MELPA built this archive from upstream commit
/// `b616343ffe17060d521b214b8e90f5da1e880934`.
pub const ATL_MARKUP_MELPA_PIN: (&str, &str) = ("atl-markup", "20240101.933");

/// The exact atomic-chrome package selected by the comprehensive websocket,
/// browser-buffer, HTTP protocol, process, and server-lifecycle parity corpus.
/// MELPA built this archive from upstream commit
/// `f1b077be7e414f457191d72dcf5eedb4371f9309`.
pub const ATOMIC_CHROME_MELPA_PIN: (&str, &str) = ("atomic-chrome", "20230304.112");

/// The exact auth-source-gopass package selected by the comprehensive path,
/// process, backend-registration, cache, and credential-workflow parity
/// corpus. MELPA built this archive from upstream commit
/// `6f7f0cc0d682f66d11f7fac4fa5c1e79904232da`.
pub const AUTH_SOURCE_GOPASS_MELPA_PIN: (&str, &str) = ("auth-source-gopass", "20230109.1213");

/// The exact auth-source-xoauth2 package selected by the comprehensive token,
/// credential-provider, transport, protocol, and auth-source-workflow parity
/// corpus. MELPA built this archive from upstream commit
/// `99a03f8ce835412943d311b2746e77fcf5a1b500`.
pub const AUTH_SOURCE_XOAUTH2_MELPA_PIN: (&str, &str) = ("auth-source-xoauth2", "20220804.2219");

/// The exact aurel package selected by the comprehensive AUR URL,
/// parsing, filtering, package-management, and UI workflow parity corpus.
/// MELPA built this archive from upstream commit
/// `c571cc44ea3b9aa96399056bff22919efffbbb06`.
pub const AUREL_MELPA_PIN: (&str, &str) = ("aurel", "20260429.458");

/// The exact audacious package selected by the comprehensive command,
/// playlist, song-selection, metadata, and end-to-end playback parity corpus.
/// MELPA built this archive from upstream commit
/// `65c37f12a5c774a0ae434beee27ff7737006dd2f`.
pub const AUDACIOUS_MELPA_PIN: (&str, &str) = ("audacious", "20210917.51");

/// The exact aurora-config-mode package selected by the comprehensive
/// metadata, prompting, command, Python-derived mode, font-lock, and practical
/// configuration-workflow parity corpus. MELPA built this archive from
/// upstream commit `8273ec7937a21b469b9dbb6c11714255b890f410`.
pub const AURORA_CONFIG_MODE_MELPA_PIN: (&str, &str) = ("aurora-config-mode", "20180216.2302");

/// The exact auth-source-1password package selected by the comprehensive
/// metadata, secret-reference, CLI, backend, cache, and end-to-end auth-source
/// parity corpus. MELPA built this archive from upstream commit
/// `10961bdc8a3ed551dde29fde416843058bea2374`.
pub const AUTH_SOURCE_1PASSWORD_MELPA_PIN: (&str, &str) =
    ("auth-source-1password", "20260221.2058");

/// The exact auth-source-keytar package selected by the comprehensive
/// credential lookup, parsing, backend registration, cache, and auth-source
/// workflow parity corpus. MELPA built this archive from upstream commit
/// `ae32dd807aa3cff59e4384ce8c9d7de259e45998`.
pub const AUTH_SOURCE_KEYTAR_MELPA_PIN: (&str, &str) = ("auth-source-keytar", "20251231.1726");

/// The exact auto-auto-indent package selected by the comprehensive
/// indentation, editing-command, post-command, timer, and practical typing
/// workflow parity corpus. MELPA built this archive from upstream commit
/// `0139378577f936d34b20276af6f022fb457af490`.
pub const AUTO_AUTO_INDENT_MELPA_PIN: (&str, &str) = ("auto-auto-indent", "20131106.1903");

/// The exact es-lib package selected as auto-auto-indent's runtime utility
/// dependency. MELPA built this archive from upstream commit
/// `753b27363e39c10edc9e4e452bdbbbe4d190df4a`.
pub const ES_LIB_MELPA_PIN: (&str, &str) = ("es-lib", "20141111.1830");

/// The exact Keytar package selected as auth-source-keytar's runtime
/// credential-provider dependency and by the practical credential lifecycle,
/// shell-quoting, executable discovery, and npm installation parity corpus.
/// MELPA built this archive from upstream commit
/// `f0485df065bcdc8f446be3e00aa77a43629ec84e`.
pub const KEYTAR_MELPA_PIN: (&str, &str) = ("keytar", "20251231.1727");

/// The exact Llama package selected by the practical data-pipeline, callback,
/// closure, macro-contract, completion, and fontification parity corpus.
/// MELPA built this archive from upstream commit
/// `4d4024048053b898a01521046e0f063ee47615b0`.
pub const LLAMA_MELPA_PIN: (&str, &str) = ("llama", "20260601.1455");

/// The exact auto-async-byte-compile package selected by the comprehensive
/// metadata, save-hook, asynchronous process, status, display, and real
/// byte-compilation lifecycle parity corpus. MELPA built this archive from
/// upstream commit `8681e74ddb8481789c5dbb3cafabb327db4c4484`.
pub const AUTO_ASYNC_BYTE_COMPILE_MELPA_PIN: (&str, &str) =
    ("auto-async-byte-compile", "20160916.454");

/// The exact auto-compile package selected by the comprehensive source
/// recognition, byte-compilation, mode-line, save/load advice, recursive
/// toggle, and failure-recovery parity corpus. MELPA built this archive from
/// upstream commit `4db3a0e497feecc8b3dbeeefacdf363ae60a6392`.
pub const AUTO_COMPILE_MELPA_PIN: (&str, &str) = ("auto-compile", "20260601.1449");

/// The exact auto-dark package selected by the comprehensive metadata, theme,
/// customization, platform-detection, command-adapter, listener, timer, hook,
/// and global-mode lifecycle parity corpus. MELPA built this archive from
/// upstream commit `6d1e8d2fc493dccbf05c9191611805c7e7881c70`.
pub const AUTO_DARK_MELPA_PIN: (&str, &str) = ("auto-dark", "20260313.2356");

/// The exact auto-dictionary package selected by the comprehensive language
/// scoring, dictionary switching, idle-timer, Flyspell filtering, conditional
/// insertion, and multilingual workflow parity corpus. MELPA built this
/// archive from upstream commit `b364e08009fe0062cf0927d8a0582fad5a12b8e7`.
pub const AUTO_DICTIONARY_MELPA_PIN: (&str, &str) = ("auto-dictionary", "20150410.1610");

/// The exact auto-dim-other-buffers package selected by the comprehensive
/// face-remapping, window-selection, focus, customization, hook, advice, and
/// global-mode lifecycle parity corpus. MELPA built this archive from upstream
/// commit `cf0263073470190b85f6013066856126aac67d19`.
pub const AUTO_DIM_OTHER_BUFFERS_MELPA_PIN: (&str, &str) =
    ("auto-dim-other-buffers", "20260624.950");

/// The exact auto-highlight-symbol package selected by the comprehensive
/// symbol detection, overlay, navigation, edit, timer, lifecycle, and
/// multi-buffer workflow parity corpus. MELPA built this archive from
/// upstream commit `e84da32e7cf1baefb0a9eef42a2fc842cf18f8b3`.
pub const AUTO_HIGHLIGHT_SYMBOL_MELPA_PIN: (&str, &str) = ("auto-highlight-symbol", "20260101.552");

/// The exact auto-indent-mode package selected by the comprehensive
/// indentation, yank, deletion, kill, repository, lifecycle, and practical
/// editing workflow parity corpus. MELPA built this archive from upstream
/// commit `664006b67329a8e27330541547f8c2187dab947c`.
pub const AUTO_INDENT_MODE_MELPA_PIN: (&str, &str) = ("auto-indent-mode", "20211029.11");

/// The exact auto-minor-mode package selected by the comprehensive filename,
/// magic-content, advice, repeat-activation, use-package, and practical file
/// workflow parity corpus. MELPA built this archive from upstream commit
/// `c62f4e04c7b73835c399f0348bea0ade2720bcbb`.
pub const AUTO_MINOR_MODE_MELPA_PIN: (&str, &str) = ("auto-minor-mode", "20180527.1123");

/// The exact auto-read-only package selected by the comprehensive filename
/// matching, project suppression, hook, global-mode, and practical read-only
/// workflow parity corpus. MELPA built this archive from upstream commit
/// `206d4559762fe6ef9e91de8f9dc43e1e41c0f42c`.
pub const AUTO_READ_ONLY_MELPA_PIN: (&str, &str) = ("auto-read-only", "20260521.1659");

/// The exact auto-org-md package selected by the comprehensive export,
/// hook-lifecycle, global-state, and practical Org-to-Markdown workflow parity
/// corpus. MELPA built this archive from upstream commit
/// `9318338bdb7fe8bd698d88f3af89b2d6413efdd2`.
pub const AUTO_ORG_MD_MELPA_PIN: (&str, &str) = ("auto-org-md", "20180213.2343");

/// The exact auto-package-update package selected by the comprehensive update
/// selection, scheduling, prompting, package transaction, results buffer, old
/// version cleanup, and async lifecycle parity corpus. MELPA built this archive
/// from upstream commit `e966c6c95de1742d867250dc15b1c6bd570b6ea5`.
pub const AUTO_PACKAGE_UPDATE_MELPA_PIN: (&str, &str) = ("auto-package-update", "20260601.1804");

/// The exact ht package selected by the practical configuration, nested state,
/// job pipeline, custom-key, and snapshot parity corpus, and as
/// auto-highlight-symbol's hash-table dependency. MELPA built this archive from upstream commit
/// `1c49aad1c820c86f7ee35bf9fff8429502f60fef`.
pub const HT_MELPA_PIN: (&str, &str) = ("ht", "20230703.558");

/// The exact Hydra package selected by the practical command-family,
/// transient-keymap, extension, radio, and source-editing parity corpus.
/// MELPA built this archive from upstream commit
/// `59a2a45a35027948476d1d7751b0f0215b1e61aa`.
pub const HYDRA_MELPA_PIN: (&str, &str) = ("hydra", "20250316.1254");

/// The exact inf-ruby package selected by the practical comint-mode, source
/// dispatch, completion, project-console, and debugger lifecycle parity corpus.
/// MELPA built this archive from upstream commit
/// `274398a24288a7db430a656b580ffbf889ca02aa`.
pub const INF_RUBY_MELPA_PIN: (&str, &str) = ("inf-ruby", "20251224.216");

/// The exact iter2 package selected by the practical resumable-workflow,
/// composition, resource-cleanup, editor-state, nonlocal-exit, and tracing
/// parity corpus. MELPA built this archive from upstream commit
/// `632232b5ee627bf5d299db0b7714b3b687a0124c`.
pub const ITER2_MELPA_PIN: (&str, &str) = ("iter2", "20250209.1516");

/// The exact Ivy package selected by the practical interactive selection,
/// action dispatch, completing-read lifecycle, search-language,
/// completion-in-region, and resumable-session parity corpus. MELPA built
/// this archive from upstream commit
/// `0d02f5063d36ff4fa6138f0973c83c6d3874fba0`.
pub const IVY_MELPA_PIN: (&str, &str) = ("ivy", "20260413.2102");

/// The exact Ivy Rich package selected by the practical transformer,
/// buffer-dashboard, project-cache, file/bookmark, and package-catalog parity
/// corpus. MELPA built this archive from upstream commit
/// `aff9b6bd53e0fdcf350ab83c90e64e651b47dba4`.
pub const IVY_RICH_MELPA_PIN: (&str, &str) = ("ivy-rich", "20230425.1422");

/// The exact js2-mode package selected by the practical parsing, diagnostics,
/// indentation, navigation, JSON-path, Imenu, and editor-aid parity corpus.
/// MELPA built this archive from upstream commit
/// `41d0e7f5ef51109c682016baa6fc6846e03e8517`.
pub const JS2_MODE_MELPA_PIN: (&str, &str) = ("js2-mode", "20260627.1342");

/// The exact multiple-cursors package selected for practical multi-line,
/// occurrence-based, ordered, region-transforming, alignment, lifecycle, and
/// focused-context editing parity, and required by js2-refactor's scope-aware
/// rename workflow. This MELPA version is pinned to upstream commit
/// `94b8b07a4bab87f803123723b68227565429dfa1`.
pub const MULTIPLE_CURSORS_MELPA_PIN: (&str, &str) = ("multiple-cursors", "20260419.931");

/// The exact Names package selected for practical collision-free modules,
/// split declarations, macro pipelines, keyword APIs, derived modes, and
/// customization metadata parity. This MELPA version is pinned to upstream
/// commit `45a272fae915148d9a74d4cb3c39917b272ee9c3`.
pub const NAMES_MELPA_PIN: (&str, &str) = ("names", "20221227.1825");

/// The exact js2-refactor package selected by the practical scope rewrite,
/// signature migration, extraction, IIFE, and structural editing parity
/// corpus. MELPA built this archive from upstream commit
/// `e1177c728ae52a5e67157fb18ee1409d8e95386a`.
pub const JS2_REFACTOR_MELPA_PIN: (&str, &str) = ("js2-refactor", "20250210.1811");

/// The exact Keyfreq package selected by the practical command accounting,
/// report generation, export, cooperative persistence, and autosave lifecycle
/// parity corpus. MELPA built this archive from upstream commit
/// `c6955162307f37c2ac631d9daf118781009f8dda`.
pub const KEYFREQ_MELPA_PIN: (&str, &str) = ("keyfreq", "20231107.106");

/// The exact lv package required by the practical Hydra parity corpus and
/// selected for the hint-window lifecycle, refresh, layout, GUI separator,
/// failure-atomicity, and pre-existing-buffer parity corpus. MELPA built this
/// archive from upstream commit
/// `87873d788891029d9e44fa5458321d6a05849b94`.
pub const LV_MELPA_PIN: (&str, &str) = ("lv", "20200507.1518");

/// The exact m-buffer package selected for the scoped search, marker-safe
/// rewrite, line classification, log segmentation, annotation, and stateless
/// location parity corpus. MELPA built this archive from upstream commit
/// `5e7714835b2289f61dad24c0b5cf98d28fc313b0`.
pub const M_BUFFER_MELPA_PIN: (&str, &str) = ("m-buffer", "20241215.2214");

/// The exact Macrostep package selected for the practical inline expansion,
/// nested lifecycle, local environment, compiler macro, pretty-printing,
/// separate-buffer, and failure-atomicity parity corpus. MELPA built this
/// archive from upstream commit
/// `d0928626b4711dcf9f8f90439d23701118724199`.
pub const MACROSTEP_MELPA_PIN: (&str, &str) = ("macrostep", "20250202.2205");

/// The exact Mag Menu package selected for the practical command option,
/// rendered menu, keyboard interaction, action dispatch, help, and
/// splitter-backed window lifecycle parity corpus. MELPA built this archive
/// from upstream commit
/// `9b9277021cd09fb1dba64b1d2a00705d20914bd6`.
pub const MAG_MENU_MELPA_PIN: (&str, &str) = ("mag-menu", "20150505.1850");

/// The exact Makey package selected for the practical generated-command,
/// mixed command-line/Lisp option, rendered popup, keyboard dispatch, help,
/// literal action, and window restoration parity corpus. MELPA built this
/// archive from upstream commit
/// `a61781e69d3b451551e269446e1c5f624ab81137`.
pub const MAKEY_MELPA_PIN: (&str, &str) = ("makey", "20131231.1430");

/// The exact Markdown Mode package selected for the practical release-note
/// editing, outline reorganization, task-list, reference and footnote,
/// report-table, and fenced-code parsing parity corpus. MELPA built this
/// archive from upstream commit
/// `f441e8bc9951e73b12c61e9198658488dd8e86e1`.
pub const MARKDOWN_MODE_MELPA_PIN: (&str, &str) = ("markdown-mode", "20260722.40");

/// The exact Math Symbol Lists package selected for the practical completion,
/// Unicode formula rendering, package-requirement, conflict-resolution,
/// scripted-character, and full-corpus integrity parity suite. MELPA built
/// this archive from upstream commit
/// `ac3eb053d3b576fcdd192b0ac6ad5090ea3a7079`.
pub const MATH_SYMBOL_LISTS_MELPA_PIN: (&str, &str) = ("math-symbol-lists", "20220828.2047");

/// The exact Maude Mode package selected for the practical module editing,
/// indentation, navigation, abbrev authoring, source transport, and inferior
/// diagnostic parity corpus. MELPA built this archive from upstream commit
/// `2e1f68a890493d964f933d6e40b0ede047f70ede`.
pub const MAUDE_MODE_MELPA_PIN: (&str, &str) = ("maude-mode", "20230504.937");

/// The exact Mozc package selected for the practical input-mode lifecycle,
/// key translation, placeholder editing, preedit and candidate rendering,
/// helper framing, and session protocol parity corpus. This MELPA version is
/// pinned to upstream commit
/// `76887c679e1e4f156102e4bc62ea9cf9174678a3`.
pub const MOZC_MELPA_PIN: (&str, &str) = ("mozc", "20260624.1355");

/// The exact Dash package selected by the live lifecycle and comprehensive
/// API parity corpora.
pub const DASH_MELPA_PIN: (&str, &str) = ("dash", "20260221.1346");

/// The exact Evil package selected by the comprehensive API parity corpus.
pub const EVIL_MELPA_PIN: (&str, &str) = ("evil", "20260603.654");

/// The exact Bind-Key release selected from GNU ELPA by the comprehensive API
/// parity corpus.
pub const BIND_KEY_GNU_ELPA_PIN: (&str, &str) = ("bind-key", "2.4.1");

/// The exact BUI package selected by the practical service dashboard,
/// marking, filtering, detail action, and history parity corpus, and as
/// aurel's runtime buffer-interface dependency.
pub const BUI_MELPA_PIN: (&str, &str) = ("bui", "20260502.730");

/// The exact Casual package selected by the practical EditKit, Elisp, CSV,
/// Dired, and Ibuffer menu-command parity corpus.
pub const CASUAL_MELPA_PIN: (&str, &str) = ("casual", "20260718.1803");

/// The exact CCC package selected by the practical buffer-local cursor,
/// frame-color baseline, terminal fallback, and setup lifecycle parity corpus.
pub const CCC_MELPA_PIN: (&str, &str) = ("ccc", "20260322.1316");

/// The exact CDB package selected by the practical indexed lookup, collision,
/// binary payload, enumeration, and cached-reader lifecycle parity corpus.
pub const CDB_MELPA_PIN: (&str, &str) = ("cdb", "20230318.2152");

/// The exact Chinese Word at Point package selected by the practical external
/// segmentation, mixed-language extraction, and bounds-driven editing corpus.
pub const CHINESE_WORD_AT_POINT_MELPA_PIN: (&str, &str) = ("chinese-word-at-point", "20170811.941");

/// The exact CSV Mode release selected from GNU ELPA by the practical quoted
/// row, column editing, sorting, alignment, and transpose parity corpus.
pub const CSV_MODE_GNU_ELPA_PIN: (&str, &str) = ("csv-mode", "1.27");

/// The exact Datetime Format package selected by the practical protocol date,
/// timezone, DST transition, scheduler normalization, and validation corpus.
pub const DATETIME_FORMAT_MELPA_PIN: (&str, &str) = ("datetime-format", "20240105.1901");

/// The exact DDSKK package selected by the practical Japanese input,
/// dictionary conversion, learned-candidate, numeric, and punctuation corpus.
pub const DDSKK_MELPA_PIN: (&str, &str) = ("ddskk", "20260329.1317");

/// The exact Avy package selected by the practical keyboard-driven jump,
/// cross-window, dispatch action, line editing, and cancellation corpus.
pub const AVY_MELPA_PIN: (&str, &str) = ("avy", "20241101.1357");

/// The exact Avy Menu package selected by the practical rendered menu,
/// multi-level selection, inactive item, and cancellation lifecycle corpus.
pub const AVY_MENU_MELPA_PIN: (&str, &str) = ("avy-menu", "20230606.1519");

/// The exact BERT package selected by the practical external-term fixture,
/// RPC, signed metric, UTF-8 binary, and bulk tuple parity corpus.
pub const BERT_MELPA_PIN: (&str, &str) = ("bert", "20131117.1014");

/// The exact Compat release selected from GNU ELPA by the comprehensive API
/// parity corpus.
pub const COMPAT_GNU_ELPA_PIN: (&str, &str) = ("compat", "31.0.0.2");

/// The exact Clojure Mode package selected by the practical project namespace,
/// formatting, structural refactoring, and source-navigation parity corpus.
pub const CLOJURE_MODE_MELPA_PIN: (&str, &str) = ("clojure-mode", "20260709.952");

/// The exact Company package selected by the practical interactive
/// completion, CAPF, asynchronous backend, and file workflow parity corpus.
pub const COMPANY_MELPA_PIN: (&str, &str) = ("company", "20260721.100");

/// The exact Cond-Let package selected by the practical conditional binding,
/// validation pipeline, authorization, and queue workflow parity corpus.
pub const COND_LET_MELPA_PIN: (&str, &str) = ("cond-let", "20260701.1237");

/// The exact Consult package selected by the practical line, symbol, and
/// buffer-navigation workflow parity corpus.
pub const CONSULT_MELPA_PIN: (&str, &str) = ("consult", "20260716.1105");

/// The exact f package selected by the comprehensive API parity corpus.
pub const F_MELPA_PIN: (&str, &str) = ("f", "20241003.1131");

/// The exact Magit package containing the Git-Commit source selected by the
/// comprehensive API parity corpus.
pub const GIT_COMMIT_MELPA_PIN: (&str, &str) = ("magit", "20260724.2338");

/// The exact General package selected by the comprehensive API parity corpus.
pub const GENERAL_MELPA_PIN: (&str, &str) = ("general", "20250612.2309");

/// The exact goto-chg package selected by the comprehensive API parity corpus.
pub const GOTO_CHG_MELPA_PIN: (&str, &str) = ("goto-chg", "20240407.1110");

/// The exact Helm package selected by the practical source, matching, action,
/// completion, imenu, and occur parity corpus, and as audacious' runtime
/// user-interface dependency.
pub const HELM_MELPA_PIN: (&str, &str) = ("helm", "20260728.709");

/// The exact helm-core package selected by the practical source-extension,
/// candidate-buffer, pipeline, preview, and path parity corpus, and required
/// by the Helm parity corpus.
pub const HELM_CORE_MELPA_PIN: (&str, &str) = ("helm-core", "20260720.1307");

/// The exact wfnames package required by the practical Helm parity corpus.
pub const WFNAMES_MELPA_PIN: (&str, &str) = ("wfnames", "20260706.903");

/// The exact Magit package selected by the comprehensive API parity corpus.
pub const MAGIT_MELPA_PIN: (&str, &str) = ("magit", "20260724.2338");

/// The exact magit-section package selected by the comprehensive API parity
/// corpus.
pub const MAGIT_SECTION_MELPA_PIN: (&str, &str) = ("magit-section", "20260722.2131");

/// The exact Projectile package selected by the comprehensive API parity
/// corpus.
pub const PROJECTILE_MELPA_PIN: (&str, &str) = ("projectile", "20260728.945");

/// The exact s package selected by the live lifecycle and comprehensive API
/// parity corpora.
pub const S_MELPA_PIN: (&str, &str) = ("s", "20220902.1511");

/// The exact Yasnippet package selected by the direct parity corpus and as
/// angular-snippets' manually documented runtime dependency.
pub const YASNIPPET_MELPA_PIN: (&str, &str) = ("yasnippet", "20250602.1342");

/// The exact Transient package selected by the comprehensive API parity
/// corpus.
pub const TRANSIENT_MELPA_PIN: (&str, &str) = ("transient", "20260725.1105");

/// The exact Use-Package release selected from GNU ELPA by the comprehensive
/// API parity corpus.
pub const USE_PACKAGE_GNU_ELPA_PIN: (&str, &str) = ("use-package", "2.4.6");

/// The exact Which-Key package selected by the comprehensive API parity corpus.
pub const WHICH_KEY_MELPA_PIN: (&str, &str) = ("which-key", "20240620.2145");

/// The exact With-Editor package selected by the comprehensive API parity
/// corpus.
pub const WITH_EDITOR_MELPA_PIN: (&str, &str) = ("with-editor", "20260701.1252");

/// Resolve the checkout used by a normal Cargo run or an extracted Nextest
/// archive.
pub fn workspace_root() -> PathBuf {
    if let Some(root) = std::env::var_os("NEXTEST_WORKSPACE_ROOT") {
        return PathBuf::from(root);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("neomacs-melpa-tests is a workspace member")
        .to_path_buf()
}

/// Per-scenario filesystem and subprocess isolation.
pub struct MelpaSandbox {
    case_root: tempfile::TempDir,
    home: PathBuf,
    tmp: PathBuf,
}

impl MelpaSandbox {
    /// Create a sandbox below `<workspace>/tmp/melpa`.
    pub fn new(label: &str) -> Result<Self, String> {
        let base = workspace_root().join("tmp/melpa");
        fs::create_dir_all(&base).map_err(|error| {
            format!(
                "failed to create MELPA scratch directory {}: {error}",
                base.display()
            )
        })?;
        let prefix = format!("{}-", sanitize_label(label));
        let case_root = tempfile::Builder::new()
            .prefix(&prefix)
            .tempdir_in(&base)
            .map_err(|error| {
                format!(
                    "failed to create MELPA scenario directory in {}: {error}",
                    base.display()
                )
            })?;
        let home = case_root.path().join("home");
        let tmp = case_root.path().join("tmp");
        let xdg_config = case_root.path().join("xdg/config");
        let xdg_cache = case_root.path().join("xdg/cache");
        let xdg_data = case_root.path().join("xdg/data");
        let xdg_state = case_root.path().join("xdg/state");
        for directory in [&home, &tmp, &xdg_config, &xdg_cache, &xdg_data, &xdg_state] {
            fs::create_dir_all(directory).map_err(|error| {
                format!(
                    "failed to create MELPA sandbox directory {}: {error}",
                    directory.display()
                )
            })?;
        }
        fs::create_dir_all(home.join(".emacs.d"))
            .map_err(|error| format!("failed to create isolated .emacs.d: {error}"))?;

        Ok(Self {
            case_root,
            home,
            tmp,
        })
    }

    pub fn root(&self) -> &Path {
        self.case_root.path()
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn tmp_dir(&self) -> &Path {
        &self.tmp
    }

    /// Apply the deterministic process environment shared by install and
    /// restart/probe processes.
    pub fn configure(&self, command: &mut Command) {
        configure_process_environment(command, self.root(), &self.home, &self.tmp);
    }
}

fn configure_process_environment(command: &mut Command, root: &Path, home: &Path, tmp: &Path) {
    command
        .current_dir(root)
        .env("HOME", home)
        .env("TMPDIR", tmp)
        .env("TMP", tmp)
        .env("TEMP", tmp)
        .env("XDG_CONFIG_HOME", root.join("xdg/config"))
        .env("XDG_CACHE_HOME", root.join("xdg/cache"))
        .env("XDG_DATA_HOME", root.join("xdg/data"))
        .env("XDG_STATE_HOME", root.join("xdg/state"))
        .env("LANG", "C.UTF-8")
        .env("LC_ALL", "C.UTF-8")
        .env("TZ", "UTC")
        .env("USER", "melpa-test")
        .env("LOGNAME", "melpa-test")
        .env("HOSTNAME", "melpa-host")
        .env("EMAIL", "melpa-test@melpa-host")
        .env("TERM", "dumb")
        .env("NEOMACS_TEST_SANDBOX_ROOT", root)
        .env("NEOMACS_TEST_WORKSPACE_ROOT", workspace_root())
        .env_remove("EMACSLOADPATH")
        .env("GIT_CEILING_DIRECTORIES", workspace_root());
}

fn sanitize_label(label: &str) -> String {
    let sanitized = label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "scenario".to_string()
    } else {
        sanitized
    }
}

/// An editor executable that can run a package scenario.
#[derive(Clone, Debug)]
pub struct EmacsRuntime {
    pub name: String,
    pub executable: PathBuf,
    extra_env: Vec<(OsString, OsString)>,
    timeout: Duration,
}

impl EmacsRuntime {
    pub fn new(name: impl Into<String>, executable: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            executable: executable.into(),
            extra_env: Vec::new(),
            timeout: DEFAULT_PROCESS_TIMEOUT,
        }
    }

    pub fn neomacs() -> Self {
        Self::new("neomacs", neomacs_binary())
    }

    /// GNU Emacs oracle selected explicitly by environment, then from the
    /// developer's adjacent source checkout, and finally from `PATH`.
    pub fn gnu_emacs() -> Self {
        for variable in [
            "NEOMACS_MELPA_ORACLE_EMACS",
            "NEOVM_ORACLE_EMACS",
            "ORACLE_EMACS",
        ] {
            if let Some(path) = std::env::var_os(variable) {
                return Self::new("gnu-emacs", PathBuf::from(path));
            }
        }
        let source_checkout =
            PathBuf::from("/home/exec/Projects/github.com/emacs-mirror/emacs/src/emacs");
        if source_checkout.is_file() {
            return Self::new("gnu-emacs", source_checkout);
        }
        Self::new("gnu-emacs", "emacs")
    }

    pub fn with_env(mut self, name: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.extra_env.push((name.into(), value.into()));
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        for (name, value) in &self.extra_env {
            command.env(name, value);
        }
        command
    }
}

/// Package archive used by a scenario.
#[derive(Clone, Debug)]
pub struct PackageSource {
    archives: Vec<(String, PathBuf)>,
}

impl PackageSource {
    pub fn frozen(archive_dir: impl Into<PathBuf>) -> Self {
        Self {
            archives: vec![("frozen".to_string(), archive_dir.into())],
        }
    }

    pub fn local<I, N, P>(archives: I) -> Self
    where
        I: IntoIterator<Item = (N, P)>,
        N: Into<String>,
        P: Into<PathBuf>,
    {
        Self {
            archives: archives
                .into_iter()
                .map(|(name, path)| (name.into(), path.into()))
                .collect(),
        }
    }

    fn archive_form(&self) -> String {
        let entries = self
            .archives
            .iter()
            .map(|(name, directory)| {
                let directory = directory
                    .canonicalize()
                    .unwrap_or_else(|_| directory.clone());
                let directory = format!("{}/", directory.display());
                format!("({} . {})", elisp_string(name), elisp_string(&directory))
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("'({entries})")
    }
}

/// Packages and the post-restart Elisp probe that define one compatibility
/// scenario.
#[derive(Clone, Debug)]
pub struct PackageScenario {
    pub name: String,
    packages: PackageSelection,
    pub probe: String,
}

#[derive(Clone, Debug)]
enum PackageSelection {
    Unversioned(Vec<String>),
    Versioned(Vec<PackagePin>),
}

/// An exact package name/version selected for a live archive scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagePin {
    pub name: String,
    pub version: String,
}

impl PackageScenario {
    pub fn new<I, P>(name: impl Into<String>, packages: I, probe: impl Into<String>) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<String>,
    {
        Self {
            name: name.into(),
            packages: PackageSelection::Unversioned(packages.into_iter().map(Into::into).collect()),
            probe: probe.into(),
        }
    }

    /// Define a scenario whose selected third-party packages have exact
    /// versions.
    pub fn versioned<I, N, V>(
        name: impl Into<String>,
        packages: I,
        probe: impl Into<String>,
    ) -> Self
    where
        I: IntoIterator<Item = (N, V)>,
        N: Into<String>,
        V: Into<String>,
    {
        Self {
            name: name.into(),
            packages: PackageSelection::Versioned(
                packages
                    .into_iter()
                    .map(|(name, version)| PackagePin {
                        name: name.into(),
                        version: version.into(),
                    })
                    .collect(),
            ),
            probe: probe.into(),
        }
    }

    pub fn from_probe_file<I, P>(
        name: impl Into<String>,
        packages: I,
        probe_path: impl AsRef<Path>,
    ) -> Result<Self, String>
    where
        I: IntoIterator<Item = P>,
        P: Into<String>,
    {
        let probe_path = probe_path.as_ref();
        let probe = fs::read_to_string(probe_path).map_err(|error| {
            format!(
                "failed to read package probe {}: {error}",
                probe_path.display()
            )
        })?;
        Ok(Self::new(name, packages, probe))
    }

    /// Build a package-agnostic probe of the post-restart autoload surface.
    ///
    /// This is the scalable baseline for a package corpus: it does not guess
    /// arguments or invoke arbitrary package commands. It inventories
    /// autoloaded functions/macros, custom variables, and emitted bytecode for
    /// the complete dependency graph. Curated probes can be added separately
    /// when meaningful behavior and inputs are known.
    pub fn autoload_surface<I, P>(name: impl Into<String>, packages: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<String>,
    {
        let packages = packages.into_iter().map(Into::into).collect::<Vec<_>>();
        let package_strings = packages
            .iter()
            .map(|package| elisp_string(package))
            .collect::<Vec<_>>()
            .join(" ");
        let probe = format!(
            r##"(let* ((requested
                         (mapcar #'intern '({package_strings})))
                       (libraries (make-hash-table :test 'equal))
                       (known-library-p
                        (lambda (library)
                          (and
                           (stringp library)
                           (or (gethash library libraries)
                               (gethash
                                (file-name-sans-extension library)
                                libraries)
                               (gethash
                                (file-name-base library)
                                libraries)))))
                       (autoloads nil)
                       (customs nil)
                       (bytecode nil))
                  (dolist (package requested)
                    (unless (package-installed-p package)
                      (error "requested package was not installed: %S" package)))
                  (dolist (entry package-alist)
                    (let* ((description (cadr entry))
                           (directory (package-desc-dir description))
                           (files
                            (and directory
                                 (file-directory-p directory)
                                 (directory-files-recursively
                                  directory "\\.elc?\\'")))
                           (compiled nil))
                      (dolist (file files)
                        (let* ((relative
                                (file-relative-name file directory))
                               (library
                                (file-name-sans-extension relative)))
                          (puthash library t libraries)
                          (puthash (file-name-base library) t libraries)
                          (when (string-suffix-p ".elc" relative)
                            (push relative compiled))))
                      (push
                       (list
                        (car entry)
                        (package-version-join
                         (package-desc-version description))
                        (sort compiled #'string<))
                       bytecode)))
                  (mapatoms
                   (lambda (symbol)
                     (let ((definition
                            (and (fboundp symbol)
                                 (symbol-function symbol))))
                       (when (and (autoloadp definition)
                                  (funcall known-library-p (nth 1 definition)))
                         (push
                          (list symbol
                                (nth 1 definition)
                                (if (eq (nth 4 definition) 'macro)
                                    'macro
                                  (if (nth 3 definition)
                                      'command
                                    'function)))
                          autoloads)))
                     (let ((custom-libraries nil))
                       (dolist (library (get symbol 'custom-loads))
                         (let ((library-name
                                (cond
                                 ((stringp library) library)
                                 ((symbolp library) (symbol-name library)))))
                           (when (and library-name
                                      (funcall known-library-p library-name))
                             (push library-name custom-libraries))))
                       (when custom-libraries
                         (push
                          (list symbol
                                (sort custom-libraries #'string<))
                          customs)))))
                  (list
                   :autoloads
                   (sort autoloads
                         (lambda (left right)
                           (string< (symbol-name (car left))
                                    (symbol-name (car right)))))
                   :customs
                   (sort customs
                         (lambda (left right)
                           (string< (symbol-name (car left))
                                    (symbol-name (car right)))))
                   :bytecode
                   (sort bytecode
                         (lambda (left right)
                           (string< (symbol-name (car left))
                                    (symbol-name (car right)))))))"##
        );
        Self::new(name, packages, probe)
    }

    fn package_names(&self) -> Vec<&str> {
        match &self.packages {
            PackageSelection::Unversioned(packages) => {
                packages.iter().map(String::as_str).collect()
            }
            PackageSelection::Versioned(packages) => packages
                .iter()
                .map(|package| package.name.as_str())
                .collect(),
        }
    }

    fn package_pins(&self) -> Option<&[PackagePin]> {
        match &self.packages {
            PackageSelection::Unversioned(_) => None,
            PackageSelection::Versioned(packages) => Some(packages),
        }
    }
}

/// One ERT selector loaded from an Emacs Lisp test file.
#[derive(Clone, Debug)]
pub struct ErtScenario {
    pub name: String,
    pub test_file: PathBuf,
    pub selector: String,
}

impl ErtScenario {
    pub fn new(
        name: impl Into<String>,
        test_file: impl Into<PathBuf>,
        selector: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            test_file: test_file.into(),
            selector: selector.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioPhase {
    Install,
    RestartProbe,
    QuickstartProbe,
    VcInstall,
    VcRestart,
    VcUpgrade,
    VcDelete,
    VcRestartAfterDelete,
    Ert,
}

#[derive(Debug)]
pub struct PhaseReport {
    pub phase: ScenarioPhase,
    pub duration: Duration,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub struct ScenarioReport {
    pub runtime: String,
    pub scenario: String,
    pub phases: Vec<PhaseReport>,
    pub installed_packages: Vec<InstalledPackage>,
    pub outcome: EvalOutcome,
}

/// GNU Emacs and Neomacs reports after package lifecycle parity is verified.
#[derive(Debug)]
pub struct OracleScenarioReport {
    pub neomacs: ScenarioReport,
    pub gnu_emacs: ScenarioReport,
}

/// GNU Emacs and Neomacs outcomes for one direct Elisp form.
#[derive(Debug)]
pub struct ElispOracleReport {
    pub neomacs: EvalOutcome,
    pub gnu_emacs: EvalOutcome,
}

/// One named probe for [`CachedPackageOracle::run_batch`].
#[derive(Clone, Copy, Debug)]
pub struct OracleBatchCase<'a> {
    /// Stable case id (no `:` or whitespace). Used in failures and expect keys.
    pub id: &'a str,
    /// Elisp forms evaluated after shared package setup.
    pub probe: &'a str,
    /// Whether this case must return a value or signal an error.
    pub expected_outcome: ExpectedOutcome,
}

/// Differential outcomes for one case inside a multi-probe batch.
#[derive(Debug)]
pub struct OracleBatchCaseReport {
    pub id: String,
    pub neomacs: EvalOutcome,
    pub gnu_emacs: EvalOutcome,
}

/// The editor whose outcome violated a typed batch expectation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OracleEditor {
    GnuEmacs,
    Neomacs,
}

/// A behavioral failure for one case in an otherwise valid batch protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleBatchFailure {
    OutcomeMismatch {
        id: String,
        neomacs: EvalOutcome,
        gnu_emacs: EvalOutcome,
    },
    UnexpectedOutcome {
        id: String,
        editor: OracleEditor,
        expected: ExpectedOutcome,
        actual: EvalOutcome,
    },
}

impl OracleBatchFailure {
    pub fn id(&self) -> &str {
        match self {
            Self::OutcomeMismatch { id, .. } | Self::UnexpectedOutcome { id, .. } => id,
        }
    }
}

impl std::fmt::Display for OracleBatchFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutcomeMismatch {
                id,
                neomacs,
                gnu_emacs,
            } => write!(
                formatter,
                "case `{id}` outcome mismatch:\n  Neomacs: {neomacs}\n  GNU Emacs: {gnu_emacs}"
            ),
            Self::UnexpectedOutcome {
                id,
                editor,
                expected,
                actual,
            } => {
                let editor = match editor {
                    OracleEditor::GnuEmacs => "GNU Emacs",
                    OracleEditor::Neomacs => "Neomacs",
                };
                let expected = match expected {
                    ExpectedOutcome::Value => "a value",
                    ExpectedOutcome::Signal => "a signal",
                };
                write!(
                    formatter,
                    "case `{id}` expected {editor} to return {expected}, got {actual}"
                )
            }
        }
    }
}

/// All case outcomes and behavioral failures from one valid batch execution.
#[derive(Debug)]
pub struct OracleBatchReport {
    pub cases: Vec<OracleBatchCaseReport>,
    pub failures: Vec<OracleBatchFailure>,
}

/// Differential oracle for one exact package cached below `./tmp`.
pub struct CachedPackageOracle {
    package_name: String,
    package_user_dir: PathBuf,
    package_directory_list: Vec<PathBuf>,
    package_load_list: Vec<(String, String)>,
    source_file: PathBuf,
    activation: PackageActivation,
    prelude: String,
    timeout: Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PackageActivation {
    SourceFile,
    InstalledAutoloads,
}

fn package_activation_elisp(activation: PackageActivation) -> &'static str {
    match activation {
        PackageActivation::SourceFile => r#"(load (getenv "NEOMACS_PACKAGE_SOURCE") nil t t)"#,
        PackageActivation::InstalledAutoloads => "nil",
    }
}

/// MELPA-focused name retained for package-specific parity modules.
pub type CachedMelpaOracle = CachedPackageOracle;

impl CachedPackageOracle {
    /// Build an exact revision-pinned package from source and select its file.
    pub fn new(package: (&str, &str), source_file_name: &str) -> Result<Self, String> {
        Self::new_from_manifest_with_runtime(&EmacsRuntime::gnu_emacs(), package, source_file_name)
    }

    fn new_from_manifest_with_runtime(
        gnu_emacs: &EmacsRuntime,
        package: (&str, &str),
        source_file_name: &str,
    ) -> Result<Self, String> {
        validate_cached_source_file_name("source-built package", source_file_name)?;
        let package_dir = prepare_cached_locked_melpa_package(gnu_emacs, package)?;
        Self::from_package_dir(package, source_file_name, package_dir)
    }

    /// Prepare one pinned GNU ELPA package and select its Elisp source file.
    pub fn new_from_gnu_elpa(
        package: (&str, &str),
        source_file_name: &str,
    ) -> Result<Self, String> {
        validate_cached_source_file_name(GNU_ELPA_ARCHIVE.label, source_file_name)?;
        let package_dir = prepare_cached_gnu_elpa_package(&EmacsRuntime::gnu_emacs(), package)?;
        Self::from_package_dir(package, source_file_name, package_dir)
    }

    fn from_package_dir(
        package: (&str, &str),
        source_file_name: &str,
        package_dir: PathBuf,
    ) -> Result<Self, String> {
        let source_file = package_dir.join(source_file_name);
        if !source_file.is_file() {
            return Err(format!(
                "cached {} source `{source_file_name}` is missing below {}",
                package.0,
                package_dir.display()
            ));
        }
        let package_user_dir = package_dir
            .parent()
            .expect("cached package directory is below an ELPA directory")
            .to_path_buf();
        Ok(Self {
            package_name: package.0.to_string(),
            package_user_dir,
            package_directory_list: Vec::new(),
            package_load_list: vec![(package.0.to_string(), package.1.to_string())],
            source_file,
            activation: PackageActivation::SourceFile,
            prelude: String::new(),
            timeout: DEFAULT_PROCESS_TIMEOUT,
        })
    }

    /// Evaluate an additional setup form before loading the package source.
    pub fn with_prelude(mut self, prelude: impl Into<String>) -> Self {
        self.prelude = prelude.into();
        self
    }

    /// Exercise the package state established by `package-initialize` without
    /// loading the selected source file afterward.
    pub fn with_installed_autoloads(mut self) -> Self {
        self.activation = PackageActivation::InstalledAutoloads;
        self
    }

    fn with_prepared_dependency(
        mut self,
        package: (&str, &str),
        package_dir: PathBuf,
    ) -> Result<Self, String> {
        let package_directory = package_dir
            .parent()
            .expect("cached package directory is below an ELPA directory")
            .to_path_buf();
        if !self.package_directory_list.contains(&package_directory) {
            self.package_directory_list.push(package_directory);
        }
        if let Some((_, pinned_version)) = self
            .package_load_list
            .iter()
            .find(|(pinned_name, _)| pinned_name == package.0)
        {
            if pinned_version != package.1 {
                return Err(format!(
                    "package `{}` is already pinned to version `{pinned_version}`, cannot also pin `{}`",
                    package.0, package.1
                ));
            }
        } else {
            self.package_load_list
                .push((package.0.to_string(), package.1.to_string()));
        }
        Ok(self)
    }

    /// Make another exact source-built package cache visible as a system-wide
    /// package directory while loading the package under test.
    pub fn with_melpa_dependency(self, package: (&str, &str)) -> Result<Self, String> {
        let package_dir = prepare_cached_locked_melpa_package(&EmacsRuntime::gnu_emacs(), package)?;
        self.with_prepared_dependency(package, package_dir)
    }

    /// Make another exact GNU ELPA package cache visible as a system-wide
    /// package directory while loading the package under test.
    pub fn with_gnu_elpa_dependency(self, package: (&str, &str)) -> Result<Self, String> {
        let package_dir = prepare_cached_gnu_elpa_package(&EmacsRuntime::gnu_emacs(), package)?;
        self.with_prepared_dependency(package, package_dir)
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Run a parity case that must complete with a value in both editors.
    pub fn run_value(&self, name: &str, probe: &str) -> Result<ElispOracleReport, String> {
        self.run_expected(name, probe, ExpectedOutcome::Value)
    }

    /// Run a parity case that must signal in both editors.
    pub fn run_signal(&self, name: &str, probe: &str) -> Result<ElispOracleReport, String> {
        self.run_expected(name, probe, ExpectedOutcome::Signal)
    }

    /// Run one probe with setup inside the outcome catcher and retain every
    /// behavioral failure as report data.
    pub fn run_case(
        &self,
        name: &str,
        probe: &str,
        expected: ExpectedOutcome,
    ) -> Result<OracleBatchReport, String> {
        let neomacs = EmacsRuntime::neomacs()
            .with_env(
                "NEOMACS_PACKAGE_USER_DIR",
                self.package_user_dir.as_os_str(),
            )
            .with_env("NEOMACS_PACKAGE_SOURCE", self.source_file.as_os_str())
            .with_timeout(self.timeout);
        let gnu_emacs = EmacsRuntime::gnu_emacs()
            .with_env(
                "NEOMACS_PACKAGE_USER_DIR",
                self.package_user_dir.as_os_str(),
            )
            .with_env("NEOMACS_PACKAGE_SOURCE", self.source_file.as_os_str())
            .with_timeout(self.timeout);
        let observed = run_elisp_oracle_case(
            &neomacs,
            &gnu_emacs,
            name,
            &self.package_setup_elisp(),
            probe,
        )?;
        let mut failures = Vec::new();
        if observed.neomacs != observed.gnu_emacs {
            failures.push(OracleBatchFailure::OutcomeMismatch {
                id: name.to_string(),
                neomacs: observed.neomacs.clone(),
                gnu_emacs: observed.gnu_emacs.clone(),
            });
        }
        for (editor, actual) in [
            (OracleEditor::GnuEmacs, &observed.gnu_emacs),
            (OracleEditor::Neomacs, &observed.neomacs),
        ] {
            if !expected.matches(actual) {
                failures.push(OracleBatchFailure::UnexpectedOutcome {
                    id: name.to_string(),
                    editor,
                    expected,
                    actual: actual.clone(),
                });
            }
        }
        Ok(OracleBatchReport {
            cases: vec![OracleBatchCaseReport {
                id: name.to_string(),
                neomacs: observed.neomacs,
                gnu_emacs: observed.gnu_emacs,
            }],
            failures,
        })
    }

    /// Run many named probes in one GNU Emacs process and one Neomacs process.
    ///
    /// Shared package setup (`package-initialize`, load, prelude) runs once per
    /// editor. Probes emit separate outcome markers; a signal in one probe does
    /// not stop later probes. GNU Emacs and Neomacs evaluations run in parallel.
    pub fn run_batch(
        &self,
        batch_name: &str,
        cases: &[OracleBatchCase<'_>],
    ) -> Result<OracleBatchReport, String> {
        if cases.is_empty() {
            return Err(format!(
                "{} batch `{batch_name}` requires at least one probe",
                self.package_name
            ));
        }
        let probes: Vec<BatchProbe<'_>> = cases
            .iter()
            .map(|case| BatchProbe {
                id: case.id,
                probe: case.probe,
            })
            .collect();
        let neomacs = EmacsRuntime::neomacs()
            .with_env(
                "NEOMACS_PACKAGE_USER_DIR",
                self.package_user_dir.as_os_str(),
            )
            .with_env("NEOMACS_PACKAGE_SOURCE", self.source_file.as_os_str())
            .with_timeout(self.timeout);
        let gnu_emacs = EmacsRuntime::gnu_emacs()
            .with_env(
                "NEOMACS_PACKAGE_USER_DIR",
                self.package_user_dir.as_os_str(),
            )
            .with_env("NEOMACS_PACKAGE_SOURCE", self.source_file.as_os_str())
            .with_timeout(self.timeout);
        let setup = self.package_setup_elisp();
        let mut report = run_elisp_oracle_batch(&neomacs, &gnu_emacs, batch_name, &setup, &probes)?;
        for (case, observed) in cases.iter().zip(report.cases.iter()) {
            for (editor, actual) in [
                (OracleEditor::GnuEmacs, &observed.gnu_emacs),
                (OracleEditor::Neomacs, &observed.neomacs),
            ] {
                if !case.expected_outcome.matches(actual) {
                    report.failures.push(OracleBatchFailure::UnexpectedOutcome {
                        id: case.id.to_string(),
                        editor,
                        expected: case.expected_outcome,
                        actual: actual.clone(),
                    });
                }
            }
        }
        Ok(report)
    }

    fn package_setup_elisp(&self) -> String {
        let package_directory_list = self
            .package_directory_list
            .iter()
            .map(|directory| elisp_string(&directory.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(" ");
        let package_load_list = self
            .package_load_list
            .iter()
            .map(|(name, version)| {
                format!(
                    "(list (intern {}) {})",
                    elisp_string(name),
                    elisp_string(version)
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            r##"(progn
                   (require 'package)
                   (setq package-user-dir
                         (getenv "NEOMACS_PACKAGE_USER_DIR")
                         package-directory-list
                         (list {package_directory_list})
                         package-load-list
                         (list 'all {package_load_list})
                         load-suffixes '(".el"))
                   (package-initialize)
                   {}
                   {})"##,
            self.prelude,
            package_activation_elisp(self.activation)
        )
    }

    fn run_expected(
        &self,
        name: &str,
        probe: &str,
        expected_outcome: ExpectedOutcome,
    ) -> Result<ElispOracleReport, String> {
        let mut report = self.run_case(name, probe, expected_outcome)?;
        if !report.failures.is_empty() {
            return Err(format!(
                "{} parity case `{name}` failed:\n{}",
                self.package_name,
                report
                    .failures
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        let report = report.cases.remove(0);
        Ok(ElispOracleReport {
            neomacs: report.neomacs,
            gnu_emacs: report.gnu_emacs,
        })
    }
}

fn validate_cached_source_file_name(
    archive_label: &str,
    source_file_name: &str,
) -> Result<(), String> {
    let mut components = Path::new(source_file_name).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(format!(
            "cached {archive_label} source must be one file name, got `{source_file_name}`"
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ErtSummary {
    pub total: usize,
    pub expected: usize,
    pub unexpected: usize,
    pub skipped: usize,
}

#[derive(Debug)]
pub struct ErtReport {
    pub runtime: String,
    pub scenario: String,
    pub phase: PhaseReport,
    pub summary: ErtSummary,
}

#[derive(Debug)]
pub struct PackageVcReport {
    pub runtime: String,
    pub phases: Vec<PhaseReport>,
    pub checkpoints: Vec<String>,
}

struct PackageVcProgress {
    phases: Vec<PhaseReport>,
    checkpoints: Vec<String>,
}

impl PackageVcProgress {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            phases: Vec::with_capacity(capacity),
            checkpoints: Vec::with_capacity(capacity),
        }
    }
}

impl fmt::Display for ScenarioReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "{} scenario `{}` installed: {}",
            self.runtime,
            self.scenario,
            format_installed_packages(&self.installed_packages)
        )?;
        for phase in &self.phases {
            writeln!(
                formatter,
                "{:?}: status {:?}, {:.2?}",
                phase.phase, phase.status_code, phase.duration
            )?;
        }
        write!(formatter, "outcome: {}", self.outcome)
    }
}

impl fmt::Display for ErtReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ERT scenario `{}`: {} total, {} expected, {} unexpected, {} skipped ({:.2?})",
            self.runtime,
            self.scenario,
            self.summary.total,
            self.summary.expected,
            self.summary.unexpected,
            self.summary.skipped,
            self.phase.duration
        )
    }
}

impl fmt::Display for PackageVcReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "{} package-vc lifecycle: {}",
            self.runtime,
            self.checkpoints.join(" -> ")
        )?;
        for phase in &self.phases {
            writeln!(
                formatter,
                "{:?}: status {:?}, {:.2?}",
                phase.phase, phase.status_code, phase.duration
            )?;
        }
        Ok(())
    }
}

/// Load an Emacs Lisp test file and run one ERT selector inside an isolated
/// editor process.
pub fn run_ert_scenario(
    runtime: &EmacsRuntime,
    scenario: &ErtScenario,
) -> Result<ErtReport, String> {
    if !scenario.test_file.is_file() {
        return Err(format!(
            "ERT scenario `{}` test file does not exist: {}",
            scenario.name,
            scenario.test_file.display()
        ));
    }

    let sandbox = MelpaSandbox::new(&scenario.name)?;
    let load_directory = scenario
        .test_file
        .parent()
        .expect("ERT test files have a parent directory");
    let eval = format!(r##"(ert-run-tests-batch {})"##, scenario.selector);
    let mut command = runtime.command();
    sandbox.configure(&mut command);
    command
        .env("NEOMACS_RUNTIME_ROOT", workspace_root())
        .args(["--batch", "--quick", "-L"])
        .arg(load_directory)
        .arg("-l")
        .arg(&scenario.test_file)
        .args(["--eval", &eval]);

    let started = Instant::now();
    let output = output_with_timeout(&mut command, runtime.timeout).map_err(|error| {
        command_error_message(error, runtime, &sandbox, &scenario.name, ScenarioPhase::Ert)
    })?;
    let phase = phase_report(ScenarioPhase::Ert, started.elapsed(), output);
    if phase.status_code != Some(0) {
        return Err(format!(
            "{} ERT scenario `{}` failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
            runtime.name, scenario.name, phase.status_code, phase.stdout, phase.stderr
        ));
    }
    let summary = extract_ert_summary(&phase.stdout, &phase.stderr).ok_or_else(|| {
        format!(
            "{} ERT scenario `{}` did not emit an ERT summary\nstdout:\n{}\nstderr:\n{}",
            runtime.name, scenario.name, phase.stdout, phase.stderr
        )
    })?;
    if summary.unexpected != 0 {
        return Err(format!(
            "{} ERT scenario `{}` reported {} unexpected result(s)\nstdout:\n{}\nstderr:\n{}",
            runtime.name, scenario.name, summary.unexpected, phase.stdout, phase.stderr
        ));
    }

    Ok(ErtReport {
        runtime: runtime.name.clone(),
        scenario: scenario.name.clone(),
        phase,
        summary,
    })
}

/// Install a scenario's packages, exit the editor, and probe them in a fresh
/// process using the same isolated home.
pub fn run_scenario(
    runtime: &EmacsRuntime,
    source: &PackageSource,
    scenario: &PackageScenario,
) -> Result<ScenarioReport, String> {
    run_install_and_probe(
        runtime,
        scenario,
        install_form(source, &scenario.package_names(), ""),
        probe_form(&scenario.probe),
        ScenarioPhase::RestartProbe,
    )
}

/// Install one exact GNU ELPA package into a validated, cross-process cache.
///
/// Like the MELPA cache, this remains a workspace-local runtime artifact.
pub fn prepare_cached_gnu_elpa_package(
    gnu_emacs: &EmacsRuntime,
    package: (&str, &str),
) -> Result<PathBuf, String> {
    prepare_cached_package(gnu_emacs, package, GNU_ELPA_ARCHIVE)
}

fn package_preparation_run_id() -> String {
    std::env::var("NEXTEST_RUN_ID").unwrap_or_else(|_| format!("process-{}", std::process::id()))
}

fn publish_package_preparation_failure(
    failed_marker: &Path,
    failure_prefix: &str,
    error: String,
) -> String {
    let marker_tmp = failed_marker.with_extension(format!("{}.tmp", std::process::id()));
    let contents = format!("{failure_prefix}{error}");
    if let Err(cache_error) =
        fs::write(&marker_tmp, contents).and_then(|()| fs::rename(&marker_tmp, failed_marker))
    {
        return format!(
            "{error}\nfailed to publish shared package preparation failure {}: {cache_error}",
            failed_marker.display()
        );
    }
    error
}

/// Build one exact Tree-sitter grammar into a cross-process cache below
/// `<workspace>/tmp/melpa/tree-sitter-grammar-cache`.
///
/// GNU Emacs performs the build through its native grammar installer so the
/// compiler and shared-library conventions match the host platform. The
/// returned directory can be added to `treesit-extra-load-path` for both
/// editor adapters.
pub fn prepare_cached_tree_sitter_grammar(
    gnu_emacs: &EmacsRuntime,
    language: &str,
    repository: &str,
    revision: &str,
) -> Result<PathBuf, String> {
    prepare_cached_tree_sitter_grammar_with_source_directory(
        gnu_emacs, language, repository, revision, None,
    )
}

/// Build one exact Tree-sitter grammar whose generated sources live below a
/// repository subdirectory.
///
/// This supports grammar monorepositories such as
/// `cathaysia/tree-sitter-asciidoc`, which contains separate block and inline
/// grammars under distinct source directories.
pub fn prepare_cached_tree_sitter_grammar_from_subdirectory(
    gnu_emacs: &EmacsRuntime,
    language: &str,
    repository: &str,
    revision: &str,
    source_directory: &str,
) -> Result<PathBuf, String> {
    prepare_cached_tree_sitter_grammar_with_source_directory(
        gnu_emacs,
        language,
        repository,
        revision,
        Some(source_directory),
    )
}

fn prepare_cached_tree_sitter_grammar_with_source_directory(
    gnu_emacs: &EmacsRuntime,
    language: &str,
    repository: &str,
    revision: &str,
    source_directory: Option<&str>,
) -> Result<PathBuf, String> {
    let source_directory_is_safe = source_directory.is_none_or(|directory| {
        let path = Path::new(directory);
        !directory.is_empty()
            && !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_)))
    });
    if language.is_empty()
        || !language
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        || !repository.starts_with("https://github.com/")
        || revision.len() != 40
        || !revision
            .chars()
            .all(|character| character.is_ascii_hexdigit())
        || !source_directory_is_safe
    {
        return Err(format!(
            "cached Tree-sitter grammar requires a safe language, GitHub repository, full revision, and optional source directory, got `{language}` `{repository}` `{revision}` `{source_directory:?}`"
        ));
    }

    let root = workspace_root()
        .join("tmp/melpa/tree-sitter-grammar-cache")
        .join(language)
        .join(revision);
    fs::create_dir_all(&root).map_err(|error| {
        format!(
            "failed to create Tree-sitter grammar cache root {}: {error}",
            root.display()
        )
    })?;
    let lock_path = root.join("prepare.lock");
    let lock = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| {
            format!(
                "failed to open Tree-sitter grammar cache lock {}: {error}",
                lock_path.display()
            )
        })?;
    fs4::FileExt::lock(&lock).map_err(|error| {
        format!(
            "failed to lock Tree-sitter grammar cache {}: {error}",
            root.display()
        )
    })?;

    let home = root.join("home");
    let tmp = root.join("tmp");
    let source = root.join("source");
    let grammar_dir = home.join(".emacs.d/tree-sitter");
    let ready_marker = root.join("ready");
    let source_directory_marker = source_directory.unwrap_or("");
    let expected_marker =
        format!("{language}\t{repository}\t{revision}\t{source_directory_marker}\n");
    let cache_is_ready = grammar_library_exists(&grammar_dir, language)
        && fs::read_to_string(&ready_marker).is_ok_and(|contents| contents == expected_marker);
    if cache_is_ready {
        return Ok(grammar_dir);
    }

    for directory in [&home, &tmp, &source] {
        if directory.exists() {
            fs::remove_dir_all(directory).map_err(|error| {
                format!(
                    "failed to remove incomplete Tree-sitter grammar cache {}: {error}",
                    directory.display()
                )
            })?;
        }
    }
    if ready_marker.exists() {
        fs::remove_file(&ready_marker).map_err(|error| {
            format!(
                "failed to remove invalid Tree-sitter grammar marker {}: {error}",
                ready_marker.display()
            )
        })?;
    }
    for directory in [
        home.join(".emacs.d"),
        tmp.clone(),
        root.join("xdg/config"),
        root.join("xdg/cache"),
        root.join("xdg/data"),
        root.join("xdg/state"),
    ] {
        fs::create_dir_all(&directory).map_err(|error| {
            format!(
                "failed to create Tree-sitter grammar cache directory {}: {error}",
                directory.display()
            )
        })?;
    }

    let source_arg = source.to_string_lossy().into_owned();
    let run_git = |arguments: &[&str]| -> Result<(), String> {
        let mut command = Command::new("git");
        configure_process_environment(&mut command, &root, &home, &tmp);
        command.args(arguments);
        let output =
            output_with_timeout(&mut command, gnu_emacs.timeout).map_err(|error| match error {
                CommandError::Launch(error) => format!(
                    "failed to launch git for cached Tree-sitter grammar `{language}`: {error}"
                ),
                CommandError::TimedOut(_) => format!(
                    "git timed out while preparing cached Tree-sitter grammar `{language}`"
                ),
                CommandError::Capture(error) => format!(
                    "failed to capture git output for cached Tree-sitter grammar `{language}`: {error}"
                ),
            })?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "git failed while preparing cached Tree-sitter grammar `{language}`\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    };
    run_git(&["init", "--quiet", &source_arg])?;
    run_git(&["-C", &source_arg, "remote", "add", "origin", repository])?;
    run_git(&[
        "-C",
        &source_arg,
        "fetch",
        "--quiet",
        "--depth",
        "1",
        "origin",
        revision,
    ])?;
    run_git(&[
        "-C",
        &source_arg,
        "checkout",
        "--quiet",
        "--detach",
        "FETCH_HEAD",
    ])?;

    let language_symbol = language;
    let source_string = elisp_string(&source_arg);
    let grammar_recipe = match source_directory {
        Some(directory) => {
            let directory = elisp_string(directory);
            format!("({language_symbol} {source_string} nil {directory})")
        }
        None => format!("({language_symbol} {source_string})"),
    };
    let form = format!(
        r##"(progn
               (require 'treesit)
               (setq user-emacs-directory
                     (file-name-as-directory
                      (expand-file-name ".emacs.d" (getenv "HOME")))
                     treesit-language-source-alist
                     '({grammar_recipe}))
               (treesit-install-language-grammar '{language_symbol})
               (unless (treesit-language-available-p '{language_symbol})
                 (error "Installed Tree-sitter grammar is unavailable: %s"
                        '{language_symbol}))
               (princ "NEOMACS-TREESIT-GRAMMAR-CACHE:ready"))"##
    );
    let mut command = gnu_emacs.command();
    configure_process_environment(&mut command, &root, &home, &tmp);
    command.args(["--batch", "--quick", "--eval", &form]);
    let output =
        output_with_timeout(&mut command, gnu_emacs.timeout).map_err(|error| match error {
            CommandError::Launch(error) => format!(
                "failed to launch {} for cached Tree-sitter grammar `{language}` in {}: {error}",
                gnu_emacs.name,
                root.display()
            ),
            CommandError::TimedOut(_) => format!(
                "{} cached Tree-sitter grammar `{language}` timed out after {:?} in {}",
                gnu_emacs.name,
                gnu_emacs.timeout,
                root.display()
            ),
            CommandError::Capture(error) => format!(
                "failed to capture {} cached Tree-sitter grammar `{language}` output: {error}",
                gnu_emacs.name
            ),
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success()
        || !stdout.contains("NEOMACS-TREESIT-GRAMMAR-CACHE:ready")
        || !grammar_library_exists(&grammar_dir, language)
    {
        return Err(format!(
            "failed to prepare cached Tree-sitter grammar {language} at {revision} below {}\nstatus: {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            root.display(),
            output.status.code()
        ));
    }

    let marker_tmp = root.join(format!("ready.{}.tmp", std::process::id()));
    fs::write(&marker_tmp, &expected_marker).map_err(|error| {
        format!(
            "failed to write Tree-sitter grammar cache marker {}: {error}",
            marker_tmp.display()
        )
    })?;
    fs::rename(&marker_tmp, &ready_marker).map_err(|error| {
        format!(
            "failed to publish Tree-sitter grammar cache marker {}: {error}",
            ready_marker.display()
        )
    })?;
    Ok(grammar_dir)
}

fn grammar_library_exists(grammar_dir: &Path, language: &str) -> bool {
    let stem = format!("tree-sitter-{language}");
    fs::read_dir(grammar_dir).is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            entry.file_type().is_ok_and(|file_type| file_type.is_file())
                && entry.file_name().to_string_lossy().contains(&stem)
        })
    })
}

fn prepare_cached_package(
    gnu_emacs: &EmacsRuntime,
    package: (&str, &str),
    archive: PackageArchiveSpec,
) -> Result<PathBuf, String> {
    let (name, version) = package;
    if name.is_empty()
        || version.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '@'))
        || !version.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '+')
        })
    {
        return Err(format!(
            "cached {} package must have a safe hard-coded name and version, got `{name}` `{version}`",
            archive.label
        ));
    }

    let root = workspace_root()
        .join("tmp/melpa")
        .join(archive.cache_directory)
        .join(name)
        .join(version);
    fs::create_dir_all(&root).map_err(|error| {
        format!(
            "failed to create package cache root {}: {error}",
            root.display()
        )
    })?;
    let lock_path = root.join("prepare.lock");
    let lock = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| {
            format!(
                "failed to open package cache lock {}: {error}",
                lock_path.display()
            )
        })?;
    fs4::FileExt::lock(&lock)
        .map_err(|error| format!("failed to lock package cache {}: {error}", root.display()))?;

    let home = root.join("home");
    let tmp = root.join("tmp");
    let package_dir = home.join(".emacs.d/elpa").join(format!("{name}-{version}"));
    let descriptor = package_dir.join(format!("{name}-pkg.el"));
    let ready_marker = root.join("ready");
    let failed_marker = root.join("failed");
    let expected_marker = format!("{name}\t{version}\n");
    let cache_is_ready = descriptor.is_file()
        && fs::read_to_string(&ready_marker).is_ok_and(|contents| contents == expected_marker);
    if cache_is_ready {
        return Ok(package_dir);
    }
    let failure_prefix = format!(
        "run-id\t{}\nidentity\t{expected_marker}error\n",
        package_preparation_run_id()
    );
    if let Ok(contents) = fs::read_to_string(&failed_marker)
        && let Some(error) = contents.strip_prefix(&failure_prefix)
    {
        return Err(error.to_string());
    }

    if home.exists() {
        fs::remove_dir_all(&home).map_err(|error| {
            format!(
                "failed to remove incomplete package cache {}: {error}",
                home.display()
            )
        })?;
    }
    if ready_marker.exists() {
        fs::remove_file(&ready_marker).map_err(|error| {
            format!(
                "failed to remove invalid package cache marker {}: {error}",
                ready_marker.display()
            )
        })?;
    }
    if failed_marker.exists() {
        fs::remove_file(&failed_marker).map_err(|error| {
            format!(
                "failed to remove stale package preparation failure {}: {error}",
                failed_marker.display()
            )
        })?;
    }
    for directory in [
        home.join(".emacs.d"),
        tmp.clone(),
        root.join("xdg/config"),
        root.join("xdg/cache"),
        root.join("xdg/data"),
        root.join("xdg/state"),
    ] {
        fs::create_dir_all(&directory).map_err(|error| {
            format!(
                "failed to create package cache directory {}: {error}",
                directory.display()
            )
        })?;
    }

    let name_string = elisp_string(name);
    let version_string = elisp_string(version);
    let archive_name_string = elisp_string(archive.name);
    let archive_url_string = elisp_string(archive.url);
    let package_archives = format!(
        r##"(list
                      (cons {archive_name_string}
                            {archive_url_string}))"##
    );
    let form = format!(
        r##"(progn
               (require 'package)
               (setq package-user-dir
                     (expand-file-name ".emacs.d/elpa" (getenv "HOME"))
                     package-check-signature nil
                     package-archives {package_archives})
               (package-refresh-contents)
               (let* ((package-name {name_string})
                      (expected-version {version_string})
                      (package-symbol (intern package-name))
                      (description
                       (cadr
                        (assq package-symbol package-archive-contents)))
                      (archive-version
                       (and description
                            (package-version-join
                             (package-desc-version description)))))
                 (unless description
                   (error "Package is absent from selected archive: %s"
                          package-name))
                 (unless (equal archive-version expected-version)
                   (error
                    "Package version changed: %s expected %s, current %s"
                    package-name expected-version archive-version))
                 (package-install description)
                 (package-initialize)
                 (let* ((installed
                         (cadr (assq package-symbol package-alist)))
                        (installed-version
                         (and installed
                              (package-version-join
                               (package-desc-version installed))))
                        (directory
                         (and installed (package-desc-dir installed)))
                        (descriptor
                         (and directory
                              (expand-file-name
                               (concat package-name "-pkg.el")
                               directory))))
                   (unless (equal installed-version expected-version)
                     (error
                      "Installed package version mismatch: %s expected %s, got %s"
                      package-name expected-version installed-version))
                   (unless (and descriptor (file-readable-p descriptor))
                     (error
                      "Installed package descriptor is unreadable: %s"
                      descriptor))))
               (princ "NEOMACS-PACKAGE-CACHE:ready"))"##
    );
    let mut command = gnu_emacs.command();
    configure_process_environment(&mut command, &root, &home, &tmp);
    command.args(["--batch", "--quick", "--eval", &form]);
    let output = match output_with_timeout(&mut command, gnu_emacs.timeout) {
        Ok(output) => output,
        Err(error) => {
            let error = match error {
                CommandError::Launch(error) => format!(
                    "failed to launch {} for cached package `{name}` in {}: {error}",
                    gnu_emacs.name,
                    root.display()
                ),
                CommandError::TimedOut(_) => format!(
                    "{} cached package `{name}` timed out after {:?} in {}",
                    gnu_emacs.name,
                    gnu_emacs.timeout,
                    root.display()
                ),
                CommandError::Capture(error) => format!(
                    "failed to capture {} cached package `{name}` output: {error}",
                    gnu_emacs.name
                ),
            };
            return Err(publish_package_preparation_failure(
                &failed_marker,
                &failure_prefix,
                error,
            ));
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success()
        || !stdout.contains("NEOMACS-PACKAGE-CACHE:ready")
        || !descriptor.is_file()
    {
        let error = format!(
            "failed to prepare cached {} package {name} {version} below {}\nstatus: {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            archive.label,
            root.display(),
            output.status.code()
        );
        return Err(publish_package_preparation_failure(
            &failed_marker,
            &failure_prefix,
            error,
        ));
    }

    let marker_tmp = root.join(format!("ready.{}.tmp", std::process::id()));
    fs::write(&marker_tmp, &expected_marker).map_err(|error| {
        format!(
            "failed to write package cache marker {}: {error}",
            marker_tmp.display()
        )
    })?;
    fs::rename(&marker_tmp, &ready_marker).map_err(|error| {
        format!(
            "failed to publish package cache marker {}: {error}",
            ready_marker.display()
        )
    })?;
    Ok(package_dir)
}

/// Run the same package lifecycle and probe against GNU Emacs and Neomacs.
///
/// The editors receive separate homes but the same package source and probe.
/// Package/version graph differences and normalized value/signal differences
/// are both oracle failures.
pub fn run_oracle_scenario(
    neomacs: &EmacsRuntime,
    gnu_emacs: &EmacsRuntime,
    source: &PackageSource,
    scenario: &PackageScenario,
) -> Result<OracleScenarioReport, String> {
    let gnu_report = run_scenario(gnu_emacs, source, scenario)
        .map_err(|error| format!("GNU Emacs baseline failed: {error}"))?;
    let neomacs_report = run_scenario(neomacs, source, scenario)
        .map_err(|error| format!("Neomacs comparison failed: {error}"))?;

    if neomacs_report.installed_packages != gnu_report.installed_packages {
        return Err(format!(
            "package graph mismatch for scenario `{}`\n  Neomacs: {}\n  GNU Emacs: {}",
            scenario.name,
            format_installed_packages(&neomacs_report.installed_packages),
            format_installed_packages(&gnu_report.installed_packages)
        ));
    }
    if neomacs_report.outcome != gnu_report.outcome {
        return Err(format!(
            "oracle outcome mismatch for scenario `{}`\n  Neomacs: {}\n  GNU Emacs: {}",
            scenario.name, neomacs_report.outcome, gnu_report.outcome
        ));
    }

    Ok(OracleScenarioReport {
        neomacs: neomacs_report,
        gnu_emacs: gnu_report,
    })
}

/// Run the same setup and Elisp form in isolated GNU Emacs and Neomacs
/// processes without installing a package.
///
/// This is useful for dense behavioral corpora that load one previously
/// prepared package source while the package lifecycle remains covered by a
/// separate scenario.
pub fn run_elisp_oracle(
    neomacs: &EmacsRuntime,
    gnu_emacs: &EmacsRuntime,
    name: &str,
    setup: &str,
    probe: &str,
) -> Result<ElispOracleReport, String> {
    let report = run_elisp_oracle_case(neomacs, gnu_emacs, name, setup, probe)?;
    if report.neomacs != report.gnu_emacs {
        return Err(format!(
            "oracle outcome mismatch for direct form `{name}`\n  Neomacs: {}\n  GNU Emacs: {}",
            report.neomacs, report.gnu_emacs
        ));
    }
    Ok(report)
}

fn run_elisp_oracle_case(
    neomacs: &EmacsRuntime,
    gnu_emacs: &EmacsRuntime,
    name: &str,
    setup: &str,
    probe: &str,
) -> Result<ElispOracleReport, String> {
    fn evaluate(
        runtime: &EmacsRuntime,
        name: &str,
        setup: &str,
        probe: &str,
    ) -> Result<EvalOutcome, String> {
        let sandbox = MelpaSandbox::new(name)?;
        let form = wrap_elisp_outcome(setup, probe, OUTCOME_MARKER);
        let phase = run_outcome_phase(runtime, &sandbox, name, ScenarioPhase::RestartProbe, &form)?;
        extract_marked_outcome(&phase.stdout, OUTCOME_MARKER).map_err(|error| {
            format!(
                "{} direct oracle `{name}` emitted an invalid outcome: {error}\nstdout:\n{}\nstderr:\n{}",
                runtime.name, phase.stdout, phase.stderr
            )
        })
    }

    let gnu_outcome = evaluate(gnu_emacs, name, setup, probe)
        .map_err(|error| format!("GNU Emacs baseline failed: {error}"))?;
    let neomacs_outcome = evaluate(neomacs, name, setup, probe)
        .map_err(|error| format!("Neomacs comparison failed: {error}"))?;
    Ok(ElispOracleReport {
        neomacs: neomacs_outcome,
        gnu_emacs: gnu_outcome,
    })
}

/// Run the same setup and many named probes in one process per editor.
///
/// GNU Emacs and Neomacs evaluations run concurrently. Each probe id must
/// appear exactly once in both editors' ordered debugging-output protocol,
/// and the outcomes must match pairwise.
pub fn run_elisp_oracle_batch(
    neomacs: &EmacsRuntime,
    gnu_emacs: &EmacsRuntime,
    batch_name: &str,
    setup: &str,
    cases: &[BatchProbe<'_>],
) -> Result<OracleBatchReport, String> {
    fn evaluate_batch(
        runtime: &EmacsRuntime,
        batch_name: &str,
        setup: &str,
        cases: &[BatchProbe<'_>],
    ) -> Result<Vec<(String, EvalOutcome)>, String> {
        let sandbox = MelpaSandbox::new(batch_name)?;
        let form = wrap_elisp_batch_outcomes(
            setup,
            cases,
            BATCH_BEGIN_MARKER,
            BATCH_COMPLETE_MARKER,
            OUTCOME_MARKER,
        )?;
        let phase = run_outcome_phase(
            runtime,
            &sandbox,
            batch_name,
            ScenarioPhase::RestartProbe,
            &form,
        )?;
        let expected_ids: Vec<&str> = cases.iter().map(|case| case.id).collect();
        let protocol = extract_marked_batch_protocol(
            &phase.stderr,
            BATCH_BEGIN_MARKER,
            OUTCOME_MARKER,
            BATCH_COMPLETE_MARKER,
        )
        .map_err(|error| {
            format!(
                "{} batch oracle `{batch_name}` emitted invalid protocol records: {error}\nstdout:\n{}\nstderr:\n{}",
                runtime.name, phase.stdout, phase.stderr
            )
        })?;
        let got_case_ids: Vec<&str> = protocol.case_ids.iter().map(String::as_str).collect();
        if got_case_ids != expected_ids {
            return Err(format!(
                "{} batch oracle `{batch_name}` ran cases {got_case_ids:?}, expected {expected_ids:?}\nstdout:\n{}\nstderr:\n{}",
                runtime.name, phase.stdout, phase.stderr
            ));
        }
        if let Some(active) = protocol.unfinished_case_id {
            return Err(format!(
                "{} batch oracle `{batch_name}` exited with unfinished case `{active}`\nstdout:\n{}\nstderr:\n{}",
                runtime.name, phase.stdout, phase.stderr
            ));
        }
        let got_ids: Vec<&str> = protocol
            .outcomes
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        if got_ids != expected_ids {
            return Err(format!(
                "{} batch oracle `{batch_name}` returned case ids {got_ids:?}, expected {expected_ids:?}\nstdout:\n{}\nstderr:\n{}",
                runtime.name, phase.stdout, phase.stderr
            ));
        }
        Ok(protocol
            .outcomes
            .into_iter()
            .map(|item| (item.id, item.outcome))
            .collect())
    }

    let (gnu_result, neomacs_result) = thread::scope(|scope| {
        let gnu_handle = scope.spawn(|| evaluate_batch(gnu_emacs, batch_name, setup, cases));
        let neomacs_handle = scope.spawn(|| evaluate_batch(neomacs, batch_name, setup, cases));
        (
            gnu_handle
                .join()
                .unwrap_or_else(|_| Err("GNU Emacs batch oracle thread panicked".into())),
            neomacs_handle
                .join()
                .unwrap_or_else(|_| Err("Neomacs batch oracle thread panicked".into())),
        )
    });

    let gnu_outcomes = gnu_result.map_err(|error| format!("GNU Emacs baseline failed: {error}"))?;
    let neomacs_outcomes =
        neomacs_result.map_err(|error| format!("Neomacs comparison failed: {error}"))?;

    if gnu_outcomes.len() != neomacs_outcomes.len() {
        return Err(format!(
            "oracle batch `{batch_name}` length mismatch: Neomacs {} cases, GNU Emacs {} cases",
            neomacs_outcomes.len(),
            gnu_outcomes.len()
        ));
    }

    let mut reports = Vec::with_capacity(cases.len());
    let mut failures = Vec::new();
    for ((gnu_id, gnu_outcome), (neo_id, neo_outcome)) in
        gnu_outcomes.into_iter().zip(neomacs_outcomes)
    {
        debug_assert_eq!(gnu_id, neo_id);
        if neo_outcome != gnu_outcome {
            failures.push(OracleBatchFailure::OutcomeMismatch {
                id: gnu_id.clone(),
                neomacs: neo_outcome.clone(),
                gnu_emacs: gnu_outcome.clone(),
            });
        }
        reports.push(OracleBatchCaseReport {
            id: gnu_id,
            neomacs: neo_outcome,
            gnu_emacs: gnu_outcome,
        });
    }
    Ok(OracleBatchReport {
        cases: reports,
        failures,
    })
}

/// Install packages, generate `package-quickstart-file`, then load that file
/// and probe package activation in a fresh editor process.
pub fn run_quickstart_scenario(
    runtime: &EmacsRuntime,
    source: &PackageSource,
    scenario: &PackageScenario,
) -> Result<ScenarioReport, String> {
    let quickstart_setup = r##"
           (setq package-quickstart t
                 package-quickstart-file
                 (expand-file-name ".emacs.d/package-quickstart.el"
                                   (getenv "HOME")))
           (package-quickstart-refresh)
           (unless (file-exists-p package-quickstart-file)
             (error "package quickstart file was not generated"))"##;
    let quickstart_probe = format!(
        r##"(progn
           (require 'package)
           (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME"))
                 package-quickstart t
                 package-quickstart-file
                 (expand-file-name ".emacs.d/package-quickstart.el"
                                   (getenv "HOME")))
           (load package-quickstart-file nil nil t)
           {})"##,
        scenario.probe
    );
    run_install_and_probe(
        runtime,
        scenario,
        install_form(source, &scenario.package_names(), quickstart_setup),
        wrap_elisp_outcome("", &quickstart_probe, OUTCOME_MARKER),
        ScenarioPhase::QuickstartProbe,
    )
}

/// Install packages, delete one archive package, then verify the resulting
/// package state in a fresh editor process.
pub fn run_delete_and_probe_scenario(
    runtime: &EmacsRuntime,
    source: &PackageSource,
    scenario: &PackageScenario,
    package_to_delete: &str,
) -> Result<ScenarioReport, String> {
    let delete_setup = format!(
        r##"
           (let* ((name (intern {}))
                  (description (cadr (assq name package-alist))))
             (unless description
               (error "package selected for deletion was not installed"))
             (package-delete description t)
             (when (package-installed-p name)
               (error "archive package remained installed after delete")))"##,
        elisp_string(package_to_delete)
    );
    run_install_and_probe(
        runtime,
        scenario,
        install_form(source, &scenario.package_names(), &delete_setup),
        probe_form(&scenario.probe),
        ScenarioPhase::RestartProbe,
    )
}

/// Exercise `package-vc` against a local Git repository through install,
/// restart, upgrade, delete, and restart-after-delete.
pub fn run_package_vc_lifecycle(runtime: &EmacsRuntime) -> Result<PackageVcReport, String> {
    let scenario_name = "offline-package-vc-lifecycle";
    let sandbox = MelpaSandbox::new(scenario_name)?;
    let repository = sandbox.root().join("neo-vc-fixture-remote");
    fs::create_dir_all(&repository).map_err(|error| {
        format!(
            "failed to create package-vc fixture repository {}: {error}",
            repository.display()
        )
    })?;
    let fixture_root = workspace_root().join("neomacs-melpa-tests/fixtures/package-vc");
    let package_file = repository.join("neo-vc-fixture.el");
    fs::copy(fixture_root.join("neo-vc-fixture-v1.el"), &package_file)
        .map_err(|error| format!("failed to seed package-vc v1 fixture: {error}"))?;
    initialize_git_fixture(&sandbox, &repository)?;

    let repository_string = elisp_string(&repository.to_string_lossy());
    let package_setup = r##"
           (require 'package)
           (require 'package-vc)
           (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME"))
                 package-archives nil
                 package-vc--archive-data-alist '((offline)))
           (package-initialize)"##;
    let install_form = format!(
        r##"(progn
           {package_setup}
           (package-vc-install
            '(neo-vc-fixture :url {repository_string} :vc-backend Git))
           (let* ((description (cadr (assq 'neo-vc-fixture package-alist)))
                  (directory (and description (package-desc-dir description)))
                  (bytecode (and directory
                                 (expand-file-name "neo-vc-fixture.elc" directory))))
             (unless (and description bytecode (file-exists-p bytecode))
               (error "package-vc did not install and compile v1")))
           (princ "{RESULT_MARKER}installed-v1"))"##
    );
    let restart_v1_form = format!(
        r##"(progn
           {package_setup}
           (unless (and (package-installed-p 'neo-vc-fixture)
                        (fboundp 'neo-vc-fixture-version)
                        (string= (neo-vc-fixture-version) "v1"))
             (error "package-vc v1 did not survive restart"))
           (princ "{RESULT_MARKER}restarted-v1"))"##
    );

    let mut progress = PackageVcProgress::with_capacity(5);
    run_checkpoint(
        runtime,
        &sandbox,
        scenario_name,
        ScenarioPhase::VcInstall,
        &install_form,
        &mut progress,
    )?;
    run_checkpoint(
        runtime,
        &sandbox,
        scenario_name,
        ScenarioPhase::VcRestart,
        &restart_v1_form,
        &mut progress,
    )?;

    fs::copy(fixture_root.join("neo-vc-fixture-v2.el"), &package_file)
        .map_err(|error| format!("failed to update package-vc v2 fixture: {error}"))?;
    git(&sandbox, &repository, ["add", "neo-vc-fixture.el"])?;
    git(&sandbox, &repository, ["commit", "-m", "fixture v2"])?;

    let upgrade_form = format!(
        r##"(progn
           {package_setup}
           (package-vc-upgrade
            (cadr (assq 'neo-vc-fixture package-alist)))
           (let ((deadline (+ (float-time) 30)))
             (while (and
                     (not
                      (equal
                       (package-desc-version
                        (cadr (assq 'neo-vc-fixture package-alist)))
                       '(2 0)))
                     (< (float-time) deadline))
               (accept-process-output nil 0.05)))
           (unless (equal
                    (package-desc-version
                     (cadr (assq 'neo-vc-fixture package-alist)))
                    '(2 0))
             (error "package-vc upgrade did not install v2"))
           (princ "{RESULT_MARKER}upgraded-v2"))"##
    );
    run_checkpoint(
        runtime,
        &sandbox,
        scenario_name,
        ScenarioPhase::VcUpgrade,
        &upgrade_form,
        &mut progress,
    )?;

    let delete_form = format!(
        r##"(progn
           {package_setup}
           (unless (and (fboundp 'neo-vc-fixture-version)
                        (string= (neo-vc-fixture-version) "v2"))
             (error "package-vc v2 did not survive restart"))
           (package-delete (cadr (assq 'neo-vc-fixture package-alist)) t)
           (when (package-installed-p 'neo-vc-fixture)
             (error "package-vc package remained installed after delete"))
           (princ "{RESULT_MARKER}deleted"))"##
    );
    run_checkpoint(
        runtime,
        &sandbox,
        scenario_name,
        ScenarioPhase::VcDelete,
        &delete_form,
        &mut progress,
    )?;

    let absent_form = format!(
        r##"(progn
           {package_setup}
           (when (or (package-installed-p 'neo-vc-fixture)
                     (fboundp 'neo-vc-fixture-version))
             (error "deleted package-vc package reappeared after restart"))
           (princ "{RESULT_MARKER}absent-after-restart"))"##
    );
    run_checkpoint(
        runtime,
        &sandbox,
        scenario_name,
        ScenarioPhase::VcRestartAfterDelete,
        &absent_form,
        &mut progress,
    )?;

    Ok(PackageVcReport {
        runtime: runtime.name.clone(),
        phases: progress.phases,
        checkpoints: progress.checkpoints,
    })
}

fn initialize_git_fixture(sandbox: &MelpaSandbox, repository: &Path) -> Result<(), String> {
    git(sandbox, repository, ["init", "--initial-branch=main"])?;
    git(
        sandbox,
        repository,
        ["config", "user.email", "melpa-test@example.invalid"],
    )?;
    git(sandbox, repository, ["config", "user.name", "MELPA Test"])?;
    git(sandbox, repository, ["add", "neo-vc-fixture.el"])?;
    git(sandbox, repository, ["commit", "-m", "fixture v1"])
}

fn git<const N: usize>(
    sandbox: &MelpaSandbox,
    repository: &Path,
    args: [&str; N],
) -> Result<(), String> {
    let mut command = Command::new("git");
    sandbox.configure(&mut command);
    let output = command
        .current_dir(repository)
        .args(args)
        .output()
        .map_err(|error| format!("failed to launch git in {}: {error}", repository.display()))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "git failed in {} (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        repository.display(),
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn run_checkpoint(
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
    form: &str,
    progress: &mut PackageVcProgress,
) -> Result<(), String> {
    let report = run_phase(runtime, sandbox, scenario_name, phase, form)?;
    let checkpoint = extract_marker(&report.stdout, RESULT_MARKER).ok_or_else(|| {
        format!(
            "{} scenario `{scenario_name}` did not emit `{RESULT_MARKER}` during {phase:?}\nstdout:\n{}\nstderr:\n{}",
            runtime.name, report.stdout, report.stderr
        )
    })?;
    progress.phases.push(report);
    progress.checkpoints.push(checkpoint);
    Ok(())
}

fn run_install_and_probe(
    runtime: &EmacsRuntime,
    scenario: &PackageScenario,
    install_form: String,
    probe_form: String,
    probe_phase: ScenarioPhase,
) -> Result<ScenarioReport, String> {
    let sandbox = MelpaSandbox::new(&scenario.name)?;
    let mut phases = Vec::with_capacity(2);

    let install = run_phase(
        runtime,
        &sandbox,
        &scenario.name,
        ScenarioPhase::Install,
        &install_form,
    )?;
    let installed_packages = extract_installed_packages(&install.stdout).map_err(|error| {
        format!(
            "{} scenario `{}` emitted an invalid installed-package report during Install: {error}\nstdout:\n{}\nstderr:\n{}",
            runtime.name, scenario.name, install.stdout, install.stderr
        )
    })?;
    if let Some(expected_packages) = scenario.package_pins() {
        for expected in expected_packages {
            let actual = installed_packages
                .iter()
                .find(|installed| installed.name == expected.name);
            if actual.map(|installed| installed.version.as_str()) != Some(expected.version.as_str())
            {
                return Err(format!(
                    "{} scenario `{}` installed an unexpected version of `{}`: expected {}, got {}",
                    runtime.name,
                    scenario.name,
                    expected.name,
                    expected.version,
                    actual
                        .map(|installed| installed.version.as_str())
                        .unwrap_or("<not installed>")
                ));
            }
        }
    }
    phases.push(install);

    let probe = run_outcome_phase(runtime, &sandbox, &scenario.name, probe_phase, &probe_form)
        .map_err(|error| {
            format!(
                "{error}\ninstalled packages: {}",
                format_installed_packages(&installed_packages)
            )
        })?;
    let outcome = extract_marked_outcome(&probe.stdout, OUTCOME_MARKER).map_err(|error| {
        format!(
            "{} scenario `{}` emitted an invalid oracle outcome during {probe_phase:?}: {error}\ninstalled packages: {}\nstdout:\n{}\nstderr:\n{}",
            runtime.name,
            scenario.name,
            format_installed_packages(&installed_packages),
            probe.stdout,
            probe.stderr
        )
    })?;
    phases.push(probe);

    Ok(ScenarioReport {
        runtime: runtime.name.clone(),
        scenario: scenario.name.clone(),
        phases,
        installed_packages,
        outcome,
    })
}

fn run_phase(
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
    form: &str,
) -> Result<PhaseReport, String> {
    run_phase_with_validation(runtime, sandbox, scenario_name, phase, form, true)
}

fn run_outcome_phase(
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
    form: &str,
) -> Result<PhaseReport, String> {
    run_phase_with_validation(runtime, sandbox, scenario_name, phase, form, false)
}

fn run_phase_with_validation(
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
    form: &str,
    check_editor_error_output: bool,
) -> Result<PhaseReport, String> {
    let form_directory = workspace_root().join("tmp/melpa/editor-forms");
    fs::create_dir_all(&form_directory).map_err(|error| {
        format!(
            "failed to create editor-form directory {}: {error}",
            form_directory.display()
        )
    })?;
    let form_file = tempfile::Builder::new()
        .prefix(&format!("{}-", sanitize_label(scenario_name)))
        .suffix(".form.el")
        .tempfile_in(&form_directory)
        .map_err(|error| {
            format!(
                "failed to create {phase:?} form for scenario `{scenario_name}` in {}: {error}",
                form_directory.display()
            )
        })?;
    fs::write(form_file.path(), form).map_err(|error| {
        format!(
            "failed to write {phase:?} form for scenario `{scenario_name}` to {}: {error}",
            form_file.path().display()
        )
    })?;
    let loader_file = tempfile::Builder::new()
        .prefix(&format!("{}-", sanitize_label(scenario_name)))
        .suffix(".loader.el")
        .tempfile_in(&form_directory)
        .map_err(|error| {
            format!(
                "failed to create {phase:?} loader for scenario `{scenario_name}` in {}: {error}",
                form_directory.display()
            )
        })?;
    let loader = format!(
        r##";;; -*- lexical-binding: t; -*-
(defun {TRANSPORTED_FORM_FUNCTION} ()
  (let ((form
         (with-temp-buffer
           (insert-file-contents (getenv "NEOMACS_MELPA_ORACLE_FORM_FILE"))
           (goto-char (point-min))
           (read (current-buffer)))))
    (eval form t)))
"##
    );
    fs::write(loader_file.path(), loader).map_err(|error| {
        format!(
            "failed to write {phase:?} loader for scenario `{scenario_name}` to {}: {error}",
            loader_file.path().display()
        )
    })?;
    let mut command = runtime.command();
    sandbox.configure(&mut command);
    command
        .env("NEOMACS_RUNTIME_ROOT", workspace_root())
        .env("NEOMACS_MELPA_ORACLE_FORM_FILE", form_file.path())
        .args(["--batch", "--quick", "--load"])
        .arg(loader_file.path())
        .args(["--eval", &format!("({TRANSPORTED_FORM_FUNCTION})")]);
    let started = Instant::now();
    let output = output_with_timeout(&mut command, runtime.timeout)
        .map_err(|error| command_error_message(error, runtime, sandbox, scenario_name, phase))?;
    let report = phase_report(phase, started.elapsed(), output);
    if report.status_code != Some(0) {
        return Err(format!(
            "{} scenario `{scenario_name}` failed during {phase:?} (exit {:?})\nstdout:\n{}\nstderr:\n{}",
            runtime.name, report.status_code, report.stdout, report.stderr
        ));
    }
    if check_editor_error_output {
        check_error_markers(&report.stdout, &report.stderr).map_err(|error| {
            format!(
                "{} scenario `{scenario_name}` failed during {phase:?}: {error}",
                runtime.name
            )
        })?;
    }
    Ok(report)
}

fn command_error_message(
    error: CommandError,
    runtime: &EmacsRuntime,
    sandbox: &MelpaSandbox,
    scenario_name: &str,
    phase: ScenarioPhase,
) -> String {
    match error {
        CommandError::Launch(error) => format!(
            "failed to launch {} for {phase:?} in scenario `{scenario_name}` sandbox {}: {error}",
            runtime.name,
            sandbox.root().display()
        ),
        CommandError::TimedOut(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let protocol_context = match extract_marked_batch_protocol(
                &stderr,
                BATCH_BEGIN_MARKER,
                OUTCOME_MARKER,
                BATCH_COMPLETE_MARKER,
            ) {
                Ok(protocol) => protocol
                    .unfinished_case_id
                    .map(|id| format!("; active case `{id}`"))
                    .unwrap_or_default(),
                Err(error) => format!("; invalid partial batch protocol: {error}"),
            };
            format!(
                "{} scenario `{scenario_name}` timed out during {phase:?} after {:?} in sandbox {}{protocol_context}\npartial stdout:\n{stdout}\npartial stderr:\n{stderr}",
                runtime.name,
                runtime.timeout,
                sandbox.root().display()
            )
        }
        CommandError::Capture(error) => format!(
            "failed to capture {} scenario `{scenario_name}` output during {phase:?}: {error}",
            runtime.name
        ),
    }
}

enum CommandError {
    Launch(std::io::Error),
    TimedOut(Output),
    Capture(String),
}

fn output_with_timeout(command: &mut Command, timeout: Duration) -> Result<Output, CommandError> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(CommandError::Launch)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CommandError::Capture("stdout pipe was not created".to_string()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| CommandError::Capture("stderr pipe was not created".to_string()))?;
    let stdout_reader = thread::spawn(move || read_pipe(stdout));
    let stderr_reader = thread::spawn(move || read_pipe(stderr));

    let status = match child.wait_timeout(timeout).map_err(CommandError::Launch)? {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let status = child.wait().map_err(CommandError::Launch)?;
            let stdout = stdout_reader
                .join()
                .map_err(|_| CommandError::Capture("stdout reader panicked".to_string()))?
                .map_err(|error| {
                    CommandError::Capture(format!("failed to read stdout: {error}"))
                })?;
            let stderr = stderr_reader
                .join()
                .map_err(|_| CommandError::Capture("stderr reader panicked".to_string()))?
                .map_err(|error| {
                    CommandError::Capture(format!("failed to read stderr: {error}"))
                })?;
            return Err(CommandError::TimedOut(Output {
                status,
                stdout,
                stderr,
            }));
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| CommandError::Capture("stdout reader panicked".to_string()))?
        .map_err(|error| CommandError::Capture(format!("failed to read stdout: {error}")))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| CommandError::Capture("stderr reader panicked".to_string()))?
        .map_err(|error| CommandError::Capture(format!("failed to read stderr: {error}")))?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_pipe(mut pipe: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    pipe.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn phase_report(phase: ScenarioPhase, duration: Duration, output: Output) -> PhaseReport {
    PhaseReport {
        phase,
        duration,
        status_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn extract_ert_summary(stdout: &str, stderr: &str) -> Option<ErtSummary> {
    stdout
        .lines()
        .chain(stderr.lines())
        .filter_map(parse_ert_summary_line)
        .next_back()
}

fn parse_ert_summary_line(line: &str) -> Option<ErtSummary> {
    let fields = line
        .trim()
        .trim_end_matches(')')
        .split_once("Ran ")?
        .1
        .split_whitespace()
        .map(|field| field.trim_end_matches(','))
        .collect::<Vec<_>>();
    if fields.get(1) != Some(&"tests") || fields.get(3..6) != Some(&["results", "as", "expected"]) {
        return None;
    }
    Some(ErtSummary {
        total: fields.first()?.parse().ok()?,
        expected: fields.get(2)?.parse().ok()?,
        unexpected: count_before(&fields, "unexpected").unwrap_or(0),
        skipped: count_before(&fields, "skipped").unwrap_or(0),
    })
}

fn count_before(fields: &[&str], label: &str) -> Option<usize> {
    let index = fields.iter().position(|field| *field == label)?;
    fields.get(index.checked_sub(1)?)?.parse().ok()
}

fn install_form(source: &PackageSource, packages: &[&str], post_install: &str) -> String {
    let installs = packages
        .iter()
        .map(|package| format!("(package-install '{package})"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r##"(progn
           (require 'package)
           (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME"))
                 package-archives {}
                 package-check-signature nil)
           (package-initialize)
           (package-refresh-contents)
           {}
           {}
           (let ((installed
                  (mapcar
                   (lambda (entry)
                     (cons (car entry)
                           (package-version-join
                            (package-desc-version (cadr entry)))))
                   package-alist)))
             (setq installed
                   (sort installed
                         (lambda (left right)
                           (string< (symbol-name (car left))
                                    (symbol-name (car right))))))
             (dolist (entry installed)
               (princ "\n{INSTALLED_MARKER}")
               (princ (symbol-name (car entry)))
               (princ "\t")
               (princ (cdr entry)))))"##,
        source.archive_form(),
        installs,
        post_install
    )
}

fn probe_form(probe: &str) -> String {
    let setup = r##"
           (require 'package)
           (setq package-user-dir (expand-file-name ".emacs.d/elpa" (getenv "HOME")))
           (package-initialize)"##;
    wrap_elisp_outcome(setup, probe, OUTCOME_MARKER)
}

fn extract_marker(stdout: &str, marker: &str) -> Option<String> {
    stdout
        .lines()
        .filter_map(|line| line.split_once(marker).map(|(_, value)| value.trim()))
        .next_back()
        .map(str::to_string)
}

fn extract_installed_packages(stdout: &str) -> Result<Vec<InstalledPackage>, String> {
    let mut installed = Vec::new();
    for value in stdout
        .lines()
        .filter_map(|line| line.split_once(INSTALLED_MARKER).map(|(_, value)| value))
    {
        let (name, version) = value.trim().split_once('\t').ok_or_else(|| {
            format!(r##"expected `{INSTALLED_MARKER}<name>\t<version>`, got `{value}`"##)
        })?;
        installed.push(InstalledPackage {
            name: name.to_string(),
            version: version.to_string(),
        });
    }
    if installed.is_empty() {
        return Err(format!("did not emit `{INSTALLED_MARKER}`"));
    }
    Ok(installed)
}

fn format_installed_packages(installed: &[InstalledPackage]) -> String {
    installed
        .iter()
        .map(|package| format!("{}@{}", package.name, package.version))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn elisp_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn check_error_markers(stdout: &str, stderr: &str) -> Result<(), String> {
    for needle in [
        "wrong-type-argument",
        "void-function",
        "file-missing",
        "invalid-read-syntax",
        "end-of-file",
        "Error:",
    ] {
        if stdout.contains(needle) || stderr.contains(needle) {
            return Err(format!(
                "editor emitted `{needle}`:\nstdout:\n{stdout}\nstderr:\n{stderr}"
            ));
        }
    }
    Ok(())
}

/// The path to the `neomacs` binary (override with `NEOMACS_BIN`).
pub fn neomacs_binary() -> PathBuf {
    std::env::var_os("NEOMACS_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root().join("target/release/neomacs"))
}

#[cfg(test)]
mod parity_tests;
