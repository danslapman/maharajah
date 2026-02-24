/// Adds two integers together.
///
/// # Examples
/// ```
/// assert_eq!(add(2, 3), 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Computes the factorial of `n` recursively.
/// Returns 1 for n <= 1.
pub fn factorial(n: u64) -> u64 {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

/// Returns the larger of two values using a generic bound.
pub fn max_val<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

pub fn square(x: f64) -> f64 {
    x * x
}

/// A simple stack backed by a Vec.
pub struct Stack<T> {
    data: Vec<T>,
}

impl<T> Stack<T> {
    /// Creates an empty stack.
    pub fn new() -> Self {
        Stack { data: Vec::new() }
    }

    /// Pushes a value onto the top of the stack.
    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    /// Pops a value from the top of the stack, or None if empty.
    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    /// Returns true if the stack contains no elements.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}
