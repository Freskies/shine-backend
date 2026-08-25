//! One description of every rule the enrolment form enforces.
//!
//! [`RULES`] and [`LOOSE_FIELDS`] are the single table. They are read twice:
//!
//! - [`validate`] enforces them when the form is submitted. This is the authoritative pass;
//!   the browser can be bypassed, and the values end up in an official UISP document.
//! - [`client_rules`] projects them into the JSON the enrolment page hands to
//!   `enrollment.js`, which turns each entry into the native constraint attributes
//!   (`pattern`, `minlength`, `min`, …) the browser already knows how to enforce.
//!
//! Adding or changing a rule in the table is therefore enough: the two sides cannot drift
//! apart, and no round trip is spent on a check that is a pure function of what the
//! applicant typed. The one thing that does *not* travel to the client is `required`, which
//! stays in the markup and in `syncConditionalSections()` — the browser is the only party
//! that knows which conditional sections are currently on screen.
//!
//! Two checks have no client-side counterpart at all, because a regex cannot express them:
//! the fiscal-code check character and its agreement with the declared birth date. See
//! [`fiscal_code`].

mod fiscal_code;
mod provinces;

pub use provinces::PROVINCES;

use crate::pdf::membership_2026_27::templates::MembershipForm;
use chrono::{Months, NaiveDate, TimeDelta};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;

/// The format `type="date"` posts and `<input min|max>` expects.
const ISO: &str = "%Y-%m-%d";

/// The format the membership document prints dates in.
const ITALIAN_DATE: &str = "%d/%m/%Y";

/// Nobody enrolling in 2026 was born before this, and a date this old is a mistyped year
/// rather than a birthday. It also bounds the century guess in
/// [`fiscal_code::agrees_with_birth_date`].
const EARLIEST_BIRTH_DATE: &str = "1920-01-01";

/// Cap on one canvas signature, as base64 characters.
///
/// A signature pad at phone resolution produces 10–60 KB. This leaves generous headroom
/// while keeping a hand-crafted request from making the PDF compiler chew on megabytes.
const MAX_SIGNATURE_CHARS: usize = 2 * 1024 * 1024;

// --- Formats ---

/// A character-set rule and the Italian sentence that explains it.
///
/// The pattern is stored *unanchored*: HTML wraps a `pattern` attribute in `^(?:…)$` of its
/// own accord, and [`COMPILED`] does the same before matching, so the browser and the
/// server agree by construction rather than by both remembering to write the anchors.
///
/// Every pattern is written to be valid both as a Rust regex and as a JavaScript one under
/// the `v` flag HTML compiles `pattern` with. In practice that means two habits: always
/// `[0-9]` and never `\d`, whose meaning differs between the two; and `/` and `-` escaped
/// inside a character class, which `v` mode requires.
#[derive(Clone, Copy)]
pub struct Format {
	pub pattern: &'static str,
	/// Shown when the value does not match. Describes what *is* allowed — a message that
	/// only says "formato non valido" leaves the applicant guessing.
	pub hint: &'static str,
}

/// Letters, apostrophes, hyphens and spaces.
///
/// Permissive on purpose. A rule strict enough to be interesting is also strict enough to
/// reject somebody's actual name, and a wrong name on the form is caught by a human reading
/// the email — a rejected enrolment is not.
pub const PERSON_NAME: Format = Format {
	pattern: r"[\p{L}][\p{L}'\- ]*",
	hint: "Sono ammesse solo lettere, apostrofi, trattini e spazi.",
};

/// As [`PERSON_NAME`], plus the full stop that abbreviated place names carry ("S. Agata").
pub const PLACE: Format = Format {
	pattern: r"[\p{L}][\p{L}'\-\. ]*",
	hint: "Sono ammesse solo lettere, apostrofi, trattini, punti e spazi.",
};

pub const ADDRESS: Format = Format {
	pattern: r"[\p{L}0-9][\p{L}0-9'\-\.,°\/ ]*",
	hint: "Scrivi solo via o piazza: il numero civico va nel campo accanto.",
};

pub const HOUSE_NUMBER: Format = Format {
	pattern: r"[\p{L}0-9][\p{L}0-9\/\- ]*",
	hint: "Per esempio 12, 12/A oppure 12 bis.",
};

pub const POSTAL_CODE: Format = Format {
	pattern: r"[0-9]{5}",
	hint: "Il CAP è composto da 5 cifre.",
};

/// An optional `+`, then 8 to 15 digits spaced however the applicant likes.
///
/// The spacing is stripped by [`normalize`] before the number reaches the PDF, so the
/// document always shows one unambiguous string no matter how it was typed.
pub const PHONE: Format = Format {
	pattern: r"\+?(?:[ \.\-]*[0-9]){8,15}[ \.\-]*",
	hint: "Servono da 8 a 15 cifre, con o senza prefisso internazionale.",
};

/// Deliberately loose.
///
/// The only address that can be proved deliverable is one that receives mail, and the
/// confirmation message is what does that. Anything stricter starts rejecting valid
/// addresses to catch typos it cannot see anyway.
pub const EMAIL: Format = Format {
	pattern: r"[^@ ]+@[^@ \.]+(\.[^@ \.]+)+",
	hint: "Controlla l'indirizzo: manca la @ oppure il dominio.",
};

/// The sixteen characters of an Italian fiscal code, including the letters that stand in
/// for digits in an omocodia.
///
/// Shape only — the check character is [`fiscal_code::checksum_is_valid`]. Uppercase-only
/// because both [`normalize`] and the page force the field uppercase before it is judged.
pub const FISCAL_CODE: Format = Format {
	pattern: r"[A-Z]{6}[0-9LMNPQRSTUV]{2}[ABCDEHLMPRST][0-9LMNPQRSTUV]{2}[A-Z][0-9LMNPQRSTUV]{3}[A-Z]",
	hint: "Il codice fiscale è composto da 16 caratteri.",
};

