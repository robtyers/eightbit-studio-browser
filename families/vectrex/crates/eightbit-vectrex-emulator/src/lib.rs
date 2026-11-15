//! Original Vectrex digital development machine and guest executive.
//! All cartridge and executive instructions execute on the shared original CPU.
#![allow(missing_docs)]
pub mod cartridge;
pub mod firmware;
pub mod machine;
pub mod session;
use serde::Serialize;
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
pub fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error {
        code,
        message: message.into(),
    }
}
#[must_use]
pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}
