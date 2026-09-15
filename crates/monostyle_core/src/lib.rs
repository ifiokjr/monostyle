//! Core domain types for monostyle.
//!
//! This crate defines the vocabulary the rest of the tool speaks: spans, findings,
//! language profiles, the category/severity taxonomy, and the two headline scores.
//! It deliberately holds no parsing or analysis logic so that every other crate can
//! depend on these types without depending on each other.

pub mod category;
pub mod finding;
pub mod language;
pub mod score;
pub mod severity;
pub mod span;

pub use category::Category;
pub use finding::Finding;
pub use finding::FindingBuilder;
pub use language::Language;
pub use score::Score;
pub use score::ScoringConfig;
pub use severity::Severity;
pub use span::Span;
