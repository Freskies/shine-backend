//! The two fiscal-code checks a regex cannot express.
//!
//! Both run on the server only. The check character needs a lookup table, a modulo, and
//! an agreement between the encoded birthday. The declared one needs a calendar —
//! reimplementing either in the browser would be the duplication this module set out to
//! avoid, so the page validates the *shape* of the code and leaves these to the submission.

use chrono::{Datelike, NaiveDate};

/// Value of each character when it sits in an odd position, in `0-9A-Z` order.
///
/// Digits and the first ten letters share their values, which is why the first twenty-five
/// entries look repetitive rather than wrong.
const ODD: [u32; 36] = [
	1, 0, 5, 7, 9, 13, 15, 17, 19, 21, // 0-9
	1, 0, 5, 7, 9, 13, 15, 17, 19, 21, // A-J
	2, 4, 18, 20, 11, 3, 6, 8, 12, 14, // K-T
	16, 10, 22, 25, 24, 23, // U-Z
];

/// Month letters, in calendar order: January is `A`, December is `T`.
const MONTHS: [char; 12] = ['A', 'B', 'C', 'D', 'E', 'H', 'L', 'M', 'P', 'R', 'S', 'T'];

/// Position of `c` in the `0-9A-Z` alphabet the tables are indexed by.
fn index(c: char) -> Option<usize> {
	match c {
		'0'..='9' => Some(c as usize - '0' as usize),
		'A'..='Z' => Some(c as usize - 'A' as usize + 10),
		_ => None,
	}
}

/// Value of `c` in an even position: digits count themselves, letters count their rank.
fn even_value(c: char) -> Option<u32> {
	match c {
		'0'..='9' => Some(c as u32 - '0' as u32),
		'A'..='Z' => Some(c as u32 - 'A' as u32),
		_ => None,
	}
}

/// Reads a digit that may have been replaced by a letter.
///
/// When two people would otherwise share a code, the tax office substitutes letters for
/// digits from the right — `L` for 0 through `V` for 9 — so a numeric position can legally
/// hold either.
fn digit(c: char) -> Option<u32> {
	match c {
		'0'..='9' => Some(c as u32 - '0' as u32),
		'L' => Some(0),
		'M' => Some(1),
		'N' => Some(2),
		'P' => Some(3),
		'Q' => Some(4),
		'R' => Some(5),
		'S' => Some(6),
		'T' => Some(7),
		'U' => Some(8),
		'V' => Some(9),
		_ => None,
	}
}

/// True when the sixteenth character is the one, the first fifteen imply.
///
/// This is what turns a single mistyped letter from an accepted code into a rejected one,
/// and it is the reason the field is worth validating at all: the shape alone accepts
/// almost any sixteen characters in the right arrangement.
pub fn checksum_is_valid(code: &str) -> bool {
	// Uppercase ASCII throughout, so bytes and characters line up and a slice is safe.
	if code.len() != 16 || !code.bytes().all(|b| b.is_ascii_alphanumeric()) {
		return false;
	}

	// Computed over the code *as written*, omocodia letters included: substituting the
	// digits back first would produce a different — and wrong — check character.
	let mut sum = 0;
	for (position, c) in code.chars().take(15).enumerate() {
		// Positions are counted from one, so an even index is an odd position.
		let value = if position % 2 == 0 {
			match index(c) {
				Some(i) => ODD[i],
				None => return false,
			}
		} else {
			match even_value(c) {
				Some(v) => v,
				None => return false,
			}
		};
		sum += value;
	}

	let expected = (b'A' + (sum % 26) as u8) as char;
	code.chars().nth(15) == Some(expected)
}

/// True when the birthday encoded in `code` is the one the applicant typed into the date field.
///
/// Catches the case the checksum cannot: a perfectly valid code belonging to somebody else,
/// or a date typed into the wrong field. The code stores only two year digits, so the
/// century comes from `declared` — which means a hundred-year error still passes, and the
/// 1920 floor on the date field is what covers that.
pub fn agrees_with_birth_date(code: &str, declared: NaiveDate) -> bool {
	let c: Vec<char> = code.chars().collect();
	if c.len() != 16 {
		return false;
	}

	let (Some(y1), Some(y0)) = (digit(c[6]), digit(c[7])) else {
		return false;
	};
	let Some(month) = MONTHS.iter().position(|&m| m == c[8]) else {
		return false;
	};
	let (Some(d1), Some(d0)) = (digit(c[9]), digit(c[10])) else {
		return false;
	};

	// Women have forty added to the day of birth; it is the only place the code records sex,
	// and this form does not ask for it, so the offset is simply removed.
	let day = match d1 * 10 + d0 {
		d @ 41..=71 => d - 40,
		d => d,
	};

	declared.year().rem_euclid(100) as u32 == y1 * 10 + y0
		&& declared.month() as usize == month + 1
		&& declared.day() == day
}

#[cfg(test)]
mod tests {
	use super::{agrees_with_birth_date, checksum_is_valid};
	use chrono::NaiveDate;

	/// The canonical example: Mario Rossi, born in Milan on 10 December 1985 —
	/// `T` is the month letter for December, not for October.
	const MARIO: &str = "RSSMRA85T10A562S";

	#[test]
	fn a_real_code_passes() {
		assert!(checksum_is_valid(MARIO));
	}

	#[test]
	fn a_single_wrong_letter_fails() {
		// Same code with the check character bumped by one.
		assert!(!checksum_is_valid("RSSMRA85T10A562T"));
		// Same code with a typo in the surname, which moves the expected check character.
		assert!(!checksum_is_valid("RSSMRB85T10A562S"));
	}

	#[test]
	fn the_wrong_length_fails() {
		assert!(!checksum_is_valid(""));
		assert!(!checksum_is_valid("RSSMRA85T10A562"));
		assert!(!checksum_is_valid("RSSMRA85T10A562SS"));
	}

	#[test]
	fn the_encoded_date_is_read_back() {
		let born = NaiveDate::from_ymd_opt(1985, 12, 10).unwrap();
		assert!(agrees_with_birth_date(MARIO, born));

		// One day out, one month out, one year out.
		for wrong in [(1985, 12, 11), (1985, 11, 10), (1986, 12, 10)] {
			let date = NaiveDate::from_ymd_opt(wrong.0, wrong.1, wrong.2).unwrap();
			assert!(!agrees_with_birth_date(MARIO, date), "{date} was accepted");
		}
	}

	/// A woman's code carries the day plus forty; the date it means is the same.
	#[test]
	fn the_female_day_offset_is_removed() {
		let born = NaiveDate::from_ymd_opt(1985, 12, 10).unwrap();
		assert!(agrees_with_birth_date("RSSMRA85T50A562S", born));
	}

	/// Omocodia: the last digit of the day replaced by the letter that stands for it.
	#[test]
	fn omocodia_letters_read_as_digits() {
		let born = NaiveDate::from_ymd_opt(1985, 12, 10).unwrap();
		assert!(agrees_with_birth_date("RSSMRA85T1LA562S", born));
	}
}
