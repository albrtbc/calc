use std::fmt;

#[derive(Debug, Clone)]
pub struct CalcError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl CalcError {
    pub fn new(message: impl Into<String>, line: usize, col: usize) -> Self {
        Self { message: message.into(), line, col }
    }

    pub fn eval(message: impl Into<String>) -> Self {
        Self { message: message.into(), line: 0, col: 0 }
    }
}

impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line > 0 {
            write!(f, "[{}:{}] {}", self.line, self.col, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for CalcError {}

pub type Result<T> = std::result::Result<T, CalcError>;
