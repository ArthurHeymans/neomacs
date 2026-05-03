use std::{collections::HashMap, fmt};

use neovm_compiler::ssa::SsaLambdaTemplate;

use crate::value::LispValue;

pub struct Runtime {
    cons_cells: Vec<Box<Cons>>,
    symbols: Vec<Box<Symbol>>,
    strings: Vec<Box<LispString>>,
    vectors: Vec<Box<VectorObject>>,
    hash_tables: Vec<Box<HashTableObject>>,
    functions: Vec<Box<FunctionObject>>,
    lexical_cells: Vec<Box<LexicalCell>>,
    floats: Vec<Box<FloatObj>>,
    interned_symbols: HashMap<String, LispValue>,
    dynamic_bindings: Vec<DynamicBinding>,
    features: Vec<LispValue>,
    nil_plist: LispValue,
    true_plist: LispValue,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            cons_cells: Vec::new(),
            symbols: Vec::new(),
            strings: Vec::new(),
            vectors: Vec::new(),
            hash_tables: Vec::new(),
            functions: Vec::new(),
            lexical_cells: Vec::new(),
            floats: Vec::new(),
            interned_symbols: HashMap::new(),
            dynamic_bindings: Vec::new(),
            features: Vec::new(),
            nil_plist: LispValue::NIL,
            true_plist: LispValue::NIL,
        }
    }
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

    pub fn make_vector(&mut self, len: usize, init: LispValue) -> LispValue {
        self.vector(vec![init; len])
    }

    pub fn vector(&mut self, elements: Vec<LispValue>) -> LispValue {
        let mut vector = Box::new(VectorObject {
            header: HeapHeader {
                kind: HeapKind::Vector,
            },
            elements,
        });
        let addr = (&mut *vector as *mut VectorObject) as usize;
        self.vectors.push(vector);
        LispValue::from_heap_addr(addr)
    }

    pub fn hash_table(&mut self, test: HashTableTest) -> LispValue {
        let mut table = Box::new(HashTableObject {
            header: HeapHeader {
                kind: HeapKind::HashTable,
            },
            test,
            entries: Vec::new(),
        });
        let addr = (&mut *table as *mut HashTableObject) as usize;
        self.hash_tables.push(table);
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
            plist: LispValue::NIL,
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

    pub fn is_vector(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.vector_by_addr(addr).is_some())
    }

    pub fn is_hash_table(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.hash_table_by_addr(addr).is_some())
    }

    pub fn is_function(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.function_by_addr(addr).is_some())
    }

    pub fn float(&mut self, value: f64) -> LispValue {
        let mut obj = Box::new(FloatObj {
            header: HeapHeader {
                kind: HeapKind::Float,
            },
            value,
        });
        let addr = (&mut *obj as *mut FloatObj) as usize;
        self.floats.push(obj);
        LispValue::from_heap_addr(addr)
    }

    pub fn is_float(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.float_by_addr(addr).is_some())
    }

    pub fn float_data(&self, value: LispValue) -> Result<f64, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument { expected: "float", value })?;
        let obj = self.float_by_addr(addr).ok_or(RuntimeError::WrongTypeArgument { expected: "float", value })?;
        Ok(obj.value)
    }

    pub fn as_number(&self, value: LispValue) -> Option<f64> {
        if let Some(fixnum) = value.as_fixnum() {
            return Some(fixnum as f64);
        }
        if let Some(addr) = value.heap_addr() {
            if let Some(obj) = self.float_by_addr(addr) {
                return Some(obj.value);
            }
        }
        None
    }

    pub fn is_number(&self, value: LispValue) -> bool {
        value.is_fixnum() || self.is_float(value)
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

    pub fn vector_count(&self) -> usize {
        self.vectors.len()
    }

    pub fn hash_table_count_allocated(&self) -> usize {
        self.hash_tables.len()
    }

    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    pub fn lexical_cell_count(&self) -> usize {
        self.lexical_cells.len()
    }

    pub fn dynamic_binding_count(&self) -> usize {
        self.dynamic_bindings.len()
    }

    pub fn feature_count(&self) -> usize {
        self.features.len()
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
        if let Some(value) = self.dynamic_symbol_value(symbol) {
            return Ok(value);
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
        if let Some(index) = self.dynamic_binding_index(symbol) {
            self.dynamic_bindings[index].value = value;
            return Ok(value);
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
        Ok(self.dynamic_symbol_value(symbol).is_some()
            || self.expect_symbol(symbol)?.value.is_some())
    }

    pub fn bind_dynamic_by_name(
        &mut self,
        name: &str,
        value: LispValue,
    ) -> Result<(), RuntimeError> {
        let symbol = self.intern(name);
        self.bind_dynamic(symbol, value)
    }

    pub fn bind_dynamic(
        &mut self,
        symbol: LispValue,
        value: LispValue,
    ) -> Result<(), RuntimeError> {
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
        self.expect_symbol(symbol)?;
        self.dynamic_bindings.push(DynamicBinding { symbol, value });
        Ok(())
    }

    pub fn unbind_dynamic(&mut self, count: usize) -> Result<(), RuntimeError> {
        let len = self.dynamic_bindings.len();
        if count > len {
            return Err(RuntimeError::DynamicBindingUnderflow {
                requested: count,
                available: len,
            });
        }
        self.dynamic_bindings.truncate(len - count);
        Ok(())
    }

    pub fn provide(&mut self, feature: LispValue) -> Result<LispValue, RuntimeError> {
        self.expect_symbol(feature)?;
        if !self.features.contains(&feature) {
            self.features.push(feature);
        }
        Ok(feature)
    }

    pub fn featurep(&self, feature: LispValue) -> Result<bool, RuntimeError> {
        self.expect_symbol(feature)?;
        Ok(self.features.contains(&feature))
    }

    pub fn symbol_property(
        &self,
        symbol: LispValue,
        property: LispValue,
    ) -> Result<LispValue, RuntimeError> {
        if symbol.is_nil() {
            return Ok(self.plist_get(self.nil_plist, property));
        }
        if symbol.is_true() {
            return Ok(self.plist_get(self.true_plist, property));
        }
        let symbol = self.expect_symbol(symbol)?;
        Ok(self.plist_get(symbol.plist, property))
    }

    pub fn put_symbol_property(
        &mut self,
        symbol: LispValue,
        property: LispValue,
        value: LispValue,
    ) -> Result<LispValue, RuntimeError> {
        if symbol.is_nil() {
            self.nil_plist = self.plist_put(self.nil_plist, property, value);
            return Ok(value);
        }
        if symbol.is_true() {
            self.true_plist = self.plist_put(self.true_plist, property, value);
            return Ok(value);
        }
        let plist = self.expect_symbol(symbol)?.plist;
        let plist = self.plist_put(plist, property, value);
        self.expect_symbol_mut(symbol)?.plist = plist;
        Ok(value)
    }

    pub fn symbol_plist(&self, symbol: LispValue) -> Result<LispValue, RuntimeError> {
        if symbol.is_nil() {
            return Ok(self.nil_plist);
        }
        if symbol.is_true() {
            return Ok(self.true_plist);
        }
        Ok(self.expect_symbol(symbol)?.plist)
    }

    pub fn set_symbol_plist(
        &mut self,
        symbol: LispValue,
        plist: LispValue,
    ) -> Result<LispValue, RuntimeError> {
        if symbol.is_nil() {
            self.nil_plist = plist;
            return Ok(plist);
        }
        if symbol.is_true() {
            self.true_plist = plist;
            return Ok(plist);
        }
        self.expect_symbol_mut(symbol)?.plist = plist;
        Ok(plist)
    }

    pub fn plist_get(&self, mut plist: LispValue, property: LispValue) -> LispValue {
        loop {
            let Some((found_property, value, next)) = self.plist_pair(plist) else {
                return LispValue::NIL;
            };
            if found_property == property {
                return value;
            }
            plist = next;
        }
    }

    pub fn plist_put(
        &mut self,
        plist: LispValue,
        property: LispValue,
        value: LispValue,
    ) -> LispValue {
        let mut current = plist;
        while let Some((found_property, _old_value, next)) = self.plist_pair(current) {
            if found_property == property {
                if let Some(value_cell) = self.plist_value_cell(current)
                    && let Some(addr) = value_cell.heap_addr()
                    && let Some(cell) = self.cons_by_addr_mut(addr)
                {
                    cell.car = value;
                    return plist;
                }
                break;
            }
            current = next;
        }
        let value_tail = self.cons(value, plist);
        self.cons(property, value_tail)
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

    pub fn vector_len(&self, vector: LispValue) -> Result<usize, RuntimeError> {
        Ok(self.expect_vector(vector)?.elements.len())
    }

    pub fn vector_elements(&self, vector: LispValue) -> Result<Vec<LispValue>, RuntimeError> {
        Ok(self.expect_vector(vector)?.elements.clone())
    }

    pub fn vector_aref(&self, vector: LispValue, index: usize) -> Result<LispValue, RuntimeError> {
        self.expect_vector(vector)?
            .elements
            .get(index)
            .copied()
            .ok_or(RuntimeError::ArgsOutOfRange {
                value: vector,
                index,
            })
    }

    pub fn vector_aset(
        &mut self,
        vector: LispValue,
        index: usize,
        value: LispValue,
    ) -> Result<LispValue, RuntimeError> {
        let vector_object = self.expect_vector_mut(vector)?;
        let Some(slot) = vector_object.elements.get_mut(index) else {
            return Err(RuntimeError::ArgsOutOfRange {
                value: vector,
                index,
            });
        };
        *slot = value;
        Ok(value)
    }

    pub fn hash_table_count(&self, table: LispValue) -> Result<usize, RuntimeError> {
        Ok(self.expect_hash_table(table)?.entries.len())
    }

    pub fn gethash(
        &self,
        key: LispValue,
        table: LispValue,
    ) -> Result<Option<LispValue>, RuntimeError> {
        let table_object = self.expect_hash_table(table)?;
        Ok(self
            .hash_table_entry_index(table_object, key)
            .map(|index| table_object.entries[index].value))
    }

    pub fn puthash(
        &mut self,
        key: LispValue,
        value: LispValue,
        table: LispValue,
    ) -> Result<LispValue, RuntimeError> {
        let index = {
            let table_object = self.expect_hash_table(table)?;
            self.hash_table_entry_index(table_object, key)
        };
        let table_object = self.expect_hash_table_mut(table)?;
        if let Some(index) = index {
            table_object.entries[index].value = value;
        } else {
            table_object.entries.push(HashEntry { key, value });
        }
        Ok(value)
    }

    pub fn remhash(&mut self, key: LispValue, table: LispValue) -> Result<LispValue, RuntimeError> {
        let index = {
            let table_object = self.expect_hash_table(table)?;
            self.hash_table_entry_index(table_object, key)
        };
        let table_object = self.expect_hash_table_mut(table)?;
        if let Some(index) = index {
            table_object.entries.remove(index);
        }
        Ok(LispValue::NIL)
    }

    pub fn clrhash(&mut self, table: LispValue) -> Result<LispValue, RuntimeError> {
        self.expect_hash_table_mut(table)?.entries.clear();
        Ok(table)
    }

    pub fn hash_table_entries(
        &self,
        table: LispValue,
    ) -> Result<Vec<(LispValue, LispValue)>, RuntimeError> {
        Ok(self
            .expect_hash_table(table)?
            .entries
            .iter()
            .map(|entry| (entry.key, entry.value))
            .collect())
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

    fn dynamic_symbol_value(&self, symbol: LispValue) -> Option<LispValue> {
        self.dynamic_binding_index(symbol)
            .map(|index| self.dynamic_bindings[index].value)
    }

    fn dynamic_binding_index(&self, symbol: LispValue) -> Option<usize> {
        self.dynamic_bindings
            .iter()
            .rposition(|binding| binding.symbol == symbol)
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

    fn expect_vector(&self, value: LispValue) -> Result<&VectorObject, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "vectorp",
                value,
            });
        };
        self.vector_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "vectorp",
                value,
            })
    }

    fn expect_vector_mut(&mut self, value: LispValue) -> Result<&mut VectorObject, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "vectorp",
                value,
            });
        };
        self.vector_by_addr_mut(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "vectorp",
                value,
            })
    }

    fn expect_hash_table(&self, value: LispValue) -> Result<&HashTableObject, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "hash-table-p",
                value,
            });
        };
        self.hash_table_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "hash-table-p",
                value,
            })
    }

    fn expect_hash_table_mut(
        &mut self,
        value: LispValue,
    ) -> Result<&mut HashTableObject, RuntimeError> {
        let Some(addr) = value.heap_addr() else {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "hash-table-p",
                value,
            });
        };
        self.hash_table_by_addr_mut(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "hash-table-p",
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

    fn vector_by_addr(&self, addr: usize) -> Option<&VectorObject> {
        for vector in &self.vectors {
            let vector_addr = (&**vector as *const VectorObject) as usize;
            if vector_addr == addr && vector.header.kind == HeapKind::Vector {
                return Some(vector);
            }
        }
        None
    }

    fn vector_by_addr_mut(&mut self, addr: usize) -> Option<&mut VectorObject> {
        for vector in &mut self.vectors {
            let vector_addr = (&**vector as *const VectorObject) as usize;
            if vector_addr == addr && vector.header.kind == HeapKind::Vector {
                return Some(vector);
            }
        }
        None
    }

    fn hash_table_by_addr(&self, addr: usize) -> Option<&HashTableObject> {
        for table in &self.hash_tables {
            let table_addr = (&**table as *const HashTableObject) as usize;
            if table_addr == addr && table.header.kind == HeapKind::HashTable {
                return Some(table);
            }
        }
        None
    }

    fn hash_table_by_addr_mut(&mut self, addr: usize) -> Option<&mut HashTableObject> {
        for table in &mut self.hash_tables {
            let table_addr = (&**table as *const HashTableObject) as usize;
            if table_addr == addr && table.header.kind == HeapKind::HashTable {
                return Some(table);
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

    fn float_by_addr(&self, addr: usize) -> Option<&FloatObj> {
        for obj in &self.floats {
            let obj_addr = (&**obj as *const FloatObj) as usize;
            if obj_addr == addr && obj.header.kind == HeapKind::Float {
                return Some(obj);
            }
        }
        None
    }

    fn plist_pair(&self, pair_cell: LispValue) -> Option<(LispValue, LispValue, LispValue)> {
        let pair = self.cons_by_addr(pair_cell.heap_addr()?)?;
        let value_cell = self.cons_by_addr(pair.cdr.heap_addr()?)?;
        Some((pair.car, value_cell.car, value_cell.cdr))
    }

    fn plist_value_cell(&self, pair_cell: LispValue) -> Option<LispValue> {
        let pair = self.cons_by_addr(pair_cell.heap_addr()?)?;
        self.cons_by_addr(pair.cdr.heap_addr()?)?;
        Some(pair.cdr)
    }

    fn hash_table_entry_index(&self, table: &HashTableObject, key: LispValue) -> Option<usize> {
        table.entries.iter().position(|entry| match table.test {
            HashTableTest::Eq | HashTableTest::Eql => entry.key == key,
            HashTableTest::Equal => self.equal(entry.key, key),
        })
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
        if let (Some(left), Some(right)) = (
            self.vector_by_addr(left_addr),
            self.vector_by_addr(right_addr),
        ) {
            return left.elements.len() == right.elements.len()
                && left
                    .elements
                    .iter()
                    .zip(&right.elements)
                    .all(|(left, right)| self.equal_with_depth(*left, *right, depth - 1));
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
        if let Some(addr) = value.heap_addr()
            && let Some(vector) = self.vector_by_addr(addr)
        {
            let elements = vector
                .elements
                .iter()
                .map(|value| self.format_value_with_depth(*value, 63))
                .collect::<Vec<_>>();
            return format!("[{}]", elements.join(" "));
        }
        if let Some(addr) = value.heap_addr()
            && let Some(table) = self.hash_table_by_addr(addr)
        {
            return format!("#<hash-table count {}>", table.entries.len());
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
        if let Some(addr) = value.heap_addr()
            && let Some(obj) = self.float_by_addr(addr)
        {
            return format!("{}", obj.value);
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
    DynamicBindingUnderflow {
        requested: usize,
        available: usize,
    },
    ArgsOutOfRange {
        value: LispValue,
        index: usize,
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
            Self::DynamicBindingUnderflow {
                requested,
                available,
            } => write!(
                f,
                "dynamic binding underflow: requested {requested}, available {available}"
            ),
            Self::ArgsOutOfRange { value, index } => {
                write!(f, "args out of range: value {value:?}, index {index}")
            }
            Self::InvalidStringData(message) => write!(f, "invalid string data: {message}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashTableTest {
    Eq,
    Eql,
    Equal,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeapKind {
    Cons = 1,
    Symbol = 2,
    String = 3,
    Vector = 4,
    HashTable = 5,
    Function = 6,
    LexicalCell = 7,
    Float = 8,
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
    plist: LispValue,
}

#[repr(C, align(8))]
struct LispString {
    header: HeapHeader,
    data: LispStringData,
}

#[repr(C, align(8))]
struct VectorObject {
    header: HeapHeader,
    elements: Vec<LispValue>,
}

#[repr(C, align(8))]
struct HashTableObject {
    header: HeapHeader,
    test: HashTableTest,
    entries: Vec<HashEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HashEntry {
    key: LispValue,
    value: LispValue,
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

#[repr(C, align(8))]
struct FloatObj {
    header: HeapHeader,
    value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DynamicBinding {
    symbol: LispValue,
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
    fn vectors_are_heap_values_with_indexed_slots() {
        let mut runtime = Runtime::new();
        let first = LispValue::expect_fixnum(1);
        let second = LispValue::expect_fixnum(2);
        let vector = runtime.vector(vec![first, second]);

        assert!(runtime.is_vector(vector));
        assert_eq!(runtime.vector_count(), 1);
        assert_eq!(runtime.vector_len(vector), Ok(2));
        assert_eq!(runtime.vector_elements(vector), Ok(vec![first, second]));
        assert_eq!(runtime.vector_aref(vector, 1), Ok(second));
        assert_eq!(runtime.vector_aset(vector, 1, first), Ok(first));
        assert_eq!(runtime.vector_aref(vector, 1), Ok(first));
        assert_eq!(runtime.format_value(vector), "[1 1]");
    }

    #[test]
    fn equal_compares_vector_structure() {
        let mut runtime = Runtime::new();
        let left = runtime.vector(vec![
            LispValue::expect_fixnum(1),
            LispValue::expect_fixnum(2),
        ]);
        let right = runtime.vector(vec![
            LispValue::expect_fixnum(1),
            LispValue::expect_fixnum(2),
        ]);
        let different = runtime.vector(vec![
            LispValue::expect_fixnum(1),
            LispValue::expect_fixnum(3),
        ]);

        assert!(runtime.equal(left, right));
        assert!(!runtime.equal(left, different));
    }

    #[test]
    fn hash_tables_store_entries_with_configured_test() {
        let mut runtime = Runtime::new();
        let table = runtime.hash_table(super::HashTableTest::Equal);
        let left_key = runtime.string("key");
        let right_key = runtime.string("key");
        let value = LispValue::expect_fixnum(42);

        assert!(runtime.is_hash_table(table));
        assert_eq!(runtime.hash_table_count_allocated(), 1);
        assert_eq!(runtime.hash_table_count(table), Ok(0));
        assert_eq!(runtime.puthash(left_key, value, table), Ok(value));
        assert_eq!(runtime.gethash(right_key, table), Ok(Some(value)));
        assert_eq!(runtime.hash_table_count(table), Ok(1));
        assert_eq!(runtime.remhash(right_key, table), Ok(LispValue::NIL));
        assert_eq!(runtime.gethash(left_key, table), Ok(None));
        assert_eq!(runtime.clrhash(table), Ok(table));
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
    fn dynamic_bindings_shadow_globals_and_restore() {
        let mut runtime = Runtime::new();
        let symbol = runtime.intern("dyn");
        let global = LispValue::expect_fixnum(1);
        let dynamic = LispValue::expect_fixnum(2);
        let updated_dynamic = LispValue::expect_fixnum(3);

        assert_eq!(runtime.set_symbol_value(symbol, global), Ok(global));
        assert_eq!(runtime.bind_dynamic(symbol, dynamic), Ok(()));
        assert_eq!(runtime.dynamic_binding_count(), 1);
        assert_eq!(runtime.symbol_value(symbol), Ok(dynamic));
        assert_eq!(
            runtime.set_symbol_value(symbol, updated_dynamic),
            Ok(updated_dynamic)
        );
        assert_eq!(runtime.symbol_value(symbol), Ok(updated_dynamic));
        assert_eq!(runtime.unbind_dynamic(1), Ok(()));
        assert_eq!(runtime.dynamic_binding_count(), 0);
        assert_eq!(runtime.symbol_value(symbol), Ok(global));
    }

    #[test]
    fn features_track_provided_symbols() {
        let mut runtime = Runtime::new();
        let feature = runtime.intern("object-feature");

        assert_eq!(runtime.featurep(feature), Ok(false));
        assert_eq!(runtime.provide(feature), Ok(feature));
        assert_eq!(runtime.provide(feature), Ok(feature));
        assert_eq!(runtime.featurep(feature), Ok(true));
        assert_eq!(runtime.feature_count(), 1);
    }

    #[test]
    fn symbol_plists_store_properties_by_eq() {
        let mut runtime = Runtime::new();
        let symbol = runtime.intern("object-symbol");
        let property = runtime.intern("object-property");
        let first = LispValue::expect_fixnum(1);
        let second = LispValue::expect_fixnum(2);

        assert_eq!(
            runtime.symbol_property(symbol, property),
            Ok(LispValue::NIL)
        );
        assert_eq!(
            runtime.put_symbol_property(symbol, property, first),
            Ok(first)
        );
        assert_eq!(runtime.symbol_property(symbol, property), Ok(first));
        assert_eq!(
            runtime.put_symbol_property(symbol, property, second),
            Ok(second)
        );
        assert_eq!(runtime.symbol_property(symbol, property), Ok(second));
        assert_eq!(
            runtime.plist_get(runtime.symbol_plist(symbol).expect("plist"), property),
            second
        );
        assert_eq!(
            runtime.put_symbol_property(LispValue::NIL, property, first),
            Ok(first)
        );
        assert_eq!(runtime.symbol_property(LispValue::NIL, property), Ok(first));
    }

    #[test]
    fn nested_dynamic_bindings_use_topmost_value() {
        let mut runtime = Runtime::new();
        let symbol = runtime.intern("dyn");
        let outer = LispValue::expect_fixnum(1);
        let inner = LispValue::expect_fixnum(2);

        assert_eq!(runtime.bind_dynamic(symbol, outer), Ok(()));
        assert_eq!(runtime.bind_dynamic(symbol, inner), Ok(()));
        assert_eq!(runtime.symbol_value(symbol), Ok(inner));
        assert_eq!(runtime.unbind_dynamic(1), Ok(()));
        assert_eq!(runtime.symbol_value(symbol), Ok(outer));
        assert_eq!(runtime.unbind_dynamic(1), Ok(()));
        assert_eq!(
            runtime.symbol_value(symbol),
            Err(RuntimeError::VoidVariable {
                name: "dyn".to_string()
            })
        );
    }
}
