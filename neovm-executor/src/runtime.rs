use std::{collections::HashMap, fmt, sync::Arc};

use neovm_compiler::regir::RegFunction;
use neovm_compiler::ssa::SsaLambdaTemplate;
use neovm_gc::{Heap, HeapConfig, Mutator};

use crate::agent_pool::AgentPool;
use crate::thread::ThreadScheduler;
use crate::value::LispValue;

pub struct Runtime {
    /// Shared GC heap for multi-threaded allocation.
    pub(crate) gc_heap: Arc<Heap>,
    /// Cooperative thread scheduler.  Created with the Runtime,
    /// holds the main thread and any spawned child threads.
    pub(crate) scheduler: ThreadScheduler,
    pub(crate) agent_pool: AgentPool,
    /// O(1) object identity index: maps raw heap address → type tag.
    /// Eliminates the linear scans in every is_* predicate.  Updated
    /// on allocation; cleared objects are removed lazily (the lookup
    /// tolerates stale entries by falling back to Vec scans).
    object_index: HashMap<usize, HeapKind>,
    cons_cells: Vec<Box<Cons>>,
    symbols: Vec<Box<Symbol>>,
    strings: Vec<Box<LispString>>,
    vectors: Vec<Box<VectorObject>>,
    hash_tables: Vec<Box<HashTableObject>>,
    functions: Vec<Box<FunctionObject>>,
    lexical_cells: Vec<Box<LexicalCell>>,
    floats: Vec<Box<FloatObj>>,
    bignums: Vec<Box<BignumObj>>,
    /// Cache compiled RegIR for lambda functions keyed by address.
    /// Avoids re-compiling the same SSA template on every invocation.
    pub(crate) lambda_cache: HashMap<usize, RegFunction>,
    atoms: Vec<Box<AtomObj>>,
    /// GC-allocated atom addresses. Checked before Vec scan.
    gc_atoms: HashMap<usize, ()>,
    agents: Vec<Box<AgentObj>>,
    gc_agents: HashMap<usize, ()>,
    mutexes: Vec<Box<MutexObj>>,
    gc_mutexes: HashMap<usize, ()>,
    condvars: Vec<Box<CondvarObj>>,
    gc_condvars: HashMap<usize, ()>,
    interned_symbols: HashMap<String, LispValue>,
    dynamic_bindings: Vec<DynamicBinding>,
    features: Vec<LispValue>,
    nil_plist: LispValue,
    true_plist: LispValue,
    match_data: Option<MatchData>,
    load_path: Vec<String>,
}

#[derive(Clone)]
struct MatchData {
    string: String,
    groups: Vec<Option<(usize, usize)>>,
}

impl Default for Runtime {
    fn default() -> Self {
        let gc_heap = Arc::new(Heap::new(HeapConfig::default()));
        let scheduler = ThreadScheduler::new(Arc::clone(&gc_heap));
        Self {
            gc_heap,
            scheduler,
            agent_pool: AgentPool::new(),
            object_index: HashMap::new(),
            cons_cells: Vec::new(),
            symbols: Vec::new(),
            strings: Vec::new(),
            vectors: Vec::new(),
            hash_tables: Vec::new(),
            functions: Vec::new(),
            lexical_cells: Vec::new(),
            floats: Vec::new(),
            bignums: Vec::new(),
            lambda_cache: HashMap::new(),
            atoms: Vec::new(),
            gc_atoms: HashMap::new(),
            agents: Vec::new(),
            gc_agents: HashMap::new(),
            mutexes: Vec::new(),
            gc_mutexes: HashMap::new(),
            condvars: Vec::new(),
            gc_condvars: HashMap::new(),
            interned_symbols: HashMap::new(),
            dynamic_bindings: Vec::new(),
            features: Vec::new(),
            nil_plist: LispValue::NIL,
            true_plist: LispValue::NIL,
            match_data: None,
            load_path: Vec::new(),
        }
    }
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a closure in a native OS thread with a forked Runtime.
    /// The closure receives the forked Runtime and returns a LispValue
    /// result.  Returns a JoinHandle for joining and retrieving the result.
    pub fn spawn_native<F>(&self, f: F) -> std::thread::JoinHandle<LispValue>
    where
        F: FnOnce(&mut Runtime) -> LispValue + Send + 'static,
    {
        let mut forked = self.fork();
        std::thread::spawn(move || {
            let result = f(&mut forked);
            // Notify the scheduler that this thread is done.
            // (In Phase 3 full integration, this would be more graceful.)
            result
        })
    }

    /// Create a lightweight fork of this Runtime for use in a native OS
    /// thread.  The forked Runtime shares the `gc_heap` and `scheduler`
    /// (via Arc) but has its own Vec-based heaps, dynamic bindings, and
    /// thread-local state.
    pub fn fork(&self) -> Self {
        Self {
            gc_heap: Arc::clone(&self.gc_heap),
            scheduler: self.scheduler.clone(),
            agent_pool: AgentPool::new(),
            object_index: HashMap::new(),
            cons_cells: Vec::new(),
            symbols: Vec::new(),
            strings: Vec::new(),
            vectors: Vec::new(),
            hash_tables: Vec::new(),
            functions: Vec::new(),
            lexical_cells: Vec::new(),
            floats: Vec::new(),
            bignums: Vec::new(),
            lambda_cache: HashMap::new(),
            atoms: Vec::new(),
            gc_atoms: HashMap::new(),
            agents: Vec::new(),
            gc_agents: HashMap::new(),
            mutexes: Vec::new(),
            gc_mutexes: HashMap::new(),
            condvars: Vec::new(),
            gc_condvars: HashMap::new(),
            interned_symbols: HashMap::new(),
            dynamic_bindings: Vec::new(),
            features: self.features.clone(),
            nil_plist: self.nil_plist,
            true_plist: self.true_plist,
            match_data: None,
            load_path: self.load_path.clone(),
        }
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
        self.object_index.insert(addr, HeapKind::Cons);
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
        self.object_index.insert(addr, HeapKind::String);
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
        self.object_index.insert(addr, HeapKind::Vector);
        self.vectors.push(vector);
        LispValue::from_heap_addr(addr)
    }

