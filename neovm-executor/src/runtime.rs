use std::fmt;

use crate::value::LispValue;

#[derive(Default)]
pub struct Runtime {
    cons_cells: Vec<Box<Cons>>,
    pending_error: Option<RuntimeError>,
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cons(&mut self, car: LispValue, cdr: LispValue) -> LispValue {
        let mut cell = Box::new(Cons {
            header: HeapHeader {
                kind: HeapKind::Cons,
            },
            car,
            cdr,
        });
        let addr = (&mut *cell as *mut Cons) as usize;
        self.cons_cells.push(cell);
        LispValue::from_heap_addr(addr)
    }

    pub fn cons_abi(&mut self, car: i64, cdr: i64) -> i64 {
        self.cons(LispValue::from_abi_i64(car), LispValue::from_abi_i64(cdr))
            .to_abi_i64()
    }

    pub fn car(&self, pair: LispValue) -> Result<LispValue, RuntimeError> {
        if pair.is_nil() {
            return Ok(LispValue::NIL);
        }
        Ok(self.expect_cons(pair)?.car)
    }

    pub fn car_abi(&mut self, pair: i64) -> i64 {
        self.return_or_record_error(self.car(LispValue::from_abi_i64(pair)))
    }

    pub fn cdr(&self, pair: LispValue) -> Result<LispValue, RuntimeError> {
        if pair.is_nil() {
            return Ok(LispValue::NIL);
        }
        Ok(self.expect_cons(pair)?.cdr)
    }

    pub fn set_car(&mut self, pair: LispValue, car: LispValue) -> Result<LispValue, RuntimeError> {
        self.expect_cons_mut(pair)?.car = car;
        Ok(car)
    }

    pub fn set_cdr(&mut self, pair: LispValue, cdr: LispValue) -> Result<LispValue, RuntimeError> {
        self.expect_cons_mut(pair)?.cdr = cdr;
        Ok(cdr)
    }

    pub fn cdr_abi(&mut self, pair: i64) -> i64 {
        self.return_or_record_error(self.cdr(LispValue::from_abi_i64(pair)))
    }

