//! Domain model, storage traits, and configuration for oxydraw.
//!
//! This crate contains no storage-backend I/O: it defines *what* a store must do (the
//! traits in [`store`]) and the data it operates on (the types in [`model`]), but no
//! backend. The one place it touches the outside world is [`config`], which reads the
//! process environment and an optional `.env` file at startup.

pub mod config;
pub mod model;
pub mod store;
pub mod sync;
