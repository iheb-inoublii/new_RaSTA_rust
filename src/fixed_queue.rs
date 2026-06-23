#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedQueueError {
    Full,
}

pub struct FixedQueue<T: Copy, const N: usize> {
    entries: [Option<T>; N],
    head: usize,
    tail: usize,
    len: usize,
}

impl<T: Copy, const N: usize> FixedQueue<T, N> {
    pub fn new() -> Self {
        Self {
            entries: [None; N],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), FixedQueueError> {
        if self.len == N || N == 0 {
            return Err(FixedQueueError::Full);
        }
        self.entries[self.tail] = Some(value);
        self.tail = (self.tail + 1) % N;
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 || N == 0 {
            return None;
        }
        let value = self.entries[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        value
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T: Copy, const N: usize> Default for FixedQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
