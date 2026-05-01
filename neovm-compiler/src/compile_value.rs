use crate::expand_value::MacroValue;

/// Compile-time Lisp value for the compiler pipeline.
/// Carries forward through SSA → RegIR → JIT as a richer alternative to bare i64.
#[derive(Clone, Debug, PartialEq)]
pub enum CompileValue {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Char(i64),
    Symbol(String),
    String(String),
    Cons {
        car: Box<CompileValue>,
        cdr: Box<CompileValue>,
    },
    Vector(Vec<CompileValue>),
}

impl CompileValue {
    pub fn is_nil(&self) -> bool {
        matches!(self, CompileValue::Nil)
    }

    pub fn is_truthy(&self) -> bool {
        !self.is_nil()
    }

    pub fn is_cons(&self) -> bool {
        matches!(self, CompileValue::Cons { .. })
    }

    pub fn is_symbol(&self) -> bool {
        matches!(self, CompileValue::Symbol(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, CompileValue::String(_))
    }

    pub fn is_int(&self) -> bool {
        matches!(self, CompileValue::Int(_))
    }

    pub fn cons(car: CompileValue, cdr: CompileValue) -> CompileValue {
        CompileValue::Cons {
            car: Box::new(car),
            cdr: Box::new(cdr),
        }
    }

    pub fn list(items: Vec<CompileValue>) -> CompileValue {
        let mut tail = CompileValue::Nil;
        for item in items.into_iter().rev() {
            tail = CompileValue::cons(item, tail);
        }
        tail
    }

    pub fn car(&self) -> CompileValue {
        match self {
            CompileValue::Cons { car, .. } => (**car).clone(),
            _ => CompileValue::Nil,
        }
    }

    pub fn cdr(&self) -> CompileValue {
        match self {
            CompileValue::Cons { cdr, .. } => (**cdr).clone(),
            _ => CompileValue::Nil,
        }
    }

    pub fn to_vec(&self) -> Option<Vec<CompileValue>> {
        let mut items = Vec::new();
        let mut current = self;
        loop {
            match current {
                CompileValue::Nil => return Some(items),
                CompileValue::Cons { car, cdr } => {
                    items.push((**car).clone());
                    current = cdr;
                }
                _ => return None,
            }
        }
    }

    pub fn as_symbol_name(&self) -> Option<&str> {
        match self {
            CompileValue::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            CompileValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn from_bool(b: bool) -> CompileValue {
        if b {
            CompileValue::Symbol("t".into())
        } else {
            CompileValue::Nil
        }
    }

    pub fn from_macro_value(v: &MacroValue) -> Self {
        match v {
            MacroValue::Nil => CompileValue::Nil,
            MacroValue::Int(n) => CompileValue::Int(*n),
            MacroValue::Symbol(s) => CompileValue::Symbol(s.clone()),
            MacroValue::String(s) => CompileValue::String(s.clone()),
            MacroValue::Cons(pair) => CompileValue::Cons {
                car: Box::new(CompileValue::from_macro_value(&pair.car)),
                cdr: Box::new(CompileValue::from_macro_value(&pair.cdr)),
            },
            MacroValue::Vector(items) => CompileValue::Vector(
                items.iter().map(CompileValue::from_macro_value).collect(),
            ),
        }
    }

    pub fn to_macro_value(&self) -> MacroValue {
        match self {
            CompileValue::Nil => MacroValue::Nil,
            CompileValue::Bool(true) => MacroValue::Symbol("t".into()),
            CompileValue::Bool(false) => MacroValue::Nil,
            CompileValue::Int(n) => MacroValue::Int(*n),
            CompileValue::Float(f) => MacroValue::Int(*f as i64),
            CompileValue::Char(c) => MacroValue::Int(*c),
            CompileValue::Symbol(s) => MacroValue::Symbol(s.clone()),
            CompileValue::String(s) => MacroValue::String(s.clone()),
            CompileValue::Cons { car, cdr } => {
                MacroValue::cons(car.to_macro_value(), cdr.to_macro_value())
            }
            CompileValue::Vector(v) => {
                MacroValue::Vector(v.iter().map(|cv| cv.to_macro_value()).collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nil_is_falsy() {
        assert!(!CompileValue::Nil.is_truthy());
        assert!(CompileValue::Nil.is_nil());
    }

    #[test]
    fn symbol_t_is_truthy() {
        let t = CompileValue::Symbol("t".into());
        assert!(t.is_truthy());
        assert!(t.is_symbol());
    }

    #[test]
    fn int_is_truthy() {
        assert!(CompileValue::Int(42).is_truthy());
    }

    #[test]
    fn cons_car_cdr() {
        let pair = CompileValue::cons(CompileValue::Int(1), CompileValue::Int(2));
        assert_eq!(pair.car(), CompileValue::Int(1));
        assert_eq!(pair.cdr(), CompileValue::Int(2));
    }

    #[test]
    fn list_constructor() {
        let list = CompileValue::list(vec![
            CompileValue::Int(1),
            CompileValue::Int(2),
            CompileValue::Int(3),
        ]);
        let vec = list.to_vec().unwrap();
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], CompileValue::Int(1));
        assert_eq!(vec[2], CompileValue::Int(3));
    }

    #[test]
    fn from_macro_value_round_trip() {
        let mv = MacroValue::cons(
            MacroValue::Symbol("foo".into()),
            MacroValue::list(vec![MacroValue::Int(1), MacroValue::Int(2)]),
        );
        let cv = CompileValue::from_macro_value(&mv);
        assert!(cv.is_cons());
        assert_eq!(cv.car().as_symbol_name(), Some("foo"));
        let cdr_vec = cv.cdr().to_vec().unwrap();
        assert_eq!(cdr_vec.len(), 2);
    }
}