/// "Ravenna, 17/08/2026" — the wording the membership document uses, and what the server
/// pre-fills the two fields with.
pub const PLACE_AND_DATE: Format = Format {
	pattern: r"[\p{L}][\p{L}'\- ]*, ?[0-9]{2}/[0-9]{2}/[0-9]{4}",
	hint: "Scrivi luogo e data così: Ravenna, 17/08/2026.",
};

pub const SHORT_NOTE: Format = Format {
	pattern: r"[\p{L}0-9][\p{L}0-9'\-\. ]*",
	hint: "Sono ammesse solo lettere, numeri e spazi.",
};

// --- Date windows ---

/// One end of the window a date field accepts.
///
/// Resolved against today's date on every request rather than baked in at build time, so a
/// server left running for months keeps handing the browser correct bounds.
#[derive(Clone, Copy)]
pub enum DateLimit {
	/// A fixed calendar day.
	Fixed(&'static str),
	/// Today, shifted. The year shift is applied first, so `years: -18` lands exactly on
	/// the eighteenth birthday and `days: 1` steps one day past it.
	Shifted { years: i32, days: i64 },
}

impl DateLimit {
	fn resolve(self, today: NaiveDate) -> NaiveDate {
		match self {
			DateLimit::Fixed(iso) => NaiveDate::parse_from_str(iso, ISO).unwrap_or(today),
			DateLimit::Shifted { years, days } => {
				// `Months` clamps 29 February onto the 28th instead of failing, which is
				// also how an eighteenth birthday falling on a leap day is counted.
				let months = Months::new(years.unsigned_abs() * 12);
				let shifted = if years < 0 {
					today.checked_sub_months(months)
				} else {
					today.checked_add_months(months)
				};
				shifted.unwrap_or(today) + TimeDelta::days(days)
			}
		}
	}
}

/// The last day of birth that still makes somebody eighteen today.
const EIGHTEENTH_BIRTHDAY: DateLimit = DateLimit::Shifted {
	years: -18,
	days: 0,
};

/// The day after it, which is the earliest a minor can have been born.
const STILL_A_MINOR: DateLimit = DateLimit::Shifted {
	years: -18,
	days: 1,
};

const TODAY: DateLimit = DateLimit::Shifted { years: 0, days: 0 };

// --- Fields ---

/// What kind of value a field holds, and therefore how it is judged and normalized.
#[derive(Clone, Copy)]
pub enum Kind {
	Text(Format),
	/// As [`Kind::Text`], with the separators stripped on the way in.
	Phone(Format),
	/// Uppercase text from a closed list, which the page also offers as a `<datalist>`.
	Choice {
		options: &'static [&'static str],
		/// `id` of the `<datalist>` element holding the options.
		list: &'static str,
		hint: &'static str,
	},
	/// Shape, check character, and agreement with the date field named here.
	FiscalCode {
		/// Name of the rule holding this person's birth date.
		birth_date: &'static str,
	},
	/// A calendar day inside a window that moves with today's date.
	Date {
		min: DateLimit,
		max: DateLimit,
		/// Shown when the date falls before `min`.
		too_early: &'static str,
		/// Shown when it falls after `max`.
		too_late: &'static str,
	},
	/// A place and a date in one line, as the membership document prints them.
	PlaceAndDate,
	/// A canvas data URL.
	///
	/// Absent from the client rules: a hidden input has nothing for the browser to
	/// constrain, and whether the pad has strokes is something `enrollment.js` already
	/// reports next to the canvas itself.
	Signature,
}

impl Kind {
	/// The character-set rule this kind checks against, when it has one.
	fn format(self) -> Option<Format> {
		match self {
			Kind::Text(f) | Kind::Phone(f) => Some(f),
			Kind::FiscalCode { .. } => Some(FISCAL_CODE),
			Kind::PlaceAndDate => Some(PLACE_AND_DATE),
			Kind::Choice { .. } | Kind::Date { .. } | Kind::Signature => None,
		}
	}

	/// Whether the value should be uppercased before it is judged, stored and printed.
	fn is_uppercase(self) -> bool {
		matches!(self, Kind::Choice { .. } | Kind::FiscalCode { .. })
	}
}

/// Everything needed to judge one value, independent of where the value came from.
///
/// Split out of [`Rule`] so the fields that are not part of `MembershipForm` — the
/// applicant's email and the four emergency-contact columns — reuse the same machinery
/// instead of growing a second set of checks. See [`LOOSE_FIELDS`].
#[derive(Clone, Copy)]
pub struct Field {
	/// The `name` attribute, which is also what the error fragment sends back so the page
	/// can mark and focus the offending input.
	pub name: &'static str,
	/// Italian label, used in the error list.
	pub label: &'static str,
	pub kind: Kind,
	pub min_len: usize,
	pub max_len: usize,
}

/// When a field has to be filled in.
///
/// The conditional groups exist because CSS only *hides* the minor and autonomy sections:
/// their inputs are still posted, so what makes them required is the state of the two
/// toggles, not their presence in the body.
#[derive(Clone, Copy, PartialEq)]
pub enum Group {
	/// Always.
	Always,
	/// Only when "sto compilando per un minorenne" is ticked.
	Minor,
	/// Only when the minor also travels to training unaccompanied.
	Autonomy,
	/// Never; checked only when it carries something.
	Never,
}

/// A [`Field`] bound to its place in `MembershipForm`.
///
/// The two function pointers are what keep the field list in one place: without them the
/// names would be spelled out again in a `match` for reading and a third time for
/// normalizing.
pub struct Rule {
	pub field: Field,
	pub group: Group,
	pub get: fn(&MembershipForm) -> Option<&str>,
	pub set: fn(&mut MembershipForm, Option<String>),
}

