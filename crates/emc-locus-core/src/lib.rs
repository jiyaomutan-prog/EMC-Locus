//! Core domain primitives for EMC Locus.
//!
//! This crate stays independent from UI, database, and hardware-driver details.
//! It captures business rules that must remain stable across adapters.

pub mod audit;
pub mod error;
pub mod identifiers;
pub mod instrument;
pub mod metrology;
pub mod project;
pub mod quality;
pub mod repositories;
pub mod signal;
pub mod traceability;

pub use audit::*;
pub use error::*;
pub use identifiers::*;
pub use instrument::*;
pub use metrology::*;
pub use project::*;
pub use quality::*;
pub use repositories::*;
pub use signal::*;
pub use traceability::*;

#[cfg(test)]
mod tests;