    pub fn is_cons(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.cons_by_addr(addr).is_some())
    }

    pub fn cons_cell_count(&self) -> usize {
        self.cons_cells.len()
    }

    pub fn pending_error(&self) -> Option<&RuntimeError> {
        self.pending_error.as_ref()
    }

    pub fn take_pending_error(&mut self) -> Option<RuntimeError> {
        self.pending_error.take()
    }

    fn return_or_record_error(&mut self, result: Result<LispValue, RuntimeError>) -> i64 {
        match result {
            Ok(value) => value.to_abi_i64(),
            Err(error) => {
                self.pending_error = Some(error);
                LispValue::NIL.to_abi_i64()
            }
        }
    }

    fn expect_cons(&self, value: LispValue) -> Result<&Cons, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "consp",
                value,
            });
        };
        self.cons_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "consp",
                value,
            })
    }

    fn expect_cons_mut(&mut self, value: LispValue) -> Result<&mut Cons, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "consp",
                value,
            });
        };
        self.cons_by_addr_mut(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "consp",
                value,
            })
    }

    fn cons_by_addr(&self, addr: usize) -> Option<&Cons> {
        for cell in &self.cons_cells {
            let cell_addr = (&**cell as *const Cons) as usize;
            if cell_addr == addr && cell.header.kind == HeapKind::Cons {
                return Some(cell);
            }
        }
        None
    }

    fn cons_by_addr_mut(&mut self, addr: usize) -> Option<&mut Cons> {
        for cell in &mut self.cons_cells {
            let cell_addr = (&**cell as *const Cons) as usize;
            if cell_addr == addr && cell.header.kind == HeapKind::Cons {
                return Some(cell);
            }
        }
        None
    }

    pub fn equal(&self, left: LispValue, right: LispValue) -> bool {
        self.equal_with_depth(left, right, 256)
    }

    fn equal_with_depth(&self, left: LispValue, right: LispValue, depth: usize) -> bool {
        if left == right {
            return true;
        }
        if depth == 0 {
            return false;
        }
        let (Some(left_addr), Some(right_addr)) = (left.heap_addr(), right.heap_addr()) else {
            return false;
        };
        let (Some(left), Some(right)) =
            (self.cons_by_addr(left_addr), self.cons_by_addr(right_addr))
        else {
            return false;
        };
        self.equal_with_depth(left.car, right.car, depth - 1)
            && self.equal_with_depth(left.cdr, right.cdr, depth - 1)
    }

    pub fn format_value(&self, value: LispValue) -> String {
        self.format_value_with_depth(value, 64)
    }

    fn format_value_with_depth(&self, value: LispValue, depth: usize) -> String {
        if depth == 0 {
            return "#<max-depth>".to_string();
        }
        if !self.is_cons(value) {
            return format!("{value:?}");
        }
        let mut parts = Vec::new();
        let mut current = value;
        loop {
            let Some(addr) = current.heap_addr() else {
                parts.push(".".to_string());
                parts.push(format!("{current:?}"));
                break;
            };
            let Some(cell) = self.cons_by_addr(addr) else {
                parts.push(".".to_string());
                parts.push(format!("{current:?}"));
                break;
            };
            parts.push(self.format_value_with_depth(cell.car, depth - 1));
            current = cell.cdr;
            if current.is_nil() {
                break;
            }
            if !self.is_cons(current) {
                parts.push(".".to_string());
                parts.push(self.format_value_with_depth(current, depth - 1));
                break;
            }
        }
        format!("({})", parts.join(" "))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    WrongTypeArgument {
        expected: &'static str,
        value: LispValue,
    },
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongTypeArgument { expected, value } => {
                write!(f, "wrong type argument: expected {expected}, got {value:?}")
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeapKind {
    Cons = 1,
}

#[repr(C)]
struct HeapHeader {
    kind: HeapKind,
}

#[repr(C, align(8))]
struct Cons {
    header: HeapHeader,
    car: LispValue,
    cdr: LispValue,
}

#[cfg(test)]
mod tests {
    use super::{Runtime, RuntimeError};
    use crate::LispValue;

    #[test]
    fn cons_allocates_tagged_heap_value() {
        let mut runtime = Runtime::new();
        let car = LispValue::expect_fixnum(1);
        let cdr = LispValue::expect_fixnum(2);

        let pair = runtime.cons(car, cdr);

        assert!(pair.is_heap());
        assert_eq!(runtime.cons_cell_count(), 1);
        assert_eq!(runtime.car(pair), Ok(car));
        assert_eq!(runtime.cdr(pair), Ok(cdr));
    }

    #[test]
    fn pair_abi_methods_use_lisp_value_bits() {
        let mut runtime = Runtime::new();
        let car = LispValue::expect_fixnum(11);
        let cdr = LispValue::expect_fixnum(12);

        let pair = LispValue::from_abi_i64(runtime.cons_abi(car.to_abi_i64(), cdr.to_abi_i64()));

        assert!(pair.is_heap());
        assert_eq!(runtime.car_abi(pair.to_abi_i64()), car.to_abi_i64());
        assert_eq!(runtime.cdr_abi(pair.to_abi_i64()), cdr.to_abi_i64());
        assert_eq!(runtime.pending_error(), None);
    }

    #[test]
    fn car_and_cdr_of_nil_are_nil() {
        let runtime = Runtime::new();

        assert_eq!(runtime.car(LispValue::NIL), Ok(LispValue::NIL));
        assert_eq!(runtime.cdr(LispValue::NIL), Ok(LispValue::NIL));
    }

    #[test]
    fn car_and_cdr_reject_non_cons_values() {
        let runtime = Runtime::new();
        let value = LispValue::expect_fixnum(7);

        assert_eq!(
            runtime.car(value),
            Err(RuntimeError::WrongTypeArgument {
                expected: "consp",
                value
            })
        );
        assert_eq!(
            runtime.cdr(value),
            Err(RuntimeError::WrongTypeArgument {
                expected: "consp",
                value
            })
        );
    }

    #[test]
    fn set_car_and_set_cdr_mutate_pairs() {
        let mut runtime = Runtime::new();
        let pair = runtime.cons(LispValue::expect_fixnum(1), LispValue::expect_fixnum(2));

        assert_eq!(
            runtime.set_car(pair, LispValue::expect_fixnum(3)),
            Ok(LispValue::expect_fixnum(3))
        );
        assert_eq!(
            runtime.set_cdr(pair, LispValue::expect_fixnum(4)),
            Ok(LispValue::expect_fixnum(4))
        );
        assert_eq!(runtime.car(pair), Ok(LispValue::expect_fixnum(3)));
        assert_eq!(runtime.cdr(pair), Ok(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn equal_compares_cons_structure() {
        let mut runtime = Runtime::new();
        let left_tail = runtime.cons(LispValue::expect_fixnum(2), LispValue::NIL);
        let left = runtime.cons(LispValue::expect_fixnum(1), left_tail);
        let right_tail = runtime.cons(LispValue::expect_fixnum(2), LispValue::NIL);
        let right = runtime.cons(LispValue::expect_fixnum(1), right_tail);

        assert!(runtime.equal(left, right));
        assert!(!runtime.equal(left, LispValue::expect_fixnum(1)));
    }

    #[test]
    fn abi_methods_record_pending_errors() {
        let mut runtime = Runtime::new();
        let value = LispValue::expect_fixnum(7);

        assert_eq!(
            runtime.car_abi(value.to_abi_i64()),
            LispValue::NIL.to_abi_i64()
        );
        assert_eq!(
            runtime.take_pending_error(),
            Some(RuntimeError::WrongTypeArgument {
                expected: "consp",
                value
            })
        );
        assert_eq!(runtime.pending_error(), None);
    }
}