/// Every field of the membership document, in the order the form presents them — which is
/// also the order the error list comes out in.
pub static RULES: &[Rule] = &[
	// --- Applicant: an adult, or the parent or legal guardian of a minor ---
	Rule {
		field: Field {
			name: "last_name",
			label: "Cognome",
			kind: Kind::Text(PERSON_NAME),
			min_len: 2,
			max_len: 50,
		},
		group: Group::Always,
		get: |f| Some(f.last_name.as_str()),
		set: |f, v| f.last_name = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "first_name",
			label: "Nome",
			kind: Kind::Text(PERSON_NAME),
			min_len: 2,
			max_len: 50,
		},
		group: Group::Always,
		get: |f| Some(f.first_name.as_str()),
		set: |f, v| f.first_name = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "birth_place",
			label: "Luogo di nascita",
			kind: Kind::Text(PLACE),
			min_len: 2,
			max_len: 60,
		},
		group: Group::Always,
		get: |f| Some(f.birth_place.as_str()),
		set: |f, v| f.birth_place = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "birth_province",
			label: "Provincia di nascita",
			kind: Kind::Choice {
				options: PROVINCES,
				list: "province-list",
				hint: "Usa la sigla della provincia, o EE se sei nato all'estero.",
			},
			min_len: 2,
			max_len: 2,
		},
		group: Group::Always,
		get: |f| Some(f.birth_province.as_str()),
		set: |f, v| f.birth_province = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "birth_date",
			label: "Data di nascita",
			kind: Kind::Date {
				min: DateLimit::Fixed(EARLIEST_BIRTH_DATE),
				max: EIGHTEENTH_BIRTHDAY,
				too_early: "Controlla l'anno: non accettiamo date precedenti al 1920.",
				too_late: "Chi compila la richiesta deve essere maggiorenne. Se il \
				           tesserato è minorenne, compila con i tuoi dati e spunta \
				           \"Sto compilando questa richiesta per un tesserato minorenne\".",
			},
			min_len: 10,
			max_len: 10,
		},
		group: Group::Always,
		get: |f| Some(f.birth_date.as_str()),
		set: |f, v| f.birth_date = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "residence_city",
			label: "Comune di residenza",
			kind: Kind::Text(PLACE),
			min_len: 2,
			max_len: 60,
		},
		group: Group::Always,
		get: |f| Some(f.residence_city.as_str()),
		set: |f, v| f.residence_city = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "residence_address",
			label: "Indirizzo di residenza",
			kind: Kind::Text(ADDRESS),
			min_len: 3,
			max_len: 80,
		},
		group: Group::Always,
		get: |f| Some(f.residence_address.as_str()),
		set: |f, v| f.residence_address = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "residence_number",
			label: "N° civico",
			kind: Kind::Text(HOUSE_NUMBER),
			min_len: 1,
			max_len: 10,
		},
		group: Group::Always,
		get: |f| Some(f.residence_number.as_str()),
		set: |f, v| f.residence_number = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "residence_cap",
			label: "CAP",
			kind: Kind::Text(POSTAL_CODE),
			min_len: 5,
			max_len: 5,
		},
		group: Group::Always,
		get: |f| Some(f.residence_cap.as_str()),
		set: |f, v| f.residence_cap = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "residence_province",
			label: "Provincia di residenza",
			kind: Kind::Choice {
				options: PROVINCES,
				list: "province-list",
				hint: "Usa la sigla della provincia, per esempio RA.",
			},
			min_len: 2,
			max_len: 2,
		},
		group: Group::Always,
		get: |f| Some(f.residence_province.as_str()),
		set: |f, v| f.residence_province = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "phone",
			label: "Telefono cellulare",
			kind: Kind::Phone(PHONE),
			min_len: 8,
			max_len: 25,
		},
		group: Group::Always,
		get: |f| Some(f.phone.as_str()),
		set: |f, v| f.phone = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "email",
			label: "E-mail per la tessera UISP",
			kind: Kind::Text(EMAIL),
			min_len: 5,
			max_len: 254,
		},
		group: Group::Always,
		get: |f| Some(f.email.as_str()),
		set: |f, v| f.email = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "fiscal_code",
			label: "Codice fiscale",
			kind: Kind::FiscalCode {
				birth_date: "birth_date",
			},
			min_len: 16,
			max_len: 16,
		},
		group: Group::Always,
		get: |f| Some(f.fiscal_code.as_str()),
		set: |f, v| f.fiscal_code = v.unwrap_or_default(),
	},
	// --- Minor: required only while the toggle is on ---
	Rule {
		field: Field {
			name: "minor_last_name",
			label: "Cognome del minore",
			kind: Kind::Text(PERSON_NAME),
			min_len: 2,
			max_len: 50,
		},
		group: Group::Minor,
		get: |f| f.minor_last_name.as_deref(),
		set: |f, v| f.minor_last_name = v,
	},
	Rule {
		field: Field {
			name: "minor_first_name",
			label: "Nome del minore",
			kind: Kind::Text(PERSON_NAME),
			min_len: 2,
			max_len: 50,
		},
		group: Group::Minor,
		get: |f| f.minor_first_name.as_deref(),
		set: |f, v| f.minor_first_name = v,
	},
	Rule {
		field: Field {
			name: "minor_birth_place",
			label: "Luogo di nascita del minore",
			kind: Kind::Text(PLACE),
			min_len: 2,
			max_len: 60,
		},
		group: Group::Minor,
		get: |f| f.minor_birth_place.as_deref(),
		set: |f, v| f.minor_birth_place = v,
	},
	Rule {
		field: Field {
			name: "minor_birth_province",
			label: "Provincia di nascita del minore",
			kind: Kind::Choice {
				options: PROVINCES,
				list: "province-list",
				hint: "Usa la sigla della provincia, o EE se è nato all'estero.",
			},
			min_len: 2,
			max_len: 2,
		},
		group: Group::Minor,
		get: |f| f.minor_birth_province.as_deref(),
		set: |f, v| f.minor_birth_province = v,
	},
	Rule {
		field: Field {
			name: "minor_birth_date",
			label: "Data di nascita del minore",
			kind: Kind::Date {
				min: STILL_A_MINOR,
				max: TODAY,
				too_early: "Questa data appartiene a una persona maggiorenne. Se il \
				            tesserato ha compiuto 18 anni, togli la spunta e compila la \
				            richiesta con i suoi dati.",
				too_late: "La data di nascita non può essere nel futuro.",
			},
			min_len: 10,
			max_len: 10,
		},
		group: Group::Minor,
		get: |f| f.minor_birth_date.as_deref(),
		set: |f, v| f.minor_birth_date = v,
	},
	Rule {
		field: Field {
			name: "minor_residence_city",
			label: "Comune di residenza del minore",
			kind: Kind::Text(PLACE),
			min_len: 2,
			max_len: 60,
		},
		group: Group::Minor,
		get: |f| f.minor_residence_city.as_deref(),
		set: |f, v| f.minor_residence_city = v,
	},
	Rule {
		field: Field {
			name: "minor_residence_address",
			label: "Indirizzo di residenza del minore",
			kind: Kind::Text(ADDRESS),
			min_len: 3,
			max_len: 80,
		},
		group: Group::Minor,
		get: |f| f.minor_residence_address.as_deref(),
		set: |f, v| f.minor_residence_address = v,
	},
	Rule {
		field: Field {
			name: "minor_residence_number",
			label: "N° civico del minore",
			kind: Kind::Text(HOUSE_NUMBER),
			min_len: 1,
			max_len: 10,
		},
		group: Group::Minor,
		get: |f| f.minor_residence_number.as_deref(),
		set: |f, v| f.minor_residence_number = v,
	},
	Rule {
		field: Field {
			name: "minor_residence_cap",
			label: "CAP del minore",
			kind: Kind::Text(POSTAL_CODE),
			min_len: 5,
			max_len: 5,
		},
		group: Group::Minor,
		get: |f| f.minor_residence_cap.as_deref(),
		set: |f, v| f.minor_residence_cap = v,
	},
	Rule {
		field: Field {
			name: "minor_residence_province",
			label: "Provincia di residenza del minore",
			kind: Kind::Choice {
				options: PROVINCES,
				list: "province-list",
				hint: "Usa la sigla della provincia, per esempio RA.",
			},
			min_len: 2,
			max_len: 2,
		},
		group: Group::Minor,
		get: |f| f.minor_residence_province.as_deref(),
		set: |f, v| f.minor_residence_province = v,
	},
	Rule {
		field: Field {
			name: "minor_fiscal_code",
			label: "Codice fiscale del minore",
			kind: Kind::FiscalCode {
				birth_date: "minor_birth_date",
			},
			min_len: 16,
			max_len: 16,
		},
		group: Group::Minor,
		get: |f| f.minor_fiscal_code.as_deref(),
		set: |f, v| f.minor_fiscal_code = v,
	},
	// --- Signatures and their datelines ---
	Rule {
		field: Field {
			name: "place_and_date",
			label: "Luogo e data",
			kind: Kind::PlaceAndDate,
			min_len: 8,
			max_len: 60,
		},
		group: Group::Always,
		get: |f| Some(f.place_and_date.as_str()),
		set: |f, v| f.place_and_date = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "signature",
			label: "Firma",
			kind: Kind::Signature,
			min_len: 1,
			max_len: MAX_SIGNATURE_CHARS,
		},
		group: Group::Always,
		get: |f| Some(f.signature.as_str()),
		set: |f, v| f.signature = v.unwrap_or_default(),
	},
	Rule {
		field: Field {
			name: "autonomy_place_and_date",
			label: "Luogo e data dell'uscita autonoma",
			kind: Kind::PlaceAndDate,
			min_len: 8,
			max_len: 60,
		},
		group: Group::Autonomy,
		get: |f| f.autonomy_place_and_date.as_deref(),
		set: |f, v| f.autonomy_place_and_date = v,
	},
	Rule {
		field: Field {
			name: "autonomy_signature",
			label: "Firma per l'uscita autonoma",
			kind: Kind::Signature,
			min_len: 1,
			max_len: MAX_SIGNATURE_CHARS,
		},
		group: Group::Autonomy,
		get: |f| f.autonomy_signature.as_deref(),
		set: |f, v| f.autonomy_signature = v,
	},
];

