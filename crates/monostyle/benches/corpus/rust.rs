//! Inventory reconciliation for a warehouse: the shapes the scanner has to get right in Rust.
//!
//! Raw strings with hash delimiters, byte strings, lifetimes beside character literals, nested
//! block comments, and attributes above items are all here on purpose.

use std::collections::HashMap;
use std::fmt;

/// A stock-keeping unit as it appears in the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sku<'label> {
	/// The human label, borrowed from the import file.
	pub label: &'label str,
	/// Units on hand.
	pub quantity: u32,
}

impl fmt::Display for Sku<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{} ({})", self.label, self.quantity)
	}
}

/// A pattern that must not be read as a string: the hashes change the closer.
const QUERY: &str = r#"SELECT "label", quantity FROM stock WHERE label = '"#;

/// Two hashes, so a `"#` inside the text is still content.
const TEMPLATE: &str = r##"{"kind": "sku", "label": "#{label}"}"##;

/// Bytes rather than text, with an escape the scanner must step over.
const MAGIC: &[u8] = b"STK\x00\"v2\"";

/* A block comment /* that nests */ and keeps going until the second closer. */

/// Reconciles the counted stock against the ledger and reports every discrepancy.
///
/// The counted side wins: a physical count is the ground truth, so the ledger is corrected
/// toward it rather than the other way around.
pub fn reconcile<'label>(
	ledger: &HashMap<&'label str, u32>,
	counted: &[Sku<'label>],
) -> Vec<Discrepancy<'label>> {
	let mut discrepancies = Vec::new();

	for sku in counted {
		let Some(&expected) = ledger.get(sku.label) else {
			discrepancies.push(Discrepancy::Unknown(sku.label));

			continue;
		};

		if expected == sku.quantity {
			continue;
		}

		let delta = i64::from(sku.quantity) - i64::from(expected);

		discrepancies.push(match delta {
			d if d > 0 => Discrepancy::Surplus(sku.label, d as u32),
			d if d < -100 => Discrepancy::Missing(sku.label, (-d) as u32),
			_ => Discrepancy::Shortfall(sku.label, (-delta) as u32),
		});
	}

	discrepancies.sort_by(|left, right| left.label().cmp(right.label()));
	discrepancies
}

/// One difference between the count and the ledger.
#[derive(Debug, PartialEq, Eq)]
pub enum Discrepancy<'label> {
	/// Counted but never recorded.
	Unknown(&'label str),
	/// More on hand than recorded.
	Surplus(&'label str, u32),
	/// Fewer on hand than recorded.
	Shortfall(&'label str, u32),
	/// Enough fewer that theft or a misplaced pallet is likelier than a miscount.
	Missing(&'label str, u32),
}

impl<'label> Discrepancy<'label> {
	/// The label the discrepancy concerns.
	pub fn label(&self) -> &'label str {
		match self {
			Self::Unknown(label)
			| Self::Surplus(label, _)
			| Self::Shortfall(label, _)
			| Self::Missing(label, _) => label,
		}
	}

	/// A single character marker for the terse report, next to a lifetime to tell them apart.
	pub fn marker(&self) -> char {
		match self {
			Self::Unknown(_) => '?',
			Self::Surplus(..) => '+',
			Self::Shortfall(..) => '-',
			Self::Missing(..) => '!',
		}
	}
}

macro_rules! sku {
	($label:expr, $quantity:expr) => {
		Sku {
			label: $label,
			quantity: $quantity,
		}
	};
}

/// Formats a report line per discrepancy, widest label first so the columns line up.
pub fn report(discrepancies: &[Discrepancy<'_>]) -> String {
	let width = discrepancies
		.iter()
		.map(|discrepancy| discrepancy.label().len())
		.max()
		.unwrap_or(0);

	discrepancies
		.iter()
		.map(|discrepancy| {
			format!("{} {:<width$} {:?}", discrepancy.marker(), discrepancy.label(), discrepancy)
		})
		.collect::<Vec<_>>()
		.join("\n")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_matching_count_has_no_discrepancies() {
		let ledger = HashMap::from([("bolt", 10), ("nut", 4)]);
		let counted = [sku!("bolt", 10), sku!("nut", 4)];

		assert!(reconcile(&ledger, &counted).is_empty());
	}
}
