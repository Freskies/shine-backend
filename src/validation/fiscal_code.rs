//! The three fiscal-code checks a regex cannot express.
//!
//! All of them run on the server only. The check character needs a lookup table and a modulo,
//! the encoded birthday needs a calendar, and the encoded name needs the consonant rules
//! below — reimplementing any of them in the browser would be the duplication this module set
//! out to avoid. The shape is judged here too, and for a different reason: the browser can
//! match it, but a refused `pattern` is answered with one sentence about the sixteen
//! characters and the submission never gets far enough for any of the three below to say
//! something more useful.
//!
//! Between them they are what makes the field worth a check at all: the shape accepts almost
//! any sixteen characters in the right arrangement, the checksum accepts any code that was
//! ever issued to anybody, and only [`agrees_with_birth_date`] and [`agrees_with_name`] can
//! tell that a perfectly valid code belongs to somebody else. They are asked in that order —
//! name, date, then check character — because the first two can say *which* part of the code
//! is wrong and the third cannot, and any single mistyped character breaks all three at once.

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
/// This is what turns a single mistyped letter from an accepted code into a rejected one.
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

// --- The name ---

const VOWELS: [char; 5] = ['A', 'E', 'I', 'O', 'U'];

/// The base letter an accented one stands for.
///
/// Only the Latin-1 letters an Italian registry actually produces. Anything outside this set
/// is what makes [`letters`] give up rather than guess.
fn fold(c: char) -> Option<char> {
	match c {
		'A'..='Z' => Some(c),
		'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => Some('A'),
		'Ç' => Some('C'),
		'È' | 'É' | 'Ê' | 'Ë' => Some('E'),
		'Ì' | 'Í' | 'Î' | 'Ï' => Some('I'),
		'Ñ' => Some('N'),
		'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => Some('O'),
		'Ù' | 'Ú' | 'Û' | 'Ü' => Some('U'),
		'Ý' | 'Ÿ' => Some('Y'),
		_ => None,
	}
}

/// A name as the tax office reads it: uppercase, accents folded onto their base letter, and
/// the punctuation of "D'Angelo" or "De Luca" dropped, since the code is computed over the
/// letters alone.
///
/// `None` when a letter cannot be folded. That is the conservative answer and it matters: a
/// name written in an alphabet this function does not know is a name whose code it cannot
/// predict, and refusing a valid enrolment is a worse outcome than missing a check.
fn letters(name: &str) -> Option<String> {
	let mut out = String::new();
	for c in name.to_uppercase().chars() {
		if c.is_whitespace() || matches!(c, '\'' | '-' | '.' | ',') {
			continue;
		}
		out.push(fold(c)?);
	}
	Some(out)
}

/// The three characters a surname contributes: consonants first, then vowels, padded with `X`
/// when there are fewer than three letters to give.
fn surname_code(letters: &str) -> String {
	let consonants = letters.chars().filter(|c| !VOWELS.contains(c));
	let vowels = letters.chars().filter(|c| VOWELS.contains(c));
	consonants
		.chain(vowels)
		.chain("XXX".chars())
		.take(3)
		.collect()
}

/// As [`surname_code`], except that a given name with four or more consonants skips its
/// second one — the rule that makes Giovanni `GNN` rather than `GVN`.
fn given_name_code(letters: &str) -> String {
	let consonants: Vec<char> = letters.chars().filter(|c| !VOWELS.contains(c)).collect();
	if consonants.len() >= 4 {
		return [consonants[0], consonants[2], consonants[3]]
			.iter()
			.collect();
	}
	surname_code(letters)
}

/// True when the first six characters of `code` are the ones this surname and this given name
/// encode.
///
/// Omocodia only ever substitutes letters for *digits*, so these six are the same in every
/// variant of a code and can be compared as written.
///
/// Two ways out without a verdict, both deliberate: a name [`letters`] cannot fold, and a name
/// that carries no letters at all. In either case the name field has its own rule to report,
/// and this one stays quiet instead of blaming the fiscal code for it.
///
/// One false rejection is left standing on purpose. Somebody registered as "Anna Maria" whose
/// code therefore reads `NNM` will be refused if they type "Anna" alone — the message says to
/// write the name as it appears on the health card, which is also what the membership document
/// is supposed to carry.
pub fn agrees_with_name(code: &str, last_name: &str, first_name: &str) -> bool {
	let head: String = code.chars().take(6).collect();
	if head.chars().count() != 6 {
		return false;
	}

	let (Some(surname), Some(given)) = (letters(last_name), letters(first_name)) else {
		return true;
	};
	if surname.is_empty() || given.is_empty() {
		return true;
	}

	head == format!("{}{}", surname_code(&surname), given_name_code(&given))
}

#[cfg(test)]
mod tests {
	use super::{agrees_with_birth_date, agrees_with_name, checksum_is_valid};
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

	/// The check the checksum cannot make, and the one this form used to be missing: a code
	/// whose arithmetic is perfect but whose owner is somebody else.
	#[test]
	fn the_encoded_name_is_read_back() {
		assert!(agrees_with_name(MARIO, "Rossi", "Mario"));
		assert!(agrees_with_name(MARIO, " rossi ", "MARIO"));

		assert!(!agrees_with_name(MARIO, "Bianchi", "Mario"));
		assert!(!agrees_with_name(MARIO, "Rossi", "Luca"));
	}

	/// A given name with four consonants drops the second: Giovanni is `GNN`.
	#[test]
	fn a_long_given_name_skips_its_second_consonant() {
		// VRDGNN99A01A001? — only the first six characters matter here.
		assert!(agrees_with_name("VRDGNN99A01A001A", "Verdi", "Giovanni"));
		assert!(!agrees_with_name("VRDGVN99A01A001A", "Verdi", "Giovanni"));
	}

	/// Apostrophes, spaces and accents are not letters, and a name with fewer than three of
	/// them is padded with `X`.
	#[test]
	fn punctuation_is_dropped_and_short_names_are_padded() {
		assert!(agrees_with_name("DNGMRA99A01A001A", "D'Angelo", "Maria"));
		assert!(agrees_with_name("DLCNNA99A01A001A", "De Luca", "Anna"));
		// Two given names are one string: IVALDOEMILIO gives V, D, M.
		assert!(agrees_with_name("BOXVDM99A01A001A", "Bo", "Ivaldo Emilio"));
		assert!(agrees_with_name("RSSMRA99A01A001A", "Rossì", "Mario"));
	}

	/// Both ways out: nothing to compare against, and an alphabet the folding does not know.
	#[test]
	fn an_unreadable_name_is_not_a_verdict() {
		assert!(agrees_with_name(MARIO, "", "Mario"));
		assert!(agrees_with_name(MARIO, "Rossi", "   "));
		assert!(agrees_with_name(MARIO, "Иванов", "Мария"));
		// A code too short to hold a name is still a rejection.
		assert!(!agrees_with_name("RSS", "Rossi", "Mario"));
	}
}