// --- Fields outside the membership document ---

/// Where the applicant wants the recap and the UISP card sent. Not part of the document, so
/// it is not in `MembershipForm`.
pub const APPLICANT_EMAIL: Field = Field {
	name: "applicant_email",
	label: "Indirizzo email",
	kind: Kind::Text(EMAIL),
	min_len: 5,
	max_len: 254,
};

/// The four columns of an emergency-contact row. Every row posts all four, and the handler
/// keeps only the rows that carry at least a name and a phone number.
pub const CONTACT_NAME: Field = Field {
	name: "contact_name",
	label: "Nome del contatto",
	kind: Kind::Text(PERSON_NAME),
	min_len: 2,
	max_len: 50,
};

pub const CONTACT_SURNAME: Field = Field {
	name: "contact_surname",
	label: "Cognome del contatto",
	kind: Kind::Text(PERSON_NAME),
	min_len: 2,
	max_len: 50,
};

pub const CONTACT_PHONE: Field = Field {
	name: "contact_phone",
	label: "Telefono del contatto",
	kind: Kind::Phone(PHONE),
	min_len: 8,
	max_len: 25,
};

pub const CONTACT_NOTE: Field = Field {
	name: "contact_note",
	label: "Chi è il contatto",
	kind: Kind::Text(SHORT_NOTE),
	min_len: 0,
	max_len: 40,
};

