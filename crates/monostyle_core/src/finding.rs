//! Findings and their explanations.

use serde::Serialize;

use crate::Category;
use crate::Fix;
use crate::Severity;
use crate::Span;

/// A single rule violation, with the explanation needed to act on it.
///
/// Every finding must be able to answer "why did I lose points here?" without the
/// reader consulting external documentation, so [`Finding::message`] and
/// [`Finding::suggestion`] are required rather than optional.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Finding {
	/// Stable rule identifier, such as `readability/blank-line-before-control-flow`.
	pub rule: String,
	/// The score this finding moves.
	pub category: Category,
	/// How serious the finding is.
	pub severity: Severity,
	/// Where the finding applies.
	pub span: Span,
	/// One sentence explaining what is wrong.
	pub message: String,
	/// One sentence explaining how to fix it.
	pub suggestion: String,
	/// Per-finding weight, after the rule's own scaling.
	///
	/// This is the value before [`Severity`] is applied, and it is recorded so that
	/// reports can show where each point went without recomputing rule internals.
	pub weight: f64,
	/// An edit that resolves this finding, when one can be made safely.
	///
	/// Rules only attach a fix when the edit is certain: a blank line insertion is mechanical, while
	/// breaking a long line requires understanding the expression. Absence means the reader must
	/// decide, not that the finding is unimportant.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub fix: Option<Fix>,
}

impl Finding {
	/// The penalty this finding contributes to its category score.
	///
	/// Weight and severity are combined here so that the scoring engine and any
	/// reporting layer can never disagree about how much a finding costs.
	#[must_use]
	pub fn penalty(&self) -> f64 {
		self.weight * self.severity.penalty_factor()
	}
}

/// Builds a [`Finding`] with a fluent, testable API.
///
/// Rules create findings frequently and each one needs several fields, so a builder
/// keeps the rule bodies readable — which matters since the rule bodies are
/// themselves scored by this tool.
#[derive(Debug, Clone)]
pub struct FindingBuilder {
	rule: String,
	category: Category,
	severity: Severity,
	span: Span,
	weight: f64,
	message: Option<String>,
	suggestion: Option<String>,
	fix: Option<Fix>,
}

impl FindingBuilder {
	/// Starts a builder for `rule` at `span`.
	#[must_use]
	pub fn new(rule: impl Into<String>, category: Category, span: Span) -> Self {
		Self {
			rule: rule.into(),
			category,
			severity: Severity::Minor,
			span,
			weight: 1.0,
			message: None,
			suggestion: None,
			fix: None,
		}
	}

	/// Sets the severity.
	#[must_use]
	pub fn severity(mut self, severity: Severity) -> Self {
		self.severity = severity;
		self
	}

	/// Sets the rule weight.
	#[must_use]
	pub fn weight(mut self, weight: f64) -> Self {
		self.weight = weight;
		self
	}

	/// Sets the explanation of what is wrong.
	#[must_use]
	pub fn message(mut self, message: impl Into<String>) -> Self {
		self.message = Some(message.into());
		self
	}

	/// Sets the explanation of how to fix it.
	#[must_use]
	pub fn suggestion(mut self, suggestion: impl Into<String>) -> Self {
		self.suggestion = Some(suggestion.into());
		self
	}

	/// Attaches an edit that resolves this finding.
	#[must_use]
	pub fn fix(mut self, fix: Fix) -> Self {
		self.fix = Some(fix);
		self
	}

	/// Finishes the finding.
	///
	/// A missing message or suggestion becomes a readable default rather than a
	/// panic: an under-explained finding is a reporting weakness, not a reason to
	/// abandon an entire analysis run.
	#[must_use]
	pub fn build(self) -> Finding {
		Finding {
			rule: self.rule,
			category: self.category,
			severity: self.severity,
			span: self.span,
			message: self.message.unwrap_or_else(|| "rule violated".to_string()),
			suggestion: self
				.suggestion
				.unwrap_or_else(|| "review this location".to_string()),
			weight: self.weight,
			fix: self.fix,
		}
	}
}
