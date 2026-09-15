//! Core engine for Locus — iOS/macOS localization (.strings) management.
//!
//! Shared between the CLI and the future macOS app.

// Stubs use todo!() — parameters are intentionally unused until implemented.
#![allow(unused_variables)]

pub mod edit;
pub mod missing;
pub mod model;
pub mod parser;
pub mod project;
pub mod save;
pub mod search;
pub mod xcstrings;

#[cfg(test)]
pub mod test_helpers;
