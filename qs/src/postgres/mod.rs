//! Postgres Frontend and Backend Protocol
//!
//! Docs here mostly quoted from the official postgres documentation.
//!
//! ## Messaging Overview
//!
//! All communication is through a stream of messages. The first byte of a message identifies the message type,
//! and the next four bytes specify the length of the rest of the message (this length count includes itself,
//! but not the message-type byte). The remaining contents of the message are determined by the message type.
//!
//! ```text
//! ┏━━━━┳━━━━━━━━━━━━━━━━━━━┳━━━━━━┓
//! ┃ Ty ┃       Length      ┃ Body ┃
//! ┣━━━━╋━━━━━━━━━━━━━━━━━━━╋━━━━━━┫
//! ┃ u8 ┃        u32        ┃ [u8] ┃
//! ┣━━━━╋━━━━━━━━━━━━━━━━━━━╋━━━━━━┫
//! ┃ 43 ┃ 00 | 00 | 00 | 32 ┃  ..  ┃
//! ┗━━━━┻━━━━━━━━━━━━━━━━━━━┻━━━━━━┛
//! ```
//!
//! For historical reasons, the very first message sent by the client (the startup message)
//! has no initial message-type byte.
//!
//! ## [`Format`][PgFormat] and Format Codes
//!
//! Data of a particular data type might be transmitted in any of several different formats.
//! As of PostgreSQL 7.4 the only supported formats are “text” and “binary”. Text has format
//! code zero, and Binary has format code one.
//!
//! Clients can specify a format code for each transmitted parameter value and for each column of a query result.
//!
//! See [`PgFormat`] for details.
//!
//! <https://www.postgresql.org/docs/17/protocol-overview.html>

mod pg_type;
mod pg_format;

pub mod frontend;
pub mod backend;

mod error;

pub use pg_type::{Oid, PgType};
pub use pg_format::PgFormat;

pub use frontend::FrontendProtocol;
pub use backend::{BackendMessage, BackendProtocol, ErrorResponse, NoticeResponse};
pub use error::ProtocolError;