    pub fn hash_table(&mut self, test: HashTableTest) -> LispValue {
        let mut table = Box::new(HashTableObject {
            header: HeapHeader {
                kind: HeapKind::HashTable,
            },
            test,
            entries: HashMap::new(),
            equal_entries: Vec::new(),
        });
        let addr = (&mut *table as *mut HashTableObject) as usize;
        self.object_index.insert(addr, HeapKind::HashTable);
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
        self.object_index.insert(addr, HeapKind::Function);
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
        self.object_index.insert(addr, HeapKind::LexicalCell);
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
        self.object_index.insert(addr, HeapKind::Symbol);
        self.symbols.push(symbol);
        self.interned_symbols.insert(name, value);
        value
    }

    pub fn make_symbol(&mut self, name: &str) -> LispValue {
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
        self.object_index.insert(addr, HeapKind::Symbol);
        self.symbols.push(symbol);
        value
    }

    pub fn intern_soft(&self, name: &str) -> Option<LispValue> {
        match name {
            "nil" => return Some(LispValue::NIL),
            "t" => return Some(LispValue::TRUE),
            _ => {}
        }
        self.interned_symbols.get(name).copied()
    }

    #[inline(always)]
    #[inline(always)]
    pub fn car(&self, pair: LispValue) -> Result<LispValue, RuntimeError> {
        if pair.is_nil() {
            return Ok(LispValue::NIL);
        }
        Ok(self.expect_cons(pair)?.car)
    }

    #[inline(always)]
    pub fn cdr(&self, pair: LispValue) -> Result<LispValue, RuntimeError> {
        if pair.is_nil() {
            return Ok(LispValue::NIL);
        }
        Ok(self.expect_cons(pair)?.cdr)
    }

    #[inline(always)]
    pub fn set_car(&mut self, pair: LispValue, car: LispValue) -> Result<LispValue, RuntimeError> {
        self.expect_cons_mut(pair)?.car = car;
        Ok(car)
    }

    pub fn set_cdr(&mut self, pair: LispValue, cdr: LispValue) -> Result<LispValue, RuntimeError> {
        self.expect_cons_mut(pair)?.cdr = cdr;
        Ok(cdr)
    }

    /// O(1) type check using the object index.
    #[inline(always)]
    #[inline(always)]
    fn heap_kind(&self, addr: usize) -> Option<HeapKind> {
        self.object_index.get(&addr).copied()
    }

