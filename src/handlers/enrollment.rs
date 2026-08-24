use askama::Template;
use axum::extract::{Multipart, Query, State};
use axum::http::HeaderValue;
use axum::response::{Html, IntoResponse, Response};
use chrono::Local;
use urlencoding::encode;

use crate::email::{EnrollmentMail, Upload, send_enrollment};
use crate::pdf::membership_2026_27::generator;
use crate::pdf::membership_2026_27::templates::MembershipForm;
use crate::render::HtmlTemplate;
use crate::state::AppState;
use tracing::{error, info};

/// Cap on the certificate photo. Phone cameras produce 3–8 MB, and the whole thing is
/// held in memory and attached to an email, so this leaves headroom without inviting
/// anyone to post a 100 MB file.
const MAX_CERTIFICATE_BYTES: usize = 12 * 1024 * 1024;

/// How many emergency contacts one enrolment may carry.
///
/// Enforced on submission as well as in the UI: the browser can be bypassed, and an
/// unbounded list would let anyone inflate the email we send ourselves.
pub const MAX_EMERGENCY_CONTACTS: usize = 4;

/// One person to call if something happens during training.
pub struct EmergencyContact {
	pub name: String,
	pub surname: String,
	pub phone: String,
	pub note: String,
}

impl EmergencyContact {
	fn is_usable(&self) -> bool {
		!self.name.trim().is_empty() && !self.phone.trim().is_empty()
	}
}

// --- Page and fragments ---

#[derive(Template)]
#[template(path = "showcase/enrollment.html")]
pub struct EnrollmentTemplate {
	/// Pre-filled value for the "Luogo e Data" fields, e.g. "Ravenna, 24/08/2026".
	pub today: String,
}

#[derive(Template)]
#[template(path = "partials/emergency_contact_row.html")]
pub struct EmergencyContactRowTemplate;

#[derive(Template)]
#[template(path = "partials/enrollment_sent.html")]
pub struct EnrollmentSentTemplate {
	/// `None` when `WHATSAPP_NUMBER` is unset, so the template can say so plainly rather
	/// than render a link that goes nowhere.
	pub whatsapp_url: Option<String>,
	pub applicant_email: String,
}

#[derive(Template)]
#[template(path = "partials/enrollment_error.html")]
pub struct EnrollmentErrorTemplate {
	pub message: String,
}

// --- Email bodies ---

#[derive(Template)]
#[template(path = "email/applicant_summary.html")]
struct ApplicantSummary<'a> {
	form: &'a MembershipForm,
	contacts: &'a [EmergencyContact],
	certificate_name: &'a str,
}

#[derive(Template)]
#[template(path = "email/association_notice.html")]
struct AssociationNotice<'a> {
	applicant_email: &'a str,
	form: &'a MembershipForm,
	contacts: &'a [EmergencyContact],
	certificate_name: &'a str,
}

// --- Handlers ---

pub async fn enrollment_handler() -> impl IntoResponse {
	let today = Local::now().format("Ravenna, %d/%m/%Y").to_string();
	HtmlTemplate(EnrollmentTemplate { today })
}

/// How many rows the page already has, so the cap can be honored before handing out
/// another one.
#[derive(serde::Deserialize)]
pub struct RowRequest {
	#[serde(default)]
	count: usize,
}

/// Serves one blank emergency-contact row.
///
/// Kept server-side so the row's markup lives in a template next to the rest of the form,
/// instead of being duplicated as a string inside a script.
pub async fn emergency_contact_row_handler(Query(query): Query<RowRequest>) -> Response {
	// The button is hidden at the cap, so this only trips if something bypassed the UI.
	// An empty body means HTMX appends nothing.
	if query.count >= MAX_EMERGENCY_CONTACTS {
		return Html(String::new()).into_response();
	}
	HtmlTemplate(EmergencyContactRowTemplate).into_response()
}

/// Everything the wizard collected, submitted in one request.
struct Submission {
	applicant_email: String,
	form: MembershipForm,
	contacts: Vec<EmergencyContact>,
	certificate: Upload,
}

