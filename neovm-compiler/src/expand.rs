use std::collections::{HashMap, HashSet};

use crate::diagnostic::Diagnostic;
use crate::expand_eval::{MacroEnv, MacroEval};
use crate::expand_value::{MacroValue, surface_to_value, value_to_surface};
use crate::source::{SourceId, Span};
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

enum Work {
    Expand(SurfaceForm),
    RejoinDotted(Span, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExpandOutput {
    pub forms: Vec<SurfaceForm>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Tracks compiler state across multiple source files, enabling
/// `require`-based macro import and feature tracking.
pub struct CompilerSession {
    macros: HashMap<String, MacroDef>,
    symbol_macros: HashMap<String, SurfaceForm>,
    loaded_features: HashSet<String>,
    loading_stack: Vec<String>,
    builtin_libraries: HashMap<String, &'static str>,
    load_paths: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    /// Expanded forms from builtin sources, prepended to every
    /// file's forms so runtime function definitions are compiled.
    builtin_forms: Vec<SurfaceForm>,
}

impl Default for CompilerSession {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerSession {
    pub fn new() -> Self {
        let mut session = Self {
            macros: HashMap::new(),
            symbol_macros: HashMap::new(),
            loaded_features: HashSet::new(),
            loading_stack: Vec::new(),
            builtin_libraries: HashMap::new(),
            load_paths: vec!["lisp".to_string(), "lisp/emacs-lisp".to_string()],
            diagnostics: Vec::new(),
            builtin_forms: Vec::new(),
        };
        session.register_builtin("cl-lib", crate::builtin_libs::CL_LIB_SOURCE);
        // Builtin core macros that are always available (from subr.el).
        session.register_builtin(
            "core-macros",
            crate::builtin_libs::CORE_MACROS_SOURCE,
        );
        // Register core runtime functions (loadable via require).
        session.register_builtin(
            "core-functions",
            crate::builtin_libs::CORE_FUNCTIONS_SOURCE,
        );
        // Load core macros into the session immediately.
        session.load_and_expand_builtin("core-macros");
        session
    }

    fn load_and_expand_builtin(&mut self, feature: &str) {
        let Some(source) = self.builtin_libraries.get(feature).copied() else {
            return;
        };
        self.mark_loaded(feature);
        let source_file = crate::source::SourceFile::new(
            crate::source::SourceId::new(0),
            Some(feature.into()),
            source.to_string(),
        );
        let reader_output = crate::reader::read_source(&source_file);
        self.diagnostics.extend(reader_output.diagnostics);
        // Process forms to register macros (like `when`, `unless`).
        let mut expander = Expander {
            macros: std::mem::take(&mut self.macros),
            symbol_macros: std::mem::take(&mut self.symbol_macros),
            diagnostics: Vec::new(),
            pcase_counter: 0,
        };
        for form in reader_output.forms {
            expander.register_top_level_macro(&form);
            // Collect expanded forms (defun, etc.) for HIR lowering.
            let expanded = expander.expand_form(form);
            // Filter out (provide ...) forms since they're already loaded.
            if !matches!(&expanded.kind, SurfaceKind::List(items)
                if items.first().and_then(SurfaceForm::symbol_name) == Some("provide"))
            {
                self.builtin_forms.push(expanded);
            }
        }
        self.macros = expander.macros;
        self.symbol_macros = expander.symbol_macros;
        self.diagnostics.extend(expander.diagnostics);
    }

    pub fn register_builtin(&mut self, feature: &str, source: &'static str) {
        self.builtin_libraries.insert(feature.to_string(), source);
    }

    pub fn is_loaded(&self, feature: &str) -> bool {
        self.loaded_features.contains(feature)
    }

    pub fn mark_loaded(&mut self, feature: &str) {
        self.loaded_features.insert(feature.to_string());
    }

    pub fn add_load_path(&mut self, path: String) {
        if !self.load_paths.contains(&path) {
            self.load_paths.push(path);
        }
    }

    /// Expand top-level forms from a source file, processing `require`
    /// forms eagerly to import macros from required features.
    pub fn expand_file_forms(&mut self, forms: Vec<SurfaceForm>) -> ExpandOutput {
        let mut expander = Expander {
            macros: std::mem::take(&mut self.macros),
            symbol_macros: std::mem::take(&mut self.symbol_macros),
            diagnostics: Vec::new(),
            pcase_counter: 0,
        };

        let mut expanded_forms = Vec::new();

        // Pre-scan: register defmacros nested inside progn so they are
        // available for expansion of later forms in the same file.
        for form in &forms {
            Expander::pre_scan_defmacros(form, &mut expander);
        }

        for form in forms {
            // Handle top-level (require 'feature)
            if let Some(feature) = extract_require_feature(&form) {
                // Move macros back to session for recursive load
                self.macros = std::mem::take(&mut expander.macros);
                self.symbol_macros = std::mem::take(&mut expander.symbol_macros);

                if !self.is_loaded(&feature)
                    && let Some(new_forms) = self.load_feature_forms(&feature, form.span) {
                        // Move macros back into expander so the required
                        // forms are expanded with the current macro set,
                        // including any newly-defined macros from this file.
                        expander.macros = std::mem::take(&mut self.macros);
                        expander.symbol_macros = std::mem::take(&mut self.symbol_macros);
                        // Expand each required-form inline — no splice,
                        // no recursive expand_file_forms call.
                        for rf in new_forms {
                            if let Some(pf) = extract_provide_feature(&rf) {
                                self.mark_loaded(&pf);
                                expanded_forms.push(rf);
                            } else if let Some(defalias_form) =
                                expander.register_top_level_macro(&rf)
                            {
                                expanded_forms.push(defalias_form);
                            } else {
                                expanded_forms.push(expander.expand_form(rf));
                            }
                        }
                        // Move macros back to session so they persist
                        self.macros = std::mem::take(&mut expander.macros);
                        self.symbol_macros = std::mem::take(&mut expander.symbol_macros);
                    }

                // Move session macros back into expander for remaining forms
                expander.macros = std::mem::take(&mut self.macros);
                expander.symbol_macros = std::mem::take(&mut self.symbol_macros);
                expander
                    .diagnostics
                    .extend(std::mem::take(&mut self.diagnostics));

                expanded_forms.push(form);
                continue;
            }

            // Handle top-level (provide 'feature)
            if let Some(feature) = extract_provide_feature(&form) {
                self.mark_loaded(&feature);
                expanded_forms.push(form);
                continue;
            }

            if let Some(defalias_form) = expander.register_top_level_macro(&form) {
                expanded_forms.push(defalias_form);
            } else {
                expanded_forms.push(expander.expand_form(form));
            }
        }

        // Merge macros back into session
        self.macros = expander.macros;
        self.symbol_macros = expander.symbol_macros;
        self.diagnostics.extend(expander.diagnostics);

        ExpandOutput {
            forms: expanded_forms,
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }

    fn load_feature_forms(&mut self, feature: &str, _span: Span) -> Option<Vec<SurfaceForm>> {
        if self.loading_stack.len() > 50 {
            self.diagnostics.push(Diagnostic::error(format!(
                "require depth limit exceeded loading '{}'",
                feature
            )));
            return None;
        }
        if self.loading_stack.contains(&feature.to_string()) {
            self.diagnostics.push(Diagnostic::error(format!(
                "circular require dependency on '{}'",
                feature
            )));
            return None;
        }

        self.loading_stack.push(feature.to_string());

        let source_text = match self.builtin_libraries.get(feature) {
            Some(src) => src.to_string(),
            None => {
                let found = self.resolve_load_file(feature);
                match found {
                    Some((_, text)) => text,
                    None => {
                        self.diagnostics.push(Diagnostic::error(format!(
                            "cannot find source for required feature '{}'",
                            feature
                        )));
                        self.loading_stack.pop();
                        return None;
                    }
                }
            }
        };

        let source_name = format!("{feature}.el");
        let source =
            crate::source::SourceFile::new(SourceId::new(0), Some(source_name), source_text);
        let reader_output = crate::reader::read_source(&source);
        self.diagnostics.extend(reader_output.diagnostics);

        self.loading_stack.pop();
        self.loaded_features.insert(feature.to_string());

        if !self.diagnostics.iter().any(Diagnostic::is_error) {
            Some(reader_output.forms)
        } else {
            None
        }
    }

    fn resolve_load_file(&self, feature: &str) -> Option<(String, String)> {
        for dir in &self.load_paths {
            let path = format!("{dir}/{feature}.el");
            if let Ok(text) = std::fs::read_to_string(&path) {
                return Some((path, text));
            }
        }
        let path = format!("{feature}.el");
        if let Ok(text) = std::fs::read_to_string(&path) {
            return Some((path, text));
        }
        None
    }
}

fn extract_require_feature(form: &SurfaceForm) -> Option<String> {
    let SurfaceKind::List(items) = &form.kind else {
        return None;
    };
    if items.first().and_then(SurfaceForm::symbol_name) != Some("require") {
        return None;
    }
    items.get(1).and_then(|arg| {
        if let SurfaceKind::Quote(inner) = &arg.kind {
            inner.symbol_name().map(str::to_string)
        } else {
            None
        }
    })
}

fn extract_provide_feature(form: &SurfaceForm) -> Option<String> {
    let SurfaceKind::List(items) = &form.kind else {
        return None;
    };
    if items.first().and_then(SurfaceForm::symbol_name) != Some("provide") {
        return None;
    }
    items.get(1).and_then(|arg| {
        if let SurfaceKind::Quote(inner) = &arg.kind {
            inner.symbol_name().map(str::to_string)
        } else {
            None
        }
    })
}

pub fn expand_forms(forms: Vec<SurfaceForm>) -> ExpandOutput {
    let mut session = CompilerSession::new();
    session.expand_file_forms(forms)
}

struct Expander {
    macros: HashMap<String, MacroDef>,
    symbol_macros: HashMap<String, SurfaceForm>,
    diagnostics: Vec<Diagnostic>,
    pcase_counter: usize,
}

impl Expander {
    /// Pre-scan a form (and its progn sub-forms) for defmacros and register
    /// them before the main expansion pass. This makes macros defined inside
    /// progn available to later forms in the same file.
    fn pre_scan_defmacros(form: &SurfaceForm, expander: &mut Expander) {
        if let SurfaceKind::List(items) = &form.kind
            && let Some(head) = items.first().and_then(|f| f.symbol_name()) {
                match head {
                    "defmacro" => {
                        expander.register_top_level_macro(form);
                    }
                    "progn" | "prog1" | "prog2" | "eval-and-compile"
                    | "eval-when-compile" | "with-no-warnings" => {
                        for sub in &items[1..] {
                            Self::pre_scan_defmacros(sub, expander);
                        }
                    }
                    _ => {}
                }
            }
    }

    fn register_top_level_macro(&mut self, form: &SurfaceForm) -> Option<SurfaceForm> {
        let SurfaceKind::List(items) = &form.kind else {
            return None;
        };
        if items.first().and_then(SurfaceForm::symbol_name) != Some("defmacro") {
            return None;
        }
        if items.len() < 4 {
            self.error(
                form.span,
                "defmacro requires a name, parameter list, and body",
            );
            return None;
        }
        let Some(name) = items[1].symbol_name().map(str::to_string) else {
            self.error(items[1].span, "defmacro name must be a symbol");
            return None;
        };
        let Some(params) = self.parse_macro_params(&items[2]) else {
            return None;
        };
        let mut body = &items[3..];
        if matches!(
            body.first().map(|form| &form.kind),
            Some(SurfaceKind::Atom(SurfaceAtom::String(_)))
        ) {
            body = &body[1..];
        }
        while let Some(first) = body.first()
            && list_head_symbol(first) == Some("declare")
        {
            body = &body[1..];
        }
        let body = if body.is_empty() {
            vec![nil_form(form.span)]
        } else {
            body.to_vec()
        };
        let def = MacroDef {
            params,
            body,
            span: form.span,
        };
        self.macros.insert(name.clone(), def.clone());
        Some(macro_defalias_form(&name, &def, form.span))
    }

    fn expand_form(&mut self, form: SurfaceForm) -> SurfaceForm {
        // Stack-based tree traversal to avoid blowing the call stack on
        // deeply nested Elisp forms. Each Work item expands one SurfaceForm.
        // When expansion produces sub-forms that also need expanding, they
        // are pushed as Work items instead of recursing.
        let mut stack = vec![Work::Expand(form)];
        let mut results: Vec<SurfaceForm> = Vec::new();

        while let Some(work) = stack.pop() {
            match work {
                Work::Expand(form) => {
                    match form.kind {
                        SurfaceKind::List(items) => {
                            let expanded = self.expand_single_list(form.span, items);
                            // expand_single_list may have already fully expanded
                            // sub-forms (in the non-macro, non-special case) or
                            // may have returned a form whose sub-forms still need
                            // expansion. Check by structure.
                            results.push(expanded);
                        }
                        SurfaceKind::DottedList(items, tail) => {
                            // Push a reunion task, then push sub-forms in reverse
                            // so they are processed left-to-right.
                            let count = items.len() + 1; // items + tail
                            stack.push(Work::RejoinDotted(form.span, count));
                            stack.push(Work::Expand(*tail));
                            for item in items.into_iter().rev() {
                                stack.push(Work::Expand(item));
                            }
                        }
                        SurfaceKind::Vector(_)
                        | SurfaceKind::HashList(_)
                        | SurfaceKind::Record(..)
                        | SurfaceKind::CharTable(_)
                        | SurfaceKind::Labeled(..)
                        | SurfaceKind::Ref(_) => {
                            results.push(form);
                        }
                        SurfaceKind::Quote(_)
                        | SurfaceKind::FunctionQuote(_)
                        | SurfaceKind::Backquote(_)
                        | SurfaceKind::Comma(_)
                        | SurfaceKind::CommaAt(_) => {
                            results.push(form);
                        }
                        SurfaceKind::Atom(ref atom) => {
                            if let SurfaceAtom::Symbol(name) = atom
                                && let Some(expansion) = self.symbol_macros.get(name) {
                                    stack.push(Work::Expand(expansion.clone()));
                                    continue;
                                }
                            results.push(form);
                        }
                    }
                }
                Work::RejoinDotted(span, count) => {
                    // Collect `count` results from the top of results, build DottedList.
                    let split = results.len() - count;
                    let tail_idx = split + count - 1;
                    let tail = results.swap_remove(tail_idx);
                    let items: Vec<SurfaceForm> = results.drain(split..).collect();
                    results.push(SurfaceForm::new(
                        SurfaceKind::DottedList(items, Box::new(tail)),
                        span,
                    ));
                }
            }
        }

        results.pop().unwrap_or_else(|| {
            SurfaceForm::new(
                SurfaceKind::Atom(SurfaceAtom::Nil),
                Span::new(SourceId::new(0), 0, 0),
            )
        })
    }

    /// Expand a single list form (non-recursive: sub-forms are expanded
    /// iteratively via the work stack in expand_form).
    fn expand_single_list(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        let Some(head) = items.first().and_then(SurfaceForm::symbol_name) else {
            // Empty or non-symbol-headed list: expand each sub-form.
            return SurfaceForm::new(
                SurfaceKind::List(
                    items
                        .into_iter()
                        .map(|item| self.expand_form(item))
                        .collect(),
                ),
                span,
            );
        };
        // Check for semantic forms FIRST — these are handled by HIR, not macros.
        // This must precede the macro lookup so runtime macros (like `defun` from
        // byte-run.el) don't shadow the semantic lowering.
        match head {
            "quote" | "function" => return SurfaceForm::new(SurfaceKind::List(items), span),
            "defmacro" => {
                // Register the macro (even when nested inside progn) and
                // replace with a defalias form so the HIR doesn't see it.
                let form = SurfaceForm::new(
                    SurfaceKind::List(items.into_iter()
                        .map(|item| self.expand_form(item))
                        .collect()),
                    span,
                );
                if let Some(defalias_form) = self.register_top_level_macro(&form) {
                    return defalias_form;
                }
                return form;
            }
            "defun" | "defvar" | "defconst" | "defsubst" | "defcustom"
            | "defgroup" | "defface" | "defclass" | "defmethod" | "defgeneric" => {
                return SurfaceForm::new(
                    SurfaceKind::List(
                        items
                            .into_iter()
                            .map(|item| self.expand_form(item))
                            .collect(),
                    ),
                    span,
                );
            }
            _ => {}
        }
        if let Some(def) = self.macros.get(head).cloned() {
            return self.expand_macro_call(span, items, def);
        }
        match head {
            "push" => self.expand_push(span, items),
            "pushnew" | "cl-pushnew" => self.expand_pushnew(span, items),
            "pop" => self.expand_pop(span, items),
            "setf" => self.expand_setf(span, items),
            "cl-incf" | "incf" => self.expand_incf_decf(span, items, "+"),
            "cl-decf" | "decf" => self.expand_incf_decf(span, items, "-"),
            "if-let*" => self.expand_if_let(span, items),
            "when-let*" => self.expand_when_let(span, items),
            // declare-function is a compile-time declaration — discard
            "declare-function" => nil_form(span),
            // pcase-let* -> let* with destructuring support
            "pcase-let*" => {
                if items.len() >= 3 {
                    let bindings_form = &items[1];
                    let body: Vec<SurfaceForm> = items[2..]
                        .iter()
                        .map(|f| self.expand_form(f.clone()))
                        .collect();
                    let destructured = self.expand_pcase_let_bindings(
                        span, bindings_form, body,
                    );
                    self.expand_form(destructured)
                } else {
                    let expanded: Vec<SurfaceForm> =
                        items.into_iter().map(|f| self.expand_form(f)).collect();
                    SurfaceForm::new(SurfaceKind::List(expanded), span)
                }
            }
            "pcase" => {
                if items.len() >= 3 {
                    self.expand_pcase(span, items)
                } else {
                    let expanded: Vec<SurfaceForm> =
                        items.into_iter().map(|f| self.expand_form(f)).collect();
                    SurfaceForm::new(SurfaceKind::List(expanded), span)
                }
            }
            // cl-with-gensyms -> let (simplified: uses symbol names as-is)
            "cl-with-gensyms" => {
                if items.len() >= 3 {
                    let bindings = items[1].clone();
                    let body: Vec<SurfaceForm> = items[2..]
                        .iter()
                        .map(|f| self.expand_form(f.clone()))
                        .collect();
                    list_form(
                        vec![symbol_form("let", span), bindings.clone()]
                            .into_iter()
                            .chain(body)
                            .collect(),
                        span,
                    )
                } else {
                    nil_form(span)
                }
            }
            // cl-check-type: (cl-check-type FORM TYPE [STRING])
            "cl-check-type" => {
                if items.len() >= 3 {
                    let form = self.expand_form(items[1].clone());
                    let type_form = &items[2];
                    let v = symbol_form("--ct-val--", span);
                    let binding = list_form(vec![v.clone(), form], span);
                    let type_check = list_form(
                        vec![
                            symbol_form("eq", span),
                            list_form(vec![symbol_form("type-of", span), v.clone()], span),
                            quote_form(type_form.clone(), span),
                        ],
                        span,
                    );
                    let err_msg = SurfaceForm::new(
                        SurfaceKind::Atom(SurfaceAtom::String("Wrong type argument".into())),
                        span,
                    );
                    let check = list_form(
                        vec![
                            symbol_form("if", span),
                            list_form(vec![symbol_form("not", span), type_check], span),
                            list_form(vec![symbol_form("error", span), err_msg], span),
                            v.clone(),
                        ],
                        span,
                    );
                    list_form(
                        vec![
                            symbol_form("let", span),
                            list_form(vec![binding], span),
                            check,
                        ],
                        span,
                    )
                } else if items.len() >= 2 {
                    self.expand_form(items[1].clone())
                } else {
                    nil_form(span)
                }
            }
            // cl-assert: (cl-assert FORM [SHOW-ARGS-STRING MESSAGE args...])
            "cl-assert" => {
                if items.len() >= 2 {
                    let form = self.expand_form(items[1].clone());
                    let v = symbol_form("--assert-val--", span);
                    let binding = list_form(vec![v.clone(), form], span);
                    let err_msg = SurfaceForm::new(
                        SurfaceKind::Atom(SurfaceAtom::String("Assertion failed".into())),
                        span,
                    );
                    let check = list_form(
                        vec![
                            symbol_form("if", span),
                            list_form(vec![symbol_form("not", span), v.clone()], span),
                            list_form(vec![symbol_form("error", span), err_msg], span),
                            v.clone(),
                        ],
                        span,
                    );
                    list_form(
                        vec![
                            symbol_form("let", span),
                            list_form(vec![binding], span),
                            check,
                        ],
                        span,
                    )
                } else {
                    nil_form(span)
                }
            }
            "destructuring-bind" => self.expand_destructuring_bind(span, items),
            "flet" | "cl-flet" => self.expand_flet(span, items),
            "labels" | "cl-labels" => self.expand_labels(span, items),
            "cl-the" => {
                // (cl-the TYPE FORM) — type assertion, returns FORM at runtime
                if items.len() >= 3 {
                    self.expand_form(items[2].clone())
                } else {
                    nil_form(span)
                }
            }
            "cl-defun" => self.expand_cl_defun(span, items),
            "cl-macrolet" => self.expand_cl_macrolet(span, items),
            "cl-symbol-macrolet" => self.expand_cl_symbol_macrolet(span, items),
            "letrec" => self.expand_letrec(span, items),
            "cl-loop" => self.expand_cl_loop(span, items),
            "cl-case" => self.expand_cl_case(span, items),
            "cl-destructuring-bind" => {
                if items.len() >= 4 {
                    let bindings_form = list_form(
                        vec![list_form(vec![items[1].clone(), items[2].clone()], span)],
                        span,
                    );
                    let body: Vec<SurfaceForm> = items[3..]
                        .iter()
                        .map(|f| self.expand_form(f.clone()))
                        .collect();
                    let destructured = self.expand_pcase_let_bindings(
                        span, &bindings_form, body,
                    );
                    self.expand_form(destructured)
                } else {
                    let expanded: Vec<SurfaceForm> =
                        items.into_iter().map(|f| self.expand_form(f)).collect();
                    SurfaceForm::new(SurfaceKind::List(expanded), span)
                }
            }
            kw @ ("cl-do" | "cl-do*") => {
                let sequential = kw == "cl-do*";
                self.expand_cl_do(span, items, sequential)
            }
            "cl-dolist" => self.expand_cl_dolist(span, items),
            "cl-dotimes" => self.expand_cl_dotimes(span, items),
            "psetq" => self.expand_psetq(span, items),
            "psetf" | "cl-psetf" => self.expand_psetf(span, items),
            "cl-rotatef" => self.expand_cl_rotatef(span, items),
            "cl-shiftf" => self.expand_cl_shiftf(span, items),

            // Editor macros: no-ops that expand to (progn body...) since
            // the executor doesn't yet have buffer/point primitives.
            "save-excursion" | "save-restriction" | "with-current-buffer"
            | "with-temp-buffer" | "with-temp-file" | "with-temp-message"
            | "with-output-to-string" => {
                let body = &items[1..];
                if body.is_empty() {
                    nil_form(span)
                } else {
                    let progn = list_form(
                        std::iter::once(symbol_form("progn", span))
                            .chain(body.iter().map(|f| self.expand_form(f.clone())))
                            .collect(),
                        span,
                    );
                    self.expand_form(progn)
                }
            }

            _ => SurfaceForm::new(
                SurfaceKind::List(
                    items
                        .into_iter()
                        .map(|item| self.expand_form(item))
                        .collect(),
                ),
                span,
            ),
        }
    }

    fn expand_macro_call(
        &mut self,
        span: Span,
        items: Vec<SurfaceForm>,
        def: MacroDef,
    ) -> SurfaceForm {
        // First expansion attempt.
        let mut form = match self.invoke_macro(&def, &items[1..]) {
            Some(expanded) => expanded,
            None => {
                // Expansion failed. Return original form.
                return SurfaceForm::new(SurfaceKind::List(items), span);
            }
        };

        // Iteratively re-expand if the result is another macro call.
        for _ in 0..100 {
            let SurfaceKind::List(ref expansion_items) = form.kind else {
                break;
            };
            let Some(head) = expansion_items.first().and_then(SurfaceForm::symbol_name) else {
                break;
            };
            let Some(next_def) = self.macros.get(head).cloned() else {
                break;
            };
            let expansion_span = form.span;
            let expansion_items =
                match std::mem::replace(&mut form.kind, SurfaceKind::Atom(SurfaceAtom::Nil)) {
                    SurfaceKind::List(items) => items,
                    other => {
                        self.error(form.span, format!("macro returned non-list: {other:?}"));
                        return form;
                    }
                };
            form = match self.invoke_macro(&next_def, &expansion_items[1..]) {
                Some(expanded) => expanded,
                None => {
                    form = SurfaceForm::new(SurfaceKind::List(expansion_items), expansion_span);
                    break;
                }
            };
        }

        // Tree-expand sub-forms without re-invoking the top-level macro.
        // We manually decompose the form instead of calling expand_form
        // to avoid the macro lookup loop.
        {
            match form.kind {
                SurfaceKind::List(items) => {
                    let expanded_items: Vec<SurfaceForm> = items
                        .into_iter()
                        .map(|item| self.expand_form(item))
                        .collect();
                    SurfaceForm::new(SurfaceKind::List(expanded_items), form.span)
                }
                _ => form,
            }
        }
    }

    fn invoke_macro(&mut self, def: &MacroDef, args: &[SurfaceForm]) -> Option<SurfaceForm> {
        let arg_values: Vec<MacroValue> = args.iter().map(surface_to_value).collect();

        if arg_values.len() < def.params.required.len() {
            // Arity mismatch — likely due to incomplete macro loading, pass through
            return None;
        }
        let max_arity = def
            .params
            .rest
            .is_none()
            .then_some(def.params.required.len() + def.params.optional.len());
        if let Some(max_arity) = max_arity
            && arg_values.len() > max_arity
        {
            self.error(
                def.span,
                format!(
                    "macro requires at most {max_arity} arguments, got {}",
                    arg_values.len()
                ),
            );
            return None;
        }

        let mut env = MacroEnv::default();
        for (name, arg) in def.params.required.iter().zip(arg_values.iter()) {
            env.bind(name.clone(), arg.clone());
        }
        let optional_start = def.params.required.len();
        for (index, name) in def.params.optional.iter().enumerate() {
            env.bind(
                name.clone(),
                arg_values
                    .get(optional_start + index)
                    .cloned()
                    .unwrap_or(MacroValue::Nil),
            );
        }
        if let Some(rest) = &def.params.rest {
            let rest_start = arg_values
                .len()
                .min(optional_start + def.params.optional.len());
            env.bind(
                rest.clone(),
                MacroValue::list(arg_values[rest_start..].to_vec()),
            );
        }
        if let Some(environment) = &def.params.environment {
            env.bind(environment.clone(), MacroValue::Nil);
        }

        let mut macro_eval = MacroEval::new();
        match macro_eval.eval_progn(&def.body, &mut env) {
            Ok(result) => {
                self.diagnostics.extend(macro_eval.into_diagnostics());
                Some(value_to_surface(&result, def.span))
            }
            Err(()) => {
                self.diagnostics.extend(macro_eval.into_diagnostics());
                None
            }
        }
    }

    fn expand_pushnew(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (pushnew X PLACE) => cond-cons via setf only if X not already member
        // Expand to: (let* ((--pn-- X))
        //              (unless (member --pn-- PLACE)
        //                (setf PLACE (cons --pn-- PLACE))))
        if items.len() < 3 {
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let value = &items[1];
        let place = &items[2];
        let tmp = symbol_form("--pushnew--", span);
        let unless_form = list_form(vec![
            symbol_form("unless", span),
            list_form(vec![symbol_form("member", span), tmp.clone(), place.clone()], span),
            list_form(vec![
                symbol_form("setf", span),
                place.clone(),
                list_form(vec![symbol_form("cons", span), tmp.clone(), place.clone()], span),
            ], span),
        ], span);
        let expanded = list_form(vec![
            symbol_form("let*", span),
            list_form(vec![list_form(vec![tmp, value.clone()], span)], span),
            unless_form,
        ], span);
        self.expand_form(expanded)
    }

    fn expand_push(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() != 3 {
            // Wrong arity — expand sub-forms, pass through
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let Some(place) = items[2].symbol_name().map(str::to_string) else {
            // Non-symbol place (e.g., list access) — expand sub-forms, pass through
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        };
        let value = items[1].clone();
        let expanded = list_form(
            vec![
                symbol_form("setq", span),
                symbol_form(&place, items[2].span),
                list_form(
                    vec![
                        symbol_form("cons", span),
                        value,
                        symbol_form(&place, items[2].span),
                    ],
                    span,
                ),
            ],
            span,
        );
        self.expand_form(expanded)
    }

    fn expand_pop(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() != 2 {
            // Wrong arity — expand sub-forms, pass through
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let Some(place) = items[1].symbol_name().map(str::to_string) else {
            // Non-symbol place — expand sub-forms, pass through
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        };
        let place_span = items[1].span;
        let expanded = list_form(
            vec![
                symbol_form("car-safe", span),
                list_form(
                    vec![
                        symbol_form("prog1", span),
                        symbol_form(&place, place_span),
                        list_form(
                            vec![
                                symbol_form("setq", span),
                                symbol_form(&place, place_span),
                                list_form(
                                    vec![symbol_form("cdr", span), symbol_form(&place, place_span)],
                                    span,
                                ),
                            ],
                            span,
                        ),
                    ],
                    span,
                ),
            ],
            span,
        );
        self.expand_form(expanded)
    }

    fn expand_incf_decf(&mut self, span: Span, items: Vec<SurfaceForm>, op: &str) -> SurfaceForm {
        // (cl-incf place delta) → (setf place (+ place delta))
        // (cl-incf place) → (setf place (+ place 1))
        if items.len() < 2 {
            return nil_form(span);
        }
        let place = &items[1];
        let delta = if items.len() >= 3 {
            items[2].clone()
        } else {
            int_form(1, span)
        };
        let expanded = list_form(
            vec![
                symbol_form("setf", span),
                place.clone(),
                list_form(vec![symbol_form(op, span), place.clone(), delta], span),
            ],
            span,
        );
        self.expand_form(expanded)
    }

    fn expand_setf(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (setf place value [place value ...])
        // Handle pairs of place/value
        if items.len() < 3 || items.len().is_multiple_of(2) {
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }

        // Handle single pair for now: (setf place value)
        if items.len() == 3 {
            return self.expand_setf_pair(span, &items[1], &items[2]);
        }

        // Multiple pairs: (setf p1 v1 p2 v2 ...)
        // Expand as nested progn
        let mut forms = Vec::new();
        let mut i = 1;
        while i + 1 < items.len() {
            forms.push(self.expand_setf_pair(span, &items[i], &items[i + 1]));
            i += 2;
        }
        if forms.len() == 1 {
            forms.pop().unwrap()
        } else {
            let expanded = list_form(
                std::iter::once(symbol_form("progn", span))
                    .chain(forms)
                    .collect(),
                span,
            );
            self.expand_form(expanded)
        }
    }

    fn expand_setf_pair(
        &mut self,
        span: Span,
        place: &SurfaceForm,
        value: &SurfaceForm,
    ) -> SurfaceForm {
        // (setf sym val) → (setq sym val)
        if let Some(name) = place.symbol_name() {
            let expanded = list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(name, place.span),
                    value.clone(),
                ],
                span,
            );
            return self.expand_form(expanded);
        }

        // (setf (func args...) val) → dispatch on func
        let SurfaceKind::List(place_items) = &place.kind else {
            // Unknown place form — pass through
            let expanded: Vec<SurfaceForm> =
                vec![symbol_form("setf", span), place.clone(), value.clone()];
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        };

        let Some(func_name) = place_items.first().and_then(SurfaceForm::symbol_name) else {
            let expanded = vec![symbol_form("setf", span), place.clone(), value.clone()];
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        };

        match func_name {
            "car" if place_items.len() == 2 => {
                // (setf (car x) v) → (setcar x v)
                let expanded = list_form(
                    vec![
                        symbol_form("setcar", span),
                        place_items[1].clone(),
                        value.clone(),
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            "cdr" if place_items.len() == 2 => {
                // (setf (cdr x) v) → (setcdr x v)
                let expanded = list_form(
                    vec![
                        symbol_form("setcdr", span),
                        place_items[1].clone(),
                        value.clone(),
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            "aref" if place_items.len() == 3 => {
                // (setf (aref v i) val) → (aset v i val)
                let expanded = list_form(
                    vec![
                        symbol_form("aset", span),
                        place_items[1].clone(),
                        place_items[2].clone(),
                        value.clone(),
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            "gethash" if place_items.len() >= 3 => {
                // (setf (gethash key table) val) → (puthash key val table)
                let expanded = list_form(
                    vec![
                        symbol_form("puthash", span),
                        place_items[1].clone(),
                        value.clone(),
                        place_items[2].clone(),
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            "nth" if place_items.len() == 3 => {
                // (setf (nth n list) val)
                // → (let ((--v-- val)) (setcar (nthcdr n list) --v--) --v--)
                let temp = symbol_form("--setf-val--", span);
                let expanded = list_form(
                    vec![
                        symbol_form("let", span),
                        list_form(
                            vec![list_form(vec![temp.clone(), value.clone()], span)],
                            span,
                        ),
                        list_form(
                            vec![
                                symbol_form("setcar", span),
                                list_form(
                                    vec![
                                        symbol_form("nthcdr", span),
                                        place_items[1].clone(),
                                        place_items[2].clone(),
                                    ],
                                    span,
                                ),
                                temp.clone(),
                            ],
                            span,
                        ),
                        temp,
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            "elt" if place_items.len() == 3 => {
                // NOTE: the place form is (elt INDEX SEQUENCE) — index first
                // (non-standard Emacs order), matching the codebase convention.
                // Emit type-dispatch: arrayp→aset, nil→nthcdr+setcar.
                let temp_v = symbol_form("--setf-val--", span);
                let temp_s = symbol_form("--setf-seq--", span);
                let temp_n = symbol_form("--setf-idx--", span);
                let seq_val = place_items[2].clone();
                let idx_val = place_items[1].clone();
                let expanded = list_form(
                    vec![
                        symbol_form("let", span),
                        list_form(
                            vec![
                                list_form(vec![temp_s.clone(), seq_val], span),
                                list_form(vec![temp_n.clone(), idx_val], span),
                                list_form(vec![temp_v.clone(), value.clone()], span),
                            ],
                            span,
                        ),
                        list_form(
                            vec![
                                symbol_form("if", span),
                                list_form(vec![symbol_form("arrayp", span), temp_s.clone()], span),
                                list_form(
                                    vec![
                                        symbol_form("progn", span),
                                        list_form(
                                            vec![
                                                symbol_form("aset", span),
                                                temp_s.clone(),
                                                temp_n.clone(),
                                                temp_v.clone(),
                                            ],
                                            span,
                                        ),
                                        temp_v.clone(),
                                    ],
                                    span,
                                ),
                                list_form(
                                    vec![
                                        symbol_form("progn", span),
                                        list_form(
                                            vec![
                                                symbol_form("setcar", span),
                                                list_form(
                                                    vec![
                                                        symbol_form("nthcdr", span),
                                                        temp_n.clone(),
                                                        temp_s.clone(),
                                                    ],
                                                    span,
                                                ),
                                                temp_v.clone(),
                                            ],
                                            span,
                                        ),
                                        temp_v.clone(),
                                    ],
                                    span,
                                ),
                            ],
                            span,
                        ),
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            "symbol-value" if place_items.len() == 2 => {
                // (setf (symbol-value sym) val) → (set sym val)
                let expanded = list_form(
                    vec![
                        symbol_form("set", span),
                        place_items[1].clone(),
                        value.clone(),
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            "symbol-function" if place_items.len() == 2 => {
                // (setf (symbol-function sym) val) → (fset sym val)
                let expanded = list_form(
                    vec![
                        symbol_form("fset", span),
                        place_items[1].clone(),
                        value.clone(),
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            "symbol-plist" if place_items.len() == 2 => {
                // (setf (symbol-plist sym) val) → (setplist sym val)
                let expanded = list_form(
                    vec![
                        symbol_form("setplist", span),
                        place_items[1].clone(),
                        value.clone(),
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            "plist-get" if place_items.len() == 3 => {
                // (setf (plist-get plist key) val) → (plist-put plist key val)
                let expanded = list_form(
                    vec![
                        symbol_form("plist-put", span),
                        place_items[1].clone(),
                        place_items[2].clone(),
                        value.clone(),
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            "get" if place_items.len() == 3 => {
                // (setf (get sym prop) val) → (put sym prop val)
                let expanded = list_form(
                    vec![
                        symbol_form("put", span),
                        place_items[1].clone(),
                        place_items[2].clone(),
                        value.clone(),
                    ],
                    span,
                );
                self.expand_form(expanded)
            }
            _ => {
                // Unknown place — pass through as-is
                let expanded = vec![symbol_form("setf", span), place.clone(), value.clone()];
                SurfaceForm::new(SurfaceKind::List(expanded), span)
            }
        }
    }

    /// Expand pcase-let* bindings into a let* form with destructuring support.
    /// Handles both simple symbol bindings and backquote patterns like `(,x ,y).
    fn expand_pcase_let_bindings(
        &mut self,
        span: Span,
        bindings_form: &SurfaceForm,
        body: Vec<SurfaceForm>,
    ) -> SurfaceForm {
        let SurfaceKind::List(bindings) = &bindings_form.kind else {
            return list_form(
                std::iter::once(symbol_form("let*", span))
                    .chain(std::iter::once(bindings_form.clone()))
                    .chain(body)
                    .collect(),
                span,
            );
        };
        let mut let_bindings: Vec<SurfaceForm> = Vec::new();
        for binding in bindings {
            let (pat, expr) = match &binding.kind {
                SurfaceKind::List(items) if items.len() == 2 => {
                    (items[0].clone(), items[1].clone())
                }
                _ => continue,
            };
            if pat.symbol_name().is_some() {
                // Simple symbol binding: (x expr)
                let_bindings.push(list_form(vec![pat, expr], span));
            } else if let SurfaceKind::Backquote(template) = &pat.kind {
                // Backquote destructuring: `(,x ,y) -> car/cdr chain
                let tmp = format!("--pcase-dst-{}--", self.pcase_counter);
                self.pcase_counter += 1;
                let_bindings.push(list_form(
                    vec![symbol_form(&tmp, span), expr],
                    span,
                ));
                self.emit_pcase_destructure(span, template, &tmp, &mut let_bindings, false);
            } else if let SurfaceKind::List(_) = &pat.kind {
                // Simple list pattern: (x y z) -> car/cdr chain
                // Bare symbols are bindings (unlike backquote where only
                // ,X and ,@X are bindings).
                let tmp = format!("--pcase-dst-{}--", self.pcase_counter);
                self.pcase_counter += 1;
                let_bindings.push(list_form(
                    vec![symbol_form(&tmp, span), expr],
                    span,
                ));
                self.emit_pcase_destructure(span, &pat, &tmp, &mut let_bindings, true);
            } else {
                // Unknown pattern: bind to underscore (value is evaluated but ignored)
                let_bindings.push(list_form(
                    vec![symbol_form("_", span), expr],
                    span,
                ));
            }
        }
        let mut result = vec![symbol_form("let*", span), list_form(let_bindings, span)];
        result.extend(body);
        list_form(result, span)
    }

    /// Emit let* bindings that destructure a backquote template into its car/cdr components.
    fn emit_pcase_destructure(
        &mut self,
        span: Span,
        template: &SurfaceForm,
        src: &str,
        bindings: &mut Vec<SurfaceForm>,
        simple: bool,
    ) {
        match &template.kind {
            SurfaceKind::Atom(atom) => {
                // In simple patterns, bare symbols are variable bindings.
                // In backquote patterns, bare symbols are literal matches (ignored).
                if simple
                    && let SurfaceAtom::Symbol(name) = atom {
                        bindings.push(list_form(
                            vec![
                                symbol_form(name, template.span),
                                symbol_form(src, span),
                            ],
                            span,
                        ));
                    }
            }
            SurfaceKind::Comma(inner) => {
                // Unquote: bind the current source to the variable
                if let Some(name) = inner.symbol_name() {
                    bindings.push(list_form(
                        vec![
                            symbol_form(name, inner.span),
                            symbol_form(src, span),
                        ],
                        span,
                    ));
                }
            }
            SurfaceKind::List(items) => {
                // Destructure a list: car for head, cdr for tail
                let head_tmp = format!("--pcase-dst-{}--", self.pcase_counter);
                self.pcase_counter += 1;
                bindings.push(list_form(
                    vec![
                        symbol_form(&head_tmp, span),
                        list_form(
                            vec![symbol_form("car", span), symbol_form(src, span)],
                            span,
                        ),
                    ],
                    span,
                ));
                self.emit_pcase_destructure(span, &items[0], &head_tmp, bindings, simple);
                // Advance src to cdr for remaining elements
                if items.len() > 1 {
                    let cdr_src = format!("--pcase-dst-{}--", self.pcase_counter);
                    self.pcase_counter += 1;
                    bindings.push(list_form(
                        vec![
                            symbol_form(&cdr_src, span),
                            list_form(
                                vec![symbol_form("cdr", span), symbol_form(src, span)],
                                span,
                            ),
                        ],
                        span,
                    ));
                    // Build a "list" of remaining items
                    let rest = SurfaceForm::new(
                        SurfaceKind::List(items[1..].to_vec()),
                        span,
                    );
                    self.emit_pcase_destructure(span, &rest, &cdr_src, bindings, simple);
                }
            }
            _ => {
                // Unsupported pattern: ignore
            }
        }
    }

    fn expand_if_let(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() < 3 {
            self.error(span, "if-let* requires bindings and a then form");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        let Some(bindings) = self.parse_if_let_bindings(&items[1]) else {
            return SurfaceForm::new(SurfaceKind::List(items), span);
        };
        let then_form = items[2].clone();
        let else_forms = items[3..].to_vec();
        let expanded = build_if_let_form(bindings, then_form, else_forms, span);
        self.expand_form(expanded)
    }

    fn expand_when_let(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() < 2 {
            self.error(span, "when-let* requires bindings");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        let Some(bindings) = self.parse_if_let_bindings(&items[1]) else {
            return SurfaceForm::new(SurfaceKind::List(items), span);
        };
        let then_form = list_form(
            std::iter::once(symbol_form("progn", span))
                .chain(items[2..].iter().cloned())
                .collect(),
            span,
        );
        let expanded = build_if_let_form(bindings, then_form, Vec::new(), span);
        self.expand_form(expanded)
    }

    fn parse_if_let_bindings(&mut self, form: &SurfaceForm) -> Option<Vec<IfLetBinding>> {
        let SurfaceKind::List(items) = &form.kind else {
            if matches!(form.kind, SurfaceKind::Atom(SurfaceAtom::Nil)) {
                return Some(Vec::new());
            }
            self.error(form.span, "if-let* bindings must be a proper list");
            return None;
        };
        items
            .iter()
            .enumerate()
            .map(|(index, item)| self.parse_if_let_binding(item, index))
            .collect()
    }

    fn parse_if_let_binding(&mut self, form: &SurfaceForm, index: usize) -> Option<IfLetBinding> {
        if let Some(name) = form.symbol_name() {
            return Some(IfLetBinding {
                name: name.to_string(),
                value: form.clone(),
                span: form.span,
            });
        }
        let SurfaceKind::List(items) = &form.kind else {
            self.error(
                form.span,
                "if-let* binding must be SYMBOL, (SYMBOL VALUE), or (VALUE)",
            );
            return None;
        };
        match items.as_slice() {
            [value] => Some(IfLetBinding {
                name: generated_if_let_name(form.span, index),
                value: value.clone(),
                span: form.span,
            }),
            [name, value] => {
                let Some(name) = name.symbol_name() else {
                    self.error(name.span, "if-let* binding name must be a symbol");
                    return None;
                };
                let name = if name == "_" {
                    generated_if_let_name(form.span, index)
                } else {
                    name.to_string()
                };
                Some(IfLetBinding {
                    name,
                    value: value.clone(),
                    span: form.span,
                })
            }
            _ => {
                self.error(
                    form.span,
                    "if-let* binding must be SYMBOL, (SYMBOL VALUE), or (VALUE)",
                );
                None
            }
        }
    }

    fn parse_macro_params(&mut self, form: &SurfaceForm) -> Option<MacroParams> {
        let SurfaceKind::List(items) = &form.kind else {
            self.error(form.span, "defmacro parameter list must be a proper list");
            return None;
        };
        let mut params = MacroParams::default();
        let mut section = MacroParamSection::Required;
        let mut index = 0;
        while index < items.len() {
            let item = &items[index];
            let Some(name) = item.symbol_name() else {
                self.error(item.span, "defmacro parameter name must be a symbol");
                return None;
            };
            match name {
                "&optional" => {
                    if section != MacroParamSection::Required {
                        self.error(item.span, "&optional is out of order");
                        return None;
                    }
                    section = MacroParamSection::Optional;
                    index += 1;
                    continue;
                }
                "&rest" | "&body" => {
                    if section == MacroParamSection::Rest {
                        self.error(item.span, "duplicate rest parameter");
                        return None;
                    }
                    section = MacroParamSection::Rest;
                    index += 1;
                    continue;
                }
                "&environment" => {
                    let Some(next) = items.get(index + 1) else {
                        self.error(item.span, "&environment requires a parameter");
                        return None;
                    };
                    let Some(environment) = next.symbol_name() else {
                        self.error(next.span, "&environment parameter must be a symbol");
                        return None;
                    };
                    params.environment = Some(environment.to_string());
                    index += 2;
                    continue;
                }
                _ if name.starts_with('&') => {
                    self.error(
                        item.span,
                        "defmacro lambda-list keyword is not supported yet",
                    );
                    return None;
                }
                _ => {}
            }
            match section {
                MacroParamSection::Required => params.required.push(name.to_string()),
                MacroParamSection::Optional => params.optional.push(name.to_string()),
                MacroParamSection::Rest => {
                    if params.rest.is_some() {
                        self.error(item.span, "rest accepts only one parameter");
                        return None;
                    }
                    params.rest = Some(name.to_string());
                }
            }
            index += 1;
        }
        if section == MacroParamSection::Rest && params.rest.is_none() {
            self.error(form.span, "rest requires a parameter");
            return None;
        }
        Some(params)
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message.into()).with_span(span));
    }

    fn expand_pcase(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (pcase EXPR CLAUSE ...) -> (let ((--val-- EXPR)) (cond (MATCH BODY) ...))
        let val_expr = self.expand_form(items[1].clone());
        let val_sym = "--pcase-val--";
        let val_binding = list_form(vec![symbol_form(val_sym, span), val_expr], span);
        let mut cond_clauses = Vec::new();
        for clause in &items[2..] {
            let parts = match &clause.kind {
                SurfaceKind::List(p) => p,
                _ => continue,
            };
            if parts.is_empty() {
                continue;
            };
            let pattern = &parts[0];
            let body_forms: Vec<SurfaceForm> = parts[1..]
                .iter()
                .map(|f| self.expand_form(f.clone()))
                .collect();
            let body = if body_forms.len() == 1 {
                body_forms.into_iter().next().unwrap()
            } else {
                list_form(
                    std::iter::once(symbol_form("progn", span))
                        .chain(body_forms)
                        .collect(),
                    span,
                )
            };
            let (condition, bindings) = self.expand_pcase_pattern(pattern, val_sym, span);
            let (wrapped_cond, body_with_bindings) = if bindings.is_empty() {
                (condition, body)
            } else {
                let bindings_form = list_form(bindings.clone(), span);
                let cond_wrapped = list_form(
                    vec![symbol_form("let*", span), bindings_form.clone(), condition],
                    span,
                );
                let body_wrapped = list_form(
                    std::iter::once(symbol_form("let*", span))
                        .chain(std::iter::once(bindings_form))
                        .chain(std::iter::once(body))
                        .collect(),
                    span,
                );
                (cond_wrapped, body_wrapped)
            };
            cond_clauses.push(list_form(vec![wrapped_cond, body_with_bindings], span));
        }
        let cond_form = list_form(
            std::iter::once(symbol_form("cond", span))
                .chain(cond_clauses)
                .collect(),
            span,
        );
        list_form(
            vec![
                symbol_form("let", span),
                list_form(vec![val_binding], span),
                cond_form,
            ],
            span,
        )
    }

    fn expand_pcase_pattern(
        &mut self,
        pattern: &SurfaceForm,
        val_sym: &str,
        span: Span,
    ) -> (SurfaceForm, Vec<SurfaceForm>) {
        use crate::surface::{SurfaceAtom, SurfaceKind};
        let val_ref = symbol_form(val_sym, span);
        match &pattern.kind {
            // _ matches everything
            SurfaceKind::Atom(SurfaceAtom::Symbol(name)) if name == "_" => {
                (symbol_form("t", span), Vec::new())
            }
            // nil and t are matched literally
            SurfaceKind::Atom(SurfaceAtom::Symbol(name)) if name == "nil" || name == "t" => {
                let cond = list_form(
                    vec![symbol_form("eq", span), val_ref, symbol_form(name, span)],
                    span,
                );
                (cond, Vec::new())
            }
            // Bare symbol: always matches, binds the symbol
            SurfaceKind::Atom(SurfaceAtom::Symbol(name)) => {
                let binding = list_form(vec![symbol_form(name, span), val_ref], span);
                (symbol_form("t", span), vec![binding])
            }
            // Integer literal: compare with =
            SurfaceKind::Atom(SurfaceAtom::Int(n)) => {
                let cond = list_form(
                    vec![symbol_form("=", span), val_ref, int_form(*n, span)],
                    span,
                );
                (cond, Vec::new())
            }
            // String literal: compare with equal
            SurfaceKind::Atom(SurfaceAtom::String(s)) => {
                let str_form =
                    SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::String(s.clone())), span);
                let cond = list_form(vec![symbol_form("equal", span), val_ref, str_form], span);
                (cond, Vec::new())
            }
            // 'quoted value: compare with equal
            SurfaceKind::Quote(inner) => {
                let cond = list_form(
                    vec![
                        symbol_form("equal", span),
                        val_ref,
                        quote_form(*inner.clone(), span),
                    ],
                    span,
                );
                (cond, Vec::new())
            }
            // `backquote pattern: destructure and match structure
            SurfaceKind::Backquote(inner) => self.expand_pcase_backquote(inner, val_sym, span),
            // (guard EXPR): use EXPR as condition
            SurfaceKind::List(parts) if !parts.is_empty() => {
                if let SurfaceKind::Atom(SurfaceAtom::Symbol(head)) = &parts[0].kind {
                    match head.as_str() {
                        "guard" => {
                            let expr = if parts.len() > 1 {
                                self.expand_form(parts[1].clone())
                            } else {
                                symbol_form("nil", span)
                            };
                            (expr, Vec::new())
                        }
                        "pred" => {
                            let func = if parts.len() > 1 {
                                function_quote_form(parts[1].clone(), span)
                            } else {
                                symbol_form("nil", span)
                            };
                            let cond =
                                list_form(vec![symbol_form("funcall", span), func, val_ref], span);
                            (cond, Vec::new())
                        }
                        "quote" if parts.len() > 1 => {
                            let cond = list_form(
                                vec![symbol_form("equal", span), val_ref, parts[1].clone()],
                                span,
                            );
                            (cond, Vec::new())
                        }
                        "and" => {
                            let mut all_conds = Vec::new();
                            let mut all_bindings = Vec::new();
                            for sub_pat in &parts[1..] {
                                let (cond, bindings) =
                                    self.expand_pcase_pattern(sub_pat, val_sym, span);
                                all_conds.push(cond);
                                all_bindings.extend(bindings);
                            }
                            let cond = if all_conds.is_empty() {
                                symbol_form("t", span)
                            } else if all_conds.len() == 1 {
                                all_conds.into_iter().next().unwrap()
                            } else {
                                list_form(
                                    std::iter::once(symbol_form("and", span))
                                        .chain(all_conds)
                                        .collect(),
                                    span,
                                )
                            };
                            (cond, all_bindings)
                        }
                        "or" => {
                            let mut all_conds = Vec::new();
                            let mut first_bindings: Option<Vec<SurfaceForm>> = None;
                            for sub_pat in &parts[1..] {
                                let (cond, bindings) =
                                    self.expand_pcase_pattern(sub_pat, val_sym, span);
                                all_conds.push(cond);
                                if first_bindings.is_none() {
                                    first_bindings = Some(bindings);
                                }
                            }
                            let cond = if all_conds.is_empty() {
                                symbol_form("nil", span)
                            } else if all_conds.len() == 1 {
                                all_conds.into_iter().next().unwrap()
                            } else {
                                list_form(
                                    std::iter::once(symbol_form("or", span))
                                        .chain(all_conds)
                                        .collect(),
                                    span,
                                )
                            };
                            (cond, first_bindings.unwrap_or_default())
                        }
                        "not" => {
                            if parts.len() > 1 {
                                let (cond, _) = self.expand_pcase_pattern(&parts[1], val_sym, span);
                                let negated = list_form(vec![symbol_form("not", span), cond], span);
                                (negated, Vec::new())
                            } else {
                                (symbol_form("nil", span), Vec::new())
                            }
                        }
                        "app" => {
                            if parts.len() >= 3 {
                                let func = function_quote_form(parts[1].clone(), span);
                                let fresh_id = self.pcase_counter;
                                self.pcase_counter += 1;
                                let fresh_sym = format!("--pcase-app-{}--", fresh_id);
                                let app_call = list_form(
                                    vec![symbol_form("funcall", span), func, val_ref.clone()],
                                    span,
                                );
                                let app_binding =
                                    list_form(vec![symbol_form(&fresh_sym, span), app_call], span);
                                let (sub_cond, mut sub_bindings) =
                                    self.expand_pcase_pattern(&parts[2], &fresh_sym, span);
                                sub_bindings.insert(0, app_binding);
                                (sub_cond, sub_bindings)
                            } else {
                                (symbol_form("t", span), Vec::new())
                            }
                        }
                        _ => {
                            let cond = list_form(
                                vec![symbol_form("equal", span), val_ref, pattern.clone()],
                                span,
                            );
                            (cond, Vec::new())
                        }
                    }
                } else {
                    let cond = list_form(
                        vec![symbol_form("equal", span), val_ref, pattern.clone()],
                        span,
                    );
                    (cond, Vec::new())
                }
            }
            _ => (symbol_form("t", span), Vec::new()),
        }
    }

    /// Expand a pcase backquote pattern by destructuring the template.
    /// `(,x ,y ,z) matching against val becomes:
    ///   condition: (and (consp val) (consp (cdr val)) (consp (cddr val))
    ///                    (null (cdddr val)))
    ///   bindings: ((x (car val)) (y (cadr val)) (z (caddr val)))
    fn expand_pcase_backquote(
        &mut self,
        template: &SurfaceForm,
        val_sym: &str,
        span: Span,
    ) -> (SurfaceForm, Vec<SurfaceForm>) {
        let mut conditions = Vec::new();
        let mut bindings = Vec::new();
        self.destructure_bq_pattern(template, val_sym, &mut conditions, &mut bindings, span);

        let cond = if conditions.is_empty() {
            symbol_form("t", span)
        } else if conditions.len() == 1 {
            conditions.into_iter().next().unwrap()
        } else {
            list_form(
                std::iter::once(symbol_form("and", span))
                    .chain(conditions)
                    .collect(),
                span,
            )
        };
        (cond, bindings)
    }

    /// Build an inline (nthcdr n val) expression.
    fn nthcdr_expr(n: usize, val: &str, span: Span) -> SurfaceForm {
        if n == 0 {
            return symbol_form(val, span);
        }
        let mut expr = symbol_form(val, span);
        for _ in 0..n {
            expr = list_form(vec![symbol_form("cdr", span), expr], span);
        }
        expr
    }

    /// Build an inline (nth n val) = (car (nthcdr n val)) expression.
    fn nth_expr(n: usize, val: &str, span: Span) -> SurfaceForm {
        if n == 0 {
            return list_form(vec![symbol_form("car", span), symbol_form(val, span)], span);
        }
        list_form(
            vec![symbol_form("car", span), Self::nthcdr_expr(n, val, span)],
            span,
        )
    }

    fn destructure_bq_pattern(
        &mut self,
        pattern: &SurfaceForm,
        val_expr: &str,
        conditions: &mut Vec<SurfaceForm>,
        bindings: &mut Vec<SurfaceForm>,
        span: Span,
    ) {
        use crate::surface::{SurfaceAtom, SurfaceKind};
        match &pattern.kind {
            // ,var: bind the variable to the current value expression
            SurfaceKind::Comma(inner) => {
                if let SurfaceKind::Atom(SurfaceAtom::Symbol(name)) = &inner.kind {
                    let val_ref = symbol_form(val_expr, span);
                    bindings.push(list_form(vec![symbol_form(name, span), val_ref], span));
                }
            }
            // (a b c ...): destructure as a list
            SurfaceKind::List(elements) => {
                // Check that val is a consp for each cdr level needed
                for i in 0..elements.len() {
                    conditions.push(list_form(
                        vec![
                            symbol_form("consp", span),
                            Self::nthcdr_expr(i, val_expr, span),
                        ],
                        span,
                    ));
                }
                // Check the tail is nil
                conditions.push(list_form(
                    vec![
                        symbol_form("null", span),
                        Self::nthcdr_expr(elements.len(), val_expr, span),
                    ],
                    span,
                ));

                // Destructure each element
                for (i, elem) in elements.iter().enumerate() {
                    let elem_expr = format!("__bq_{}", bindings.len());
                    // Bind the element to a temp name so nested destructuring works
                    bindings.push(list_form(
                        vec![
                            symbol_form(&elem_expr, span),
                            Self::nth_expr(i, val_expr, span),
                        ],
                        span,
                    ));
                    self.destructure_bq_pattern(elem, &elem_expr, conditions, bindings, span);
                }
            }
            // Literal atoms: compare with equal
            SurfaceKind::Atom(_) => {
                let val_ref = symbol_form(val_expr, span);
                conditions.push(list_form(
                    vec![symbol_form("equal", span), val_ref, pattern.clone()],
                    span,
                ));
            }
            // Other: use equal
            _ => {
                let val_ref = symbol_form(val_expr, span);
                conditions.push(list_form(
                    vec![symbol_form("equal", span), val_ref, pattern.clone()],
                    span,
                ));
            }
        }
    }

    fn expand_destructuring_bind(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (destructuring-bind pattern expr body...)
        if items.len() < 4 {
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let pattern = &items[1];
        let expr = &items[2];
        let body: Vec<SurfaceForm> = items[3..].to_vec();
        let expanded = self.destructure_pattern(pattern, expr.clone(), body, span, 0);
        self.expand_form(expanded)
    }

    fn destructure_pattern(
        &self,
        pattern: &SurfaceForm,
        expr: SurfaceForm,
        body: Vec<SurfaceForm>,
        span: Span,
        depth: usize,
    ) -> SurfaceForm {
        match &pattern.kind {
            SurfaceKind::Atom(atom) => {
                if let Some(name) = match atom {
                    SurfaceAtom::Symbol(s) => Some(s.as_str()),
                    SurfaceAtom::Nil => Some("nil"),
                    SurfaceAtom::True => Some("t"),
                    _ => None,
                } {
                    if name == "nil" || name == "_" {
                        let mut forms = vec![expr];
                        forms.extend(body);
                        return list_form(
                            vec![symbol_form("progn", span)]
                                .into_iter()
                                .chain(forms)
                                .collect(),
                            span,
                        );
                    }
                    let binding =
                        list_form(vec![symbol_form(name, pattern.span), expr], pattern.span);
                    let mut result = vec![symbol_form("let", span), list_form(vec![binding], span)];
                    result.extend(body);
                    list_form(result, span)
                } else {
                    let mut forms = vec![expr];
                    forms.extend(body);
                    list_form(
                        vec![symbol_form("progn", span)]
                            .into_iter()
                            .chain(forms)
                            .collect(),
                        span,
                    )
                }
            }
            SurfaceKind::Quote(_) | SurfaceKind::FunctionQuote(_) => {
                let mut forms = vec![expr];
                forms.extend(body);
                list_form(
                    vec![symbol_form("progn", span)]
                        .into_iter()
                        .chain(forms)
                        .collect(),
                    span,
                )
            }
            SurfaceKind::List(patterns) => {
                self.destructure_list_pattern(patterns, expr, body, span, depth)
            }
            SurfaceKind::DottedList(required, rest) => {
                // Treat as (a b . c) where required=[a,b] and rest=c
                let mut patterns = required.clone();
                patterns.push(symbol_form("&rest", rest.span));
                patterns.push((**rest).clone());
                self.destructure_list_pattern(&patterns, expr, body, span, depth)
            }
            _ => {
                let mut forms = vec![expr];
                forms.extend(body);
                list_form(
                    vec![symbol_form("progn", span)]
                        .into_iter()
                        .chain(forms)
                        .collect(),
                    span,
                )
            }
        }
    }

    fn destructure_list_pattern(
        &self,
        patterns: &[SurfaceForm],
        expr: SurfaceForm,
        body: Vec<SurfaceForm>,
        span: Span,
        depth: usize,
    ) -> SurfaceForm {
        let tmp = symbol_form(&format!("\0dsb.{}.{}", depth, span.start), span);

        let mut required: Vec<SurfaceForm> = Vec::new();
        let mut optional: Vec<SurfaceForm> = Vec::new();
        let mut rest_pattern: Option<SurfaceForm> = None;

        let mut mode = 0;
        for pat in patterns {
            if let Some(name) = pat.symbol_name() {
                if name == "&optional" {
                    mode = 1;
                    continue;
                }
                if name == "&rest" {
                    mode = 2;
                    continue;
                }
            }
            match mode {
                0 => required.push(pat.clone()),
                1 => optional.push(pat.clone()),
                2 => {
                    rest_pattern = Some(pat.clone());
                    mode = 3;
                }
                _ => {}
            }
        }

        let mut bindings = vec![list_form(vec![tmp.clone(), expr], span)];
        let mut current_list = tmp.clone();

        for (i, pat) in required.iter().enumerate() {
            let car_form = list_form(vec![symbol_form("car", span), current_list.clone()], span);
            bindings.push(list_form(vec![pat.clone(), car_form], span));
            let next = if i + 1 < required.len() || !optional.is_empty() || rest_pattern.is_some() {
                let next_tmp = symbol_form(&format!("\0dsb.{}.{}.cdr", depth, i), span);
                let cdr_form =
                    list_form(vec![symbol_form("cdr", span), current_list.clone()], span);
                bindings.push(list_form(vec![next_tmp.clone(), cdr_form], span));
                next_tmp
            } else {
                current_list.clone()
            };
            current_list = next;
        }

        for (i, pat) in optional.iter().enumerate() {
            let car_form = list_form(vec![symbol_form("car", span), current_list.clone()], span);
            let binding = list_form(vec![pat.clone(), car_form], span);
            bindings.push(binding);
            let next = if i + 1 < optional.len() || rest_pattern.is_some() {
                let next_tmp = symbol_form(&format!("\0dsb.{}.{}.opt", depth, i), span);
                let cdr_form =
                    list_form(vec![symbol_form("cdr", span), current_list.clone()], span);
                bindings.push(list_form(vec![next_tmp.clone(), cdr_form], span));
                next_tmp
            } else {
                current_list.clone()
            };
            current_list = next;
        }

        if let Some(rest_pat) = rest_pattern {
            bindings.push(list_form(vec![rest_pat, current_list], span));
        }

        let mut result = vec![symbol_form("let*", span), list_form(bindings, span)];
        result.extend(body);
        list_form(result, span)
    }

    fn expand_flet(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (flet ((name (params) body...) ...) body...)
        if items.len() < 3 {
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let bindings_form = &items[1];
        let body: Vec<SurfaceForm> = items[2..].to_vec();

        let bindings = self.parse_flet_bindings(bindings_form, span);
        let mut let_bindings = Vec::new();
        let mut prog_body = Vec::new();

        for (name, params, fbody) in bindings {
            let lambda = list_form(
                vec![symbol_form("lambda", span), params]
                    .into_iter()
                    .chain(fbody)
                    .collect(),
                span,
            );
            let binding = list_form(vec![symbol_form(&name, span), lambda], span);
            let_bindings.push(binding);
        }

        prog_body.extend(body);
        let mut result = vec![symbol_form("let", span), list_form(let_bindings, span)];
        result.extend(prog_body);
        self.expand_form(list_form(result, span))
    }

    fn expand_labels(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (labels ((name (params) body...) ...) body...)
        // Expand to: (let ((name1 nil) ...) (setq name1 (lambda ...)) ... body...)
        if items.len() < 3 {
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let bindings_form = &items[1];
        let body: Vec<SurfaceForm> = items[2..].to_vec();

        let bindings = self.parse_flet_bindings(bindings_form, span);
        let label_names: Vec<String> = bindings.iter().map(|(n, _, _)| n.clone()).collect();
        let mut let_bindings = Vec::new();
        let mut setqs = Vec::new();

        for (name, params, fbody) in bindings {
            // (let ((name nil)) ...)
            let_bindings.push(list_form(
                vec![symbol_form(&name, span), nil_form(span)],
                span,
            ));
            // (setq name (lambda (params) body...))
            let lambda = list_form(
                vec![symbol_form("lambda", span), params]
                    .into_iter()
                    .chain(fbody)
                    .collect(),
                span,
            );
            setqs.push(list_form(
                vec![symbol_form("setq", span), symbol_form(&name, span), lambda],
                span,
            ));
        }

        // Rewrite function calls to label names in the body AND lambda bodies to use funcall
        let rewritten_body: Vec<SurfaceForm> = body
            .into_iter()
            .map(|f| Self::rewrite_labels_calls(&f, &label_names))
            .collect();
        let rewritten_setqs: Vec<SurfaceForm> = setqs
            .into_iter()
            .map(|f| Self::rewrite_labels_calls(&f, &label_names))
            .collect();

        let mut progn_body = rewritten_setqs;
        progn_body.extend(rewritten_body);
        let progn = list_form(
            vec![symbol_form("progn", span)]
                .into_iter()
                .chain(progn_body)
                .collect(),
            span,
        );
        let result = list_form(
            vec![
                symbol_form("let", span),
                list_form(let_bindings, span),
                progn,
            ],
            span,
        );
        self.expand_form(result)
    }

    fn expand_letrec(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (letrec ((var1 val1) (var2 val2) ...) body...)
        // Expand to: (let ((var1 nil) (var2 nil) ...) (setq var1 val1) (setq var2 val2) ... body...)
        if items.len() < 3 {
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let bindings_form = &items[1];
        let body: Vec<SurfaceForm> = items[2..].to_vec();

        let bindings = match &bindings_form.kind {
            SurfaceKind::List(b) => b,
            _ => {
                return SurfaceForm::new(
                    SurfaceKind::List(items.into_iter().map(|f| self.expand_form(f)).collect()),
                    span,
                );
            }
        };

        let mut let_bindings = Vec::new();
        let mut setqs = Vec::new();

        for binding in bindings {
            let (name, value) = match &binding.kind {
                SurfaceKind::List(parts) if parts.len() >= 2 => match &parts[0].kind {
                    SurfaceKind::Atom(SurfaceAtom::Symbol(n)) => (n.clone(), parts[1].clone()),
                    _ => continue,
                },
                SurfaceKind::List(parts) if parts.len() == 1 => match &parts[0].kind {
                    SurfaceKind::Atom(SurfaceAtom::Symbol(n)) => (n.clone(), nil_form(span)),
                    _ => continue,
                },
                _ => continue,
            };
            let_bindings.push(list_form(
                vec![symbol_form(&name, span), nil_form(span)],
                span,
            ));
            setqs.push(list_form(
                vec![symbol_form("setq", span), symbol_form(&name, span), value],
                span,
            ));
        }

        let mut progn_body = setqs;
        progn_body.extend(body);
        let progn = list_form(
            vec![symbol_form("progn", span)]
                .into_iter()
                .chain(progn_body)
                .collect(),
            span,
        );
        let result = list_form(
            vec![
                symbol_form("let", span),
                list_form(let_bindings, span),
                progn,
            ],
            span,
        );
        self.expand_form(result)
    }

    /// Rewrite (name args...) to (funcall name args...) when name is a label.
    fn rewrite_labels_calls(form: &SurfaceForm, label_names: &[String]) -> SurfaceForm {
        match &form.kind {
            SurfaceKind::List(items) if !items.is_empty() => {
                if let SurfaceKind::Atom(SurfaceAtom::Symbol(name)) = &items[0].kind
                    && label_names.contains(name) {
                        // (name args...) -> (funcall name args...)
                        let mut new_items = vec![
                            symbol_form("funcall", form.span),
                            symbol_form(name, form.span),
                        ];
                        new_items.extend(
                            items[1..]
                                .iter()
                                .cloned()
                                .map(|arg| Self::rewrite_labels_calls(&arg, label_names)),
                        );
                        return list_form(new_items, form.span);
                    }
                let rewritten: Vec<SurfaceForm> = items
                    .iter()
                    .map(|item| Self::rewrite_labels_calls(item, label_names))
                    .collect();
                SurfaceForm::new(SurfaceKind::List(rewritten), form.span)
            }
            SurfaceKind::List(_items) => form.clone(),
            _ => form.clone(),
        }
    }

    fn parse_flet_bindings(
        &self,
        bindings_form: &SurfaceForm,
        _span: Span,
    ) -> Vec<(String, SurfaceForm, Vec<SurfaceForm>)> {
        let SurfaceKind::List(bindings) = &bindings_form.kind else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for binding in bindings {
            let SurfaceKind::List(items) = &binding.kind else {
                continue;
            };
            if items.len() < 3 {
                continue;
            }
            let Some(name) = items[0].symbol_name().map(str::to_string) else {
                continue;
            };
            let params = items[1].clone();
            let body: Vec<SurfaceForm> = items[2..].to_vec();
            result.push((name, params, body));
        }
        result
    }

    fn expand_cl_defun(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (cl-defun name (args &optional opt &rest rest &key k1 (k2 default) &aux (a val)) body...)
        // Expand to (defun name (&rest --cl-rest--) (destructuring-bind (...) --cl-rest-- (let* (...) body...)))
        if items.len() < 4 {
            return nil_form(span);
        }
        let Some(name) = items[1].symbol_name().map(str::to_string) else {
            return nil_form(span);
        };
        let params_form = &items[2];
        let body: Vec<SurfaceForm> = items[3..].to_vec();

        let (required, optional, rest, key, aux) = self.parse_cl_lambda_list(params_form);

        if optional.is_empty() && rest.is_none() && key.is_empty() && aux.is_empty() {
            // Simple case: all required params, expand to plain defun
            let mut result = vec![
                symbol_form("defun", span),
                symbol_form(&name, span),
                params_form.clone(),
            ];
            result.extend(body.into_iter().map(|f| self.expand_form(f)));
            return list_form(result, span);
        }

        // Complex case: use a single &rest arg and destructuring-bind
        let rest_arg = symbol_form("--cl-rest--", span);
        let rest_params = list_form(vec![symbol_form("&rest", span), rest_arg.clone()], span);

        let mut dbind_params: Vec<SurfaceForm> =
            required.iter().map(|s| symbol_form(s, span)).collect();
        if !optional.is_empty() {
            dbind_params.push(symbol_form("&optional", span));
            for s in &optional {
                dbind_params.push(symbol_form(s, span));
            }
        }

        // For &key: capture remaining args into a keys variable
        let keys_var = if !key.is_empty() {
            let keys_name = rest.clone().unwrap_or_else(|| "--cl-keys--".to_string());
            dbind_params.push(symbol_form("&rest", span));
            dbind_params.push(symbol_form(&keys_name, span));
            Some(keys_name)
        } else {
            if let Some(ref r) = rest {
                dbind_params.push(symbol_form("&rest", span));
                dbind_params.push(symbol_form(r, span));
            }
            None
        };

        let dbind_pattern = list_form(dbind_params, span);

        // Build the inner body from expanded body forms
        let mut inner_body: Vec<SurfaceForm> =
            body.into_iter().map(|f| self.expand_form(f)).collect();

        // Wrap with &aux bindings (inside destructuring scope so aux can reference params)
        if !aux.is_empty() {
            let mut aux_bindings: Vec<SurfaceForm> = Vec::new();
            for (param_name, value) in &aux {
                let init = value.clone().unwrap_or_else(|| nil_form(span));
                aux_bindings.push(list_form(vec![symbol_form(param_name, span), init], span));
            }
            let aux_body = if inner_body.len() == 1 {
                inner_body.into_iter().next().unwrap()
            } else {
                list_form(
                    std::iter::once(symbol_form("progn", span))
                        .chain(inner_body)
                        .collect(),
                    span,
                )
            };
            inner_body = vec![list_form(
                vec![
                    symbol_form("let*", span),
                    list_form(aux_bindings, span),
                    aux_body,
                ],
                span,
            )];
        }

        // Wrap with &key bindings (inside destructuring scope so key can reference keys_var)
        if let Some(ref keys_name) = keys_var {
            let mut key_bindings: Vec<SurfaceForm> = Vec::new();
            for (param_name, default) in &key {
                let keyword = symbol_form(&format!(":{}", param_name), span);
                let getter = list_form(
                    vec![
                        symbol_form("plist-get", span),
                        symbol_form(keys_name, span),
                        keyword,
                    ],
                    span,
                );
                let init = match default {
                    Some(def) => {
                        list_form(vec![symbol_form("or", span), getter, def.clone()], span)
                    }
                    None => getter,
                };
                key_bindings.push(list_form(vec![symbol_form(param_name, span), init], span));
            }
            if !key_bindings.is_empty() {
                let key_body = if inner_body.len() == 1 {
                    inner_body.into_iter().next().unwrap()
                } else {
                    list_form(
                        std::iter::once(symbol_form("progn", span))
                            .chain(inner_body)
                            .collect(),
                        span,
                    )
                };
                inner_body = vec![list_form(
                    vec![
                        symbol_form("let*", span),
                        list_form(key_bindings, span),
                        key_body,
                    ],
                    span,
                )];
            }
        }

        // Build the destructuring-bind around everything
        let mut dbind_form = vec![
            symbol_form("destructuring-bind", span),
            dbind_pattern,
            rest_arg,
        ];
        dbind_form.extend(inner_body);
        let expanded_dbind = self.expand_form(list_form(dbind_form, span));

        let defun_body = vec![
            symbol_form("defun", span),
            symbol_form(&name, span),
            rest_params,
            expanded_dbind,
        ];
        list_form(defun_body, span)
    }

    fn parse_cl_lambda_list(
        &self,
        params_form: &SurfaceForm,
    ) -> (
        Vec<String>,
        Vec<String>,
        Option<String>,
        Vec<(String, Option<SurfaceForm>)>,
        Vec<(String, Option<SurfaceForm>)>,
    ) {
        let items = match &params_form.kind {
            SurfaceKind::List(items) => items,
            _ => return (Vec::new(), Vec::new(), None, Vec::new(), Vec::new()),
        };
        let mut required = Vec::new();
        let mut optional = Vec::new();
        let mut rest = None;
        let mut key: Vec<(String, Option<SurfaceForm>)> = Vec::new();
        let mut aux: Vec<(String, Option<SurfaceForm>)> = Vec::new();
        let mut section = 0; // 0=required, 1=optional, 2=rest, 3=key, 4=aux

        for item in items {
            if let Some(name) = item.symbol_name() {
                match name {
                    "&optional" => section = 1,
                    "&rest" | "&body" => section = 2,
                    "&key" => section = 3,
                    "&allow-other-keys" => {} // skip, stay in key section
                    "&aux" => section = 4,
                    _ => match section {
                        0 => required.push(name.to_string()),
                        1 => optional.push(name.to_string()),
                        2 if rest.is_none() => {
                            rest = Some(name.to_string());
                        }
                        3 => key.push((name.to_string(), None)),
                        4 => aux.push((name.to_string(), None)),
                        _ => {}
                    },
                }
                continue;
            }
            // List-form params: (name default) for &key or &aux
            if let SurfaceKind::List(parts) = &item.kind {
                if parts.is_empty() {
                    continue;
                }
                if let Some(n) = parts[0].symbol_name() {
                    match section {
                        3 => key.push((n.to_string(), parts.get(1).cloned())),
                        4 => aux.push((n.to_string(), parts.get(1).cloned())),
                        _ => {}
                    }
                }
            }
        }
        (required, optional, rest, key, aux)
    }

    fn expand_cl_macrolet(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (cl-macrolet ((name (args) body...) ...) body...)
        // Register each binding as a macro, expand the body, then unregister.
        if items.len() < 3 {
            return nil_form(span);
        }
        let bindings_form = &items[1];
        let body = &items[2..];

        let SurfaceKind::List(bindings) = &bindings_form.kind else {
            return self.expand_progn(span, body.to_vec());
        };

        let mut defined_names: Vec<String> = Vec::new();
        for binding in bindings {
            let SurfaceKind::List(bitems) = &binding.kind else {
                continue;
            };
            if bitems.len() < 3 {
                continue;
            }
            let Some(name) = bitems[0].symbol_name().map(str::to_string) else {
                continue;
            };
            let params_form = &bitems[1];
            let macro_body: Vec<SurfaceForm> = bitems[2..].to_vec();
            let macro_params = self.parse_macro_params(params_form).unwrap_or_default();
            let def = MacroDef {
                params: macro_params,
                body: macro_body,
                span: binding.span,
            };
            self.macros.insert(name.clone(), def);
            defined_names.push(name);
        }

        let result = self.expand_progn(span, body.to_vec());

        for name in &defined_names {
            self.macros.remove(name);
        }

        result
    }

    fn expand_cl_symbol_macrolet(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (cl-symbol-macrolet ((name expansion) ...) body...)
        if items.len() < 2 {
            return nil_form(span);
        }
        // Parse bindings
        let bindings = match &items[1].kind {
            SurfaceKind::List(binding_items) => {
                let mut pairs = Vec::new();
                for item in binding_items {
                    if let SurfaceKind::List(b) = &item.kind
                        && b.len() == 2
                        && let Some(name) = b[0].symbol_name()
                    {
                        pairs.push((name.to_string(), b[1].clone()));
                    }
                }
                pairs
            }
            _ => Vec::new(),
        };

        // Save old bindings and set new ones
        let saved: Vec<(String, Option<SurfaceForm>)> = bindings
            .iter()
            .map(|(name, _)| {
                let old = self.symbol_macros.get(name).cloned();
                (name.clone(), old)
            })
            .collect();

        for (name, expansion) in &bindings {
            self.symbol_macros.insert(name.clone(), expansion.clone());
        }

        // Expand body with symbol macros in scope
        let body: Vec<SurfaceForm> = items[2..].to_vec();
        let result = self.expand_progn(span, body);

        // Restore old bindings
        for (name, old) in saved {
            match old {
                Some(prev) => {
                    self.symbol_macros.insert(name, prev);
                }
                None => {
                    self.symbol_macros.remove(&name);
                }
            }
        }

        result
    }

    fn expand_progn(&mut self, span: Span, forms: Vec<SurfaceForm>) -> SurfaceForm {
        let expanded: Vec<SurfaceForm> = forms.into_iter().map(|f| self.expand_form(f)).collect();
        match expanded.len() {
            0 => nil_form(span),
            1 => expanded.into_iter().next().unwrap(),
            _ => list_form(
                std::iter::once(symbol_form("progn", span))
                    .chain(expanded)
                    .collect(),
                span,
            ),
        }
    }

    // ── cl-case expansion ──────────────────────────────────────────────

    fn expand_cl_case(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() < 3 {
            self.error(span, "cl-case requires an expression and at least one clause");
            return nil_form(span);
        }
        let expr = &items[1];
        let clauses = &items[2..];
        let temp = "--cl-case--val--";
        // Build (let ((temp expr)) (cond ...))
        let mut cond_clauses = Vec::new();
        for clause in clauses {
            let SurfaceKind::List(clause_items) = &clause.kind else { continue };
            let Some((key_form, body)) = clause_items.split_first() else { continue };
            if body.is_empty() { continue; }
            let test = if key_form.symbol_name() == Some("t")
                || key_form.symbol_name() == Some("otherwise")
            {
                SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::True), span)
            } else if let SurfaceKind::List(keys) = &key_form.kind {
                list_form(vec![
                    symbol_form("memq", span),
                    symbol_form(temp, span),
                    list_form(vec![
                        symbol_form("quote", span),
                        SurfaceForm::new(SurfaceKind::List(keys.clone()), span),
                    ], span),
                ], span)
            } else {
                list_form(vec![
                    symbol_form("eql", span),
                    symbol_form(temp, span),
                    list_form(vec![
                        symbol_form("quote", span),
                        key_form.clone(),
                    ], span),
                ], span)
            };
            cond_clauses.push(list_form(
                std::iter::once(test)
                    .chain(std::iter::once(list_form(
                        std::iter::once(symbol_form("progn", span))
                            .chain(body.iter().cloned())
                            .collect(),
                        span,
                    )))
                    .collect(),
                span,
            ));
        }
        let result = vec![
            symbol_form("let", span),
            list_form(vec![list_form(vec![
                symbol_form(temp, span),
                expr.clone(),
            ], span)], span),
            list_form(
                std::iter::once(symbol_form("cond", span))
                    .chain(cond_clauses)
                    .collect(),
                span,
            ),
        ];
        let expanded = list_form(result, span);
        self.expand_form(expanded)
    }

    // ── cl-do / cl-do* expansion ────────────────────────────────────────

    fn expand_cl_do(
        &mut self,
        span: Span,
        items: Vec<SurfaceForm>,
        sequential: bool,
    ) -> SurfaceForm {
        if items.len() < 3 {
            self.error(span, "cl-do requires bindings, end-test, and body");
            return nil_form(span);
        }
        let bindings_form = &items[1];
        let end_form = &items[2];
        let body = &items[3..];

        // Parse bindings: ((VAR INIT [STEP]) ...)
        let bindings_list = match &bindings_form.kind {
            SurfaceKind::List(b) => b.clone(),
            _ => {
                self.error(bindings_form.span, "cl-do bindings must be a list");
                return nil_form(span);
            }
        };

        let let_kind = if sequential { "let*" } else { "let" };

        // Build let-bindings (just VAR INIT)
        let mut let_bindings: Vec<SurfaceForm> = Vec::new();
        let mut step_vars: Vec<SurfaceForm> = Vec::new();
        let mut step_vals: Vec<SurfaceForm> = Vec::new();
        for binding in &bindings_list {
            let SurfaceKind::List(parts) = &binding.kind else { continue };
            if parts.is_empty() { continue; }
            let var = &parts[0];
            let init = if parts.len() > 1 { parts[1].clone() } else { nil_form(span) };
            let_bindings.push(list_form(vec![var.clone(), init], span));
            if parts.len() > 2 {
                step_vars.push(var.clone());
                step_vals.push(parts[2].clone());
            }
        }
        // Build step forms: single psetq for parallel, individual setq for sequential
        let mut step_forms: Vec<SurfaceForm> = Vec::new();
        if !step_vars.is_empty() {
            if sequential {
                for (var, val) in step_vars.iter().zip(step_vals.iter()) {
                    step_forms.push(list_form(vec![
                        symbol_form("setq", span), var.clone(), val.clone(),
                    ], span));
                }
            } else {
                let mut psetq_parts = vec![symbol_form("psetq", span)];
                for (var, val) in step_vars.iter().zip(step_vals.iter()) {
                    psetq_parts.push(var.clone());
                    psetq_parts.push(val.clone());
                }
                step_forms.push(list_form(psetq_parts, span));
            }
        }

        // Parse end form: (TEST [RESULT...])
        let (end_test, end_result) = if let SurfaceKind::List(ef) = &end_form.kind {
            if ef.is_empty() {
                (nil_form(span), vec![nil_form(span)])
            } else {
                let test = ef[0].clone();
                let result = if ef.len() > 1 {
                    ef[1..].to_vec()
                } else {
                    vec![nil_form(span)]
                };
                (test, result)
            }
        } else {
            (end_form.clone(), vec![nil_form(span)])
        };

        // Build the while loop
        let mut while_body: Vec<SurfaceForm> = Vec::new();
        // Body forms
        while_body.extend(body.iter().cloned());
        // Step forms at end
        while_body.extend(step_forms);

        let not_end = list_form(vec![symbol_form("not", span), end_test], span);
        let while_form = list_form(
            vec![
                symbol_form("while", span),
                not_end,
                list_form(
                    std::iter::once(symbol_form("progn", span))
                        .chain(while_body)
                        .collect(),
                    span,
                ),
            ],
            span,
        );

        // Build (let/let* (bindings) while-loop result...)
        let mut result = vec![
            symbol_form(let_kind, span),
            list_form(let_bindings, span),
            while_form,
        ];
        result.extend(end_result);
        let loop_body = list_form(result, span);
        // Wrap in catch for cl-return support.
        let with_catch = list_form(vec![
            symbol_form("catch", span),
            list_form(vec![symbol_form("quote", span),
                symbol_form("--cl-block-nil--", span)], span),
            loop_body,
        ], span);
        self.expand_form(with_catch)
    }

    // ── cl-dolist expansion ─────────────────────────────────────────────

    fn expand_cl_dolist(
        &mut self,
        span: Span,
        items: Vec<SurfaceForm>,
    ) -> SurfaceForm {
        if items.len() < 2 {
            self.error(span, "cl-dolist requires a spec and body");
            return nil_form(span);
        }
        // Parse spec: (VAR LIST [RESULT])
        let SurfaceKind::List(spec_parts) = &items[1].kind else {
            self.error(items[1].span, "cl-dolist spec must be a list");
            return nil_form(span);
        };
        if spec_parts.is_empty() {
            self.error(items[1].span, "cl-dolist spec requires at least a variable");
            return nil_form(span);
        }
        let var = spec_parts[0].clone();
        let list_expr = if spec_parts.len() > 1 { spec_parts[1].clone() } else { nil_form(span) };
        let result = if spec_parts.len() > 2 { vec![spec_parts[2].clone()] } else { vec![nil_form(span)] };
        let body = &items[2..];

        let tail = format!("--dolist-tail--{}--", self.pcase_counter);
        self.pcase_counter += 1;

        // (let ((TAIL LIST))
        //   (while TAIL
        //     (let ((VAR (car TAIL)))
        //       BODY...
        //       (setq TAIL (cdr TAIL))))
        //   RESULT)
        let while_body = list_form(
            vec![
                symbol_form("progn", span),
                list_form(vec![
                    symbol_form("let", span),
                    list_form(vec![list_form(vec![
                        var.clone(),
                        list_form(vec![symbol_form("car", span), symbol_form(&tail, span)], span),
                    ], span)], span),
                    list_form(
                        std::iter::once(symbol_form("progn", span))
                            .chain(body.iter().cloned())
                            .chain(std::iter::once(list_form(vec![
                                symbol_form("setq", span),
                                symbol_form(&tail, span),
                                list_form(vec![symbol_form("cdr", span), symbol_form(&tail, span)], span),
                            ], span)))
                            .collect(),
                        span,
                    ),
                ], span),
            ],
            span,
        );

        let mut parts = vec![
            symbol_form("let", span),
            list_form(vec![list_form(vec![
                symbol_form(&tail, span),
                list_expr.clone(),
            ], span)], span),
            list_form(vec![
                symbol_form("while", span),
                symbol_form(&tail, span),
                while_body,
            ], span),
        ];
        parts.extend(result);
        let loop_body = list_form(parts, span);
        // Wrap in catch for cl-return support.
        let with_catch = list_form(vec![
            symbol_form("catch", span),
            list_form(vec![symbol_form("quote", span),
                symbol_form("--cl-block-nil--", span)], span),
            loop_body,
        ], span);
        self.expand_form(with_catch)
    }

    // ── cl-dotimes expansion ────────────────────────────────────────────

    fn expand_cl_dotimes(
        &mut self,
        span: Span,
        items: Vec<SurfaceForm>,
    ) -> SurfaceForm {
        if items.len() < 2 {
            self.error(span, "cl-dotimes requires a spec and body");
            return nil_form(span);
        }
        let SurfaceKind::List(spec_parts) = &items[1].kind else {
            self.error(items[1].span, "cl-dotimes spec must be a list");
            return nil_form(span);
        };
        if spec_parts.is_empty() {
            self.error(items[1].span, "cl-dotimes spec requires at least a variable");
            return nil_form(span);
        }
        let var = spec_parts[0].clone();
        let count = if spec_parts.len() > 1 { spec_parts[1].clone() } else { nil_form(span) };
        let result = if spec_parts.len() > 2 { vec![spec_parts[2].clone()] } else { vec![nil_form(span)] };
        let body = &items[2..];

        // (let ((VAR 0))
        //   (while (< VAR COUNT)
        //     BODY...
        //     (setq VAR (1+ VAR)))
        //   RESULT)
        let while_body = list_form(
            vec![
                symbol_form("progn", span),
                list_form(
                    std::iter::once(symbol_form("progn", span))
                        .chain(body.iter().cloned())
                        .chain(std::iter::once(list_form(vec![
                            symbol_form("setq", span),
                            var.clone(),
                            list_form(vec![symbol_form("1+", span), var.clone()], span),
                        ], span)))
                        .collect(),
                    span,
                ),
            ],
            span,
        );

        let mut parts = vec![
            symbol_form("let", span),
            list_form(vec![list_form(vec![
                var.clone(),
                SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(0)), span),
            ], span)], span),
            list_form(vec![
                symbol_form("while", span),
                list_form(vec![
                    symbol_form("<", span),
                    var,
                    count,
                ], span),
                while_body,
            ], span),
        ];
        parts.extend(result);
        let loop_body = list_form(parts, span);
        // Wrap in catch for cl-return support.
        let with_catch = list_form(vec![
            symbol_form("catch", span),
            list_form(vec![symbol_form("quote", span),
                symbol_form("--cl-block-nil--", span)], span),
            loop_body,
        ], span);
        self.expand_form(with_catch)
    }

    // ── psetq expansion ────────────────────────────────────────────────

    fn expand_psetq(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() < 3 || items.len() % 2 != 1 {
            self.error(span, "psetq requires an even number of arguments");
            return nil_form(span);
        }
        let pairs = &items[1..];
        let mut let_bindings = Vec::new();
        let mut setq_forms = Vec::new();
        for i in (0..pairs.len()).step_by(2) {
            let var = &pairs[i];
            let val = &pairs[i + 1];
            let tmp = format!("--psetq-tmp-{}--", i / 2);
            let_bindings.push(list_form(
                vec![symbol_form(&tmp, span), val.clone()], span));
            setq_forms.push(list_form(
                vec![symbol_form("setq", span), var.clone(),
                     symbol_form(&tmp, span)], span));
        }
        let mut result = vec![
            symbol_form("let", span),
            list_form(let_bindings, span),
        ];
        result.extend(setq_forms);
        result.push(nil_form(span));
        let expanded = list_form(result, span);
        self.expand_form(expanded)
    }

    fn expand_cl_shiftf(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (cl-shiftf PLACE1 ... PLACEn NEWVAL)
        // Save temps for first n-1 places, then psetf each place rightward
        if items.len() < 3 {
            self.error(span, "cl-shiftf requires at least one place and a new value");
            return nil_form(span);
        }
        let all = &items[1..];
        let n_places = all.len() - 1; // last is newval
        if n_places == 0 { return nil_form(span); }
        // Save all places in temps (including last, whose old value is lost)
        let mut let_bindings = Vec::new();
        for i in 0..n_places {
            let tmp = format!("--shiftf-{}--", i);
            let_bindings.push(list_form(
                vec![symbol_form(&tmp, span), all[i].clone()], span));
        }
        let newval = &all[n_places];
        let mut psetf_pairs = Vec::new();
        for i in 0..n_places {
            psetf_pairs.push(all[i].clone());
            if i < n_places - 1 {
                // place[i] gets the OLD value of place[i+1] (saved in temp[i+1])
                psetf_pairs.push(symbol_form(
                    &format!("--shiftf-{}--", i + 1), span));
            } else {
                // Last place gets newval
                psetf_pairs.push(newval.clone());
            }
        }
        let mut psetf_form = vec![symbol_form("psetf", span)];
        psetf_form.extend(psetf_pairs);
        let mut result = vec![
            symbol_form("let*", span),
            list_form(let_bindings, span),
            list_form(psetf_form, span),
            nil_form(span),
        ];
        let expanded = list_form(result, span);
        self.expand_form(expanded)
    }

    fn expand_cl_rotatef(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (cl-rotatef A B C) => let* bind temp copies of first N-1 places,
        // then psetf to rotate: A←B, B←C, C←temp(A)
        if items.len() < 3 {
            self.error(span, "cl-rotatef requires at least two places");
            return nil_form(span);
        }
        let places = &items[1..];
        let n = places.len();
        if n < 2 {
            return nil_form(span);
        }
        // Create temp vars for first n-1 places (all but the last)
        let mut let_bindings = Vec::new();
        for i in 0..n.saturating_sub(1) {
            let tmp = format!("--rotatef-{}--", i);
            let_bindings.push(list_form(
                vec![symbol_form(&tmp, span), places[i].clone()], span));
        }
        // Build psetf pairs: place[i] gets the original value of place[i-1]
        // (wrapping: place[0] gets place[n-1]).
        let mut psetf_pairs = Vec::new();
        for i in 0..n {
            psetf_pairs.push(places[i].clone());
            let src = if i == 0 { n - 1 } else { i - 1 };
            if src < n - 1 {
                psetf_pairs.push(symbol_form(
                    &format!("--rotatef-{src}--"), span));
            } else {
                // Last place has no temp — use original form
                psetf_pairs.push(places[src].clone());
            }
        }
        if psetf_pairs.is_empty() {
            return nil_form(span);
        }
        let mut psetf_form = vec![symbol_form("psetf", span)];
        psetf_form.extend(psetf_pairs);
        let mut result = vec![
            symbol_form("let*", span),
            list_form(let_bindings, span),
            list_form(psetf_form, span),
            nil_form(span),
        ];
        let expanded = list_form(result, span);
        self.expand_form(expanded)
    }

    fn expand_psetf(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (psetf PLACE VAL PLACE VAL ...) — parallel setf
        // Expands to: (let* ((__v1 VAL1) (__v2 VAL2))
        //               (setf PLACE1 __v1) (setf PLACE2 __v2) nil)
        if items.len() < 3 || items.len() % 2 != 1 {
            self.error(span, "psetf requires an even number of arguments");
            return nil_form(span);
        }
        let pairs = &items[1..];
        let mut let_bindings = Vec::new();
        let mut setf_forms = Vec::new();
        for i in (0..pairs.len()).step_by(2) {
            let place = &pairs[i];
            let val = &pairs[i + 1];
            let tmp = format!("--psetf-tmp-{}--", i / 2);
            let_bindings.push(list_form(
                vec![symbol_form(&tmp, span), val.clone()], span));
            setf_forms.push(list_form(
                vec![symbol_form("setf", span), place.clone(),
                     symbol_form(&tmp, span)], span));
        }
        let mut result = vec![
            symbol_form("let", span),
            list_form(let_bindings, span),
        ];
        result.extend(setf_forms);
        result.push(nil_form(span));
        let expanded = list_form(result, span);
        self.expand_form(expanded)
    }

    // ── cl-loop expansion ──────────────────────────────────────────────

    fn expand_cl_loop(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() < 2 {
            return nil_form(span);
        }
        let clauses = match self.parse_loop_clauses(span, &items[1..]) {
            Some(c) => c,
            None => return nil_form(span),
        };
        if clauses.is_empty() {
            return nil_form(span);
        }
        let result = self.build_loop_expansion(span, &clauses);
        self.expand_form(result)
    }

    fn build_loop_expansion(&self, span: Span, clauses: &[LoopClause]) -> SurfaceForm {
        let mut acc_counter = 0usize;
        let mut list_counter = 0usize;
        let mut has_return = false;

        // Classify clauses
        let mut for_clauses: Vec<&LoopClause> = Vec::new();
        let mut body_clauses: Vec<&LoopClause> = Vec::new();
        let mut while_conds: Vec<SurfaceForm> = Vec::new();
        let mut initially_body: Vec<SurfaceForm> = Vec::new();
        let mut finally_body: Vec<SurfaceForm> = Vec::new();
        let mut with_bindings: Vec<(String, SurfaceForm)> = Vec::new();
        let mut accums: Vec<(AccumKind, String, SurfaceForm)> = Vec::new(); // (kind, var, init)
        // Extract named block tag before classification (must be first pass
        // so it's available for all clause handlers).
        let loop_tag: String = clauses
            .iter()
            .find_map(|c| match c {
                LoopClause::Named { name } => Some(name.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "--cl-loop-tag--".to_string());

        let mut has_repeat = false;
        let mut repeat_count: Option<SurfaceForm> = None;
        let mut has_always_never = false;
        let mut has_thereis = false;

        for clause in clauses {
            match clause {
                LoopClause::ForFrom { .. }
                | LoopClause::ForIn { .. }
                | LoopClause::ForOn { .. }
                | LoopClause::ForAcross { .. }
                | LoopClause::ForEquals { .. } => for_clauses.push(clause),
                LoopClause::While { cond } => while_conds.push(cond.clone()),
                LoopClause::Until { cond } => while_conds.push(list_form(
                    vec![symbol_form("null", span), cond.clone()],
                    span,
                )),
                LoopClause::With { var, expr } => with_bindings.push((var.clone(), expr.clone())),
                LoopClause::Repeat { count } => {
                    // repeat N → counter var, checked in while test, decremented in body
                    let counter_var = "--cl-repeat--";
                    while_conds.push(list_form(
                        vec![
                            symbol_form(">", span),
                            symbol_form(counter_var, span),
                            SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(0)), span),
                        ],
                        span,
                    ));
                    has_repeat = true;
                    repeat_count = Some(count.clone());
                }
                LoopClause::Named { .. } => { /* handled before the loop */ }
                LoopClause::Initially { body } => initially_body.extend(body.iter().cloned()),
                LoopClause::Finally { body } => {
                    // Process finally body forms, converting (return expr) to (throw ...)
                    for f in body {
                        if let SurfaceKind::List(items) = &f.kind
                            && items.first().and_then(|i| i.symbol_name()) == Some("return")
                        {
                            has_return = true;
                            let expr = if items.len() > 1 {
                                items[1].clone()
                            } else {
                                nil_form(span)
                            };
                            finally_body.push(list_form(
                                vec![
                                    symbol_form("throw", span),
                                    list_form(
                                        vec![
                                            symbol_form("quote", span),
                                            symbol_form(&loop_tag, span),
                                        ],
                                        span,
                                    ),
                                    expr,
                                ],
                                span,
                            ));
                        } else {
                            if matches!(&f.kind, SurfaceKind::List(items) if items.first().and_then(|i| i.symbol_name()) == Some("throw"))
                            {
                                has_return = true;
                            }
                            finally_body.push(f.clone());
                        }
                    }
                }
                LoopClause::Return { .. } => {
                    has_return = true;
                    body_clauses.push(clause);
                }
                LoopClause::Always { .. } | LoopClause::Never { .. } => {
                    has_always_never = true;
                    body_clauses.push(clause);
                }
                LoopClause::Thereis { .. } => {
                    has_thereis = true;
                    body_clauses.push(clause);
                }
                LoopClause::If {
                    then_clauses,
                    else_clauses,
                    ..
                } => {
                    if clauses_contain_return(then_clauses)
                        || else_clauses
                            .as_ref()
                            .is_some_and(|ec| clauses_contain_return(ec))
                    {
                        has_return = true;
                    }
                    if !has_always_never {
                        has_always_never = clauses.iter().any(|c| {
                            matches!(c, LoopClause::Always { .. } | LoopClause::Never { .. })
                        });
                    }
                    if !has_thereis {
                        has_thereis = clauses
                            .iter()
                            .any(|c| matches!(c, LoopClause::Thereis { .. }));
                    }
                    body_clauses.push(clause);
                }
                _ => body_clauses.push(clause),
            }
        }

        // Allocate accumulators for collection/aggregation clauses (including nested in when/if)
        let mut accum_map: Vec<(AccumKind, Option<String>, String)> = Vec::new(); // (kind, into_name, var_name)
        let all_accum_clauses = Self::collect_accum_kinds(body_clauses.iter().copied());
        for (kind, into_name) in all_accum_clauses {
            // Reuse existing accumulator of same kind + into name
            if accum_map
                .iter()
                .any(|(k, n, _)| *k == kind && n.as_ref() == into_name.as_ref())
            {
                continue;
            }
            let var_name = into_name
                .clone()
                .unwrap_or_else(|| format!("--cl-acc-{}--", acc_counter));
            let init = match kind {
                AccumKind::Collect | AccumKind::Append | AccumKind::Nconc => nil_form(span),
                AccumKind::Sum | AccumKind::Count => {
                    SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(0)), span)
                }
                AccumKind::Minimize | AccumKind::Maximize => nil_form(span),
            };
            accums.push((kind, var_name.clone(), init));
            accum_map.push((kind, into_name, var_name));
            acc_counter += 1;
        }

        // Build let-bindings
        let mut let_bindings: Vec<SurfaceForm> = Vec::new();

        // Accumulator bindings
        for (_, name, init) in &accums {
            let_bindings.push(list_form(vec![symbol_form(name, span), init.clone()], span));
        }

        // For-from bindings and list temp bindings
        let mut for_from_info: Vec<(
            String,
            SurfaceForm,
            Option<(SurfaceForm, EndDirection)>,
            Option<SurfaceForm>,
        )> = Vec::new(); // var, start, end, step
        let mut for_in_info: Vec<(
            String,
            String,
            SurfaceForm,
            Option<SurfaceForm>,
            Option<SurfaceForm>,
        )> = Vec::new(); // var, list-temp, list-expr, step-fn, destructure
        let mut for_on_info: Vec<(String, String, SurfaceForm)> = Vec::new(); // var, list-temp, list-expr
        let mut for_across_info: Vec<(String, String, String)> = Vec::new(); // var, vec-temp, idx-temp
        let mut for_eq_info: Vec<(String, SurfaceForm)> = Vec::new(); // var, expr (no then)
        let mut for_eq_step: Vec<(String, SurfaceForm)> = Vec::new(); // var, step (has then)

        for clause in &for_clauses {
            match clause {
                LoopClause::ForFrom {
                    var,
                    start,
                    end,
                    step,
                } => {
                    let_bindings.push(list_form(vec![symbol_form(var, span), start.clone()], span));
                    for_from_info.push((var.clone(), start.clone(), end.clone(), step.clone()));
                }
                LoopClause::ForIn {
                    var,
                    list_expr,
                    step_fn,
                    destructure,
                } => {
                    let list_temp = format!("--cl-list-{}--", list_counter);
                    list_counter += 1;
                    let_bindings.push(list_form(
                        vec![symbol_form(&list_temp, span), list_expr.clone()],
                        span,
                    ));
                    let_bindings.push(list_form(
                        vec![symbol_form(var, span), nil_form(span)],
                        span,
                    ));
                    for_in_info.push((
                        var.clone(),
                        list_temp,
                        list_expr.clone(),
                        step_fn.clone(),
                        destructure.clone(),
                    ));
                }
                LoopClause::ForOn { var, list_expr } => {
                    let list_temp = format!("--cl-list-{}--", list_counter);
                    list_counter += 1;
                    let_bindings.push(list_form(
                        vec![symbol_form(&list_temp, span), list_expr.clone()],
                        span,
                    ));
                    for_on_info.push((var.clone(), list_temp, list_expr.clone()));
                }
                LoopClause::ForAcross { var, vec_expr } => {
                    let vec_temp = format!("--cl-vec-{}--", list_counter);
                    let idx_temp = format!("--cl-idx-{}--", list_counter);
                    list_counter += 1;
                    let_bindings.push(list_form(
                        vec![symbol_form(&vec_temp, span), vec_expr.clone()],
                        span,
                    ));
                    let_bindings.push(list_form(
                        vec![symbol_form(&idx_temp, span), int_form(0, span)],
                        span,
                    ));
                    let_bindings.push(list_form(
                        vec![symbol_form(var, span), nil_form(span)],
                        span,
                    ));
                    for_across_info.push((var.clone(), vec_temp, idx_temp));
                }
                LoopClause::ForEquals {
                    var,
                    expr,
                    then_expr,
                } => {
                    if let Some(step) = then_expr {
                        // for x = init then step: bind init, step goes after body
                        let_bindings
                            .push(list_form(vec![symbol_form(var, span), expr.clone()], span));
                        for_eq_step.push((var.clone(), step.clone()));
                    } else {
                        // for x = expr: bind nil, setq expr at body start
                        let_bindings.push(list_form(
                            vec![symbol_form(var, span), nil_form(span)],
                            span,
                        ));
                        for_eq_info.push((var.clone(), expr.clone()));
                    }
                }
                _ => {}
            }
        }

        // With bindings
        for (var, expr) in &with_bindings {
            let_bindings.push(list_form(vec![symbol_form(var, span), expr.clone()], span));
        }

        // Repeat binding
        if has_repeat
            && let Some(count) = &repeat_count {
                let_bindings.push(list_form(
                    vec![symbol_form("--cl-repeat--", span), count.clone()],
                    span,
                ));
            }

        // Always/never flag variable
        if has_always_never {
            let_bindings.push(list_form(
                vec![symbol_form("--cl-always--", span), symbol_form("t", span)],
                span,
            ));
        }

        // Thereis result variable
        if has_thereis {
            let_bindings.push(list_form(
                vec![symbol_form("--cl-thereis--", span), nil_form(span)],
                span,
            ));
        }

        // Build while test
        let mut while_tests: Vec<SurfaceForm> = Vec::new();

        // For-from: comparison based on direction when end is specified
        for (var, start, end, _) in &for_from_info {
            let _ = start;
            if let Some((end_val, direction)) = end {
                let cmp_op = match direction {
                    EndDirection::To => "<=",
                    EndDirection::Downto => ">=",
                    EndDirection::Below => "<",
                    EndDirection::Above => ">",
                };
                while_tests.push(list_form(
                    vec![
                        symbol_form(cmp_op, span),
                        symbol_form(var, span),
                        end_val.clone(),
                    ],
                    span,
                ));
            }
            // No end means open-ended loop — only while/until conditions control termination
        }

        // For-in/for-on: list-temp truthiness
        for (_, list_temp, _, _, _) in &for_in_info {
            while_tests.push(symbol_form(list_temp, span));
        }
        for (_, list_temp, _) in &for_on_info {
            while_tests.push(symbol_form(list_temp, span));
        }
        // For-across: (< idx (length vec))
        for (_, vec_temp, idx_temp) in &for_across_info {
            while_tests.push(list_form(
                vec![
                    symbol_form("<", span),
                    symbol_form(idx_temp, span),
                    list_form(
                        vec![symbol_form("length", span), symbol_form(vec_temp, span)],
                        span,
                    ),
                ],
                span,
            ));
        }

        // Explicit while/until conditions
        while_tests.extend(while_conds);

        // always/never short-circuit: stop the loop when the flag becomes nil
        if has_always_never {
            while_tests.push(symbol_form("--cl-always--", span));
        }

        let while_test = if while_tests.is_empty() {
            symbol_form("t", span)
        } else if while_tests.len() == 1 {
            while_tests.into_iter().next().unwrap()
        } else {
            list_form(
                std::iter::once(symbol_form("and", span))
                    .chain(while_tests)
                    .collect(),
                span,
            )
        };

        // Build while body
        let mut while_body: Vec<SurfaceForm> = Vec::new();

        // For-equals: setq at body start
        for (var, expr) in &for_eq_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    expr.clone(),
                ],
                span,
            ));
        }

        // For-in: setq var (car --list--), then destructure if pattern present
        for (var, list_temp, _, _, destructure) in &for_in_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    list_form(
                        vec![symbol_form("car", span), symbol_form(list_temp, span)],
                        span,
                    ),
                ],
                span,
            ));
            // Add destructuring bindings for patterns like (key . val)
            if let Some(pattern) = destructure {
                Self::emit_destructure_bindings(
                    &mut while_body,
                    pattern,
                    &symbol_form(var, span),
                    span,
                );
            }
        }

        // For-on: setq var --list--
        for (var, list_temp, _) in &for_on_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    symbol_form(list_temp, span),
                ],
                span,
            ));
        }

        // For-across: setq var (aref vec idx)
        for (var, vec_temp, idx_temp) in &for_across_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    list_form(
                        vec![
                            symbol_form("aref", span),
                            symbol_form(vec_temp, span),
                            symbol_form(idx_temp, span),
                        ],
                        span,
                    ),
                ],
                span,
            ));
        }

        // Body clauses (collect, sum, do, return, if, etc.)
        for clause in &body_clauses {
            match clause {
                LoopClause::Collect { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Collect, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("cons", span),
                                    expr.clone(),
                                    symbol_form(&acc_var, span),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Append { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Append, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("append", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Nconc { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Nconc, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("nconc", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Sum { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Sum, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("+", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Count { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Count, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("+", span),
                                    symbol_form(&acc_var, span),
                                    list_form(
                                        vec![
                                            symbol_form("if", span),
                                            expr.clone(),
                                            SurfaceForm::new(
                                                SurfaceKind::Atom(SurfaceAtom::Int(1)),
                                                span,
                                            ),
                                            SurfaceForm::new(
                                                SurfaceKind::Atom(SurfaceAtom::Int(0)),
                                                span,
                                            ),
                                        ],
                                        span,
                                    ),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Do { body } => {
                    while_body.extend(body.iter().cloned());
                }
                LoopClause::Return { expr } => {
                    while_body.push(list_form(
                        vec![
                            symbol_form("throw", span),
                            list_form(
                                vec![
                                    symbol_form("quote", span),
                                    symbol_form(&loop_tag, span),
                                ],
                                span,
                            ),
                            expr.clone(),
                        ],
                        span,
                    ));
                }
                LoopClause::If {
                    cond,
                    then_clauses,
                    else_clauses,
                } => {
                    let then_body = self.build_if_body(span, then_clauses, &accum_map, &loop_tag);
                    let if_form = if let Some(else_cls) = else_clauses {
                        let else_body = self.build_if_body(span, else_cls, &accum_map, &loop_tag);
                        list_form(
                            vec![
                                symbol_form("if", span),
                                cond.clone(),
                                self.wrap_progn(then_body, span),
                                self.wrap_progn(else_body, span),
                            ],
                            span,
                        )
                    } else {
                        list_form(
                            vec![
                                symbol_form("if", span),
                                cond.clone(),
                                self.wrap_progn(then_body, span),
                            ],
                            span,
                        )
                    };
                    while_body.push(if_form);
                }
                LoopClause::Always { expr } => {
                    // (if (null expr) (setq --cl-always-- nil))
                    while_body.push(list_form(
                        vec![
                            symbol_form("if", span),
                            list_form(vec![symbol_form("null", span), expr.clone()], span),
                            list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form("--cl-always--", span),
                                    nil_form(span),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Never { expr } => {
                    // (if expr (setq --cl-always-- nil))
                    while_body.push(list_form(
                        vec![
                            symbol_form("if", span),
                            expr.clone(),
                            list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form("--cl-always--", span),
                                    nil_form(span),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Thereis { expr } => {
                    // (if (and (null --cl-thereis--) expr)
                    //     (setq --cl-thereis-- expr))
                    while_body.push(list_form(
                        vec![
                            symbol_form("if", span),
                            list_form(
                                vec![
                                    symbol_form("and", span),
                                    list_form(
                                        vec![
                                            symbol_form("null", span),
                                            symbol_form("--cl-thereis--", span),
                                        ],
                                        span,
                                    ),
                                    expr.clone(),
                                ],
                                span,
                            ),
                            list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form("--cl-thereis--", span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Minimize { expr, into } => {
                    // (if (or (null acc) (< expr acc)) (setq acc expr))
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Minimize, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("if", span),
                            list_form(
                                vec![
                                    symbol_form("or", span),
                                    list_form(
                                        vec![
                                            symbol_form("null", span),
                                            symbol_form(&acc_var, span),
                                        ],
                                        span,
                                    ),
                                    list_form(
                                        vec![
                                            symbol_form("<", span),
                                            expr.clone(),
                                            symbol_form(&acc_var, span),
                                        ],
                                        span,
                                    ),
                                ],
                                span,
                            ),
                            list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Maximize { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Maximize, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("if", span),
                            list_form(
                                vec![
                                    symbol_form("or", span),
                                    list_form(
                                        vec![
                                            symbol_form("null", span),
                                            symbol_form(&acc_var, span),
                                        ],
                                        span,
                                    ),
                                    list_form(
                                        vec![
                                            symbol_form(">", span),
                                            expr.clone(),
                                            symbol_form(&acc_var, span),
                                        ],
                                        span,
                                    ),
                                ],
                                span,
                            ),
                            list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                _ => {}
            }
        }

        // For-in advance: setq --list-- (step-fn --list--) or (cdr --list--)
        for (_, list_temp, _, step_fn, _) in &for_in_info {
            let advance_expr = if let Some(step) = step_fn {
                list_form(
                    vec![
                        symbol_form("funcall", span),
                        step.clone(),
                        symbol_form(list_temp, span),
                    ],
                    span,
                )
            } else {
                list_form(
                    vec![symbol_form("cdr", span), symbol_form(list_temp, span)],
                    span,
                )
            };
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(list_temp, span),
                    advance_expr,
                ],
                span,
            ));
        }

        // For-on advance: setq --list-- (cdr --list--)
        for (_, list_temp, _) in &for_on_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(list_temp, span),
                    list_form(
                        vec![symbol_form("cdr", span), symbol_form(list_temp, span)],
                        span,
                    ),
                ],
                span,
            ));
        }

        // For-across advance: setq --idx-- (+ --idx-- 1)
        for (_, _, idx_temp) in &for_across_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(idx_temp, span),
                    list_form(
                        vec![
                            symbol_form("+", span),
                            symbol_form(idx_temp, span),
                            int_form(1, span),
                        ],
                        span,
                    ),
                ],
                span,
            ));
        }

        // For-from advance: setq var (+ var step)
        for (var, _, end, step) in &for_from_info {
            let default_step = match end {
                Some((_, EndDirection::Downto | EndDirection::Above)) => -1,
                _ => 1,
            };
            let step_val = step.clone().unwrap_or_else(|| {
                SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(default_step)), span)
            });
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    list_form(
                        vec![symbol_form("+", span), symbol_form(var, span), step_val],
                        span,
                    ),
                ],
                span,
            ));
        }

        // For-equals step (for x = init then step): update at end of iteration
        for (var, step) in &for_eq_step {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    step.clone(),
                ],
                span,
            ));
        }

        // Repeat counter decrement
        if has_repeat {
            let counter_var = "--cl-repeat--";
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(counter_var, span),
                    list_form(
                        vec![
                            symbol_form("-", span),
                            symbol_form(counter_var, span),
                            SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(1)), span),
                        ],
                        span,
                    ),
                ],
                span,
            ));
        }

        // Build result expression (after while loop)
        let mut after_while: Vec<SurfaceForm> = Vec::new();

        // nreverse for collect accumulators
        for (kind, name, _) in &accums {
            if *kind == AccumKind::Collect {
                after_while.push(list_form(
                    vec![
                        symbol_form("setq", span),
                        symbol_form(name, span),
                        list_form(
                            vec![symbol_form("nreverse", span), symbol_form(name, span)],
                            span,
                        ),
                    ],
                    span,
                ));
            }
        }

        // Finally body
        after_while.extend(finally_body.iter().cloned());

        // Result expression — use first default accumulator (without `into`)
        let default_accum = accum_map
            .iter()
            .find(|(_, into_name, _)| into_name.is_none())
            .map(|(_, _, var_name)| var_name.clone());
        if let Some(var_name) = default_accum {
            after_while.push(symbol_form(&var_name, span));
        } else if has_always_never {
            after_while.push(symbol_form("--cl-always--", span));
        } else if has_thereis {
            after_while.push(symbol_form("--cl-thereis--", span));
        } else if finally_body.is_empty() {
            after_while.push(nil_form(span));
        }

        // Build the let body
        let mut let_body: Vec<SurfaceForm> = Vec::new();
        let_body.extend(initially_body.iter().cloned());
        let_body.push(list_form(
            std::iter::once(symbol_form("while", span))
                .chain(std::iter::once(while_test))
                .chain(while_body)
                .collect(),
            span,
        ));
        let_body.extend(after_while);

        let let_form = list_form(
            std::iter::once(symbol_form("let", span))
                .chain(std::iter::once(list_form(let_bindings, span)))
                .chain(let_body)
                .collect(),
            span,
        );

        // Wrap in catch/throw if return clause present, or if named block
        // (so cl-return-from can target the named block).
        let has_named = loop_tag != "--cl-loop-tag--";
        if has_return || has_named {
            list_form(
                vec![
                    symbol_form("catch", span),
                    list_form(
                        vec![
                            symbol_form("quote", span),
                            symbol_form(&loop_tag, span),
                        ],
                        span,
                    ),
                    let_form,
                ],
                span,
            )
        } else {
            let_form
        }
    }

    /// Emit setq bindings for a destructuring pattern like (key . val) or (a b c).
    fn emit_destructure_bindings(
        body: &mut Vec<SurfaceForm>,
        pattern: &SurfaceForm,
        source: &SurfaceForm,
        span: Span,
    ) {
        match &pattern.kind {
            SurfaceKind::DottedList(items, tail) => {
                // (a b . rest) — a=(car source), b=(cadr source), rest=(cddr source)
                let mut access = source.clone();
                for item in items {
                    if let Some(name) = item.symbol_name() {
                        body.push(list_form(
                            vec![
                                symbol_form("setq", span),
                                symbol_form(name, span),
                                list_form(vec![symbol_form("car", span), access.clone()], span),
                            ],
                            span,
                        ));
                        access = list_form(vec![symbol_form("cdr", span), access], span);
                    }
                }
                if let Some(tail_name) = tail.symbol_name() {
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(tail_name, span),
                            access,
                        ],
                        span,
                    ));
                }
            }
            SurfaceKind::List(items) => {
                // (a b c) — a=(car source), b=(cadr source), c=(caddr source)
                let mut access = source.clone();
                for (i, item) in items.iter().enumerate() {
                    if let Some(name) = item.symbol_name() {
                        if i == 0 {
                            body.push(list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form(name, span),
                                    list_form(vec![symbol_form("car", span), source.clone()], span),
                                ],
                                span,
                            ));
                            access =
                                list_form(vec![symbol_form("cdr", span), source.clone()], span);
                        } else {
                            body.push(list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form(name, span),
                                    list_form(vec![symbol_form("car", span), access.clone()], span),
                                ],
                                span,
                            ));
                            access = list_form(vec![symbol_form("cdr", span), access], span);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn find_accum_var(
        &self,
        accum_map: &[(AccumKind, Option<String>, String)],
        kind: AccumKind,
        into: &Option<String>,
    ) -> String {
        accum_map
            .iter()
            .find(|(k, n, _)| *k == kind && n.as_ref() == into.as_ref())
            .map(|(_, _, name)| name.clone())
            .unwrap_or_else(|| "--cl-acc-unknown--".into())
    }

    /// Collect all AccumKinds with their into-names from clauses, recursing into when/if branches.
    fn collect_accum_kinds<'a>(
        clauses: impl IntoIterator<Item = &'a LoopClause>,
    ) -> Vec<(AccumKind, Option<String>)> {
        let mut kinds = Vec::new();
        for clause in clauses {
            match clause {
                LoopClause::Collect { into, .. } => kinds.push((AccumKind::Collect, into.clone())),
                LoopClause::Append { into, .. } => kinds.push((AccumKind::Append, into.clone())),
                LoopClause::Nconc { into, .. } => kinds.push((AccumKind::Nconc, into.clone())),
                LoopClause::Sum { into, .. } => kinds.push((AccumKind::Sum, into.clone())),
                LoopClause::Count { into, .. } => kinds.push((AccumKind::Count, into.clone())),
                LoopClause::Minimize { into, .. } => {
                    kinds.push((AccumKind::Minimize, into.clone()))
                }
                LoopClause::Maximize { into, .. } => {
                    kinds.push((AccumKind::Maximize, into.clone()))
                }
                LoopClause::If {
                    then_clauses,
                    else_clauses,
                    ..
                } => {
                    kinds.extend(Self::collect_accum_kinds(then_clauses.iter()));
                    if let Some(else_cls) = else_clauses {
                        kinds.extend(Self::collect_accum_kinds(else_cls.iter()));
                    }
                }
                _ => {}
            }
        }
        kinds
    }

    fn build_if_body(
        &self,
        span: Span,
        clauses: &[LoopClause],
        accum_map: &[(AccumKind, Option<String>, String)],
        tag: &str,
    ) -> Vec<SurfaceForm> {
        let mut body: Vec<SurfaceForm> = Vec::new();
        for clause in clauses {
            match clause {
                LoopClause::Collect { expr, into } => {
                    let acc_var = self.find_accum_var(accum_map, AccumKind::Collect, into);
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("cons", span),
                                    expr.clone(),
                                    symbol_form(&acc_var, span),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Append { expr, into } => {
                    let acc_var = self.find_accum_var(accum_map, AccumKind::Append, into);
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("append", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Nconc { expr, into } => {
                    let acc_var = self.find_accum_var(accum_map, AccumKind::Nconc, into);
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("nconc", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Sum { expr, into } => {
                    let acc_var = self.find_accum_var(accum_map, AccumKind::Sum, into);
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("+", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Count { expr, into } => {
                    let acc_var = self.find_accum_var(accum_map, AccumKind::Count, into);
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("+", span),
                                    symbol_form(&acc_var, span),
                                    list_form(
                                        vec![
                                            symbol_form("if", span),
                                            expr.clone(),
                                            SurfaceForm::new(
                                                SurfaceKind::Atom(SurfaceAtom::Int(1)),
                                                span,
                                            ),
                                            SurfaceForm::new(
                                                SurfaceKind::Atom(SurfaceAtom::Int(0)),
                                                span,
                                            ),
                                        ],
                                        span,
                                    ),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Do { body: b } => body.extend(b.iter().cloned()),
                LoopClause::Return { expr } => {
                    body.push(list_form(
                        vec![
                            symbol_form("throw", span),
                            list_form(
                                vec![
                                    symbol_form("quote", span),
                                    symbol_form(tag, span),
                                ],
                                span,
                            ),
                            expr.clone(),
                        ],
                        span,
                    ));
                }
                _ => {}
            }
        }
        body
    }

    fn wrap_progn(&self, forms: Vec<SurfaceForm>, span: Span) -> SurfaceForm {
        match forms.len() {
            0 => nil_form(span),
            1 => forms.into_iter().next().unwrap(),
            _ => list_form(
                std::iter::once(symbol_form("progn", span))
                    .chain(forms)
                    .collect(),
                span,
            ),
        }
    }

    // ── cl-loop clause parser ──────────────────────────────────────────

    fn parse_into_keyword(items: &[SurfaceForm], pos: &mut usize) -> Option<String> {
        if *pos < items.len() && items[*pos].symbol_name() == Some("into") {
            *pos += 1;
            if *pos < items.len() {
                let name = items[*pos].symbol_name().map(str::to_string);
                *pos += 1;
                return name;
            }
        }
        None
    }

    fn parse_loop_clauses(&mut self, span: Span, items: &[SurfaceForm]) -> Option<Vec<LoopClause>> {
        let mut clauses = Vec::new();
        let mut pos = 0;
        while pos < items.len() {
            // Non-keyword forms at the top level are treated as implicit do body
            let keyword = match items[pos].symbol_name() {
                Some(kw) => kw,
                None => {
                    // Implicit do: treat this form as a body expression
                    let body_form = items[pos].clone();
                    pos += 1;
                    clauses.push(LoopClause::Do {
                        body: vec![body_form],
                    });
                    continue;
                }
            };
            match keyword {
                "for" => {
                    pos += 1;
                    let clause = self.parse_for_clause(span, items, &mut pos)?;
                    clauses.push(clause);
                }
                "collect" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(items, &mut pos);
                    clauses.push(LoopClause::Collect { expr, into });
                }
                "append" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(items, &mut pos);
                    clauses.push(LoopClause::Append { expr, into });
                }
                "nconc" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(items, &mut pos);
                    clauses.push(LoopClause::Nconc { expr, into });
                }
                "sum" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(items, &mut pos);
                    clauses.push(LoopClause::Sum { expr, into });
                }
                "count" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(items, &mut pos);
                    clauses.push(LoopClause::Count { expr, into });
                }
                "minimize" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(items, &mut pos);
                    clauses.push(LoopClause::Minimize { expr, into });
                }
                "maximize" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(items, &mut pos);
                    clauses.push(LoopClause::Maximize { expr, into });
                }
                "thereis" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::Thereis {
                        expr: items[pos].clone(),
                    });
                    pos += 1;
                }
                "do" => {
                    pos += 1;
                    let body = self.collect_until_keyword(items, &mut pos);
                    clauses.push(LoopClause::Do { body });
                }
                "while" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::While {
                        cond: items[pos].clone(),
                    });
                    pos += 1;
                }
                "until" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::Until {
                        cond: items[pos].clone(),
                    });
                    pos += 1;
                }
                "return" => {
                    pos += 1;
                    let expr = if pos < items.len() {
                        let e = items[pos].clone();
                        pos += 1;
                        e
                    } else {
                        nil_form(span)
                    };
                    clauses.push(LoopClause::Return { expr });
                }
                "if" | "when" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let cond = items[pos].clone();
                    pos += 1;
                    let then_clauses = self.parse_sub_clauses(span, items, &mut pos)?;
                    let else_clauses =
                        if pos < items.len() && items[pos].symbol_name() == Some("else") {
                            pos += 1;
                            Some(self.parse_sub_clauses(span, items, &mut pos)?)
                        } else {
                            None
                        };
                    if pos < items.len() && items[pos].symbol_name() == Some("end") {
                        pos += 1;
                    }
                    clauses.push(LoopClause::If {
                        cond,
                        then_clauses,
                        else_clauses,
                    });
                }
                "with" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let var = items[pos].symbol_name()?.to_string();
                    pos += 1;
                    let expr = if pos < items.len() && items[pos].symbol_name() == Some("=") {
                        pos += 1;
                        if pos >= items.len() {
                            return None;
                        }
                        let e = items[pos].clone();
                        pos += 1;
                        e
                    } else {
                        nil_form(span)
                    };
                    clauses.push(LoopClause::With { var, expr });
                }
                "initially" => {
                    pos += 1;
                    let body = self.collect_until_keyword(items, &mut pos);
                    clauses.push(LoopClause::Initially { body });
                }
                "finally" => {
                    pos += 1;
                    // Handle "finally return expr" — store as (return expr); the actual
                    // throw tag is injected by build_loop_expansion with the correct tag.
                    if pos < items.len() && items[pos].symbol_name() == Some("return") {
                        pos += 1;
                        if pos < items.len() {
                            let expr = items[pos].clone();
                            pos += 1;
                            clauses.push(LoopClause::Finally {
                                body: vec![list_form(
                                    vec![
                                        symbol_form("return", span),
                                        expr,
                                    ],
                                    span,
                                )],
                            });
                        } else {
                            let body = self.collect_until_keyword(items, &mut pos);
                            clauses.push(LoopClause::Finally { body });
                        }
                    } else {
                        let body = self.collect_until_keyword(items, &mut pos);
                        clauses.push(LoopClause::Finally { body });
                    }
                }
                "always" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::Always {
                        expr: items[pos].clone(),
                    });
                    pos += 1;
                }
                "never" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::Never {
                        expr: items[pos].clone(),
                    });
                    pos += 1;
                }
                "named" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let name = items[pos].symbol_name().map(str::to_string)?;
                    clauses.push(LoopClause::Named { name });
                    pos += 1;
                }
                "repeat" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::Repeat {
                        count: items[pos].clone(),
                    });
                    pos += 1;
                }
                _ => {
                    // Unknown keyword — treat as implicit do body expression
                    let body_form = items[pos].clone();
                    pos += 1;
                    clauses.push(LoopClause::Do {
                        body: vec![body_form],
                    });
                }
            }
        }
        Some(clauses)
    }

    fn parse_for_clause(
        &mut self,
        span: Span,
        items: &[SurfaceForm],
        pos: &mut usize,
    ) -> Option<LoopClause> {
        if *pos >= items.len() {
            return None;
        }
        // Check if the variable is a destructuring pattern like (key . val)
        let (var, destructure_pattern) = if let Some(name) = items[*pos].symbol_name() {
            (name.to_string(), None)
        } else {
            // Destructuring pattern — use a temp var
            let pattern = items[*pos].clone();
            let temp = format!("--cl-destructure-{}--", self.pcase_counter);
            self.pcase_counter += 1;
            (temp, Some(pattern))
        };
        *pos += 1;

        // Peek at next keyword to determine sub-type
        if *pos >= items.len() {
            // bare: (for var) — treat as for-equals nil
            return Some(LoopClause::ForEquals {
                var,
                expr: nil_form(span),
                then_expr: None,
            });
        }

        let next_kw = items[*pos].symbol_name().unwrap_or("");
        match next_kw {
            "from" | "upfrom" | "downfrom" => {
                let _is_down = next_kw == "downfrom";
                *pos += 1;
                let start = items.get(*pos)?.clone();
                *pos += 1;
                let mut end = None;
                let mut step = None;
                while *pos < items.len() {
                    let kw = items[*pos].symbol_name().unwrap_or("");
                    match kw {
                        "to" | "upto" => {
                            *pos += 1;
                            end = Some((items.get(*pos)?.clone(), EndDirection::To));
                            *pos += 1;
                        }
                        "downto" => {
                            *pos += 1;
                            end = Some((items.get(*pos)?.clone(), EndDirection::Downto));
                            *pos += 1;
                        }
                        "below" => {
                            *pos += 1;
                            end = Some((items.get(*pos)?.clone(), EndDirection::Below));
                            *pos += 1;
                        }
                        "above" => {
                            *pos += 1;
                            end = Some((items.get(*pos)?.clone(), EndDirection::Above));
                            *pos += 1;
                        }
                        "by" => {
                            *pos += 1;
                            step = Some(items.get(*pos)?.clone());
                            *pos += 1;
                        }
                        _ => break,
                    }
                }
                Some(LoopClause::ForFrom {
                    var,
                    start,
                    end,
                    step,
                })
            }
            // Implicit from 0: (for var to N) or (for var below N) etc.
            "to" | "upto" | "downto" | "below" | "above" => {
                let dir = match next_kw {
                    "to" | "upto" => EndDirection::To,
                    "downto" => EndDirection::Downto,
                    "below" => EndDirection::Below,
                    "above" => EndDirection::Above,
                    other => {
                        self.error(span, format!("cl-loop: unexpected direction keyword `{other}`"));
                        return None;
                    }
                };
                *pos += 1;
                let end_val = items.get(*pos)?.clone();
                *pos += 1;
                let mut step = None;
                if *pos < items.len() && items[*pos].symbol_name() == Some("by") {
                    *pos += 1;
                    step = Some(items.get(*pos)?.clone());
                    *pos += 1;
                }
                Some(LoopClause::ForFrom {
                    var,
                    start: SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(0)), span),
                    end: Some((end_val, dir)),
                    step,
                })
            }
            "in" => {
                *pos += 1;
                let list_expr = items.get(*pos)?.clone();
                *pos += 1;
                let step_fn = if *pos < items.len() && items[*pos].symbol_name() == Some("by") {
                    *pos += 1;
                    if *pos < items.len() {
                        let step = items[*pos].clone();
                        *pos += 1;
                        Some(step)
                    } else {
                        None
                    }
                } else {
                    None
                };
                Some(LoopClause::ForIn {
                    var,
                    list_expr,
                    step_fn,
                    destructure: destructure_pattern,
                })
            }
            "on" => {
                *pos += 1;
                let list_expr = items.get(*pos)?.clone();
                *pos += 1;
                Some(LoopClause::ForOn { var, list_expr })
            }
            "across" => {
                *pos += 1;
                let vec_expr = items.get(*pos)?.clone();
                *pos += 1;
                Some(LoopClause::ForAcross { var, vec_expr })
            }
            "=" => {
                *pos += 1;
                let expr = items.get(*pos)?.clone();
                *pos += 1;
                // Optional "then step-expr"
                let then_expr = if *pos < items.len() && items[*pos].symbol_name() == Some("then") {
                    *pos += 1;
                    if *pos < items.len() {
                        let step = items[*pos].clone();
                        *pos += 1;
                        Some(step)
                    } else {
                        None
                    }
                } else {
                    None
                };
                Some(LoopClause::ForEquals {
                    var,
                    expr,
                    then_expr,
                })
            }
            _ => {
                // No recognized sub-keyword: treat as for-equals with the next form
                Some(LoopClause::ForEquals {
                    var,
                    expr: nil_form(span),
                    then_expr: None,
                })
            }
        }
    }

    fn parse_sub_clauses(
        &self,
        span: Span,
        items: &[SurfaceForm],
        pos: &mut usize,
    ) -> Option<Vec<LoopClause>> {
        let mut clauses = Vec::new();
        while *pos < items.len() {
            let kw = items[*pos].symbol_name().unwrap_or("");
            match kw {
                "else" | "end" => break,
                "collect" => {
                    *pos += 1;
                    if *pos >= items.len() {
                        break;
                    }
                    clauses.push(LoopClause::Collect {
                        expr: items[*pos].clone(),
                        into: None,
                    });
                    *pos += 1;
                }
                "sum" => {
                    *pos += 1;
                    if *pos >= items.len() {
                        break;
                    }
                    clauses.push(LoopClause::Sum {
                        expr: items[*pos].clone(),
                        into: None,
                    });
                    *pos += 1;
                }
                "count" => {
                    *pos += 1;
                    if *pos >= items.len() {
                        break;
                    }
                    clauses.push(LoopClause::Count {
                        expr: items[*pos].clone(),
                        into: None,
                    });
                    *pos += 1;
                }
                "do" => {
                    *pos += 1;
                    let body = self.collect_until_keyword(items, pos);
                    clauses.push(LoopClause::Do { body });
                }
                "return" => {
                    *pos += 1;
                    let expr = if *pos < items.len() {
                        let e = items[*pos].clone();
                        *pos += 1;
                        e
                    } else {
                        nil_form(span)
                    };
                    clauses.push(LoopClause::Return { expr });
                }
                "append" => {
                    *pos += 1;
                    if *pos >= items.len() {
                        break;
                    }
                    clauses.push(LoopClause::Append {
                        into: None,
                        expr: items[*pos].clone(),
                    });
                    *pos += 1;
                }
                "nconc" => {
                    *pos += 1;
                    if *pos >= items.len() {
                        break;
                    }
                    clauses.push(LoopClause::Nconc {
                        into: None,
                        expr: items[*pos].clone(),
                    });
                    *pos += 1;
                }
                _ => break,
            }
        }
        Some(clauses)
    }

    fn collect_until_keyword(&self, items: &[SurfaceForm], pos: &mut usize) -> Vec<SurfaceForm> {
        let mut body = Vec::new();
        while *pos < items.len() {
            let kw = items[*pos].symbol_name().unwrap_or("");
            if is_loop_keyword(kw) {
                break;
            }
            body.push(items[*pos].clone());
            *pos += 1;
        }
        body
    }
}

