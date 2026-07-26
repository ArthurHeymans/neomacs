use expect_test::expect;

use super::assert_ac_php_core_parity;

#[test]
fn ac_php_core_ports_every_upstream_parser_ert_fixture() {
    let elisp_form = r##"(mapcar
               (lambda (line)
                 (list
                  line
                  (ac-php-remove-unnecessary-items-4-complete-method
                   (ac-php-split-line-4-complete-method
                    line))))
               '(" this->asdfa ( \t (new class1( ))->run()->ss"
                 " $this->func"
                 "this"
                 "return this->sdfa&& this->ttt->ss"
                 "return this->sdfa ||  ClassT::getV"
                 "return (($this->tt())->kk())->ss "
                 "\"sdfa\" => $this->tt "
                 "$ss > $this->tt "
                 "  } else  if ($role   == Erole:: "
                 "$ss <= $this->tt "
                 "$this->ss(0 <= $this->tt)->kk "
                 "$this->ss? this->tt "
                 "   \t  tt "
                 "   \t  if (this->ss?this->tt "
                 "   \t  if (this->ss?this->tt:this->kk "
                 "   \t  parent::ss"
                 "   \t $v >= $ff? \"sdfa\" : parent::ss . parent::xx"
                 "(yii\\web\\Application(config))->ru"))"##;
    let expect = expect![[
        r#"OK ((" this->asdfa ( \11 (new class1( ))->run()->ss" ("class1(" "." "run(" "." "ss")) (" $this->func" ("this" "." "func")) ("this" ("this")) ("return this->sdfa&& this->ttt->ss" ("this" "." "ttt" "." "ss")) ("return this->sdfa ||  ClassT::getV" ("ClassT::" "." "getV")) ("return (($this->tt())->kk())->ss " ("this" "." "tt(" "." "kk(" "." "ss")) ("\"sdfa\" => $this->tt " ("this" "." "tt")) ("$ss > $this->tt " ("this" "." "tt")) ("  } else  if ($role   == Erole:: " ("Erole::" ".")) ("$ss <= $this->tt " ("this" "." "tt")) ("$this->ss(0 <= $this->tt)->kk " ("this" "." "ss(" "." "kk")) ("$this->ss? this->tt " ("this" "." "tt")) ("   \11  tt " ("tt")) ("   \11  if (this->ss?this->tt " ("this" "." "tt")) ("   \11  if (this->ss?this->tt:this->kk " ("this" "." "kk")) ("   \11  parent::ss" ("parent::" "." "ss")) ("   \11 $v >= $ff? \"sdfa\" : parent::ss . parent::xx" ("parent::" "." "xx")) ("(yii\\web\\Application(config))->ru" ("yii\\web\\Application(" "." "ru")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_parser_helpers_cover_separators_nested_nodes_points_and_semicolons() {
    let elisp_form = r##"(list
               (mapcar
                (lambda (arguments)
                  (apply
                   #'ac-php-split-string-with-separator
                   arguments))
                '(("abc.def.g" "\\." ".")
                  ("abc.def." "\\." "." nil)
                  (".abc..def." "\\." "." t)
                  ("plain" "\\." ".")
                  (nil "\\." ".")))
               (mapcar
                (lambda (arguments)
                  (apply
                   #'ac-php--get-clean-node
                   arguments))
                '((("A" ";" "B" "C"))
                  (("A" "B" "C" "D") 2)
                  ((nil ";" nil "tail"))
                  (())))
               (mapcar
                #'ac-php--get-node-parser-data
                '((("outer")
                   ("inner" "__POINT__"))
                  ("A" ";" "B" "__POINT__")
                  ("A" ("B" ("C" "__POINT__")))
                  ("A" "B")))
               (mapcar
                #'ac-php--get-key-list-from-parser-data
                '((("factory")
                   "member"
                   ("argument")
                   "tail")
                  ("root"
                   "property")
                  ((("nested")
                    "__POINT__")
                   "."
                   "leaf")
                  ("call"
                   ("argument")))))"##;
    let expect = expect![[
        r#"OK ((("abc" "." "def" "." "g") ("abc" "." "def" "." "") ("." "abc" "." "." "def" ".") ("plain") nil) (("B" "C") ("A" "B") (nil "tail") nil) (("inner") ("B") ("C") nil) (("factory" "member(" "tail") ("root" "property") ("nested" "__POINT__" "." "leaf") ("call(")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}

#[test]
fn ac_php_core_tokenizer_and_parser_cover_nullsafe_static_callable_and_literal_edges() {
    let elisp_form = r##"(mapcar
               (lambda (line)
                 (let ((tokens
                        (ac-php-split-line-4-complete-method
                         line)))
                   (list
                    line
                    tokens
                    (ac-php-remove-unnecessary-items-4-complete-method
                     tokens))))
               '("$service?->client?->send($request)->status"
                 "self :: instance() :: child() -> value"
                 "array($handler, \"run\")"
                 "[$handler, 'run']"
                 "$object->method(\"a.b\", $x >= 2)->next"
                 "yield new \\Acme\\Factory($config)"
                 "$left !== $right ? $yes : $no"
                 "$items[0]->property"
                 "$object->{dynamic}->method()"
                 "case Foo::BAR: $this->value"))"##;
    let expect = expect![[
        r#"OK (("$service?->client?->send($request)->status" ("service" "." "client" "." "send" "(" "request" ")" "." "status") ("service" "." "client" "." "send(" "." "status")) ("self :: instance() :: child() -> value" ("self::" "." "instance" "(" ")" "::" "." "child" "(" ")" "." "value") ("self::" "." "instance(" "::" "." "child(" "." "value")) ("array($handler, \"run\")" ("array" "(" "handler" ";" "string" ")") ("array(")) ("[$handler, 'run']" ("(" "handler" ";" "'run'" ")") ("'run'")) ("$object->method(\"a.b\", $x >= 2)->next" ("object" "." "method" "(" "string" ";" "x" ";" "2" ")" "." "next") ("object" "." "method(" "." "next")) ("yield new \\Acme\\Factory($config)" (";" ";" "\\Acme\\Factory" "(" "config" ")") ("\\Acme\\Factory(")) ("$left !== $right ? $yes : $no" ("left" ";" ";" "right" ";" "yes" ";" "no") ("no")) ("$items[0]->property" ("items" "(" "0" ")" "." "property") ("items(" "." "property")) ("$object->{dynamic}->method()" ("object" "." ";" "dynamic" ";" "." "method" "(" ")") ("." "method(")) ("case Foo::BAR: $this->value" (";" "Foo::" "." "BAR" ";" "this" "." "value") ("this" "." "value")))"#
    ]];

    assert_ac_php_core_parity(elisp_form, expect);
}