/// Reads the multipart body into a [`Submission`].
///
/// The membership fields are re-encoded and handed to serde so the existing
/// `MembershipForm` deserialization — including its defaults and optional fields — stays
/// the single description of that document.
async fn parse_submission(mut multipart: Multipart) -> Result<Submission, String> {
	let mut fields: Vec<(String, String)> = Vec::new();
	let mut names = Vec::new();
	let mut surnames = Vec::new();
	let mut phones = Vec::new();
	let mut notes = Vec::new();
	let mut applicant_email = String::new();
	let mut certificate: Option<Upload> = None;

	while let Some(field) = multipart
		.next_field()
		.await
		.map_err(|e| format!("malformed upload: {e}"))?
	{
		let name = field.name().unwrap_or_default().to_string();

		if name == "certificate" {
			let filename = field
				.file_name()
				.filter(|f| !f.trim().is_empty())
				.unwrap_or("certificato")
				.to_string();
			let content_type = field
				.content_type()
				.unwrap_or("application/octet-stream")
				.to_string();
			let bytes = field
				.bytes()
				.await
				.map_err(|e| format!("could not read the photo: {e}"))?;

			if bytes.is_empty() {
				continue;
			}
			if bytes.len() > MAX_CERTIFICATE_BYTES {
				return Err("La foto del certificato è troppo grande.".to_string());
			}
			certificate = Some(Upload {
				filename,
				content_type,
				bytes: bytes.to_vec(),
			});
			continue;
		}

		let value = field
			.text()
			.await
			.map_err(|e| format!("could not read field {name}: {e}"))?;

		match name.as_str() {
			"applicant_email" => applicant_email = value,
			"contact_name" => names.push(value),
			"contact_surname" => surnames.push(value),
			"contact_phone" => phones.push(value),
			"contact_note" => notes.push(value),
			_ => fields.push((name, value)),
		}
	}

	let query: String = fields
		.iter()
		.map(|(k, v)| format!("{}={}", encode(k), encode(v)))
		.collect::<Vec<_>>()
		.join("&");
	let form: MembershipForm = serde_html_form::from_str(&query)
		.map_err(|e| format!("membership fields did not deserialize: {e}"))?;

	// The four columns are submitted in DOM order, and every row always posts all four,
	// so index alignment holds; `get` keeps a malformed body from panicking anyway.
	let contacts: Vec<EmergencyContact> = (0..names.len())
		.map(|i| EmergencyContact {
			name: names[i].trim().to_string(),
			surname: surnames
				.get(i)
				.cloned()
				.unwrap_or_default()
				.trim()
				.to_string(),
			phone: phones
				.get(i)
				.cloned()
				.unwrap_or_default()
				.trim()
				.to_string(),
			note: notes.get(i).cloned().unwrap_or_default().trim().to_string(),
		})
		.filter(EmergencyContact::is_usable)
		.collect();

	if contacts.len() > MAX_EMERGENCY_CONTACTS {
		return Err(format!(
			"too many emergency contacts: {} (max {MAX_EMERGENCY_CONTACTS})",
			contacts.len()
		));
	}

	if applicant_email.trim().is_empty() {
		return Err("Manca l'indirizzo email.".to_string());
	}
	let certificate = certificate.ok_or("Manca la foto del certificato medico.")?;

	Ok(Submission {
		applicant_email: applicant_email.trim().to_string(),
		form,
		contacts,
		certificate,
	})
}

/// Wraps a prepared message into a `wa.me` link, or `None` when no number is configured.
fn whatsapp_url(state: &AppState, message: &str) -> Option<String> {
	let number = state.config.whatsapp_number.as_ref()?;
	Some(format!("https://wa.me/{}?text={}", number, encode(message)))
}

/// Builds the WhatsApp message the applicant sends as the final confirmation.
fn whatsapp_message(form: &MembershipForm) -> String {
	let subject = match form.minor_first_name.as_deref().map(str::trim) {
		Some(minor) if !minor.is_empty() => format!(
			" per {} {}",
			minor.trim(),
			form.minor_last_name.as_deref().unwrap_or("").trim()
		),
		_ => "".to_string(),
	};

	format!(
		"Sono {} {} e ho appena completato l'iscrizione online{}. \
		 Ho inviato il certificato medico e il modulo di tesseramento. \
		 Potete confermarmi che è tutto arrivato e dirmi quando posso venire la prima volta?",
		form.first_name.trim(),
		form.last_name.trim(),
		subject
	)
}

/// Serves the last wizard step on its own, so it can be looked at without filling in the
/// whole form and sending two real emails.
///
/// Compiled only in debug builds: it is a development aid and has no business answering on
/// a deployed server. Call `window.__phase3()` from the browser console to swap it in.
#[cfg(debug_assertions)]
pub async fn enrollment_preview_sent_handler(State(state): State<AppState>) -> Response {
	let mut response = HtmlTemplate(EnrollmentSentTemplate {
		whatsapp_url: whatsapp_url(&state, "Anteprima del messaggio di conferma."),
		applicant_email: "anteprima@example.com".to_string(),
	}).into_response();

	// As in the real success path, this lets the page drop its "you will lose your data" guard.
	response
		.headers_mut()
		.insert("HX-Trigger", HeaderValue::from_static("enrollmentSent"));
	response
}

