//! Postgres Protocol
//!
//! startup phase at [`Startup`]
//!
//! for supported backend message, see [`BackendMessage`]
//!
//! [`Startup`]: startup::Startup

pub mod frontend;
pub mod backend;
pub mod authentication;

mod ext;

pub use backend::BackendMessage;
pub use frontend::FrontendMessage;

