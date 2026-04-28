use std::rc::Rc;

use crate::source::Span;
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

/// Lisp value for macro-time evaluation.
/// Separates computed values from parsed syntax (SurfaceForm).
#[derive(Clone, Debug, PartialEq)]
pub enum MacroValue {
    Nil,
    Int(i64),
    Symbol(String),
    String(String),
    Cons(Rc<MacroCons>),
    Vector(Vec<MacroValue>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MacroCons {
    pub car: MacroValue,
    pub cdr: MacroValue,
}

impl MacroValue {
    pub fn is_nil(&self) -> bool {
        matches!(self, MacroValue::Nil)
    }

    pub fn is_truthy(&self) -> bool {
        !self.is_nil()
    }

    pub fn is_cons(&self) -> bool {
        matches!(self, MacroValue::Cons(_))
    }

    pub fn is_list(&self) -> bool {
        self.is_nil() || self.is_cons()
    }

    pub fn is_symbol(&self) -> bool {
        matches!(self, MacroValue::Symbol(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, MacroValue::String(_))
    }

    pub fn is_int(&self) -> bool {
        matches!(self, MacroValue::Int(_))
    }

    pub fn cons(car: MacroValue, cdr: MacroValue) -> MacroValue {
        MacroValue::Cons(Rc::new(MacroCons { car, cdr }))
    }

    pub fn list(items: Vec<MacroValue>) -> MacroValue {
        let mut tail = MacroValue::Nil;
        for item in items.into_iter().rev() {
            tail = MacroValue::cons(item, tail);
        }
        tail
    }

    pub fn car(&self) -> MacroValue {
        match self {
            MacroValue::Cons(pair) => pair.car.clone(),
            _ => MacroValue::Nil,
        }
    }

    pub fn cdr(&self) -> MacroValue {
        match self {
            MacroValue::Cons(pair) => pair.cdr.clone(),
            _ => MacroValue::Nil,
        }
    }

    /// Collect proper list into Vec. Returns None if not a proper list.
    pub fn to_vec(&self) -> Option<Vec<MacroValue>> {
        let mut items = Vec::new();
        let mut current = self;
        loop {
            match current {
                MacroValue::Nil => return Some(items),
                MacroValue::Cons(pair) => {
                    items.push(pair.car.clone());
                    current = &pair.cdr;
                }
                _ => return None,
            }
        }
    }

    pub fn as_symbol_name(&self) -> Option<&str> {
        match self {
            MacroValue::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            MacroValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            MacroValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn from_bool(b: bool) -> MacroValue {
        if b {
            MacroValue::Symbol("t".into())
        } else {
            MacroValue::Nil
        }
    }

    /// (assq key alist) — find first pair whose car is eq to key
    /// alist is a list of cons cells: ((key1 . val1) (key2 . val2) ...)
    pub fn assq(&self, key: &MacroValue) -> MacroValue {
        let mut current = self.clone();
        while let MacroValue::Cons(cell) = &current {
            let car_val = cell.car.clone();
            if let MacroValue::Cons(pair) = &car_val {
                // Element is a cons cell — check if its car matches key
                if pair.car.eq_value(key) {
                    return car_val;
                }
            } else if car_val.eq_value(key) {
                // Element is an atom that matches key
                return car_val;
            }
            current = cell.cdr.clone();
        }
        MacroValue::Nil
    }

    /// (memq element list) — find tail starting with element
    pub fn memq(&self, el: &MacroValue) -> MacroValue {
        let mut current = self.clone();
        while let MacroValue::Cons(cell) = &current {
            if cell.car.eq_value(el) {
                return current.clone();
            }
            current = cell.cdr.clone();
        }
        MacroValue::Nil
    }

    /// (butlast list n) — return list without last n elements
    pub fn butlast(&self, n: usize) -> MacroValue {
        let Some(vec) = self.to_vec() else {
            return MacroValue::Nil;
        };
        if n >= vec.len() {
            return MacroValue::Nil;
        }
        MacroValue::list(vec[..vec.len() - n].to_vec())
    }

    /// (delq element list) — remove elements eq to element
    pub fn delq(&self, el: &MacroValue) -> MacroValue {
        let Some(vec) = self.to_vec() else {
            return self.clone();
        };
        let filtered: Vec<_> = vec.into_iter().filter(|v| !v.eq_value(el)).collect();
        MacroValue::list(filtered)
    }

    fn eq_value(&self, other: &MacroValue) -> bool {
        match (self, other) {
            (MacroValue::Nil, MacroValue::Nil) => true,
            (MacroValue::Int(a), MacroValue::Int(b)) => a == b,
            (MacroValue::Symbol(a), MacroValue::Symbol(b)) => a == b,
            (MacroValue::String(a), MacroValue::String(b)) => a == b,
            _ => false,
        }
    }

    /// (plist-get plist prop) — get value from property list
    /// plist is (prop1 val1 prop2 val2 ...)
    pub fn plist_get(&self, prop: &MacroValue) -> MacroValue {
        let Some(vec) = self.to_vec() else {
            return MacroValue::Nil;
        };
        let mut i = 0;
        while i + 1 < vec.len() {
            if vec[i].eq_value(prop) {
                return vec[i + 1].clone();
            }
            i += 2;
        }
        MacroValue::Nil
    }

    /// (last list n) — return last n cons cells
    pub fn last(&self, n: usize) -> MacroValue {
        let Some(vec) = self.to_vec() else {
            return MacroValue::Nil;
        };
        if vec.is_empty() {
            return MacroValue::Nil;
        }
        if n == 0 {
            return self.clone();
        }
        let start = vec.len().saturating_sub(n);
        // For n=1, return the last element (not a list)
        if n == 1 {
            return vec.last().cloned().unwrap_or(MacroValue::Nil);
        }
        MacroValue::list(vec[start..].to_vec())
    }

    /// (remove element list) — remove by equal
    pub fn remove(&self, el: &MacroValue) -> MacroValue {
        let Some(vec) = self.to_vec() else {
            return self.clone();
        };
        let filtered: Vec<_> = vec.into_iter().filter(|v| v != el).collect();
        MacroValue::list(filtered)
    }

    /// (plist-put plist prop val) — set property in plist, return new plist
    pub fn plist_put(&self, prop: &MacroValue, val: MacroValue) -> MacroValue {
        let Some(mut vec) = self.to_vec() else {
            return MacroValue::list(vec![prop.clone(), val]);
        };
        let mut i = 0;
        while i + 1 < vec.len() {
            if vec[i].eq_value(prop) {
                vec[i + 1] = val;
                return MacroValue::list(vec);
            }
            i += 2;
        }
        // Not found — append
        vec.push(prop.clone());
        vec.push(val);
        MacroValue::list(vec)
    }

    /// (reverse list) — reverse a list
    pub fn reverse(&self) -> MacroValue {
        let Some(mut vec) = self.to_vec() else {
            return MacroValue::Nil;
        };
        vec.reverse();
        MacroValue::list(vec)
    }
}

/// Convert unevaluated SurfaceForm syntax to a MacroValue.
/// No evaluation — purely a representation change.
pub fn surface_to_value(form: &SurfaceForm) -> MacroValue {
    match &form.kind {
        SurfaceKind::Atom(atom) => atom_to_value(atom),
        SurfaceKind::List(items) => {
            MacroValue::list(items.iter().map(surface_to_value).collect())
        }
        SurfaceKind::DottedList(items, tail) => {
            let mut result = surface_to_value(tail);
            for item in items.iter().rev() {
                result = MacroValue::cons(surface_to_value(item), result);
            }
            result
        }
        SurfaceKind::Vector(items) => {
            MacroValue::Vector(items.iter().map(surface_to_value).collect())
        }
        SurfaceKind::Quote(inner) => {
            MacroValue::list(vec![
                MacroValue::Symbol("quote".into()),
                surface_to_value(inner),
            ])
        }
        SurfaceKind::FunctionQuote(inner) => {
            MacroValue::list(vec![
                MacroValue::Symbol("function".into()),
                surface_to_value(inner),
            ])
        }
        SurfaceKind::Backquote(inner) => {
            MacroValue::list(vec![
                MacroValue::Symbol("backquote".into()),
                surface_to_value(inner),
            ])
        }
        SurfaceKind::Comma(inner) => {
            MacroValue::list(vec![
                MacroValue::Symbol("unquote".into()),
                surface_to_value(inner),
            ])
        }
        SurfaceKind::CommaAt(inner) => {
            MacroValue::list(vec![
                MacroValue::Symbol("splice-unquote".into()),
                surface_to_value(inner),
            ])
        }
    }
}

fn atom_to_value(atom: &SurfaceAtom) -> MacroValue {
    match atom {
        SurfaceAtom::Nil => MacroValue::Nil,
        SurfaceAtom::True => MacroValue::Symbol("t".into()),
        SurfaceAtom::Int(n) => MacroValue::Int(*n),
        SurfaceAtom::Float(_) => MacroValue::Nil,
        SurfaceAtom::Symbol(s) => MacroValue::Symbol(s.clone()),
        SurfaceAtom::String(s) => MacroValue::String(s.clone()),
        SurfaceAtom::Char(c) => MacroValue::Int(*c),
    }
}

/// Convert a MacroValue back to SurfaceForm syntax for the compiler pipeline.
pub fn value_to_surface(value: &MacroValue, span: Span) -> SurfaceForm {
    match value {
        MacroValue::Nil => SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Nil), span),
        MacroValue::Int(n) => SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(*n)), span),
        MacroValue::Symbol(s) => {
            let atom = SurfaceAtom::symbol(s);
            SurfaceForm::new(SurfaceKind::Atom(atom), span)
        }
        MacroValue::String(s) => {
            SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::String(s.clone())), span)
        }
        MacroValue::Cons(pair) => {
            let mut items = Vec::new();
            let mut current = pair.as_ref();
            loop {
                items.push(value_to_surface(&current.car, span));
                match &current.cdr {
                    MacroValue::Nil => {
                        return SurfaceForm::new(SurfaceKind::List(items), span);
                    }
                    MacroValue::Cons(next) => {
                        current = next;
                    }
                    other => {
                        let tail = value_to_surface(other, span);
                        return SurfaceForm::new(
                            SurfaceKind::DottedList(items, Box::new(tail)),
                            span,
                        );
                    }
                }
            }
        }
        MacroValue::Vector(items) => {
            let forms: Vec<SurfaceForm> =
                items.iter().map(|v| value_to_surface(v, span)).collect();
            SurfaceForm::new(SurfaceKind::Vector(forms), span)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceId;

    fn test_span() -> Span {
        Span::new(SourceId::new(0), 0, 1)
    }

    fn sym(name: &str) -> SurfaceForm {
        SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::symbol(name)), test_span())
    }

    fn int_form(n: i64) -> SurfaceForm {
        SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(n)), test_span())
    }

    fn str_form(s: &str) -> SurfaceForm {
        SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::String(s.into())), test_span())
    }

    fn nil_form() -> SurfaceForm {
        SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Nil), test_span())
    }

    fn list_form(items: Vec<SurfaceForm>) -> SurfaceForm {
        SurfaceForm::new(SurfaceKind::List(items), test_span())
    }

    #[test]
    fn atom_round_trips() {
        let span = test_span();
        let cases: Vec<SurfaceForm> = vec![
            nil_form(),
            int_form(42),
            sym("foo"),
            str_form("hello"),
        ];
        for form in &cases {
            let value = surface_to_value(form);
            let back = value_to_surface(&value, span);
            assert_eq!(&back, form, "round-trip failed for {:?}", form.kind);
        }
    }

    #[test]
    fn proper_list_round_trips() {
        let span = test_span();
        let form = list_form(vec![sym("a"), int_form(1), sym("b")]);
        let value = surface_to_value(&form);
        let back = value_to_surface(&value, span);
        assert_eq!(back, form);
    }

    #[test]
    fn dotted_list_preserves_tail() {
        let span = test_span();
        let form = SurfaceForm::new(
            SurfaceKind::DottedList(vec![sym("a"), sym("b")], Box::new(sym("c"))),
            span,
        );
        let value = surface_to_value(&form);
        assert!(value.is_cons());
        let back = value_to_surface(&value, span);
        assert_eq!(back, form);
    }

    #[test]
    fn nested_list_round_trips() {
        let span = test_span();
        let inner = list_form(vec![int_form(1), int_form(2)]);
        let outer = list_form(vec![sym("foo"), inner]);
        let value = surface_to_value(&outer);
        let back = value_to_surface(&value, span);
        assert_eq!(back, outer);
    }

    #[test]
    fn vector_round_trips() {
        let span = test_span();
        let form = SurfaceForm::new(
            SurfaceKind::Vector(vec![int_form(1), sym("x")]),
            span,
        );
        let value = surface_to_value(&form);
        let back = value_to_surface(&value, span);
        assert_eq!(back, form);
    }

    #[test]
    fn nil_is_falsy() {
        assert!(!MacroValue::Nil.is_truthy());
        assert!(MacroValue::Nil.is_nil());
        assert!(MacroValue::Nil.is_list());
    }

    #[test]
    fn symbol_t_is_truthy() {
        let t = MacroValue::Symbol("t".into());
        assert!(t.is_truthy());
        assert!(t.is_symbol());
        assert!(!t.is_nil());
    }

    #[test]
    fn list_constructor() {
        let list = MacroValue::list(vec![
            MacroValue::Int(1),
            MacroValue::Int(2),
            MacroValue::Int(3),
        ]);
        assert!(list.is_cons());
        let vec = list.to_vec().unwrap();
        assert_eq!(vec.len(), 3);
    }

    #[test]
    fn cons_car_cdr() {
        let pair = MacroValue::cons(MacroValue::Int(1), MacroValue::Int(2));
        assert_eq!(pair.car(), MacroValue::Int(1));
        assert_eq!(pair.cdr(), MacroValue::Int(2));
    }
}