/// Receives the whole enrolment, emails both copies, and returns the final step.
pub async fn enrollment_submit_handler(
	State(state): State<AppState>,
	multipart: Multipart,
) -> Response {
	let submission = match parse_submission(multipart).await {
		Ok(s) => s,
		Err(detail) => return enrollment_error(&detail),
	};

	let certificate_name = submission.certificate.filename.clone();

	// One line per stage, so a failure points to the step that broke rather than just the
	// whole submission. The email address is the only personal field logged: it is what
	// makes a report traceable when somebody says "it did not work".
	info!(
		email = %submission.applicant_email,
		contacts = submission.contacts.len(),
		certificate = %certificate_name,
		certificate_bytes = submission.certificate.bytes.len(),
		minor = submission.form.is_minor.is_some(),
		"enrollment received"
	);

	// Rendered before the PDF: `generate` consumes the form and rewrites its fields with
	// Typst escape sequences, which would otherwise show up in the emails.
	let applicant_body = match (ApplicantSummary {
		form: &submission.form,
		contacts: &submission.contacts,
		certificate_name: &certificate_name,
	}).render()
	{
		Ok(body) => body,
		Err(e) => return enrollment_error(&format!("recap template failed: {e}")),
	};

	let association_body = match (AssociationNotice {
		applicant_email: &submission.applicant_email,
		form: &submission.form,
		contacts: &submission.contacts,
		certificate_name: &certificate_name,
	}).render()
	{
		Ok(body) => body,
		Err(e) => return enrollment_error(&format!("notice template failed: {e}")),
	};

	let whatsapp = whatsapp_url(&state, &whatsapp_message(&submission.form));
	let applicant_name = format!(
		"{} {}",
		submission.form.first_name.trim(),
		submission.form.last_name.trim()
	);

	let form = submission.form;
	let pdf = match tokio::task::spawn_blocking(move || generator::generate(form)).await {
		Ok(Ok(bytes)) => bytes,
		Ok(Err(detail)) => return enrollment_error(&detail),
		Err(e) => return enrollment_error(&format!("PDF task aborted: {e}")),
	};
	info!(pdf_bytes = pdf.len(), "membership PDF generated");

	let mail = EnrollmentMail {
		applicant_address: submission.applicant_email.clone(),
		applicant_subject: "Shine Parkour — riepilogo della tua iscrizione".to_string(),
		applicant_body,
		// The applicant's address rides in the subject: there is no Reply-To to carry it,
		// since Yahoo refuses one that is not the authenticated mailbox.
		association_subject: format!(
			"Nuova iscrizione — {applicant_name} <{}>",
			submission.applicant_email
		),
		association_body,
		membership_pdf: pdf,
		certificate: Some(submission.certificate),
	};

	if let Err(e) = send_enrollment(&state.config, mail).await {
		return enrollment_error(&format!("email delivery failed: {e}"));
	}

	info!(
		email = %submission.applicant_email,
		"enrollment complete, both emails delivered"
	);

	let mut response = HtmlTemplate(EnrollmentSentTemplate {
		whatsapp_url: whatsapp,
		applicant_email: submission.applicant_email,
	}).into_response();

	// The form posts into the small feedback area so that a failure leaves everything the
	// applicant typed on screen. Success needs the opposite, so it retargets the whole
	// wizard and replaces it — one endpoint, two swap behaviors, decided server-side.
	let headers = response.headers_mut();
	headers.insert(
		"HX-Retarget",
		HeaderValue::from_static("#enrollment-wizard"),
	);
	headers.insert("HX-Reswap", HeaderValue::from_static("outerHTML"));
	// Lets the page drop its "you will lose your data" guard.
	headers.insert("HX-Trigger", HeaderValue::from_static("enrollmentSent"));
	response
}

/// Returns the error fragment with `200 OK` on purpose: HTMX ignores the body of a non-2xx
/// response by default, so a 4xx/5xx would leave the applicant staring at an unchanged
/// page after filling in everything. `detail` is logged, not shown.
fn enrollment_error(detail: &str) -> Response {
	error!(detail, "enrollment failed");
	HtmlTemplate(EnrollmentErrorTemplate {
		message: "Non è stato possibile completare l'invio. Controlla i dati e riprova: \
		          se il problema resta, mandaci una mail a shineparkour@yahoo.it."
			.to_string(),
	}).into_response()
}