/// The fields above, so [`client_rules`] and [`COMPILED`] see them too.
pub static LOOSE_FIELDS: &[Field] = &[
	APPLICANT_EMAIL,
	CONTACT_NAME,
	CONTACT_SURNAME,
	CONTACT_PHONE,
	CONTACT_NOTE,
];

/// Every pattern in the table, compiled once and anchored.
///
/// Keyed by the pattern source rather than by field name, so the handful of shared formats
/// above cost one `Regex` each instead of one per field that uses them.
static COMPILED: LazyLock<HashMap<&'static str, Regex>> = LazyLock::new(|| {
	RULES
		.iter()
		.map(|r| &r.field)
		.chain(LOOSE_FIELDS)
		.filter_map(|f| f.kind.format())
		.map(|format| {
			// The patterns are compile-time constants checked by `every_pattern_compiles`,
			// so a failure here is a bug that the test suite has already caught.
			let anchored = Regex::new(&format!("^(?:{})$", format.pattern))
				.expect("a pattern in RULES is not a valid regex");
			(format.pattern, anchored)
		})
		.collect()
});

// --- Errors ---

/// One rejected field, as the page needs it: a name to focus, a label to name, and a
/// sentence explaining what to do.
pub struct FieldError {
	/// The input's `name`. Repeated names — the contact columns — carry a `:index` suffix
	/// so the page can tell the rows apart.
	pub field: String,
	pub label: String,
	pub message: String,
}

impl Field {
	fn error(&self, message: impl Into<String>) -> FieldError {
		FieldError {
			field: self.name.to_string(),
			label: self.label.to_string(),
			message: message.into(),
		}
	}

	/// Judges one value. `None` means it passed.
	///
	/// `required` is the caller's decision, because for the conditional sections it depends
	/// on the two toggles rather than on anything visible here. `birth_date` is read only by
	/// [`Kind::FiscalCode`]; pass `None` for every other kind.
	pub fn check(
		&self,
		value: &str,
		required: bool,
		today: NaiveDate,
		birth_date: Option<&str>,
	) -> Option<FieldError> {
		let value = value.trim();

		if value.is_empty() {
			return required.then(|| self.error("Questo campo è obbligatorio."));
		}

		// Characters, not bytes: an accented name is shorter than its UTF-8 length, and
		// `maxlength` in the browser counts UTF-16 units, which is closer to this than to
		// a byte count.
		let length = value.chars().count();
		if length < self.min_len {
			return Some(self.error(format!("Servono almeno {} caratteri.", self.min_len)));
		}
		if length > self.max_len {
			return Some(self.error(format!("Non può superare i {} caratteri.", self.max_len)));
		}

		if let Some(format) = self.kind.format()
			&& !COMPILED[format.pattern].is_match(value)
		{
			return Some(self.error(format.hint));
		}

		match self.kind {
			Kind::Text(_) | Kind::Phone(_) => None,

			Kind::Choice { options, hint, .. } => {
				(!options.contains(&value)).then(|| self.error(hint))
			}

			// The pattern above has already established the shape, so what is left is the
			// arithmetic and the cross-check against the date field.
			Kind::FiscalCode { .. } => {
				if !fiscal_code::checksum_is_valid(value) {
					return Some(self.error(
						"Il codice fiscale non è valido: controlla di averlo copiato \
						 correttamente.",
					));
				}
				// Skipped when the date itself was rejected, so one mistyped birthday does
				// not come back as two separate problems.
				let declared = birth_date
					.and_then(|d| NaiveDate::parse_from_str(d.trim(), ISO).ok())
					.filter(|d| *d <= today);
				match declared {
					Some(date) if !fiscal_code::agrees_with_birth_date(value, date) => {
						Some(self.error(
							"Il codice fiscale non corrisponde alla data di nascita \
							 indicata: controlla entrambi.",
						))
					}
					_ => None,
				}
			}

			Kind::Date {
				min,
				max,
				too_early,
				too_late,
			} => {
				let Ok(date) = NaiveDate::parse_from_str(value, ISO) else {
					return Some(self.error("Questa non è una data valida."));
				};
				if date < min.resolve(today) {
					return Some(self.error(too_early));
				}
				if date > max.resolve(today) {
					return Some(self.error(too_late));
				}
				None
			}

			// The pattern has already established `dd/mm/yyyy`, so what is left is whether
			// those digits are a day that exists — 31/02 satisfies the shape and nothing
			// else. Saying so beats repeating the format the applicant already got right.
			//
			// Split on the comma rather than on a space: the space after it is optional in
			// the pattern, while a place name may well contain one.
			Kind::PlaceAndDate => {
				let day = value.rsplit(',').next().unwrap_or_default().trim();
				NaiveDate::parse_from_str(day, ITALIAN_DATE)
					.is_err()
					.then(|| self.error("Questo giorno non esiste sul calendario."))
			}

			// A canvas that was drawn on serializes to a `data:` URL. The bytes themselves
			// are decoded by the PDF generator, which is where a corrupt one would surface.
			Kind::Signature => (!value.starts_with("data:image/png;base64,"))
				.then(|| self.error("Traccia la firma nel riquadro prima di inviare.")),
		}
	}
}

// --- Normalization ---