    #[inline(always)]
    pub fn is_cons(&self, value: LispValue) -> bool {
        value.heap_addr().is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::Cons))
    }

    #[inline(always)]
    pub fn is_symbol(&self, value: LispValue) -> bool {
        value.is_nil() || value.is_true()
            || value.heap_addr().is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::Symbol))
    }

    #[inline(always)]
    pub fn is_string(&self, value: LispValue) -> bool {
        value.heap_addr().is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::String))
    }

    #[inline(always)]
    pub fn is_vector(&self, value: LispValue) -> bool {
        value.heap_addr().is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::Vector))
    }

    #[inline(always)]
    pub fn is_hash_table(&self, value: LispValue) -> bool {
        value.heap_addr().is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::HashTable))
    }

    #[inline(always)]
    pub fn is_function(&self, value: LispValue) -> bool {
        value.heap_addr().is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::Function))
    }

    pub fn float(&mut self, value: f64) -> LispValue {
        let mut obj = Box::new(FloatObj {
            header: HeapHeader {
                kind: HeapKind::Float,
            },
            value,
        });
        let addr = (&mut *obj as *mut FloatObj) as usize;
        self.object_index.insert(addr, HeapKind::Float);
        self.floats.push(obj);
        LispValue::from_heap_addr(addr)
    }

    #[inline(always)]
    pub fn is_float(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::Float))
    }

    pub fn float_data(&self, value: LispValue) -> Result<f64, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "float",
            value,
        })?;
        if self.heap_kind(addr) != Some(HeapKind::Float) {
            return Err(RuntimeError::WrongTypeArgument { expected: "float", value });
        }
        let obj: &FloatObj = unsafe { Self::deref_heap(addr) };
        Ok(obj.value)
    }

    pub fn as_number(&self, value: LispValue) -> Option<f64> {
        if let Some(fixnum) = value.as_fixnum() {
            return Some(fixnum as f64);
        }
        if let Some(addr) = value.heap_addr()
            && self.heap_kind(addr) == Some(HeapKind::Float)
        {
            let obj: &FloatObj = unsafe { Self::deref_heap(addr) };
            return Some(obj.value);
        }
        None
    }

    pub fn is_number(&self, value: LispValue) -> bool {
        value.is_fixnum() || self.is_float(value) || self.is_bignum(value)
    }

    pub fn bignum(&mut self, value: rug::Integer) -> LispValue {
        let mut obj = Box::new(BignumObj {
            header: HeapHeader {
                kind: HeapKind::Bignum,
            },
            value,
        });
        let addr = (&mut *obj as *mut BignumObj) as usize;
        self.object_index.insert(addr, HeapKind::Bignum);
        self.bignums.push(obj);
        LispValue::from_heap_addr(addr)
    }

    pub fn is_bignum(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::Bignum))
    }

    pub fn as_integer(&self, value: LispValue) -> Option<rug::Integer> {
        if let Some(fixnum) = value.as_fixnum() {
            return Some(rug::Integer::from(fixnum));
        }
        if let Some(addr) = value.heap_addr() {
            if let Some(obj) = self.bignum_by_addr(addr) {
                return Some(obj.value.clone());
            }
        }
        None
    }

    /// Extract the bignum value, returning an error if it's not a bignum.
    /// Create a clone since the bignum is stored in a Vec and might move.
    pub fn bignum_data(&self, value: LispValue) -> Result<rug::Integer, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "bignum",
            value,
        })?;
        if self.heap_kind(addr) != Some(HeapKind::Bignum) {
            return Err(RuntimeError::WrongTypeArgument { expected: "bignum", value });
        }
        let obj: &BignumObj = unsafe { Self::deref_heap(addr) };
        Ok(obj.value.clone())
    }

    // --- Atoms ---

    pub fn make_atom(&mut self, value: LispValue) -> LispValue {
        let val_u64 = value.to_abi_i64() as u64;
        // Try GC allocation first
        if let Some(addr) = self.make_atom_gc(val_u64) {
            return LispValue::from_heap_addr(addr);
        }
        // Fall back to Vec-based allocation
        let mut boxed = Box::new(AtomObj {
            header: HeapHeader { kind: HeapKind::Atom },
            value: std::sync::atomic::AtomicU64::new(val_u64),
        });
        let addr = (&mut *boxed as *mut AtomObj) as usize;
        self.object_index.insert(addr, HeapKind::Atom);
        self.atoms.push(boxed);
        LispValue::from_heap_addr(addr)
    }

    fn make_atom_gc(&mut self, val_u64: u64) -> Option<usize> {
        let mut mutator = self.gc_heap.mutator();
        let mut scope = mutator.handle_scope();
        let obj = AtomObj {
            header: HeapHeader { kind: HeapKind::Atom },
            value: std::sync::atomic::AtomicU64::new(val_u64),
        };
        let root = mutator.alloc(&mut scope, obj).ok()?;
        let ptr = root.as_gc().as_non_null().as_ptr();
        let addr = ptr as usize;
        self.object_index.insert(addr, HeapKind::Atom);
        self.gc_atoms.insert(addr, ());
        Some(addr)
    }

    pub fn is_atom(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::Atom))
    }

    /// Read the current value of an atom (lock-free).
    pub fn atom_deref(&self, atom: LispValue) -> Result<LispValue, RuntimeError> {
        let obj = self.atom_obj(atom)?;
        let raw = obj.value.load(std::sync::atomic::Ordering::Relaxed);
        Ok(LispValue::from_abi_i64(raw as i64))
    }

    /// Atomically set the atom to `new_value`. Returns the new value.
    pub fn atom_reset(&self, atom: LispValue, new_value: LispValue) -> Result<LispValue, RuntimeError> {
        let obj = self.atom_obj(atom)?;
        obj.value
            .store(new_value.to_abi_i64() as u64, std::sync::atomic::Ordering::Relaxed);
        Ok(new_value)
    }

    /// Low-level CAS loop step for atom-swap!.  Reads current value,
    /// returns it to the caller so the interpreter can compute the new
    /// value.  The caller must call `atom_cas` with the result.
    pub fn atom_read_for_swap(&self, atom: LispValue) -> Result<LispValue, RuntimeError> {
        let obj = self.atom_obj(atom)?;
        let raw = obj.value.load(std::sync::atomic::Ordering::Acquire);
        Ok(LispValue::from_abi_i64(raw as i64))
    }

    /// Try to CAS the atom from `expected` to `new_value`.  Returns the
    /// actual value after the attempt (which equals `new_value` on success).
    pub fn atom_cas(
        &self,
        atom: LispValue,
        expected: LispValue,
        new_value: LispValue,
    ) -> Result<(LispValue, bool), RuntimeError> {
        let obj = self.atom_obj(atom)?;
        let expected_raw = expected.to_abi_i64() as u64;
        let new_raw = new_value.to_abi_i64() as u64;
        match obj.value.compare_exchange(
            expected_raw,
            new_raw,
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
        ) {
            Ok(_) => Ok((new_value, true)),
            Err(actual) => Ok((LispValue::from_abi_i64(actual as i64), false)),
        }
    }

    /// Compare-and-set: if atom's current value equals `old`, set to `new`.
    /// Returns t on success, nil on failure.
    pub fn atom_compare_and_set(
        &self,
        atom: LispValue,
        old: LispValue,
        new: LispValue,
    ) -> Result<bool, RuntimeError> {
        let obj = self.atom_obj(atom)?;
        let old_raw = old.to_abi_i64() as u64;
        let new_raw = new.to_abi_i64() as u64;
        Ok(obj
            .value
            .compare_exchange(
                old_raw,
                new_raw,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok())
    }

    // --- Agents ---

    pub fn make_agent(&mut self, value: LispValue) -> LispValue {
        let inner = std::sync::Arc::new(std::sync::Mutex::new(AgentInner {
            value,
            queue: Vec::new(),
            error: None,
        }));
        let mut boxed = Box::new(AgentObj {
            header: HeapHeader {
                kind: HeapKind::Agent,
            },
            inner,
        });
        let addr = (&mut *boxed as *mut AgentObj) as usize;
        self.object_index.insert(addr, HeapKind::Agent);
        self.agents.push(boxed);
        LispValue::from_heap_addr(addr)
    }

    pub fn is_agent(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::Agent))
    }

    /// Read the current agent value (non-blocking, may be stale).
    pub fn agent_deref(&self, agent: LispValue) -> Result<LispValue, RuntimeError> {
        let obj = self.agent_obj(agent)?;
        let inner = obj.inner.lock().map_err(|_| RuntimeError::WrongTypeArgument {
            expected: "agent (not poisoned)",
            value: agent,
        })?;
        Ok(inner.value)
    }

    /// Queue an action on the agent. Returns the agent.
    pub fn agent_send(
        &self,
        agent: LispValue,
        func: LispValue,
        args: &[LispValue],
        via_pool: bool,
    ) -> Result<LispValue, RuntimeError> {
        let obj = self.agent_obj(agent)?;
        let mut inner = obj.inner.lock().map_err(|_| RuntimeError::WrongTypeArgument {
            expected: "agent (not poisoned)",
            value: agent,
        })?;
        inner.queue.push(AgentAction {
            func,
            args: args.to_vec(),
            via_pool,
        });
        Ok(agent)
    }

    /// Pop the next pending action for this agent.  Returns `None` if
    /// the queue is empty.  The caller (interpreter) executes the action
    /// and calls `agent_update` with the result.
    pub fn agent_pop_action(&self, agent: LispValue) -> Result<Option<AgentAction>, RuntimeError> {
        let obj = self.agent_obj(agent)?;
        let mut inner = obj.inner.lock().map_err(|_| RuntimeError::WrongTypeArgument {
            expected: "agent (not poisoned)",
            value: agent,
        })?;
        if inner.queue.is_empty() {
            return Ok(None);
        }
        Ok(Some(inner.queue.remove(0)))
    }

    /// Update the agent's value after executing an action.  On error,
    /// the agent's error field is set.
    pub fn agent_update(
        &self,
        agent: LispValue,
        new_value: LispValue,
        error: Option<LispValue>,
    ) -> Result<(), RuntimeError> {
        let obj = self.agent_obj(agent)?;
        let mut inner = obj.inner.lock().map_err(|_| RuntimeError::WrongTypeArgument {
            expected: "agent (not poisoned)",
            value: agent,
        })?;
        if let Some(err) = error {
            inner.error = Some(err);
        } else {
            inner.value = new_value;
        }
        Ok(())
    }

    pub fn agent_has_actions(&self, agent: LispValue) -> Result<bool, RuntimeError> {
        let obj = self.agent_obj(agent)?;
        let inner = obj.inner.lock().map_err(|_| RuntimeError::WrongTypeArgument {
            expected: "agent (not poisoned)",
            value: agent,
        })?;
        Ok(!inner.queue.is_empty())
    }

    pub fn agent_error(&self, agent: LispValue) -> Result<Option<LispValue>, RuntimeError> {
        let obj = self.agent_obj(agent)?;
        let inner = obj.inner.lock().map_err(|_| RuntimeError::WrongTypeArgument {
            expected: "agent (not poisoned)",
            value: agent,
        })?;
        Ok(inner.error)
    }

    // --- Atom/Agent helpers ---

    fn atom_obj(&self, atom: LispValue) -> Result<&AtomObj, RuntimeError> {
        let addr = atom.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "atom",
            value: atom,
        })?;
        self.atom_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "atom",
                value: atom,
            })
    }

    fn atom_by_addr(&self, addr: usize) -> Option<&AtomObj> {
        // Check GC-allocated atoms first (O(1))
        if self.gc_atoms.contains_key(&addr) {
            return Some(unsafe { Self::deref_heap(addr) });
        }
        // Fall back to Vec scan
        for obj in &self.atoms {
            if (&**obj as *const AtomObj) as usize == addr {
                return Some(obj);
            }
        }
        None
    }

    fn agent_obj(&self, agent: LispValue) -> Result<&AgentObj, RuntimeError> {
        let addr = agent.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "agent",
            value: agent,
        })?;
        self.agent_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "agent",
                value: agent,
            })
    }

    fn agent_by_addr(&self, addr: usize) -> Option<&AgentObj> {
        if self.gc_agents.contains_key(&addr) {
            return Some(unsafe { Self::deref_heap(addr) });
        }
        for obj in &self.agents {
            if (&**obj as *const AgentObj) as usize == addr {
                return Some(obj);
            }
        }
        None
    }

    // --- Mutexes ---

    pub fn make_mutex(&mut self, name: String) -> LispValue {
        let mut boxed = Box::new(MutexObj {
            header: HeapHeader {
                kind: HeapKind::Mutex,
            },
            inner: parking_lot::Mutex::new(()),
            name,
        });
        let addr = (&mut *boxed as *mut MutexObj) as usize;
        self.object_index.insert(addr, HeapKind::Mutex);
        self.mutexes.push(boxed);
        LispValue::from_heap_addr(addr)
    }

    pub fn is_mutex(&self, value: LispValue) -> bool {
        value
            .heap_addr()
            .is_some_and(|addr| self.heap_kind(addr) == Some(HeapKind::Mutex))
    }

    pub fn mutex_lock(&self, mutex: LispValue) -> Result<(), RuntimeError> {
        let obj = self.mutex_obj(mutex)?;
        obj.inner.lock();
        Ok(())
    }

    pub fn mutex_unlock(&self, mutex: LispValue) -> Result<(), RuntimeError> {
        let obj = self.mutex_obj(mutex)?;
        // Safety: the mutex must be locked by the current thread.
        // parking_lot::Mutex::unlock is unsafe for this reason.
        unsafe { obj.inner.force_unlock() };
        Ok(())
    }

    // --- Condition Variables ---

    pub fn make_condvar(&mut self, name: String) -> LispValue {
        let mut boxed = Box::new(CondvarObj {
            header: HeapHeader {
                kind: HeapKind::Condvar,
            },
            inner: parking_lot::Condvar::new(),
            name,
        });
        let addr = (&mut *boxed as *mut CondvarObj) as usize;
        self.object_index.insert(addr, HeapKind::Condvar);
        self.condvars.push(boxed);
        LispValue::from_heap_addr(addr)
    }

    pub fn condvar_wait(&self, cv: LispValue, mutex: LispValue) -> Result<(), RuntimeError> {
        let cv_obj = self.condvar_obj(cv)?;
        let mtx_obj = self.mutex_obj(mutex)?;
        // Unlock the mutex, wait, re-lock.  parking_lot::Condvar::wait
        // requires a MutexGuard, which we acquire fresh here.  Callers
        // must hold the mutex before calling condition-wait.
        let mut guard = mtx_obj.inner.lock();
        cv_obj.inner.wait(&mut guard);
        Ok(())
    }

    pub fn condvar_notify(&self, cv: LispValue) -> Result<(), RuntimeError> {
        let obj = self.condvar_obj(cv)?;
        obj.inner.notify_one();
        Ok(())
    }

    pub fn condvar_notify_all(&self, cv: LispValue) -> Result<(), RuntimeError> {
        let obj = self.condvar_obj(cv)?;
        obj.inner.notify_all();
        Ok(())
    }

    // --- Mutex/Condvar helpers ---

    fn mutex_obj(&self, mutex: LispValue) -> Result<&MutexObj, RuntimeError> {
        let addr = mutex.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "mutex",
            value: mutex,
        })?;
        self.mutex_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "mutex",
                value: mutex,
            })
    }

    fn mutex_by_addr(&self, addr: usize) -> Option<&MutexObj> {
        if self.gc_mutexes.contains_key(&addr) {
            return Some(unsafe { Self::deref_heap(addr) });
        }
        for obj in &self.mutexes {
            if (&**obj as *const MutexObj) as usize == addr {
                return Some(obj);
            }
        }
        None
    }

    fn condvar_obj(&self, cv: LispValue) -> Result<&CondvarObj, RuntimeError> {
        let addr = cv.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "condition-variable",
            value: cv,
        })?;
        self.condvar_by_addr(addr)
            .ok_or(RuntimeError::WrongTypeArgument {
                expected: "condition-variable",
                value: cv,
            })
    }

    fn condvar_by_addr(&self, addr: usize) -> Option<&CondvarObj> {
        if self.gc_condvars.contains_key(&addr) {
            return Some(unsafe { Self::deref_heap(addr) });
        }
        for obj in &self.condvars {
            if (&**obj as *const CondvarObj) as usize == addr {
                return Some(obj);
            }
        }
        None
    }

    pub fn set_match_data(&mut self, string: String, groups: Vec<Option<(usize, usize)>>) {
        self.match_data = Some(MatchData { string, groups });
    }

    pub fn clear_match_data(&mut self) {
        self.match_data = None;
    }

    pub fn match_beginning(&self, group: usize) -> LispValue {
        let Some(ref md) = self.match_data else {
            return LispValue::NIL;
        };
        match md.groups.get(group) {
            Some(Some((start, _))) => LispValue::expect_fixnum(*start as i64),
            _ => LispValue::NIL,
        }
    }

    pub fn match_end(&self, group: usize) -> LispValue {
        let Some(ref md) = self.match_data else {
            return LispValue::NIL;
        };
        match md.groups.get(group) {
            Some(Some((_, end))) => LispValue::expect_fixnum(*end as i64),
            _ => LispValue::NIL,
        }
    }

    pub fn match_string(&mut self, group: usize) -> LispValue {
        let Some(ref md) = self.match_data else {
            return LispValue::NIL;
        };
        match md.groups.get(group) {
            Some(Some((start, end))) => {
                let s = md.string[*start..*end].to_string();
                self.string(s)
            }
            _ => LispValue::NIL,
        }
    }

    pub fn replace_match(
        &mut self,
        replacement: LispValue,
        string: Option<LispValue>,
    ) -> Option<LispValue> {
        let rep = match self.string_contents(replacement) {
            Ok(s) => s.to_string(),
            Err(_) => return None,
        };
        let Some(ref md) = self.match_data else {
            return Some(self.string(rep));
        };
        // Build replacement text with backreferences expanded
        let mut repl_text = String::new();
        let mut chars = rep.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('&') => {
                        if let Some(Some((s, e))) = md.groups.first() {
                            repl_text.push_str(&md.string[*s..*e]);
                        }
                    }
                    Some(d @ '1'..='9') => {
                        let idx = (d as u32 - '0' as u32) as usize;
                        if let Some(Some((s, e))) = md.groups.get(idx) {
                            repl_text.push_str(&md.string[*s..*e]);
                        }
                    }
                    Some(c) => {
                        repl_text.push('\\');
                        repl_text.push(c);
                    }
                    None => repl_text.push('\\'),
                }
            } else {
                repl_text.push(ch);
            }
        }
        // If STRING is provided, replace the match in it
        if let Some(s) = string {
            if !s.is_nil() {
                let contents = match self.string_contents(s) {
                    Ok(c) => c.to_string(),
                    Err(_) => return Some(self.string(repl_text)),
                };
                if let Some(Some((start, end))) = md.groups.first() {
                    let mut modified =
                        String::with_capacity(contents.len() - (end - start) + repl_text.len());
                    modified.push_str(&contents[..*start]);
                    modified.push_str(&repl_text);
                    modified.push_str(&contents[*end..]);
                    return Some(self.string(modified));
                }
            }
        }
        Some(self.string(repl_text))
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

    pub fn set_symbol_unbound(&mut self, symbol: LispValue) -> Result<(), RuntimeError> {
        if symbol.is_nil() || symbol.is_true() {
            return Err(RuntimeError::ConstantSymbol {
                name: if symbol.is_nil() { "nil" } else { "t" }.to_string(),
            });
        }
        self.expect_symbol_mut(symbol)?.value = None;
        Ok(())
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

    pub fn add_load_path(&mut self, path: String) {
        if !self.load_path.contains(&path) {
            self.load_path.push(path);
        }
    }

    pub fn resolve_load_file(&self, name: &str) -> Option<String> {
        for dir in &self.load_path {
            let path = format!("{dir}/{name}.el");
            if std::path::Path::new(&path).exists() {
                return Some(path);
            }
            let path = format!("{dir}/{name}.elc");
            if std::path::Path::new(&path).exists() {
                return Some(path);
            }
        }
        let path = format!("{name}.el");
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
        None
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

    pub fn fmakunbound(&mut self, symbol: LispValue) -> Result<LispValue, RuntimeError> {
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
        self.expect_symbol_mut(symbol)?.function = None;
        Ok(symbol)
    }

    pub fn string_contents(&self, string: LispValue) -> Result<&str, RuntimeError> {
        self.expect_string(string)?
            .data
            .as_str()
            .ok_or_else(|| RuntimeError::InvalidStringData("string is not valid UTF-8".to_string()))
    }

    /// Get string contents as an owned String, handling Emacs extended UTF-8.
    /// Raw bytes 0x80..=0x9F are mapped to Unicode U+0080..U+009F.
    pub fn string_contents_emacs(&self, string: LispValue) -> Result<String, RuntimeError> {
        let data = &self.expect_string(string)?.data;
        if let Some(s) = data.as_str() {
            return Ok(s.to_string());
        }
        Ok(data.to_utf8_string())
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
        let ht = self.expect_hash_table(table)?;
        Ok(match ht.test {
            HashTableTest::Equal => ht.equal_entries.len(),
            _ => ht.entries.len(),
        })
    }

    pub fn hash_table_test(&self, table: LispValue) -> Result<HashTableTest, RuntimeError> {
        Ok(self.expect_hash_table(table)?.test)
    }

    pub fn gethash(
        &self,
        key: LispValue,
        table: LispValue,
    ) -> Result<Option<LispValue>, RuntimeError> {
        let table_object = self.expect_hash_table(table)?;
        match table_object.test {
            HashTableTest::Equal => {
                let pos = table_object.equal_entries.iter().position(|entry| self.equal(entry.key, key));
                Ok(pos.map(|i| table_object.equal_entries[i].value))
            }
            _ => Ok(table_object.entries.get(&(key.to_abi_i64() as u64)).copied()),
        }
    }

    pub fn puthash(
        &mut self,
        key: LispValue,
        value: LispValue,
        table: LispValue,
    ) -> Result<LispValue, RuntimeError> {
        // For equal tables, find the entry position before taking the
        // mutable borrow (since equal() needs &self).
        let equal_pos = if self.expect_hash_table(table)?.test == HashTableTest::Equal {
            self.expect_hash_table(table)?
                .equal_entries
                .iter()
                .position(|entry| self.equal(entry.key, key))
        } else {
            None
        };
        let table_object = self.expect_hash_table_mut(table)?;
        match table_object.test {
            HashTableTest::Equal => {
                if let Some(i) = equal_pos {
                    table_object.equal_entries[i].value = value;
                } else {
                    table_object.equal_entries.push(HashEntry { key, value });
                }
            }
            _ => {
                table_object.entries.insert(key.to_abi_i64() as u64, value);
            }
        }
        Ok(value)
    }

    pub fn remhash(&mut self, key: LispValue, table: LispValue) -> Result<LispValue, RuntimeError> {
        let equal_pos = if self.expect_hash_table(table)?.test == HashTableTest::Equal {
            self.expect_hash_table(table)?
                .equal_entries
                .iter()
                .position(|entry| self.equal(entry.key, key))
        } else {
            None
        };
        let table_object = self.expect_hash_table_mut(table)?;
        match table_object.test {
            HashTableTest::Equal => {
                if let Some(i) = equal_pos {
                    table_object.equal_entries.remove(i);
                }
            }
            _ => {
                table_object.entries.remove(&(key.to_abi_i64() as u64));
            }
        }
        Ok(LispValue::NIL)
    }

    pub fn clrhash(&mut self, table: LispValue) -> Result<LispValue, RuntimeError> {
        let ht = self.expect_hash_table_mut(table)?;
        ht.entries.clear();
        ht.equal_entries.clear();
        Ok(table)
    }

    pub fn hash_table_entries(
        &self,
        table: LispValue,
    ) -> Result<Vec<(LispValue, LispValue)>, RuntimeError> {
        let ht = self.expect_hash_table(table)?;
        match ht.test {
            HashTableTest::Equal => Ok(ht.equal_entries.iter().map(|e| (e.key, e.value)).collect()),
            _ => Ok(ht.entries.iter().map(|(k, v)| (LispValue::from_abi_i64(*k as i64), *v)).collect()),
        }
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

    /// Dereference a heap pointer directly (no Vec scan).
    /// Box pointers are stable because there is no GC compaction.
    /// Safety verified by O(1) heap_kind() type check.
    #[inline(always)]
    unsafe fn deref_heap<'a, T>(addr: usize) -> &'a T {
        unsafe { &*(addr as *const T) }
    }

    #[inline(always)]
    unsafe fn deref_heap_mut<'a, T>(addr: usize) -> &'a mut T {
        unsafe { &mut *(addr as *mut T) }
    }

    fn expect_cons(&self, value: LispValue) -> Result<&Cons, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "consp",
            value,
        })?;
        if self.heap_kind(addr) != Some(HeapKind::Cons) {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "consp",
                value,
            });
        }
        Ok(unsafe { Self::deref_heap(addr) })
    }

    fn expect_cons_mut(&mut self, value: LispValue) -> Result<&mut Cons, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "consp",
            value,
        })?;
        if self.heap_kind(addr) != Some(HeapKind::Cons) {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "consp",
                value,
            });
        }
        Ok(unsafe { Self::deref_heap_mut(addr) })
    }

    fn expect_symbol(&self, value: LispValue) -> Result<&Symbol, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "symbolp",
            value,
        })?;
        if self.heap_kind(addr) != Some(HeapKind::Symbol) {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "symbolp",
                value,
            });
        }
        Ok(unsafe { Self::deref_heap(addr) })
    }

    fn expect_symbol_mut(&mut self, value: LispValue) -> Result<&mut Symbol, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "symbolp",
            value,
        })?;
        if self.heap_kind(addr) != Some(HeapKind::Symbol) {
            return Err(RuntimeError::WrongTypeArgument {
                expected: "symbolp",
                value,
            });
        }
        Ok(unsafe { Self::deref_heap_mut(addr) })
    }

    fn expect_string(&self, value: LispValue) -> Result<&LispString, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument { expected: "stringp", value })?;
        if self.heap_kind(addr) != Some(HeapKind::String) { return Err(RuntimeError::WrongTypeArgument { expected: "stringp", value }); }
        Ok(unsafe { Self::deref_heap(addr) })
    }

    fn expect_vector(&self, value: LispValue) -> Result<&VectorObject, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument { expected: "vectorp", value })?;
        if self.heap_kind(addr) != Some(HeapKind::Vector) { return Err(RuntimeError::WrongTypeArgument { expected: "vectorp", value }); }
        Ok(unsafe { Self::deref_heap(addr) })
    }

    fn expect_vector_mut(&mut self, value: LispValue) -> Result<&mut VectorObject, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument { expected: "vectorp", value })?;
        if self.heap_kind(addr) != Some(HeapKind::Vector) { return Err(RuntimeError::WrongTypeArgument { expected: "vectorp", value }); }
        Ok(unsafe { Self::deref_heap_mut(addr) })
    }

    fn expect_hash_table(&self, value: LispValue) -> Result<&HashTableObject, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument { expected: "hash-table-p", value })?;
        if self.heap_kind(addr) != Some(HeapKind::HashTable) { return Err(RuntimeError::WrongTypeArgument { expected: "hash-table-p", value }); }
        Ok(unsafe { Self::deref_heap(addr) })
    }

    fn expect_hash_table_mut(&mut self, value: LispValue) -> Result<&mut HashTableObject, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument { expected: "hash-table-p", value })?;
        if self.heap_kind(addr) != Some(HeapKind::HashTable) { return Err(RuntimeError::WrongTypeArgument { expected: "hash-table-p", value }); }
        Ok(unsafe { Self::deref_heap_mut(addr) })
    }

    fn expect_function(&self, value: LispValue) -> Result<&FunctionObject, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument { expected: "functionp", value })?;
        if self.heap_kind(addr) != Some(HeapKind::Function) { return Err(RuntimeError::WrongTypeArgument { expected: "functionp", value }); }
        Ok(unsafe { Self::deref_heap(addr) })
    }

    fn expect_lexical_cell(&self, value: LispValue) -> Result<&LexicalCell, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument { expected: "lexical-cell", value })?;
        if self.heap_kind(addr) != Some(HeapKind::LexicalCell) { return Err(RuntimeError::WrongTypeArgument { expected: "lexical-cell", value }); }
        Ok(unsafe { Self::deref_heap(addr) })
    }

    fn expect_lexical_cell_mut(&mut self, value: LispValue) -> Result<&mut LexicalCell, RuntimeError> {
        let addr = value.heap_addr().ok_or(RuntimeError::WrongTypeArgument { expected: "lexical-cell", value })?;
        if self.heap_kind(addr) != Some(HeapKind::LexicalCell) { return Err(RuntimeError::WrongTypeArgument { expected: "lexical-cell", value }); }
        Ok(unsafe { Self::deref_heap_mut(addr) })
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

    pub fn string_set_char(
        &mut self,
        string: LispValue,
        index: usize,
        ch: char,
    ) -> Result<(), RuntimeError> {
        let addr = string.heap_addr().ok_or(RuntimeError::WrongTypeArgument {
            expected: "string",
            value: string,
        })?;
        for s in &mut self.strings {
            let s_addr = (&**s as *const LispString) as usize;
            if s_addr == addr {
                let bytes = &mut s.data.data;
                if ch as u32 <= 0x7F {
                    if index < bytes.len() && bytes[index] <= 0x7F {
                        bytes[index] = ch as u8;
                        return Ok(());
                    }
                }
                return Err(RuntimeError::InvalidStringData(
                    "aset: string mutation only supports ASCII replacement".to_string(),
                ));
            }
        }
        Err(RuntimeError::WrongTypeArgument {
            expected: "string",
            value: string,
        })
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

    fn bignum_by_addr(&self, addr: usize) -> Option<&BignumObj> {
        for obj in &self.bignums {
            let obj_addr = (&**obj as *const BignumObj) as usize;
            if obj_addr == addr && obj.header.kind == HeapKind::Bignum {
                return Some(obj);
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
        // Floats: compare by bit pattern for total ordering (NaN != NaN, same as Emacs).
        if let (Some(left), Some(right)) = (
            self.float_by_addr(left_addr),
            self.float_by_addr(right_addr),
        ) {
            return left.value.to_bits() == right.value.to_bits();
        }
        // Bignums: compare values
        if let (Some(left), Some(right)) = (
            self.bignum_by_addr(left_addr),
            self.bignum_by_addr(right_addr),
        ) {
            return left.value == right.value;
        }
        false
    }

    pub fn format_value(&self, value: LispValue) -> String {
        let mut seen = std::collections::HashSet::new();
        self.format_value_cycle_safe(value, 64, &mut seen)
    }

    fn format_value_cycle_safe(
        &self,
        value: LispValue,
        depth: usize,
        seen: &mut std::collections::HashSet<usize>,
    ) -> String {
        if depth == 0 {
            return "#<max-depth>".to_string();
        }
        if !self.is_cons(value) {
            return self.format_atom_value_inner(value, depth, seen);
        }
        if let Some(addr) = value.heap_addr() {
            if seen.contains(&addr) {
                return "#<cycle>".to_string();
            }
            seen.insert(addr);
            let result = self.format_cons_list(value, depth, seen);
            seen.remove(&addr);
            return result;
        }
        self.format_cons_list(value, depth, seen)
    }

    fn format_cons_list(
        &self,
        value: LispValue,
        depth: usize,
        seen: &mut std::collections::HashSet<usize>,
    ) -> String {
        let mut parts = Vec::new();
        let mut current = value;
        loop {
            let Some(addr) = current.heap_addr() else {
                parts.push(".".to_string());
                parts.push(self.format_atom_value_inner(current, depth, seen));
                break;
            };
            let Some(cell) = self.cons_by_addr(addr) else {
                parts.push(".".to_string());
                parts.push(self.format_atom_value_inner(current, depth, seen));
                break;
            };
            parts.push(self.format_value_cycle_safe(cell.car, depth - 1, seen));
            current = cell.cdr;
            if current.is_nil() {
                break;
            }
            if !self.is_cons(current) {
                parts.push(".".to_string());
                parts.push(self.format_value_cycle_safe(current, depth - 1, seen));
                break;
            }
        }
        format!("({})", parts.join(" "))
    }

    fn format_atom_value_inner(
        &self,
        value: LispValue,
        depth: usize,
        seen: &mut std::collections::HashSet<usize>,
    ) -> String {
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
            if seen.contains(&addr) {
                return "#<cycle>".to_string();
            }
            seen.insert(addr);
            let elements = vector
                .elements
                .iter()
                .map(|v| self.format_value_cycle_safe(*v, depth - 1, seen))
                .collect::<Vec<_>>();
            seen.remove(&addr);
            return format!("[{}]", elements.join(" "));
        }
        if let Some(addr) = value.heap_addr()
            && let Some(float) = self.float_by_addr(addr)
        {
            return format!("{}", float.value);
        }
        if let Some(addr) = value.heap_addr()
            && let Some(bignum) = self.bignum_by_addr(addr)
        {
            return format!("{}", bignum.value);
        }
        if let Some(addr) = value.heap_addr() {
            return format!("#<object 0x{addr:x}>");
        }
        if let Some(n) = value.as_fixnum() {
            return n.to_string();
        }
        if value.is_nil() {
            return "nil".to_string();
        }
        if value.is_true() {
            return "t".to_string();
        }
        if let Some(c) = value.as_char() {
            return format!("?{}", c.escape_debug());
        }
        "#<unknown>".to_string()
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
            return format!("#<hash-table count {}>", table.entries.len() + table.equal_entries.len());
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
        if let Some(addr) = value.heap_addr()
            && let Some(obj) = self.bignum_by_addr(addr)
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

/// Count characters in Emacs extended UTF-8 byte sequence.
/// Bytes 0x80..=0x9F count as single eight-bit characters.
/// Bytes 0xA0..=0xBF are continuation bytes (only after lead bytes).
/// Lead bytes follow standard UTF-8: C0-DF (1 cont), E0-EF (2 cont), F0-F7 (3 cont).
fn count_emacs_utf8_chars(bytes: &[u8]) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            0x00..=0x7F => {
                count += 1;
                i += 1;
            }
            0x80..=0x9F => {
                // Raw eight-bit character (Emacs extended UTF-8)
                count += 1;
                i += 1;
            }
            0xC0..=0xDF => {
                count += 1;
                i += 2; // skip continuation byte
            }
            0xE0..=0xEF => {
                count += 1;
                i += 3; // skip 2 continuation bytes
            }
            0xF0..=0xF7 => {
                count += 1;
                i += 4; // skip 3 continuation bytes
            }
            _ => {
                // Continuation byte (0xA0-0xBF) or invalid (0xF8-0xFF)
                // appearing alone — count as single character
                count += 1;
                i += 1;
            }
        }
    }
    count
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
        let nchars = count_emacs_utf8_chars(value.as_bytes());
        let nbytes = value.len();
        if nbytes == nchars {
            Self::make_unibyte(value.as_bytes().to_vec())
        } else {
            Self::make_multibyte(value.as_bytes().to_vec(), nchars)
        }
    }

    /// Create from raw Emacs extended UTF-8 bytes.
    /// Bytes in 0x80..=0x9F are counted as single eight-bit characters.
    pub fn from_emacs_utf8(bytes: Vec<u8>) -> Self {
        let nchars = count_emacs_utf8_chars(&bytes);
        let nbytes = bytes.len();
        if nbytes == nchars {
            Self::make_unibyte(bytes)
        } else {
            Self::make_multibyte(bytes, nchars)
        }
    }

    /// Create a multibyte string from standard UTF-8 bytes.
    /// Uses standard Rust char counting since bytes are valid UTF-8.
    pub fn from_utf8(value: &str) -> Self {
        let bytes = value.as_bytes().to_vec();
        let nchars = value.chars().count();
        if bytes.len() == nchars {
            Self::make_unibyte(bytes)
        } else {
            Self::make_multibyte(bytes, nchars)
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

    /// Convert Emacs extended UTF-8 bytes to a Rust String.
    /// Raw bytes 0x80..=0x9F are mapped to Unicode characters
    /// U+0080..U+009F (Latin-1 Supplement).
    pub fn to_utf8_string(&self) -> String {
        let bytes = self.sdata();
        let mut result = String::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            match b {
                0x00..=0x7F => {
                    result.push(b as char);
                    i += 1;
                }
                0x80..=0x9F => {
                    // Eight-bit character: map to U+0080..U+009F
                    result.push(char::from_u32(0x80 + (b - 0x80) as u32).unwrap_or('\u{FFFD}'));
                    i += 1;
                }
                0xC0..=0xDF if i + 1 < bytes.len() => {
                    // 2-byte UTF-8 sequence
                    if let Ok(s) = std::str::from_utf8(&bytes[i..i + 2]) {
                        result.push_str(s);
                    }
                    i += 2;
                }
                0xE0..=0xEF if i + 2 < bytes.len() => {
                    if let Ok(s) = std::str::from_utf8(&bytes[i..i + 3]) {
                        result.push_str(s);
                    }
                    i += 3;
                }
                0xF0..=0xF7 if i + 3 < bytes.len() => {
                    if let Ok(s) = std::str::from_utf8(&bytes[i..i + 4]) {
                        result.push_str(s);
                    }
                    i += 4;
                }
                _ => {
                    // Continuation byte or invalid — replace
                    result.push('\u{FFFD}');
                    i += 1;
                }
            }
        }
        result
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
    Bignum = 9,
    Atom = 10,
    Agent = 11,
    Mutex = 12,
    Condvar = 13,
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
    /// O(1) storage for `eq`/`eql` tables (identity equality).
    /// Key is the raw u64 tag of the LispValue.
    entries: HashMap<u64, LispValue>,
    /// Fallback for `equal` tables (structural equality).
    equal_entries: Vec<HashEntry>,
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

#[repr(C, align(8))]
struct BignumObj {
    header: HeapHeader,
    value: rug::Integer,
}

/// Clojure-style atom: a lock-free CAS-based mutable cell.
/// The value is an AtomicU64 storing a tagged LispValue.
#[repr(C, align(8))]
struct AtomObj {
    header: HeapHeader,
    value: std::sync::atomic::AtomicU64,
}

// SAFETY: AtomObj has no GC-managed edges (the value is AtomicU64).
unsafe impl neovm_gc::Trace for AtomObj {
    fn trace(&self, _tracer: &mut dyn neovm_gc::Tracer) {}
    fn relocate(&self, _relocator: &mut dyn neovm_gc::Relocator) {}
}

// SAFETY: AgentObj has no GC-managed edges (inner state is Arc<Mutex<>>).
unsafe impl neovm_gc::Trace for AgentObj {
    fn trace(&self, _tracer: &mut dyn neovm_gc::Tracer) {}
    fn relocate(&self, _relocator: &mut dyn neovm_gc::Relocator) {}
}

// SAFETY: MutexObj has no GC-managed edges (inner is parking_lot::Mutex<()>).
unsafe impl neovm_gc::Trace for MutexObj {
    fn trace(&self, _tracer: &mut dyn neovm_gc::Tracer) {}
    fn relocate(&self, _relocator: &mut dyn neovm_gc::Relocator) {}
}

// SAFETY: CondvarObj has no GC-managed edges (inner is parking_lot::Condvar).
unsafe impl neovm_gc::Trace for CondvarObj {
    fn trace(&self, _tracer: &mut dyn neovm_gc::Tracer) {}
    fn relocate(&self, _relocator: &mut dyn neovm_gc::Relocator) {}
}

/// Clojure-style agent: an asynchronous, serialized mutable cell.
/// Actions are queued via `send` and executed in order by the agent
/// thread pool.  The Mutex protects both value and action queue.
struct AgentObj {
    header: HeapHeader,
    inner: std::sync::Arc<std::sync::Mutex<AgentInner>>,
}

struct AgentInner {
    value: LispValue,
    queue: Vec<AgentAction>,
    error: Option<LispValue>,
}

#[derive(Clone)]
pub(crate) struct AgentAction {
    pub(crate) func: LispValue,
    pub(crate) args: Vec<LispValue>,
    pub(crate) via_pool: bool,
}

/// A mutex for Elisp thread synchronization.
/// Uses parking_lot::Mutex which supports manual lock/unlock and
/// is already a dependency via neovm-gc.
struct MutexObj {
    header: HeapHeader,
    inner: parking_lot::Mutex<()>,
    name: String,
}

/// A condition variable paired with a mutex.
struct CondvarObj {
    header: HeapHeader,
    inner: parking_lot::Condvar,
    name: String,
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
