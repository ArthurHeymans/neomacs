use expect_test::expect;

use super::assert_align_cljlet_parity;

#[test]
fn align_cljlet_calculates_all_three_defroute_column_widths_for_real_routes() {
    let elisp_form = r##"(mapcar
 (lambda (source)
   (with-temp-buffer
     (clojure-mode)
     (insert source)
     (goto-char (point-min))
     (down-list)
     (list (acl-calc-route-widths)
           (point)
           (thing-at-point 'sexp t))))
 '("(defroutes app\n(GET \"/\" [] home)\n(DELETE \"/articles/:id\" [id] (delete-article id)))"
   "(defroutes api\n(POST \"/v1/users\" [request] (create-user request))\n(PATCH \"/v1/users/:user-id/preferences\" [user-id body] (update-preferences user-id body))\n(GET \"/health\" [] {:status :ok}))"))"##;
    let expect = expect![[r#"OK (((6 15 4) 2 "defroutes") ((5 32 14) 2 "defroutes"))"#]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_defroute_columns_customization_aligns_one_two_or_three_columns() {
    let elisp_form = r##"(mapcar
 (lambda (columns)
   (with-temp-buffer
     (clojure-mode)
     (insert "(defroutes application\n(GET \"/\" [] (home))\n(DELETE \"/articles/:article-id\" [article-id] (delete-article article-id))\n(POST \"/articles\" [request] (create-article request)))")
     (goto-char (point-min))
     (let ((defroute-columns columns))
       (align-cljlet))
     (list columns (buffer-string))))
 '(0 1 2 3 5))"##;
    let expect = expect![[
        r#"OK ((0 "(defroutes application\n  (GET \"/\" [] (home))\n  (DELETE \"/articles/:article-id\" [article-id] (delete-article article-id))\n  (POST \"/articles\" [request] (create-article request)))") (1 "(defroutes application\n  (GET    \"/\" [] (home))\n  (DELETE \"/articles/:article-id\" [article-id] (delete-article article-id))\n  (POST   \"/articles\" [request] (create-article request)))") (2 "(defroutes application\n  (GET    \"/\"                     [] (home))\n  (DELETE \"/articles/:article-id\" [article-id] (delete-article article-id))\n  (POST   \"/articles\"             [request] (create-article request)))") (3 "(defroutes application\n  (GET    \"/\"                     []           (home))\n  (DELETE \"/articles/:article-id\" [article-id] (delete-article article-id))\n  (POST   \"/articles\"             [request]    (create-article request)))") (5 "(defroutes application\n  (GET    \"/\"                     []           (home))\n  (DELETE \"/articles/:article-id\" [article-id] (delete-article article-id))\n  (POST   \"/articles\"             [request]    (create-article request)))"))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_respaces_defroute_rows_then_reindents_the_complete_region() {
    let elisp_form = r##"(mapcar
 (lambda (widths)
   (with-temp-buffer
     (clojure-mode)
     (insert "(defroutes app\n(GET \"/\" [] (home))\n(DELETE \"/items/:id\" [id] (delete-item id)))")
     (goto-char (point-min))
     (down-list)
     (acl-respace-defroute-form widths)
     (list widths (buffer-string) (point))))
 '((8) (8 14) (8 14 8)))"##;
    let expect = expect![[
        r#"OK (((8) "(defroutes app\n  (GET      \"/\" [] (home))\n  (DELETE   \"/items/:id\" [id] (delete-item id)))" 90) ((8 14) "(defroutes app\n  (GET      \"/\"            [] (home))\n  (DELETE   \"/items/:id\"   [id] (delete-item id)))" 103) ((8 14 8) "(defroutes app\n  (GET      \"/\"            []       (home))\n  (DELETE   \"/items/:id\"   [id]     (delete-item id)))" 113))"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}

#[test]
fn align_cljlet_aligns_nested_and_multiline_route_handlers_without_changing_expressions() {
    let elisp_form = r##"(with-temp-buffer
  (clojure-mode)
  (insert "(defroutes api\n(GET \"/articles/:id\" [id]\n  (if-let [article (find-article id)]\n    (render article)\n    not-found))\n(POST \"/articles\" [request]\n  (-> request parse-body create-article respond))\n(ANY \"/fallback\" [] fallback-handler))")
  (goto-char (point-min))
  (let ((defroute-columns 3))
    (align-cljlet))
  (list (buffer-string)
        (how-many "find-article" (point-min) (point-max))
        (how-many "create-article" (point-min) (point-max))
        (how-many "fallback-handler" (point-min) (point-max))))"##;
    let expect = expect![[
        r#"OK ("(defroutes api\n  (GET  \"/articles/:id\" [id]\n        (if-let [article (find-article id)]\n          (render article)\n          not-found))\n  (POST \"/articles\"     [request]\n        (-> request parse-body create-article respond))\n  (ANY  \"/fallback\"     []        fallback-handler))" 1 1 1)"#
    ]];
    assert_align_cljlet_parity(elisp_form, expect);
}
