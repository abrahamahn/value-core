//! Domain-neutral primitives for exact, auditable movement of value.
//!
//! This crate performs no storage, network, clock, or process I/O. Applications own transaction
//! boundaries and persistence, and can apply the deterministic plans returned here atomically.

pub mod account;
pub mod amount;
pub mod canonical;
pub mod conversion;
pub mod hold;
pub mod idempotency;
pub mod reconciliation;
pub mod statement;
pub mod time;
pub mod transaction;

use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueError {
    message: String,
}

impl ValueError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ValueError {}

pub type ValueResult<T> = Result<T, ValueError>;

pub(crate) fn required(value: &str, message: &str) -> ValueResult<()> {
    if value.trim().is_empty() {
        Err(ValueError::new(message))
    } else {
        Ok(())
    }
}

pub(crate) fn nonempty(value: &str, message: &str) -> ValueResult<()> {
    if value.is_empty() {
        Err(ValueError::new(message))
    } else {
        Ok(())
    }
}

pub(crate) fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
