#![forbid(rust_2018_idioms)]
#![warn(missing_debug_implementations, missing_docs, unreachable_pub)]
#![deny(unsafe_code, nonstandard_style)]

//! This library provides all the necessary methods and shared state for the craftql binary.
//! Not meant to be used on its own! Primarily made for integration testing.

/// Main onfiguration.
pub mod config;
/// Trait providing extension methods for graphql_parser::schema.
pub mod extend_types;
/// Global state.
pub mod state;
/// Utilities consumed by the binary.
pub mod utils;
