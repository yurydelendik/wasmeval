use std::pin::Pin;
use std::mem;
use std::slice;
use crate::values::Val;

const DEFAULT_ARENA_SIZE: usize = 1000;

struct EvalStackArena {
    buffer: Pin<Box<[Val]>>,
    len: usize,
}

impl EvalStackArena {
    pub fn new() -> Self {
        EvalStackArena {
            buffer: Pin::new(Box::new([Default::default(); DEFAULT_ARENA_SIZE])),
            len: 0,
        }
    }
    pub fn with_at_least_capacity(capacity: usize) -> Self {
        let buffer = vec![Default::default(); capacity.max(DEFAULT_ARENA_SIZE)].into_boxed_slice();
        EvalStackArena {
            buffer: Pin::new(buffer),
            len: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn last(&self) -> &Val {
        assert!(self.len > 0);
        unsafe {
            self.buffer.get_unchecked(self.len - 1)
        }
    }
    pub fn last_mut(&mut self) -> &mut Val {
        assert!(self.len > 0);
        unsafe {
            self.buffer.get_unchecked_mut(self.len - 1)
        }
    }
    pub fn pop(&mut self) -> Option<Val> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe {
                Some(mem::replace(
                    self.buffer.get_unchecked_mut(self.len),
                    Default::default()
                ))
            }
        }
    }
    pub fn push(&mut self, val: Val) -> Result<(), Val> {
        if self.len == self.buffer.len() {
            return Err(val);
        }
        self.buffer[self.len] = val;
        self.len += 1;
        assert!(self.len <= self.buffer.len());
        Ok(())
    }
    pub fn grow_with_default(&mut self, delta: usize) -> Result<(), usize> {
        if self.len + delta >= self.buffer.len() {
            unsafe {
                for i in self.len..self.buffer.len() {
                    *self.buffer.get_unchecked_mut(i) = Default::default();
                }
            }
            let tail = self.buffer.len() - self.len;
            self.len = self.buffer.len();
            Err(tail)
        } else {
            unsafe {
                for i in self.len..self.len + delta {
                    *self.buffer.get_unchecked_mut(i) = Default::default();
                }
            }
            self.len += delta;
            assert!(self.len <= self.buffer.len());
            Ok(())
        }
    }
    pub fn truncate(&mut self, len: usize) {
        debug_assert!(len <= self.len);
        unsafe {
            for i in len..self.len {
                *self.buffer.get_unchecked_mut(i) = Default::default();
            }
        }
        self.len = len;
    }
    pub fn remove_items(&mut self, index: usize, len: usize) {
        if len == 0 {
            return;
        }
        if index + len < self.len {
            self.buffer[index+len..self.len].reverse();
            self.buffer[index..self.len].reverse();
        }
        self.truncate(self.len - len);
    }
    pub fn clear(&mut self) {
        self.truncate(0);
    }
    pub fn get_mut_ptr(&mut self, index: usize) -> *mut Val {
        unsafe { self.buffer.get_unchecked_mut(index) }
    }
}

pub struct EvalStack {
    arenas: Vec<EvalStackArena>,
    last_starts: usize,
}

impl EvalStack {
    #[allow(dead_code)]
    pub fn new() -> Self {
        EvalStack {
            arenas: vec![EvalStackArena::new()],
            last_starts: 0,
        }
    }
    fn last_arena(&self) -> &EvalStackArena {
        self.arenas.last().unwrap()
    }
    fn last_arena_mut(&mut self) -> &mut EvalStackArena {
        self.arenas.last_mut().unwrap()
    }
    fn pop_arena(&mut self) {
        drop(self.arenas.pop());
        self.last_starts -= self.last_arena().len();
    }
    fn push_arena(&mut self, arena: EvalStackArena) {
        self.last_starts += self.last_arena().len();
        self.arenas.push(arena);
    }
    pub fn pointer(&self) -> usize {
        self.last_starts + self.last_arena().len()
    }
    pub fn last(&self) -> &Val {
        self.last_arena().last()
    }
    pub fn last_mut(&mut self) -> &mut Val {
        self.last_arena_mut().last_mut()
    }
    pub fn pop(&mut self) -> Val {
        if let Some(val) = self.last_arena_mut().pop() {
            return val;
        }
        self.pop_arena();
        self.last_arena_mut().pop().unwrap()
    }
    pub fn push(&mut self, val: Val) {
        if let Err(val) = self.last_arena_mut().push(val) {
            let mut arena = EvalStackArena::new();
            let _success = arena.push(val);
            assert!(_success.is_ok());
            self.push_arena(arena);
        }
    }
    pub fn truncate(&mut self, at: usize) {
        while self.last_starts > at {
            self.pop_arena();
        }
        let tail = at - self.last_starts;
        self.last_arena_mut().truncate(tail);
    }
    pub fn grow_with_default(&mut self, delta: usize) {
        if let Err(tail) = self.last_arena_mut().grow_with_default(delta) {
            let mut arena = EvalStackArena::with_at_least_capacity(tail);
            let _result = arena.grow_with_default(tail);
            assert!(_result.is_ok());
        }
    }
    pub fn remove_items(&mut self, index: usize, mut len: usize) {
        if len == 0 {
            return;
        }
        let mut last = self.last_starts;
        let mut i = self.arenas.len() - 1;
        while last > index {
            last -= self.arenas[i].len();
            i -= 1;
        }
        if last < index && index + len > last + self.arenas[i].len() {
            assert!(i + 1 < self.arenas.len());
            let removed = self.arenas[i].len() - (index - last);
            self.arenas[i].truncate(index - last);
            assert!(self.arenas[i].len() > 0);
            len -= removed;
            self.last_starts -= removed;
            i += 1;
            assert!(last + self.arenas[i].len() == index);
            last = index;
        }
        while len > self.arenas[i].len() {
            assert!(last == index);
            assert!(i + 1 < self.arenas.len());
            let removed = self.arenas[i].len();
            self.arenas.remove(i);
            len -= removed;
            self.last_starts -= removed;
        }
        assert!(last <= index);
        self.arenas[i].remove_items(index - last, len);
        if i + 1 < self.arenas.len() {
            self.last_starts -= len;
        }
    }
    fn ensure_continuous(&mut self, index: usize, _len: usize) -> *mut Val {
        if self.last_starts > index {
            panic!("unsupported");
        }
        let arena_index = index - self.last_starts;
        self.last_arena_mut().get_mut_ptr(arena_index)
    }
    pub fn tail(&mut self, len: usize) -> &[Val] {
        let ptr = self.ensure_continuous(self.pointer() - len, len);
        unsafe { &slice::from_raw_parts(ptr, len) }
    }
    pub fn item_ptr(&mut self, index: usize) -> *const Val {
        let ptr = self.ensure_continuous(index, 1);
        ptr
    }
    pub fn item_mut_ptr(&mut self, index: usize) -> *mut Val {
        let ptr = self.ensure_continuous(index, 1);
        ptr
    }
    pub fn clear(&mut self) {
        self.arenas.clear();
        self.arenas.push(EvalStackArena::new());
        self.last_starts = 0;
    }
}
