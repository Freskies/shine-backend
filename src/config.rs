use std::env::{self, VarError};
use std::fmt;

/// Runtime configuration, read once from the environment at startup.
///
/// The enrolment flow depends on outbound email, so the settings are validated here
/// rather than at the first submission: a missing password should stop the server, not
/// silently lose somebody's enrolment.
#[derive(Clone)]
pub struct Config {
	pub smtp: SmtpConfig,
	/// Where enrolment submissions are delivered inside the association.
	pub enrollment_recipient: String,
	/// Digits only, country code included — `wa.me` rejects spaces and `+`.
	///
	/// Optional so the flow can be exercised before the number is decided; the final step
	/// then shows a placeholder instead of a broken link.
	pub whatsapp_number: Option<String>,
}

#[derive(Clone)]
pub struct SmtpConfig {
	pub host: String,
	pub port: u16,
	/// Both the login and the `From` address. Providers such as Yahoo authenticate with
	/// the mailbox address and reject a `From` that differs from it, so splitting the two
	/// would only create a way to get it wrong.
	pub mail: String,
	pub password: String,
}

#[derive(Debug)]
pub enum ConfigError {
	Missing(&'static str),
	NotUnicode(&'static str),
	BadPort(String),
	BadWhatsAppNumber(String),
}

impl fmt::Display for ConfigError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Missing(key) => write!(f, "{key} is not set"),
			Self::NotUnicode(key) => write!(f, "{key} is not valid unicode"),
			Self::BadPort(value) => write!(f, "SMTP_PORT is not a port number: {value}"),
			Self::BadWhatsAppNumber(value) => write!(
				f,
				"WHATSAPP_NUMBER must be digits only, country code included, got: {value}"
			),
		}
	}
}

impl std::error::Error for ConfigError {}

fn required(key: &'static str) -> Result<String, ConfigError> {
	match env::var(key) {
		Ok(value) if !value.trim().is_empty() => Ok(value),
		Ok(_) | Err(VarError::NotPresent) => Err(ConfigError::Missing(key)),
		Err(VarError::NotUnicode(_)) => Err(ConfigError::NotUnicode(key)),
	}
}

impl Config {
	/// Reads and validates the environment. `.env` is loaded first when present, so local
	/// development needs no exported variables.
	pub fn from_env() -> Result<Self, ConfigError> {
		let _ = dotenvy::dotenv();

		let port_raw = required("SMTP_PORT")?;
		let port = port_raw
			.parse::<u16>()
			.map_err(|_| ConfigError::BadPort(port_raw))?;

		let whatsapp_number = match env::var("WHATSAPP_NUMBER") {
			Ok(value) if !value.trim().is_empty() => {
				let value = value.trim().to_string();
				if !value.chars().all(|c| c.is_ascii_digit()) {
					return Err(ConfigError::BadWhatsAppNumber(value));
				}
				Some(value)
			}
			_ => None,
		};

		Ok(Self {
			smtp: SmtpConfig {
				host: required("SMTP_HOST")?,
				port,
				mail: required("SMTP_MAIL")?,
				password: required("SMTP_PASSWORD")?,
			},
			enrollment_recipient: required("ENROLLMENT_RECIPIENT")?,
			whatsapp_number,
		})
	}
}
