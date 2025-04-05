//! Postgres Frontend and Backend Protocol
//!
//! docs here mostly quoted from the official postgres documentation
//!
//! <https://www.postgresql.org/docs/17/protocol-overview.html>
//!
//! # Messaging Overview
//!
//! All communication is through a stream of messages. The first byte of a message identifies the message type,
//! and the next four bytes specify the length of the rest of the message (this length count includes itself,
//! but not the message-type byte). The remaining contents of the message are determined by the message type.
//!
//! ```text
//! | u8 |        i32        | body
//! |----|-------------------|-----
//! | 43 | 00 | 00 | 00 | 32 |  ..
//!
//! Message Type -> length -> body
//! ```
//!
//! For historical reasons, the very first message sent by the client (the startup message)
//! has no initial message-type byte.
//!
//! # Extended Query Overview
//!
//! In the extended-query protocol, overall execution cycle consists of a:
//!
//! - Parse step, which creates a prepared statement from a textual query string
//! - Bind step, which creates a portal given a prepared statement and values for any needed parameters;
//! - Execute step, that runs a portal's query.
//!
//! ## Prepared Statement
//!
//! A prepared statement represents the result of parsing and semantic analysis of a textual query string.
//! A prepared statement is not in itself ready to execute, because it might lack specific values for parameters.
//!
//! ## Portal
//!
//! A portal represents a ready-to-execute or already-partially-executed statement,
//! with any missing parameter values filled in.
//!
//! The backend can keep track of multiple prepared statements and portals (but note that these exist only within
//! a session, and are never shared across sessions). Existing prepared statements and portals are referenced by names
//! assigned when they were created.
//!
//! # Formats and Format Codes
//!
//! Data of a particular data type might be transmitted in any of several different formats.
//! As of PostgreSQL 7.4 the only supported formats are “text” and “binary”.
//!
//! | format | format code |
//! |--------|-------------|
//! |  text  |      0      |
//! | binary |      1      |
//!
//! Clients can specify a format code for each transmitted parameter value and for each column of a query result.
//!
//! The text representation of values is whatever strings are produced and accepted by the input/output conversion
//! functions for the *particular* data type. In the transmitted representation, there is no trailing null character;
//! the frontend must add one to received values if it wants to process them as C strings.
//! (The text format does not allow embedded nulls, by the way.)
//!
//! Binary representations for *integers* use network byte order (most significant byte first).
//! For other data types consult the documentation or source code to learn about the binary representation.
//! Keep in mind that binary representations for complex data types might change across server versions;
//! the text format is usually the more portable choice.

pub mod frontend;
pub mod backend;
pub mod error;

mod ext;

pub use frontend::FrontendProtocol;
pub use backend::{BackendProtocol, BackendMessage};

