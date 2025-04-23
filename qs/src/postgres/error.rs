//! Protocol error
use std::{fmt, str::Utf8Error, string::FromUtf8Error};

use super::BackendMessage;

/// An error when translating buffer from postgres
pub enum ProtocolError {
    Utf8Error(Utf8Error),
    Unexpected {
        expect: Option<u8>,
        found: u8,
        phase: Option<&'static str>,
    },
    UnknownAuth {
        auth: u32,
    },
}

impl BackendMessage {
    pub fn unexpected(self, phase: &'static str) -> ProtocolError {
        ProtocolError::unexpected_phase(self.msgtype(), phase)
    }
}

impl std::error::Error for ProtocolError { }

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Utf8Error(utf) => write!(f, "Postgres returns non utf8: {utf}"),
            Self::Unexpected { expect, found, phase } => {
                let found = BackendMessage::message_name(found);
                match expect {
                    Some(m) => {
                        write!(
                            f,
                            "Expected message `{}` found `{found}`",
                            BackendMessage::message_name(m),
                        )?
                    },
                    None => write!(f, "Unexpected message `{found}`")?,
                }
                if let Some(phase) = phase {
                    write!(f, " in `{phase}`")?
                }
                Ok(())
            },
            Self::UnknownAuth { auth: _ } => todo!(),
        }
    }
}

impl fmt::Debug for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
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

macro_rules! from {
    ($ty:ty: $pat:pat => $body:expr) => {
        impl From<$ty> for ProtocolError {
            fn from($pat: $ty) -> Self {
                $body
            }
        }
    };
}

from!(Utf8Error: value => Self::Utf8Error(value));
from!(FromUtf8Error: value => Self::Utf8Error(value.utf8_error()));

