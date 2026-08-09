use expect_test::expect;

use super::ParityBatchCase;

fn mode_activates_with_lighter_and_local_map() -> ParityBatchCase {
    ParityBatchCase::value(
        "mode_activates_with_lighter_and_local_map",
        r####"
(with-temp-buffer
  (restclient-mode)
  (list :mode major-mode
        :lighter mode-name
        :local-map-p (keymapp (current-local-map))))
"####,
        expect![[r#"OK (:mode restclient-mode :lighter "REST Client" :local-map-p t)"#]],
    )
}

fn defcustoms_match_upstream_defaults() -> ParityBatchCase {
    ParityBatchCase::value(
        "defcustoms_match_upstream_defaults",
        r####"
(list :log-request restclient-log-request
      :same-buf restclient-same-buffer-response
      :threshold restclient-response-size-threshold
      :vars-max restclient-vars-max-passes
      :inhibit-cookies restclient-inhibit-cookies)
"####,
        expect![[
            r#"OK (:log-request t :same-buf t :threshold 100000 :vars-max 10 :inhibit-cookies nil)"#
        ]],
    )
}

fn content_type_modes_map_mime_to_major_modes() -> ParityBatchCase {
    ParityBatchCase::value(
        "content_type_modes_map_mime_to_major_modes",
        r####"
(list :xml (cdr (assoc "text/xml" restclient-content-type-modes))
      :json (cdr (assoc "application/json" restclient-content-type-modes))
      :png (cdr (assoc "image/png" restclient-content-type-modes))
      :plain (cdr (assoc "text/plain" restclient-content-type-modes)))
"####,
        expect![[r#"OK (:xml xml-mode :json js-mode :png image-mode :plain text-mode)"#]],
    )
}

pub(super) fn workflow_batch_cases() -> Vec<ParityBatchCase> {
    vec![
        mode_activates_with_lighter_and_local_map(),
        defcustoms_match_upstream_defaults(),
        content_type_modes_map_mime_to_major_modes(),
    ]
}
