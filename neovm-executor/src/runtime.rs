use std::{collections::HashMap, fmt};

use neovm_compiler::ssa::SsaLambdaTemplate;

use crate::value::LispValue;

#[derive(Default)]
pub struct Runtime {
    cons_cells: Vec<Box<Cons>>,
    symbols: Vec<Box<Symbol>>,
    strings: Vec<Box<LispString>>,
    functions: Vec<Box<FunctionObject>>,
    lexical_cells: Vec<Box<LexicalCell>>,
    interned_symbols: HashMap<String, LispValue>,
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

    pub fn string(&mut self, value: impl AsRef<str>) -> LispValue {
        self.string_from_lisp_data(LispStringData::make_string(value.as_ref()))
    }

    pub fn string_from_bytes(
        &mut self,
        bytes: Vec<u8>,
        chars: usize,
        multibyte: bool,
    ) -> LispValue {
        let data = if multibyte {
            LispStringData::make_multibyte(bytes, chars)
        } else {
            LispStringData::make_unibyte(bytes)
        };
        self.string_from_lisp_data(data)
    }

    fn string_from_lisp_data(&mut self, data: LispStringData) -> LispValue {
        let mut string = Box::new(LispString {
            header: HeapHeader {
                kind: HeapKind::String,
            },
            data,
        });
        let addr = (&mut *string as *mut LispString) as usize;
        self.strings.push(string);
        LispValue::from_heap_addr(addr)
    }

    pub fn function(&mut self, template: SsaLambdaTemplate, captures: Vec<LispValue>) -> LispValue {
        let mut function = Box::new(FunctionObject {
            header: HeapHeader {
                kind: HeapKind::Function,
            },
            template,
            captures,
        });
        let addr = (&mut *function as *mut FunctionObject) as usize;
        self.functions.push(function);
        LispValue::from_heap_addr(addr)
    }

    pub fn lexical_cell(&mut self, value: LispValue) -> LispValue {
        let mut cell = Box::new(LexicalCell {
            header: HeapHeader {
                kind: HeapKind::LexicalCell,
            },
            value,
        });
        let addr = (&mut *cell as *mut LexicalCell) as usize;
        self.lexical_cells.push(cell);
        LispValue::from_heap_addr(addr)
    }

    pub fn intern(&mut self, name: &str) -> LispValue {
        match name {
            "nil" => return LispValue::NIL,
            "t" => return LispValue::TRUE,
            _ => {}
        }
        if let Some(symbol) = self.interned_symbols.get(name).copied() {
            return symbol;
        }
        let name = name.to_string();
        let mut symbol = Box::new(Symbol {
            header: HeapHeader {
                kind: HeapKind::Symbol,
            },
            name: name.clone(),
            value: None,
            function: None,
        });
        let addr = (&mut *symbol as *mut Symbol) as usize;
        let value = LispValue::from_heap_addr(addr);
        self.symbols.push(symbol);
        self.interned_symbols.insert(name, value);
        value
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

    pub fn is_symbol(&self, value: LispValue) -> bool {
        value.is_nil()
            || value.is_true()
            || value
                .heap_addr()
                .is_some_and(|addr| self.symbol_by_addr(addr).is_some())
    }

    pub fn is_string(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.string_by_addr(addr).is_some())
    }

