//! Complexity and readability metrics.
//!
//! This crate turns a [`LexedFile`](monostyle_lexer::LexedFile) into numbers. Two kinds
//! of measurement live here:
//!
//! - **Complexity**: [`cyclomatic`] counts independent paths, and [`cognitive`] weights
//!   those paths by how hard they are to hold in your head.
//! - **Units**: [`unit`] finds the function-like regions those metrics are attributed to,
//!   so scores can be reported per function rather than only per file.

pub mod cognitive;
pub mod cyclomatic;
pub mod halstead;
pub mod structural;
pub mod unit;

pub use cognitive::CognitiveComplexity;
pub use cognitive::cognitive_complexity;
pub use cognitive::cognitive_complexity_of_lines;
pub use cyclomatic::CyclomaticComplexity;
pub use cyclomatic::Risk;
pub use cyclomatic::complexity_of_lines;
pub use cyclomatic::cyclomatic_complexity;
pub use halstead::Halstead;
pub use halstead::MaintainabilityIndex;
pub use halstead::halstead;
pub use halstead::halstead_of_lines;
pub use halstead::maintainability_index;
pub use structural::ExitCount;
pub use structural::NPath;
pub use structural::NestingProfile;
pub use structural::exit_count;
pub use structural::exit_count_of_lines;
pub use structural::nesting_profile;
pub use structural::nesting_profile_of_lines;
pub use structural::npath;
pub use structural::npath_of_lines;
pub use unit::CodeUnit;
pub use unit::find_units;
