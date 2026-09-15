//! The monostyle analysis library.
//!
//! The binary is a thin shell over this crate, so the pipeline — lex, measure, score, report —
//! can be exercised directly from tests and reused by other tools.

pub mod aggregate;
pub mod analysis;
pub mod cli;
pub mod config;
pub mod package;
pub mod report;
pub mod style;