/// Tidies the form in place, before it is judged, emailed or printed.
///
/// Three jobs, all of which have to happen exactly once and before everything else:
///
/// - **Clearing the sections the toggles turned off.** CSS only hides them, so their inputs
///   are still posted; a parent who ticks "for a minor", fills the block in and then unticks
///   it would otherwise have that data reach the emails.
/// - **Trimming, and dropping blanks to `None`,** so a field holding only spaces is treated
///   as missing rather than as two characters of nothing.
/// - **Fixing the case and the spacing** of the values that end up on an official document,
///   so the PDF reads the same however they were typed.
pub fn normalize(form: &mut MembershipForm) {
	form.is_minor = form.is_minor.take().filter(|v| !v.trim().is_empty());
	form.commute_alone = form.commute_alone.take().filter(|v| !v.trim().is_empty());

	if form.is_minor.is_none() {
		form.commute_alone = None;
	}

	for rule in RULES {
		let active = match rule.group {
			Group::Minor if form.is_minor.is_none() => false,
			Group::Autonomy if form.commute_alone.is_none() => false,
			_ => true,
		};
		if !active {
			(rule.set)(form, None);
			continue;
		}

		let Some(raw) = (rule.get)(form) else {
			continue;
		};
		let trimmed = raw.trim();

		let value = match rule.field.kind {
			// The separators are what make one number look like several; the digits are the
			// number, and they are what the document should show.
			Kind::Phone(_) => trimmed
				.chars()
				.filter(|c| c.is_ascii_digit() || *c == '+')
				.collect(),
			_ if rule.field.kind.is_uppercase() => trimmed.to_uppercase(),
			_ => trimmed.to_string(),
		};

		(rule.set)(form, Some(value).filter(|v| !v.is_empty()));
	}
}

// --- Validation ---

/// Checks every field of the membership document, in table order.
///
/// Returns one entry per problem rather than stopping at the first, so somebody who got
/// three things wrong is told all three at once instead of resubmitting three times. Run
/// [`normalize`] first.
pub fn validate(form: &MembershipForm, today: NaiveDate) -> Vec<FieldError> {
	let minor = form.is_minor.is_some();
	let autonomy = minor && form.commute_alone.is_some();

	RULES
		.iter()
		.filter_map(|rule| {
			let required = match rule.group {
				Group::Always => true,
				Group::Minor => minor,
				Group::Autonomy => autonomy,
				Group::Never => false,
			};
			// Fields belonging to a section that is off were emptied by `normalize`, so
			// there is nothing left to judge and nothing that is missing.
			let value = (rule.get)(form).unwrap_or_default();
			if value.is_empty() && !required {
				return None;
			}

			let birth_date = match rule.field.kind {
				Kind::FiscalCode { birth_date } => value_of(form, birth_date),
				_ => None,
			};
			rule.field.check(value, required, today, birth_date)
		})
		.collect()
}

/// Reads another field by name, for the checks that span two of them.
fn value_of<'a>(form: &'a MembershipForm, name: &str) -> Option<&'a str> {
	RULES
		.iter()
		.find(|r| r.field.name == name)
		.and_then(|r| (r.get)(form))
}

// --- Client projection ---

/// One rule as the page receives it, with the date windows already resolved and the closed
/// lists already turned into an alternation the `pattern` attribute understands.
///
/// Serialized in `camelCase` so each key is already the name of the DOM property or
/// attribute `enrollment.js` assigns it to.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientRule {
	name: &'static str,
	/// Becomes the input's `title`, which is what the browser shows next to its own
	/// message, and the fallback text for a format failure.
	hint: &'static str,
	#[serde(skip_serializing_if = "Option::is_none")]
	pattern: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	min_length: Option<usize>,
	#[serde(skip_serializing_if = "Option::is_none")]
	max_length: Option<usize>,
	#[serde(skip_serializing_if = "Option::is_none")]
	min: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	max: Option<String>,
	/// Shown when the value falls below `min`.
	#[serde(skip_serializing_if = "Option::is_none")]
	min_message: Option<&'static str>,
	#[serde(skip_serializing_if = "Option::is_none")]
	max_message: Option<&'static str>,
	/// `id` of the `<datalist>` to attach.
	#[serde(skip_serializing_if = "Option::is_none")]
	list: Option<&'static str>,
	/// Whether the page should uppercase what is typed, so the value matches the
	/// uppercase-only pattern it is judged against.
	#[serde(skip_serializing_if = "std::ops::Not::not")]
	uppercase: bool,
}

impl ClientRule {
	fn from(field: &Field, today: NaiveDate) -> Option<Self> {
		// Nothing for the browser to constrain on a hidden input.
		if matches!(field.kind, Kind::Signature) {
			return None;
		}

		let mut rule = ClientRule {
			name: field.name,
			hint: "",
			pattern: field.kind.format().map(|f| f.pattern.to_string()),
			min_length: (field.min_len > 0).then_some(field.min_len),
			max_length: Some(field.max_len),
			min: None,
			max: None,
			min_message: None,
			max_message: None,
			list: None,
			uppercase: field.kind.is_uppercase(),
		};

		match field.kind {
			Kind::Text(f) | Kind::Phone(f) => rule.hint = f.hint,
			Kind::PlaceAndDate => rule.hint = PLACE_AND_DATE.hint,
			Kind::FiscalCode { .. } => rule.hint = FISCAL_CODE.hint,
			Kind::Choice {
				options,
				list,
				hint,
			} => {
				// A closed list is a regex the browser can enforce on its own; every
				// option is uppercase ASCII, so nothing here needs escaping.
				rule.pattern = Some(options.join("|"));
				rule.list = Some(list);
				rule.hint = hint;
			}
			Kind::Date {
				min,
				max,
				too_early,
				too_late,
			} => {
				// A date input measures neither length nor pattern, and a stray
				// `minlength` on one is just noise in the DOM.
				rule.min_length = None;
				rule.max_length = None;
				rule.min = Some(min.resolve(today).format(ISO).to_string());
				rule.max = Some(max.resolve(today).format(ISO).to_string());
				rule.min_message = Some(too_early);
				rule.max_message = Some(too_late);
				rule.hint = too_late;
			}
			Kind::Signature => unreachable!("returned above"),
		}

		Some(rule)
	}
}

