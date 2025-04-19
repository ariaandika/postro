//! Protocol error
mod database;

pub use database::DatabaseError;

use super::BackendMessage;

/// An error when translating buffer from postgres
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("TODO")]
    Unexpected {
        expect: Option<u8>,
        found: u8,
        phase: Option<&'static str>,
    },
    #[error("TODO")]
    UnknownAuth {
        auth: u32,
    },
}

impl ProtocolError {
    pub(crate) fn unknown(found: u8) -> ProtocolError {
        Self::Unexpected {
            expect: None,
            found,
            phase: None,
        }
    }

    pub(crate) fn unexpected(expect: u8, found: u8) -> ProtocolError {
        Self::Unexpected {
            expect: Some(expect),
            found,
            phase: None,
        }
    }

    pub(crate) fn unexpected_phase(found: u8, phase: &'static str) -> ProtocolError {
        Self::Unexpected {
            expect: None,
            found,
            phase: Some(phase),
        }
    }

    pub(crate) fn unknown_auth(auth: u32) -> ProtocolError {
        Self::UnknownAuth { auth }
    }
}

