use askama::Template;
use serde::Deserialize;

#[derive(Deserialize, Template)]
#[template(path = "pdf/membership_2026_27.typ", escape = "none")]
pub struct MembershipForm {
	// --- Applicant (adult or parent/legal guardian) ---
	pub last_name: String,
	pub first_name: String,
	pub birth_place: String,
	pub birth_province: String,
	pub birth_date: String,
	pub residence_city: String,
	pub residence_address: String,
	pub residence_number: String,
	pub residence_cap: String,
	pub residence_province: String,
	pub phone: String,
	pub email: String,
	pub fiscal_code: String,

	// --- Minor (only when enrolling a minor) ---
	#[serde(default)]
	pub is_minor: Option<String>,
	#[serde(default)]
	pub minor_last_name: Option<String>,
	#[serde(default)]
	pub minor_first_name: Option<String>,
	#[serde(default)]
	pub minor_birth_place: Option<String>,
	#[serde(default)]
	pub minor_birth_province: Option<String>,
	#[serde(default)]
	pub minor_birth_date: Option<String>,
	#[serde(default)]
	pub minor_residence_city: Option<String>,
	#[serde(default)]
	pub minor_residence_address: Option<String>,
	#[serde(default)]
	pub minor_residence_number: Option<String>,
	#[serde(default)]
	pub minor_residence_cap: Option<String>,
	#[serde(default)]
	pub minor_residence_province: Option<String>,
	#[serde(default)]
	pub minor_fiscal_code: Option<String>,

	// --- Consents
	#[serde(default)]
	pub consent_photo: bool,
	#[serde(default)]
	pub consent_publication: bool,

	// --- Commute autonomy (only when enrolling a minor) ---
	#[serde(default)]
	pub commute_alone: Option<String>,

	// --- Signatures ---
	pub place_and_date: String,
	pub signature: String,
	#[serde(default)]
	pub autonomy_place_and_date: Option<String>,
	#[serde(default)]
	pub autonomy_signature: Option<String>,
}