/// The whole table as the JSON the enrolment page embeds for `enrollment.js`.
///
/// `<` is escaped even though none of the strings contain one: the result is interpolated
/// into a `<script>` element, where a `</script>` appearing inside a string would end the
/// element early, and relying on "our data happens not to contain that" is a rule somebody
/// eventually breaks by adding a hint.
pub fn client_rules(today: NaiveDate) -> String {
	let rules: Vec<ClientRule> = RULES
		.iter()
		.map(|r| &r.field)
		.chain(LOOSE_FIELDS)
		.filter_map(|field| ClientRule::from(field, today))
		.collect();

	serde_json::to_string(&rules)
		.unwrap_or_else(|_| "[]".to_string())
		.replace('<', "\\u003c")
}

#[cfg(test)]
mod tests {
	use super::*;

	fn today() -> NaiveDate {
		NaiveDate::from_ymd_opt(2026, 8, 25).unwrap()
	}

	/// The one failure mode `COMPILED` cannot report at runtime without panicking on a live
	/// request: a pattern that is not a valid Rust regex.
	#[test]
	fn every_pattern_compiles() {
		LazyLock::force(&COMPILED);
	}

	/// Two fields sharing a name would make the error fragment ambiguous and the client
	/// rules fight each other.
	#[test]
	fn field_names_are_unique() {
		let mut names: Vec<&str> = RULES
			.iter()
			.map(|r| r.field.name)
			.chain(LOOSE_FIELDS.iter().map(|f| f.name))
			.collect();
		let total = names.len();
		names.sort_unstable();
		names.dedup();
		assert_eq!(names.len(), total, "a field name is used twice");
	}

	/// Every `Kind::FiscalCode` names a rule that exists, since a typo there would silently
	/// skip the cross-check instead of failing.
	#[test]
	fn fiscal_code_rules_point_at_a_real_date_field() {
		for rule in RULES {
			if let Kind::FiscalCode { birth_date } = rule.field.kind {
				assert!(
					RULES.iter().any(|r| r.field.name == birth_date),
					"{} points at the missing field {birth_date}",
					rule.field.name
				);
			}
		}
	}

	fn check(field: &Field, value: &str) -> Option<String> {
		field.check(value, true, today(), None).map(|e| e.message)
	}

	#[test]
	fn a_blank_required_field_is_reported_once() {
		let name = RULES[0].field;
		assert!(check(&name, "   ").is_some());
		assert!(check(&name, "Rossi").is_none());
	}

	#[test]
	fn names_accept_accents_apostrophes_and_hyphens() {
		let name = RULES[0].field;
		for accepted in ["D'Angelo", "Bonzi-Rossi", "Nuñez", "Éloïse", "De Luca"] {
			assert!(check(&name, accepted).is_none(), "{accepted} was rejected");
		}
		for rejected in ["Rossi1", "<script>", "R"] {
			assert!(check(&name, rejected).is_some(), "{rejected} was accepted");
		}
	}

	#[test]
	fn phones_accept_any_spacing_and_count_digits() {
		for accepted in [
			"3401234567",
			"340 123 4567",
			"+39 340 123 4567",
			"340-123-4567",
		] {
			assert!(
				check(&CONTACT_PHONE, accepted).is_none(),
				"{accepted} was rejected"
			);
		}
		for rejected in ["12345", "340 123 4567 890 123", "abcdefgh"] {
			assert!(
				check(&CONTACT_PHONE, rejected).is_some(),
				"{rejected} was accepted"
			);
		}
	}

	#[test]
	fn emails_need_an_at_and_a_dotted_domain() {
		for accepted in ["a@b.it", "nome.cognome@example.co.uk"] {
			assert!(
				check(&APPLICANT_EMAIL, accepted).is_none(),
				"{accepted} was rejected"
			);
		}
		for rejected in ["nome@example", "nome.example.it", "no spaces@example.it"] {
			assert!(
				check(&APPLICANT_EMAIL, rejected).is_some(),
				"{rejected} was accepted"
			);
		}
	}

	/// The rule the whole form turns on: whoever signs has to be eighteen *on the day they
	/// submit*, not merely born in a year that is eighteen years back.
	#[test]
	fn the_applicant_must_be_eighteen_today() {
		let birth_date = RULES
			.iter()
			.find(|r| r.field.name == "birth_date")
			.unwrap()
			.field;

		// Exactly eighteen today, and one day short of it.
		assert!(check(&birth_date, "2008-08-25").is_none());
		assert!(check(&birth_date, "2008-08-26").is_some());

		assert!(check(&birth_date, "1920-01-01").is_none());
		assert!(check(&birth_date, "1919-12-31").is_some());
		assert!(check(&birth_date, "2026-08-25").is_some());
	}

	/// And the mirror rule: a minor has to still be one.
	#[test]
	fn the_minor_must_not_be_eighteen_yet() {
		let birth_date = RULES
			.iter()
			.find(|r| r.field.name == "minor_birth_date")
			.unwrap()
			.field;

		assert!(check(&birth_date, "2008-08-26").is_none());
		assert!(check(&birth_date, "2008-08-25").is_some());
		assert!(check(&birth_date, "2026-08-26").is_some());
	}

	#[test]
	fn provinces_are_a_closed_list() {
		let province = RULES
			.iter()
			.find(|r| r.field.name == "birth_province")
			.unwrap()
			.field;

		assert!(check(&province, "RA").is_none());
		assert!(check(&province, "EE").is_none());
		assert!(check(&province, "XX").is_some());
		// Lowercase reaches `check` only when `normalize` has not run, and it is rejected
		// rather than silently accepted.
		assert!(check(&province, "ra").is_some());
	}