    pub fn is_function(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.function_by_addr(addr).is_some())
    }

    pub fn cons_cell_count(&self) -> usize {
        self.cons_cells.len()
    }

    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    pub fn string_count(&self) -> usize {
        self.strings.len()
    }

    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    pub fn lexical_cell_count(&self) -> usize {
        self.lexical_cells.len()
    }

    pub fn symbol_name(&self, symbol: LispValue) -> Result<String, RuntimeError> {
        if symbol.is_nil() {
            return Ok("nil".to_string());
        }
        if symbol.is_true() {
            return Ok("t".to_string());
        }
        Ok(self.expect_symbol(symbol)?.name.clone())
    }

    pub fn symbol_name_value(&mut self, symbol: LispValue) -> Result<LispValue, RuntimeError> {
        let name = self.symbol_name(symbol)?;
        Ok(self.string(name))
    }

    pub fn symbol_value(&self, symbol: LispValue) -> Result<LispValue, RuntimeError> {
        if symbol.is_nil() {
            return Ok(LispValue::NIL);
        }
        if symbol.is_true() {
            return Ok(LispValue::TRUE);
        }
        let symbol = self.expect_symbol(symbol)?;
        symbol.value.ok_or_else(|| RuntimeError::VoidVariable {
            name: symbol.name.clone(),
        })
    }

    pub fn symbol_value_by_name(&self, name: &str) -> Result<LispValue, RuntimeError> {
        match name {
            "nil" => return Ok(LispValue::NIL),
            "t" => return Ok(LispValue::TRUE),
            _ => {}
        }
        let Some(symbol) = self.interned_symbols.get(name).copied() else {
            return Err(RuntimeError::VoidVariable {
                name: name.to_string(),
            });
        };
        self.symbol_value(symbol)
    }

    pub fn set_symbol_value(
        &mut self,
        symbol: LispValue,
        value: LispValue,
    ) -> Result<LispValue, RuntimeError> {
        if symbol.is_nil() {
            return Err(RuntimeError::ConstantSymbol {
                name: "nil".to_string(),
            });
        }
        if symbol.is_true() {
            return Err(RuntimeError::ConstantSymbol {
                name: "t".to_string(),
            });
        }
        self.expect_symbol_mut(symbol)?.value = Some(value);
        Ok(value)
    }

    pub fn set_symbol_value_by_name(
        &mut self,
        name: &str,
        value: LispValue,
    ) -> Result<LispValue, RuntimeError> {
        let symbol = self.intern(name);
        self.set_symbol_value(symbol, value)
    }

    pub fn is_bound_symbol(&self, symbol: LispValue) -> Result<bool, RuntimeError> {
        if symbol.is_nil() || symbol.is_true() {
            return Ok(true);
        }
        Ok(self.expect_symbol(symbol)?.value.is_some())
    }

    pub fn symbol_function(&self, symbol: LispValue) -> Result<Option<LispValue>, RuntimeError> {
        if symbol.is_nil() || symbol.is_true() {
            return Ok(None);
        }
        Ok(self.expect_symbol(symbol)?.function)
    }

    pub fn set_symbol_function(
        &mut self,
        symbol: LispValue,
        function: LispValue,
    ) -> Result<LispValue, RuntimeError> {
        if symbol.is_nil() {
            return Err(RuntimeError::ConstantSymbol {
                name: "nil".to_string(),
            });
        }
        if symbol.is_true() {
            return Err(RuntimeError::ConstantSymbol {
                name: "t".to_string(),
            });
        }
        self.expect_symbol_mut(symbol)?.function = Some(function);
        Ok(function)
    }

    pub fn string_contents(&self, string: LispValue) -> Result<&str, RuntimeError> {
        self.expect_string(string)?
            .data
            .as_str()
            .ok_or_else(|| RuntimeError::InvalidStringData("string is not valid UTF-8".to_string()))
    }

    pub fn string_data(&self, string: LispValue) -> Result<&LispStringData, RuntimeError> {
        Ok(&self.expect_string(string)?.data)
    }

    pub fn function_parts(
        &self,
        function: LispValue,
    ) -> Result<(SsaLambdaTemplate, Vec<LispValue>), RuntimeError> {
        let function = self.expect_function(function)?;
        Ok((function.template.clone(), function.captures.clone()))
    }

    pub fn lexical_cell_get(&self, cell: LispValue) -> Result<LispValue, RuntimeError> {
        Ok(self.expect_lexical_cell(cell)?.value)
    }

    pub fn lexical_cell_set(
        &mut self,
        cell: LispValue,
        value: LispValue,
    ) -> Result<LispValue, RuntimeError> {
        self.expect_lexical_cell_mut(cell)?.value = value;
        Ok(value)
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

    fn expect_symbol(&self, value: LispValue) -> Result<&Symbol, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "symbolp",
                value,
            });
        };
        self.symbol_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "symbolp",
                value,
            })
    }

    fn expect_symbol_mut(&mut self, value: LispValue) -> Result<&mut Symbol, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "symbolp",
                value,
            });
        };
        self.symbol_by_addr_mut(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "symbolp",
                value,
            })
    }

    fn expect_string(&self, value: LispValue) -> Result<&LispString, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "stringp",
                value,
            });
        };
        self.string_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "stringp",
                value,
            })
    }

    fn expect_function(&self, value: LispValue) -> Result<&FunctionObject, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "functionp",
                value,
            });
        };
        self.function_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "functionp",
                value,
            })
    }

    fn expect_lexical_cell(&self, value: LispValue) -> Result<&LexicalCell, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "lexical-cell",
                value,
            });
        };
        self.lexical_cell_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "lexical-cell",
                value,
            })
    }

    fn expect_lexical_cell_mut(
        &mut self,
        value: LispValue,
    ) -> Result<&mut LexicalCell, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "lexical-cell",
                value,
            });
        };
        self.lexical_cell_by_addr_mut(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "lexical-cell",
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

    fn symbol_by_addr(&self, addr: usize) -> Option<&Symbol> {
        for symbol in &self.symbols {
            let symbol_addr = (&**symbol as *const Symbol) as usize;
            if symbol_addr == addr && symbol.header.kind == HeapKind::Symbol {
                return Some(symbol);
            }
        }
        None
    }

    fn symbol_by_addr_mut(&mut self, addr: usize) -> Option<&mut Symbol> {
        for symbol in &mut self.symbols {
            let symbol_addr = (&**symbol as *const Symbol) as usize;
            if symbol_addr == addr && symbol.header.kind == HeapKind::Symbol {
                return Some(symbol);
            }
        }
        None
    }

    fn string_by_addr(&self, addr: usize) -> Option<&LispString> {
        for string in &self.strings {
            let string_addr = (&**string as *const LispString) as usize;
            if string_addr == addr && string.header.kind == HeapKind::String {
                return Some(string);
            }
        }
        None
    }

    fn function_by_addr(&self, addr: usize) -> Option<&FunctionObject> {
        for function in &self.functions {
            let function_addr = (&**function as *const FunctionObject) as usize;
            if function_addr == addr && function.header.kind == HeapKind::Function {
                return Some(function);
            }
        }
        None
    }

    fn lexical_cell_by_addr(&self, addr: usize) -> Option<&LexicalCell> {
        for cell in &self.lexical_cells {
            let cell_addr = (&**cell as *const LexicalCell) as usize;
            if cell_addr == addr && cell.header.kind == HeapKind::LexicalCell {
                return Some(cell);
            }
        }
        None
    }

    fn lexical_cell_by_addr_mut(&mut self, addr: usize) -> Option<&mut LexicalCell> {
        for cell in &mut self.lexical_cells {
            let cell_addr = (&**cell as *const LexicalCell) as usize;
            if cell_addr == addr && cell.header.kind == HeapKind::LexicalCell {
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
        if let (Some(left), Some(right)) =
            (self.cons_by_addr(left_addr), self.cons_by_addr(right_addr))
        {
            return self.equal_with_depth(left.car, right.car, depth - 1)
                && self.equal_with_depth(left.cdr, right.cdr, depth - 1);
        }
        if let (Some(left), Some(right)) = (
            self.string_by_addr(left_addr),
            self.string_by_addr(right_addr),
        ) {
            return left.data.schars() == right.data.schars()
                && left.data.sbytes() == right.data.sbytes()
                && left.data.sdata() == right.data.sdata();
        }
        false
    }

    pub fn format_value(&self, value: LispValue) -> String {
        self.format_value_with_depth(value, 64)
    }

    fn format_value_with_depth(&self, value: LispValue, depth: usize) -> String {
        if depth == 0 {
            return "#<max-depth>".to_string();
        }
        if !self.is_cons(value) {
            return self.format_atom_value(value);
        }
        let mut parts = Vec::new();
        let mut current = value;
        loop {
            let Some(addr) = current.heap_addr() else {
                parts.push(".".to_string());
                parts.push(self.format_atom_value(current));
                break;
            };
            let Some(cell) = self.cons_by_addr(addr) else {
                parts.push(".".to_string());
                parts.push(self.format_atom_value(current));
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

    fn format_atom_value(&self, value: LispValue) -> String {
        if let Ok(name) = self.symbol_name(value)
            && self.is_symbol(value)
        {
            return name;
        }
        if let Some(addr) = value.heap_addr()
            && let Some(string) = self.string_by_addr(addr)
        {
            return string.data.format_debug();
        }
        if self.is_function(value) {
            return "#<function>".to_string();
        }
        if value
            .heap_addr()
            .is_some_and(|addr| self.lexical_cell_by_addr(addr).is_some())
        {
            return "#<lexical-cell>".to_string();
        }
        format!("{value:?}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LispStringData {
    size: isize,
    size_byte: isize,
    intervals: Option<LispValue>,
    data: Vec<u8>,
}

impl LispStringData {
    pub const SIZE_BYTE_UNIBYTE: isize = -1;
    pub const SIZE_BYTE_RODATA: isize = -2;
    pub const SIZE_BYTE_IMMOVABLE: isize = -3;

    pub fn new(bytes: Vec<u8>, size: isize, size_byte: isize) -> Self {
        assert!(size >= 0, "Lisp_String size must be nonnegative");
        let nbytes = if size_byte < 0 { size } else { size_byte };
        assert!(nbytes >= 0, "Lisp_String byte size must be nonnegative");
        let nbytes = usize::try_from(nbytes).expect("Lisp_String byte size must fit usize");
        assert!(
            bytes.len() >= nbytes,
            "Lisp_String data must contain at least SBYTES bytes"
        );
        let mut data = bytes.into_iter().take(nbytes).collect::<Vec<_>>();
        data.push(0);
        Self {
            size,
            size_byte,
            intervals: None,
            data,
        }
    }

    pub fn make_string(value: &str) -> Self {
        let nbytes = value.len();
        let nchars = value.chars().count();
        if nbytes == nchars {
            Self::make_unibyte(value.as_bytes().to_vec())
        } else {
            Self::make_multibyte(value.as_bytes().to_vec(), nchars)
        }
    }

    pub fn make_unibyte(bytes: Vec<u8>) -> Self {
        let size = isize::try_from(bytes.len()).expect("unibyte string length must fit isize");
        Self::new(bytes, size, Self::SIZE_BYTE_UNIBYTE)
    }

    pub fn make_multibyte(bytes: Vec<u8>, chars: usize) -> Self {
        let size = isize::try_from(chars).expect("multibyte string char length must fit isize");
        let size_byte =
            isize::try_from(bytes.len()).expect("multibyte string byte length must fit isize");
        Self::new(bytes, size, size_byte)
    }

    pub fn size_raw(&self) -> isize {
        self.size
    }

    pub fn size_byte_raw(&self) -> isize {
        self.size_byte
    }

    pub fn intervals(&self) -> Option<LispValue> {
        self.intervals
    }

    pub fn set_intervals(&mut self, intervals: Option<LispValue>) {
        self.intervals = intervals;
    }

    pub fn string_multibyte(&self) -> bool {
        self.size_byte >= 0
    }

    pub fn schars(&self) -> usize {
        usize::try_from(self.size).expect("Lisp_String size must be nonnegative")
    }

    pub fn sbytes(&self) -> usize {
        let nbytes = if self.size_byte < 0 {
            self.size
        } else {
            self.size_byte
        };
        usize::try_from(nbytes).expect("Lisp_String byte size must be nonnegative")
    }

    pub fn sdata(&self) -> &[u8] {
        &self.data[..self.sbytes()]
    }

    pub fn sdata_with_nul(&self) -> &[u8] {
        &self.data
    }

    pub fn sref(&self, index: usize) -> Option<u8> {
        self.sdata().get(index).copied()
    }

    pub fn sset(&mut self, index: usize, value: u8) -> Option<()> {
        if index >= self.sbytes() {
            return None;
        }
        let slot = self.data.get_mut(index)?;
        *slot = value;
        Some(())
    }

    pub fn bytes(&self) -> &[u8] {
        self.sdata()
    }

    pub fn char_len(&self) -> usize {
        self.schars()
    }

    pub fn byte_len(&self) -> usize {
        self.sbytes()
    }

    pub fn is_multibyte(&self) -> bool {
        self.string_multibyte()
    }

    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(self.sdata()).ok()
    }

    fn format_debug(&self) -> String {
        match self.as_str() {
            Some(value) => format!("{value:?}"),
            None => format!("#<unibyte-string {:?}>", self.sdata()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    WrongTypeArgument {
        expected: &'static str,
        value: LispValue,
    },
    VoidVariable {
        name: String,
    },
    VoidFunction {
        name: String,
    },
    ConstantSymbol {
        name: String,
    },
    InvalidStringData(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongTypeArgument { expected, value } => {
                write!(f, "wrong type argument: expected {expected}, got {value:?}")
            }
            Self::VoidVariable { name } => write!(f, "void variable: {name}"),
            Self::VoidFunction { name } => write!(f, "void function: {name}"),
            Self::ConstantSymbol { name } => write!(f, "attempt to set constant symbol: {name}"),
            Self::InvalidStringData(message) => write!(f, "invalid string data: {message}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeapKind {
    Cons = 1,
    Symbol = 2,
    String = 3,
    Function = 4,
    LexicalCell = 5,
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

#[repr(C, align(8))]
struct Symbol {
    header: HeapHeader,
    name: String,
    value: Option<LispValue>,
    function: Option<LispValue>,
}

#[repr(C, align(8))]
struct LispString {
    header: HeapHeader,
    data: LispStringData,
}

#[repr(C, align(8))]
struct FunctionObject {
    header: HeapHeader,
    template: SsaLambdaTemplate,
    captures: Vec<LispValue>,
}

#[repr(C, align(8))]
struct LexicalCell {
    header: HeapHeader,
    value: LispValue,
}

#[cfg(test)]
mod tests {
    use super::{LispStringData, Runtime, RuntimeError};
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
    fn strings_are_heap_values_with_structural_equal() {
        let mut runtime = Runtime::new();
        let left = runtime.string("alpha");
        let right = runtime.string("alpha");

        assert!(runtime.is_string(left));
        assert_eq!(runtime.string_contents(left), Ok("alpha"));
        let data = runtime.string_data(left).expect("string data");
        assert_eq!(data.size_raw(), 5);
        assert_eq!(data.size_byte_raw(), LispStringData::SIZE_BYTE_UNIBYTE);
        assert_eq!(data.schars(), 5);
        assert_eq!(data.sbytes(), 5);
        assert_eq!(data.sdata(), b"alpha");
        assert_eq!(data.sdata_with_nul(), b"alpha\0");
        assert!(!data.string_multibyte());
        assert_eq!(data.intervals(), None);
        assert_ne!(left, right);
        assert!(runtime.equal(left, right));
        assert_eq!(runtime.format_value(left), "\"alpha\"");
    }

    #[test]
    fn strings_track_bytes_and_chars_separately() {
        let mut runtime = Runtime::new();
        let string = runtime.string("λ");
        let data = runtime.string_data(string).expect("string data");

        assert_eq!(data.size_raw(), 1);
        assert_eq!(data.size_byte_raw(), 2);
        assert_eq!(data.schars(), 1);
        assert_eq!(data.sbytes(), 2);
        assert_eq!(data.sdata(), "λ".as_bytes());
        assert!(data.string_multibyte());
    }

    #[test]
    fn unibyte_strings_allow_nul_and_non_utf8_bytes() {
        let mut runtime = Runtime::new();
        let string = runtime.string_from_bytes(vec![b'a', 0, 0xff], 0, false);
        let data = runtime.string_data(string).expect("string data");

        assert_eq!(data.size_raw(), 3);
        assert_eq!(data.size_byte_raw(), LispStringData::SIZE_BYTE_UNIBYTE);
        assert_eq!(data.schars(), 3);
        assert_eq!(data.sbytes(), 3);
        assert_eq!(data.sref(1), Some(0));
        assert_eq!(data.sdata(), &[b'a', 0, 0xff]);
        assert_eq!(data.sdata_with_nul(), &[b'a', 0, 0xff, 0]);
        assert_eq!(
            runtime.string_contents(string),
            Err(RuntimeError::InvalidStringData(
                "string is not valid UTF-8".to_string()
            ))
        );
    }

    #[test]
    fn intern_reuses_symbols_and_symbol_name_allocates_string() {
        let mut runtime = Runtime::new();
        let left = runtime.intern("alpha");
        let right = runtime.intern("alpha");

        assert_eq!(left, right);
        assert!(runtime.is_symbol(left));
        assert_eq!(runtime.symbol_name(left), Ok("alpha".to_string()));
        let name = runtime.symbol_name_value(left).expect("symbol name");
        assert_eq!(runtime.string_contents(name), Ok("alpha"));
        assert_eq!(runtime.symbol_count(), 1);
    }

    #[test]
    fn symbol_value_slots_track_boundp() {
        let mut runtime = Runtime::new();
        let symbol = runtime.intern("answer");
        let value = LispValue::expect_fixnum(42);

        assert_eq!(runtime.is_bound_symbol(symbol), Ok(false));
        assert_eq!(
            runtime.symbol_value(symbol),
            Err(RuntimeError::VoidVariable {
                name: "answer".to_string()
            })
        );
        assert_eq!(runtime.set_symbol_value(symbol, value), Ok(value));
        assert_eq!(runtime.is_bound_symbol(symbol), Ok(true));
        assert_eq!(runtime.symbol_value(symbol), Ok(value));
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
