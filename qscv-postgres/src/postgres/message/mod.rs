//! Postgres Protocol
//!
//! startup phase at [`Startup`]
//!
//! for supported backend message, see [`BackendMessage`]
//!
//! [`Startup`]: startup::Startup

// Frontend Messages
pub mod startup;

// Backend Messages
pub mod backend;
pub mod authentication;

pub use backend::BackendMessage;

