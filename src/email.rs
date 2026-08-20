use crate::config::{Config, SmtpConfig};
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tracing::debug;

/// A file to attach, already in memory.
pub struct Upload {
	pub filename: String,
	pub content_type: String,
	pub bytes: Vec<u8>,
}

/// The two messages one enrolment produces.
pub struct EnrollmentMail {
	pub applicant_address: String,
	pub applicant_subject: String,
	pub applicant_body: String,
	pub association_subject: String,
	pub association_body: String,
	pub membership_pdf: Vec<u8>,
	/// The medical certificate photo. Goes to the association only — never echoed back to
	/// the applicant, who already has it.
	pub certificate: Option<Upload>,
}

#[derive(Debug)]
pub enum EmailError {
	Address(String),
	Build(lettre::error::Error),
	Transport(lettre::transport::smtp::Error),
}

impl std::fmt::Display for EmailError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Address(a) => write!(f, "invalid email address: {a}"),
			Self::Build(e) => write!(f, "could not build the message: {e}"),
			Self::Transport(e) => write!(f, "SMTP delivery failed: {e}"),
		}
	}
}

impl std::error::Error for EmailError {}

/// Builds a transport matching the port's convention.
///
/// 465 expects TLS from the first byte, while 587 and 25 start in clear text and upgrade
/// through STARTTLS. Picking the wrong one hangs until the connection times out, so it is
/// derived from the port rather than left to another setting.
fn transport(smtp: &SmtpConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, EmailError> {
	let builder = if smtp.port == 465 {
		AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.host)
	} else {
		AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp.host)
	}
	.map_err(EmailError::Transport)?;

	Ok(builder
		.port(smtp.port)
		.credentials(Credentials::new(smtp.mail.clone(), smtp.password.clone()))
		.build())
}

fn attachment(upload: &Upload) -> SinglePart {
	let content_type = ContentType::parse(&upload.content_type)
		.unwrap_or(ContentType::parse("application/octet-stream").expect("static mime is valid"));
	Attachment::new(upload.filename.clone()).body(upload.bytes.clone(), content_type)
}

/// Sends the recap to the applicant and the submission to the association.
///
/// The applicant's copy goes first: if the association's copy then fails, the person has
/// evidence of what they sent, and the error surfaces so the submission can be retried.
pub async fn send_enrollment(config: &Config, mail: EnrollmentMail) -> Result<(), EmailError> {
	let mailer = transport(&config.smtp)?;
	let from: Mailbox = config
		.smtp
		.mail
		.parse()
		.map_err(|_| EmailError::Address(config.smtp.mail.clone()))?;

	let pdf = Upload {
		filename: "tesseramento-shine-2026-27.pdf".to_string(),
		content_type: "application/pdf".to_string(),
		bytes: mail.membership_pdf,
	};

	let to_applicant = mail
		.applicant_address
		.parse()
		.map_err(|_| EmailError::Address(mail.applicant_address.clone()))?;

	let applicant = Message::builder()
		.from(from.clone())
		.to(to_applicant)
		.subject(mail.applicant_subject)
		.multipart(
			MultiPart::mixed()
				.singlepart(SinglePart::html(mail.applicant_body))
				.singlepart(attachment(&pdf)),
		)
		.map_err(EmailError::Build)?;

	mailer
		.send(applicant)
		.await
		.map_err(EmailError::Transport)?;
	debug!(to = %mail.applicant_address, "recap sent to the applicant");

	let to_association = config
		.enrollment_recipient
		.parse()
		.map_err(|_| EmailError::Address(config.enrollment_recipient.clone()))?;

	let mut association_parts = MultiPart::mixed()
		.singlepart(SinglePart::html(mail.association_body))
		.singlepart(attachment(&pdf));
	if let Some(certificate) = &mail.certificate {
		association_parts = association_parts.singlepart(attachment(certificate));
	}

	// No Reply-To on purpose. Setting it to the applicant would be convenient, but Yahoo
	// (and other providers) reject a Reply-To that is not the authenticated mailbox with
	// "550 No MIME Reply-To header matches auth mailboxes". Their address is in the subject
	// and the body instead, so replying is still one copy-paste away.
	let association = Message::builder()
		.from(from)
		.to(to_association)
		.subject(mail.association_subject)
		.multipart(association_parts)
		.map_err(EmailError::Build)?;

	mailer
		.send(association)
		.await
		.map_err(EmailError::Transport)?;
	debug!(to = %config.enrollment_recipient, "submission sent to the association");

	Ok(())
}
