//! Typed errors mapped to CLI exit codes (mirrors the Python `cairnkit.errors`).
//!
//! Exit-code convention: 0 ok · 2 usage/precondition · 3 admission-gate refusal · 4 STATE corrupt.

use std::fmt;

#[derive(Debug)]
pub enum CairnError {
    /// Bad arguments, illegal enum, missing/invalid config, illegal state op. -> 2
    Usage(String),
    /// Admission gate refused the transition. -> 3
    Gate { message: String, missing: Vec<String> },
    /// STATE.yaml unreadable / missing required fields / unknown enum. -> 4
    StateCorrupt(String),
}

impl CairnError {
    pub fn code(&self) -> i32 {
        match self {
            CairnError::Usage(_) => 2,
            CairnError::Gate { .. } => 3,
            CairnError::StateCorrupt(_) => 4,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            CairnError::Usage(m) | CairnError::StateCorrupt(m) => m,
            CairnError::Gate { message, .. } => message,
        }
    }
}

impl fmt::Display for CairnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for CairnError {}

/// Convenience constructors.
pub fn usage<S: Into<String>>(m: S) -> CairnError {
    CairnError::Usage(m.into())
}
pub fn corrupt<S: Into<String>>(m: S) -> CairnError {
    CairnError::StateCorrupt(m.into())
}

pub type Result<T> = std::result::Result<T, CairnError>;

/// Map low-level IO errors to a usage-class error (matches Python's `except OSError -> code 2`).
impl From<std::io::Error> for CairnError {
    fn from(e: std::io::Error) -> Self {
        CairnError::Usage(e.to_string())
    }
}