// ── cl-loop data structures ────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum LoopClause {
    ForFrom {
        var: String,
        start: SurfaceForm,
        end: Option<(SurfaceForm, EndDirection)>,
        step: Option<SurfaceForm>,
    },
    ForIn {
        var: String,
        list_expr: SurfaceForm,
        step_fn: Option<SurfaceForm>,
        destructure: Option<SurfaceForm>,
    },
    ForOn {
        var: String,
        list_expr: SurfaceForm,
    },
    ForAcross {
        var: String,
        vec_expr: SurfaceForm,
    },
    ForEquals {
        var: String,
        expr: SurfaceForm,
        then_expr: Option<SurfaceForm>,
    },
    Collect {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Append {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Nconc {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Sum {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Count {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Do {
        body: Vec<SurfaceForm>,
    },
    While {
        cond: SurfaceForm,
    },
    Until {
        cond: SurfaceForm,
    },
    Return {
        expr: SurfaceForm,
    },
    If {
        cond: SurfaceForm,
        then_clauses: Vec<LoopClause>,
        else_clauses: Option<Vec<LoopClause>>,
    },
    With {
        var: String,
        expr: SurfaceForm,
    },
    Finally {
        body: Vec<SurfaceForm>,
    },
    Initially {
        body: Vec<SurfaceForm>,
    },
    Always {
        expr: SurfaceForm,
    },
    Never {
        expr: SurfaceForm,
    },
    Thereis {
        expr: SurfaceForm,
    },
    Minimize {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Maximize {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Repeat {
        count: SurfaceForm,
    },
    Named {
        name: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EndDirection {
    To,     // <= (inclusive upper bound)
    Downto, // >= (inclusive lower bound)
    Below,  // <  (exclusive upper bound)
    Above,  // >  (exclusive lower bound)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccumKind {
    Collect,
    Append,
    Nconc,
    Sum,
    Count,
    Minimize,
    Maximize,
}

fn is_loop_keyword(kw: &str) -> bool {
    matches!(
        kw,
        "for"
            | "collect"
            | "append"
            | "nconc"
            | "sum"
            | "count"
            | "do"
            | "while"
            | "until"
            | "return"
            | "if"
            | "when"
            | "with"
            | "initially"
            | "finally"
            | "else"
            | "end"
            | "minimize"
            | "maximize"
            | "always"
            | "never"
            | "thereis"
            | "repeat"
            | "into"
            | "named"
    )
}

fn clauses_contain_return(clauses: &[LoopClause]) -> bool {
    clauses.iter().any(|c| match c {
        LoopClause::Return { .. } => true,
        LoopClause::If {
            then_clauses,
            else_clauses,
            ..
        } => {
            clauses_contain_return(then_clauses)
                || else_clauses
                    .as_ref()
                    .is_some_and(|ec| clauses_contain_return(ec))
        }
        _ => false,
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct MacroParams {
    required: Vec<String>,
    optional: Vec<String>,
    rest: Option<String>,
    environment: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MacroParamSection {
    Required,
    Optional,
    Rest,
}

#[derive(Clone, Debug, PartialEq)]
struct MacroDef {
    params: MacroParams,
    body: Vec<SurfaceForm>,
    span: Span,
}

#[derive(Clone, Debug, PartialEq)]
struct IfLetBinding {
    name: String,
    value: SurfaceForm,
    span: Span,
}

fn build_if_let_form(
    bindings: Vec<IfLetBinding>,
    then_form: SurfaceForm,
    else_forms: Vec<SurfaceForm>,
    span: Span,
) -> SurfaceForm {
    if bindings.is_empty() {
        return list_form(
            vec![
                symbol_form("let*", span),
                list_form(Vec::new(), span),
                then_form,
            ],
            span,
        );
    }

    let mut previous = symbol_form("t", span);
    let mut binding_forms = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let current = symbol_form(&binding.name, binding.span);
        let value = list_form(
            vec![symbol_form("and", span), previous, binding.value],
            binding.span,
        );
        binding_forms.push(list_form(vec![current.clone(), value], binding.span));
        previous = current;
    }

    let mut if_items = vec![symbol_form("if", span), previous, then_form];
    if_items.extend(else_forms);
    list_form(
        vec![
            symbol_form("let*", span),
            list_form(binding_forms, span),
            list_form(if_items, span),
        ],
        span,
    )
}

fn generated_if_let_name(span: Span, index: usize) -> String {
    format!("\0if-let.{}.{}", span.start, index)
}

fn list_head_symbol(form: &SurfaceForm) -> Option<&str> {
    let SurfaceKind::List(items) = &form.kind else {
        return None;
    };
    items.first().and_then(SurfaceForm::symbol_name)
}

fn nil_form(span: Span) -> SurfaceForm {
    symbol_form("nil", span)
}

fn symbol_form(name: &str, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::symbol(name)), span)
}

fn quote_form(inner: SurfaceForm, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::Quote(Box::new(inner)), span)
}

fn function_quote_form(inner: SurfaceForm, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::FunctionQuote(Box::new(inner)), span)
}

fn int_form(value: i64, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(value)), span)
}

fn list_form(items: Vec<SurfaceForm>, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::List(items), span)
}

fn macro_defalias_form(name: &str, def: &MacroDef, span: Span) -> SurfaceForm {
    let body = if let Some(environment) = &def.params.environment {
        vec![list_form(
            vec![
                symbol_form("let", span),
                list_form(
                    vec![list_form(
                        vec![symbol_form(environment, span), nil_form(span)],
                        span,
                    )],
                    span,
                ),
                lower_macro_body(&def.body, span),
            ],
            span,
        )]
    } else {
        def.body.clone()
    };
    let lambda = list_form(
        std::iter::once(symbol_form("lambda", span))
            .chain(std::iter::once(macro_lambda_params_form(&def.params, span)))
            .chain(body)
            .collect(),
        span,
    );
    list_form(
        vec![
            symbol_form("defalias", span),
            quote_form(symbol_form(name, span), span),
            list_form(
                vec![
                    symbol_form("cons", span),
                    quote_form(symbol_form("macro", span), span),
                    function_quote_form(lambda, span),
                ],
                span,
            ),
        ],
        span,
    )
}

fn macro_lambda_params_form(params: &MacroParams, span: Span) -> SurfaceForm {
    let mut items = params
        .required
        .iter()
        .map(|name| symbol_form(name, span))
        .collect::<Vec<_>>();
    if !params.optional.is_empty() {
        items.push(symbol_form("&optional", span));
        items.extend(params.optional.iter().map(|name| symbol_form(name, span)));
    }
    if let Some(rest) = &params.rest {
        items.push(symbol_form("&rest", span));
        items.push(symbol_form(rest, span));
    }
    list_form(items, span)
}

fn lower_macro_body(body: &[SurfaceForm], span: Span) -> SurfaceForm {
    match body {
        [] => nil_form(span),
        [only] => only.clone(),
        _ => list_form(
            std::iter::once(symbol_form("progn", span))
                .chain(body.iter().cloned())
                .collect(),
            span,
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_source;

    #[test]
    fn expands_push_and_pop_for_simple_symbol_places() {
        let artifact = compile_source(
            "push-pop.el",
            ";;; -*- lexical-binding: t; -*-\n(let ((xs nil)) (push 1 xs) (pop xs))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"setq\""));
        assert!(rendered.contains("\"car-safe\""));
    }

    #[test]
    fn expands_simple_if_let_and_when_let_star() {
        let artifact = compile_source(
            "if-let.el",
            ";;; -*- lexical-binding: t; -*-\n(progn (if-let* ((x 1) (_ x) ((+ x 1))) x 0) (when-let* ((y 2)) y))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"let*\""));
        assert!(rendered.contains("\"and\""));
        assert!(rendered.contains("\"if\""));
        assert!(rendered.contains("\"progn\""));
    }

    #[test]
    fn expands_top_level_defmacro_with_backquote() {
        let artifact = compile_source(
            "defmacro.el",
            ";;; -*- lexical-binding: t; -*-
(defmacro inc (var)
  `(setq ,var (1+ ,var)))
(let ((x 1)) (inc x) x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"inc\""));
        assert!(rendered.contains("\"defalias\""));
        assert!(rendered.contains("\"macro\""));
        assert!(rendered.contains("\"setq\""));
        assert!(!rendered.contains("\"defmacro\""));
    }

    #[test]
    fn expands_defmacro_body_using_list_functions() {
        let artifact = compile_source(
            "defmacro-list.el",
            ";;; -*- lexical-binding: t; -*-
(defmacro inc2 (var)
  (list 'setq var (list '1+ var)))
(let ((x 1)) (inc2 x) x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"setq\""));
        assert!(rendered.contains("\"1+\""));
        assert!(!rendered.contains("\"defmacro\""));
    }

    #[test]
    fn expands_defmacro_with_rest_arguments_and_splicing() {
        let artifact = compile_source(
            "defmacro-rest.el",
            ";;; -*- lexical-binding: t; -*-
(defmacro my-progn (&rest body)
  `(progn ,@body))
(my-progn 1 2 3)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"progn\""));
        assert!(!rendered.contains("\"defmacro\""));
    }

    #[test]
    fn expands_destructuring_bind_simple() {
        let artifact = compile_source(
            "dsb.el",
            ";;; -*- lexical-binding: t; -*-\n(destructuring-bind (a b) (list 1 2) (+ a b))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"car\""));
        assert!(rendered.contains("\"cdr\""));
    }

    #[test]
    fn expands_destructuring_bind_with_rest() {
        let artifact = compile_source(
            "dsb-rest.el",
            ";;; -*- lexical-binding: t; -*-\n(destructuring-bind (a &rest bs) (list 1 2 3) (list a bs))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"car\""));
        assert!(rendered.contains("\"cdr\""));
    }

    #[test]
    fn expands_destructuring_bind_with_optional() {
        let artifact = compile_source(
            "dsb-opt.el",
            ";;; -*- lexical-binding: t; -*-\n(destructuring-bind (a &optional b) (list 1) (list a b))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"car\""));
    }

    #[test]
    fn expands_flet_to_let_with_lambda() {
        let artifact = compile_source(
            "flet.el",
            ";;; -*- lexical-binding: t; -*-\n(flet ((add1 (x) (+ x 1))) (add1 5))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"lambda\""));
        assert!(rendered.contains("\"let\""));
    }

    #[test]
    fn expands_labels_to_let_with_setq() {
        let artifact = compile_source(
            "labels.el",
            ";;; -*- lexical-binding: t; -*-\n(labels ((even? (n) (if (= n 0) t (odd? (- n 1)))) (odd? (n) (if (= n 0) nil (even? (- n 1))))) (even? 4))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"lambda\""));
        assert!(rendered.contains("\"setq\""));
    }

    #[test]
    fn expands_cl_defun_simple_params() {
        let artifact = compile_source(
            "cl-defun-simple.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-defun add (x y) (+ x y))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"defun\""));
        assert!(rendered.contains("\"add\""));
    }

    #[test]
    fn expands_cl_defun_with_optional() {
        let artifact = compile_source(
            "cl-defun-opt.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-defun foo (a &optional b) (list a b))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"defun\""));
        assert!(rendered.contains("\"--cl-rest--\""));
    }

    #[test]
    fn expands_cl_macrolet_and_uses_macro() {
        let artifact = compile_source(
            "cl-macrolet.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-macrolet ((double (x) (list '+ x x))) (double 5))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        // The macro should have been expanded: (double 5) -> (+ 5 5)
        assert!(rendered.contains("\"+\""));
        assert!(rendered.contains("5"));
    }

    // ── cl-loop tests ─────────────────────────────────────────────

    #[test]
    fn cl_loop_for_from_collect() {
        let artifact = compile_source(
            "cl-loop-1.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 to 5 collect (* x x))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
        assert!(
            !rendered.contains("\"collect\""),
            "collect should be expanded away"
        );
    }

    #[test]
    fn cl_loop_for_in_collect() {
        let artifact = compile_source(
            "cl-loop-2.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3) collect (* x 2))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
        assert!(rendered.contains("\"car\""));
        assert!(rendered.contains("\"cdr\""));
    }

    #[test]
    fn cl_loop_sum() {
        let artifact = compile_source(
            "cl-loop-3.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 to 10 sum x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"+\""));
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_count() {
        let artifact = compile_source(
            "cl-loop-4.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3 4 5) count (> x 3))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"if\""));
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_do_body() {
        let artifact = compile_source(
            "cl-loop-5.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 to 3 do (foo x))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"foo\""));
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_with_binding() {
        let artifact = compile_source(
            "cl-loop-6.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop with y = 10 for x from 1 to 3 collect (+ x y))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_while_termination() {
        let artifact = compile_source(
            "cl-loop-7.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3 4 5) while (< x 4) collect x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
        assert!(rendered.contains("\"and\""));
    }

    #[test]
    fn cl_loop_return() {
        let artifact = compile_source(
            "cl-loop-8.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 to 100 if (> x 5) return x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"catch\""));
        assert!(rendered.contains("\"throw\""));
    }

    #[test]
    fn cl_loop_by_step() {
        let artifact = compile_source(
            "cl-loop-9.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 0 to 10 by 2 collect x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_initially_finally() {
        let artifact = compile_source(
            "cl-loop-10.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop initially (bar) for x from 1 to 3 collect x finally (baz))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"bar\""));
        assert!(rendered.contains("\"baz\""));
        assert!(rendered.contains("\"nreverse\""));
    }

    #[test]
    fn cl_loop_append_accumulation() {
        let artifact = compile_source(
            "cl-loop-11.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list (list 1 2) (list 3 4)) append x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"append\""));
    }

    #[test]
    fn cl_loop_for_on() {
        let artifact = compile_source(
            "cl-loop-12.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x on (list 1 2 3) collect (car x))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"car\""));
        assert!(rendered.contains("\"cdr\""));
    }

    #[test]
    fn cl_loop_empty() {
        let artifact = compile_source(
            "cl-loop-empty.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
    }

    #[test]
    fn cl_loop_always_short_circuit() {
        let artifact = compile_source(
            "cl-loop-always.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3) always (> x 0))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(
            rendered.contains("\"--cl-always--\""),
            "should have --cl-always-- flag for always clause"
        );
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_never_short_circuit() {
        let artifact = compile_source(
            "cl-loop-never.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3) never (< x 0))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(
            rendered.contains("\"--cl-always--\""),
            "never clause should use --cl-always-- flag"
        );
    }

    #[test]
    fn cl_loop_sum_accumulation() {
        let artifact = compile_source(
            "cl-loop-sum.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3) sum x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"+\""), "sum should use + operator");
        assert!(rendered.contains("\"--cl-acc-"));
    }

    #[test]
    fn cl_loop_with_and_finally() {
        let artifact = compile_source(
            "cl-loop-with2.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop with total = 0 for x in (list 1 2 3) do (setq total (+ total x)) finally return total)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"total\""));
    }

    #[test]
    fn cl_loop_do_and_message() {
        let artifact = compile_source(
            "cl-loop-do2.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 to 3 do (message \"%d\" x))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"message\""));
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_for_from_no_end() {
        // for x from 1 (no end) — should create infinite loop with no while test
        let artifact = compile_source(
            "cl-loop-noend.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 while (< x 5) collect x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
        assert!(rendered.contains("\"<\""));
    }

    #[test]
    fn cl_loop_named_creates_catch_wrapper() {
        let artifact = compile_source(
            "cl-loop-named.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop named my-block for i from 1 to 5 collect i)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"catch\""));
        assert!(rendered.contains("\"my-block\""));
    }

    #[test]
    fn pcase_let_star_destructure_backquote() {
        let artifact = compile_source(
            "pcase-let-dest.el",
            ";;; -*- lexical-binding: t; -*-\n\
             (pcase-let* ((`(,x ,y) '(1 2))) (+ x y))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"let*\""));
        assert!(rendered.contains("\"car\""));
    }
}
