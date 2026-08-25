//! The province abbreviations the membership document accepts.

/// Every Italian province abbreviation, plus `EE`.
/// `EE` is what the UISP form expects for somebody born outside Italy.
pub const PROVINCES: &[&str] = &[
	"AG", "AL", "AN", "AO", "AP", "AQ", "AR", "AT", "AV", "BA", "BG", "BI", "BL", "BN", "BO", "BR",
	"BS", "BT", "BZ", "CA", "CB", "CE", "CH", "CL", "CN", "CO", "CR", "CS", "CT", "CZ", "EE", "EN",
	"FC", "FE", "FG", "FI", "FM", "FR", "GE", "GO", "GR", "IM", "IS", "KR", "LC", "LE", "LI", "LO",
	"LT", "LU", "MB", "MC", "ME", "MI", "MN", "MO", "MS", "MT", "NA", "NO", "NU", "OR", "PA", "PC",
	"PD", "PE", "PG", "PI", "PN", "PO", "PR", "PT", "PU", "PV", "PZ", "RA", "RC", "RE", "RG", "RI",
	"RM", "RN", "RO", "SA", "SI", "SO", "SP", "SR", "SS", "SU", "SV", "TA", "TE", "TN", "TO", "TP",
	"TR", "TS", "TV", "UD", "VA", "VB", "VC", "VE", "VI", "VR", "VT", "VV",
];

#[cfg(test)]
mod tests {
	use super::PROVINCES;

	#[test]
	fn every_abbreviation_is_two_uppercase_letters() {
		for p in PROVINCES {
			assert!(
				p.len() == 2 && p.chars().all(|c| c.is_ascii_uppercase()),
				"{p} is not a two-letter uppercase abbreviation"
			);
		}
	}

	/// 107 provinces plus `EE`, sorted and with nothing listed twice. The list is also
	/// turned into a regex alternation for the browser, where a duplicate would go
	/// unnoticed, and into a `<datalist>`, which shows it in this order.
	#[test]
	fn the_list_is_complete_sorted_and_unique() {
		assert_eq!(PROVINCES.len(), 108);
		assert!(PROVINCES.windows(2).all(|w| w[0] < w[1]));
	}
}
