//! What a rule is given to run against.
//!
//! # Why units are shared through a context
//!
//! Seven rules ask for a file's function-like units, and measuring them — detection plus
//! cyclomatic, cognitive, Halstead, and maintainability figures for each — is a scan over
//! every line. When each rule built its own [`UnitSet`], that scan ran seven times per file
//! and was the dominant cost of an analysis.
//!
//! The context holds one set and hands it to whichever rule asks. It is a value owned by
//! the caller for exactly one file, so the set can never outlive the file it describes or
//! be read against a different one; an earlier attempt to share units through a cache keyed
//! on the file's address produced exactly that confusion. Rules that never look at units
//! never pay for them, because the set is built on first use.

use std::cell::OnceCell;

use monostyle_lexer::LexedFile;

use crate::config::RulesConfig;
use crate::unit_measures::UnitSet;

/// One file, its rule configuration, and the unit measurements rules share.
#[derive(Debug)]
pub struct RuleContext<'file> {
	file: &'file LexedFile,
	config: &'file RulesConfig,
	units: OnceCell<UnitSet>,
}

impl<'file> RuleContext<'file> {
	/// Pairs a lexed file with the configuration its rules should read.
	#[must_use]
	pub fn new(file: &'file LexedFile, config: &'file RulesConfig) -> Self {
		Self {
			file,
			config,
			units: OnceCell::new(),
		}
	}

	/// The file under analysis.
	#[must_use]
	pub fn file(&self) -> &'file LexedFile {
		self.file
	}

	/// The thresholds the rules measure against.
	#[must_use]
	pub fn config(&self) -> &'file RulesConfig {
		self.config
	}

	/// Every function-like unit in the file, measured once and shared by every caller.
	#[must_use]
	pub fn units(&self) -> &UnitSet {
		self.units.get_or_init(|| UnitSet::new(self.file))
	}
}