	#[test]
	fn the_fiscal_code_checksum_is_enforced() {
		let code = RULES
			.iter()
			.find(|r| r.field.name == "fiscal_code")
			.unwrap()
			.field;

		assert!(
			code.check("RSSMRA85T10A562S", true, today(), None)
				.is_none()
		);
		assert!(
			code.check("RSSMRA85T10A562T", true, today(), None)
				.is_some()
		);
	}

	#[test]
	fn the_fiscal_code_must_agree_with_the_birth_date() {
		let code = RULES
			.iter()
			.find(|r| r.field.name == "fiscal_code")
			.unwrap()
			.field;

		let right = code.check("RSSMRA85T10A562S", true, today(), Some("1985-12-10"));
		assert!(right.is_none());

		let wrong = code.check("RSSMRA85T10A562S", true, today(), Some("1985-12-11"));
		assert!(wrong.is_some());

		// An unusable date is somebody else's error to report.
		let unknown = code.check("RSSMRA85T10A562S", true, today(), Some(""));
		assert!(unknown.is_none());
	}

	#[test]
	fn place_and_date_needs_a_real_day() {
		let field = RULES
			.iter()
			.find(|r| r.field.name == "place_and_date")
			.unwrap()
			.field;

		assert!(check(&field, "Ravenna, 17/08/2026").is_none());
		assert!(check(&field, "Reggio nell'Emilia, 01/01/2026").is_none());
		assert!(check(&field, "Ravenna, 31/02/2026").is_some());
		assert!(check(&field, "Ravenna 2026").is_some());
	}

	fn adult_form() -> MembershipForm {
		MembershipForm {
			last_name: "Rossi".into(),
			first_name: "Mario".into(),
			birth_place: "Milano".into(),
			birth_province: "MI".into(),
			birth_date: "1985-12-10".into(),
			residence_city: "Ravenna".into(),
			residence_address: "Via Roma".into(),
			residence_number: "12/A".into(),
			residence_cap: "48121".into(),
			residence_province: "RA".into(),
			phone: "340 123 4567".into(),
			email: "mario@example.it".into(),
			fiscal_code: "rssmra85t10a562s".into(),
			is_minor: None,
			minor_last_name: None,
			minor_first_name: None,
			minor_birth_place: None,
			minor_birth_province: None,
			minor_birth_date: None,
			minor_residence_city: None,
			minor_residence_address: None,
			minor_residence_number: None,
			minor_residence_cap: None,
			minor_residence_province: None,
			minor_fiscal_code: None,
			consent_photo: false,
			consent_publication: false,
			commute_alone: None,
			place_and_date: "  Ravenna, 25/08/2026  ".into(),
			signature: "data:image/png;base64,AAAA".into(),
			autonomy_place_and_date: None,
			autonomy_signature: None,
		}
	}

	#[test]
	fn normalize_fixes_case_and_spacing() {
		let mut form = adult_form();
		normalize(&mut form);

		assert_eq!(form.fiscal_code, "RSSMRA85T10A562S");
		assert_eq!(form.phone, "3401234567");
		assert_eq!(form.place_and_date, "Ravenna, 25/08/2026");
		assert!(validate(&form, today()).is_empty());
	}

	/// The reason the clearing lives here rather than in the PDF generator: ticking the
	/// minor toggle, filling the block in and unticking it used to leave that data in the
	/// two emails, which are rendered before the PDF ever sees the form.
	#[test]
	fn normalize_clears_the_sections_the_toggles_turned_off() {
		let mut form = adult_form();
		form.minor_first_name = Some("Luca".into());
		form.minor_birth_date = Some("2015-01-01".into());
		form.autonomy_signature = Some("data:image/png;base64,AAAA".into());

		normalize(&mut form);

		assert_eq!(form.minor_first_name, None);
		assert_eq!(form.minor_birth_date, None);
		assert_eq!(form.autonomy_signature, None);
		assert!(validate(&form, today()).is_empty());
	}

	#[test]
	fn a_minor_enrolment_requires_the_minor_block() {
		let mut form = adult_form();
		form.is_minor = Some("true".into());
		normalize(&mut form);

		let missing = validate(&form, today());
		assert!(missing.iter().any(|e| e.field == "minor_first_name"));
		assert!(missing.iter().any(|e| e.field == "minor_fiscal_code"));
		// The applicant's own fields are still fine, so they are not in the list.
		assert!(!missing.iter().any(|e| e.field == "first_name"));
	}

	#[test]
	fn every_problem_is_reported_at_once() {
		let mut form = adult_form();
		form.residence_cap = "48".into();
		form.phone = "123".into();
		form.birth_date = "2020-01-01".into();
		normalize(&mut form);

		let errors = validate(&form, today());
		let fields: Vec<&str> = errors.iter().map(|e| e.field.as_str()).collect();
		assert!(fields.contains(&"residence_cap"));
		assert!(fields.contains(&"phone"));
		assert!(fields.contains(&"birth_date"));
	}

	/// What the page is handed has to cover every field the page shows, or a rule silently
	/// stops being enforced in the browser.
	#[test]
	fn the_client_rules_cover_every_visible_field() {
		let json = client_rules(today());
		for field in RULES.iter().map(|r| &r.field).chain(LOOSE_FIELDS) {
			if matches!(field.kind, Kind::Signature) {
				continue;
			}
			assert!(
				json.contains(&format!("\"name\":\"{}\"", field.name)),
				"{} is missing from the client rules",
				field.name
			);
		}
		assert!(
			!json.contains('<'),
			"the JSON is not safe inside a <script>"
		);
	}
}
