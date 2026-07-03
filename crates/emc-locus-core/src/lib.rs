//! Core domain primitives for EMC Locus.
//!
//! This crate stays independent from UI, database, and hardware-driver details.
//! It captures business rules that must remain stable across adapters.

pub mod application_services;
pub mod audit;
pub mod contracts;
pub mod datasets;
pub mod equipment;
pub mod error;
pub mod execution;
pub mod identifiers;
pub mod instrument;
pub mod instrument_runtime;
pub mod measurement;
pub mod metrology;
pub mod project;
pub mod quality;
pub mod reporting;
pub mod repositories;
pub mod signal;
pub mod test_definitions;
pub mod traceability;
pub mod updates;

pub use application_services::*;
pub use audit::*;
pub use contracts::*;
pub use datasets::*;
pub use equipment::*;
pub use error::*;
pub use execution::*;
pub use identifiers::*;
pub use instrument::*;
pub use instrument_runtime::*;
pub use measurement::*;
pub use metrology::*;
pub use project::*;
pub use quality::*;
pub use reporting::*;
pub use repositories::*;
pub use signal::*;
pub use traceability::*;
pub use updates::*;

#[cfg(test)]
mod tests;
