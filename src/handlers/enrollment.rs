use askama::Template;
use axum::extract::{Multipart, Query, State};
use axum::http::HeaderValue;
use axum::response::{Html, IntoResponse, Response};
use chrono::{Local, NaiveDate};
use urlencoding::encode;

use crate::email::{EnrollmentMail, Upload, send_enrollment};
use crate::pdf::membership_2026_27::generator;
use crate::pdf::membership_2026_27::templates::MembershipForm;
use crate::render::HtmlTemplate;
use crate::state::AppState;
use crate::validation::{self, FieldError};
use tracing::{error, info, warn};

/// Cap on the certificate photo. Phone cameras produce 3–8 MB, and the whole thing is
/// held in memory and attached to an email, so this leaves headroom without inviting
/// anyone to post a 100 MB file.
const MAX_CERTIFICATE_BYTES: usize = 12 * 1024 * 1024;

/// What the certificate may be: a photo of the paper one, or the PDF a doctor emailed.
///
/// Checked by declared content type rather than by sniffing the bytes. The point is to stop
/// somebody attaching a video by mistake, not to stop an attacker — nothing on this server
/// opens the file, it is forwarded to a mailbox and read by a person.
const CERTIFICATE_TYPES: [&str; 2] = ["image/", "application/pdf"];

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
	/// The field rules, as the JSON `enrollment.js` turns into constraint attributes. Comes
	/// from the same table the submission is judged against, so the page cannot enforce a
	/// rule the server does not — or miss one it does.
	pub rules_json: String,
	/// Fills the `<datalist>` the two province fields offer.
	pub provinces: &'static [&'static str],
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

/// The rejected fields, named. Kept apart from [`EnrollmentErrorTemplate`] because the two
/// say opposite things: this one is "here is what to fix", that one is "this is on us".
#[derive(Template)]
#[template(path = "partials/enrollment_invalid.html")]
pub struct EnrollmentInvalidTemplate {
	pub errors: Vec<FieldError>,
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
	let now = Local::now();
	HtmlTemplate(EnrollmentTemplate {
		today: now.format("Ravenna, %d/%m/%Y").to_string(),
		// Resolved per request, not per build: the "at least 18 years old" bound moves every
		// day, and a server left running for months would otherwise hand out a stale one.
		rules_json: validation::client_rules(now.date_naive()),
		provinces: validation::PROVINCES,
	})
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

/// Why an enrolment did not go through.
///
/// The distinction is the whole point: one of these is the applicant's to fix and says which
/// field, the other is ours and says so. Collapsing them is how a form ends up answering a
/// mistyped CAP with "if the problem persists, contact us".
enum Rejection {
	/// Something on our side broke, or the body was not a form this server produces. Logged
	/// in full, reported as [`enrollment_error`].
	Internal(String),
	/// The applicant's data did not pass. Reported field by field.
	Invalid(Vec<FieldError>),
}

impl From<FieldError> for Rejection {
	fn from(error: FieldError) -> Self {
		Rejection::Invalid(vec![error])
	}
}

/// Reads the multipart body into a [`Submission`].
///
/// The membership fields are re-encoded and handed to serde so the existing
/// `MembershipForm` deserialization — including its defaults and optional fields — stays
/// the single description of that document.
async fn parse_submission(mut multipart: Multipart) -> Result<Submission, Rejection> {
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
		.map_err(|e| Rejection::Internal(format!("malformed upload: {e}")))?
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
				.map_err(|e| Rejection::Internal(format!("could not read the photo: {e}")))?;

			if bytes.is_empty() {
				continue;
			}
			if bytes.len() > MAX_CERTIFICATE_BYTES {
				return Err(certificate_error(format!(
					"Il file pesa {} MB: il massimo è {} MB. Scatta la foto a una \
					 risoluzione più bassa, oppure carica il PDF.",
					bytes.len() / (1024 * 1024),
					MAX_CERTIFICATE_BYTES / (1024 * 1024)
				))
				.into());
			}
			if !CERTIFICATE_TYPES
				.iter()
				.any(|t| content_type.starts_with(t))
			{
				return Err(certificate_error(
					"Carica un'immagine oppure un PDF: questo file è di un altro tipo.",
				)
				.into());
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
			.map_err(|e| Rejection::Internal(format!("could not read field {name}: {e}")))?;

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
		.map_err(|e| Rejection::Internal(format!("membership fields did not deserialize: {e}")))?;

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

	// The button that adds a row is hidden at the cap, so only a bypassed UI gets here.
	if contacts.len() > MAX_EMERGENCY_CONTACTS {
		return Err(FieldError {
			field: validation::CONTACT_NAME.name.to_string(),
			label: "Contatti di emergenza".to_string(),
			message: format!("Puoi indicare al massimo {MAX_EMERGENCY_CONTACTS} contatti."),
		}
		.into());
	}

	// Structural, not a validation rule: without a file there is nothing to attach, and the
	// applicant needs sending back to the first step rather than told about a format.
	let certificate = certificate.ok_or_else(|| {
		Rejection::from(certificate_error(
			"Manca il certificato medico: torna al primo passo e caricalo.",
		))
	})?;

	Ok(Submission {
		applicant_email: applicant_email.trim().to_string(),
		form,
		contacts,
		certificate,
	})
}

/// A problem with the uploaded file.
///
/// The certificate is not part of the rule table — there is no pattern to match on a
/// multipart part — but it is reported the same way, so `enrollment.js` can send the wizard
/// back to the step that holds it.
fn certificate_error(message: impl Into<String>) -> FieldError {
	FieldError {
		field: "certificate".to_string(),
		label: "Certificato medico".to_string(),
		message: message.into(),
	}
}

/// Checks the four columns of every emergency-contact row.
///
/// Only the rows that survived [`EmergencyContact::is_usable`] are judged: a row left
/// entirely blank is somebody who changed their mind, not a mistake. The names repeat across
/// rows, so each error carries the row it came from — otherwise the page could only mark
/// every phone field at once and leave the applicant to work out which one it meant.
fn validate_contacts(contacts: &[EmergencyContact], today: NaiveDate) -> Vec<FieldError> {
	/// Field, the column it reads, and whether the row has to carry it.
	type Column = (validation::Field, fn(&EmergencyContact) -> &str, bool);

	let columns: [Column; 4] = [
		(validation::CONTACT_NAME, |c| &c.name, true),
		(validation::CONTACT_SURNAME, |c| &c.surname, false),
		(validation::CONTACT_PHONE, |c| &c.phone, true),
		(validation::CONTACT_NOTE, |c| &c.note, false),
	];

	contacts
		.iter()
		.enumerate()
		.flat_map(|(row, contact)| {
			columns.iter().filter_map(move |(field, read, required)| {
				let mut error = field.check(read(contact), *required, today, None)?;
				error.field = format!("{}:{row}", error.field);
				error.label = format!("{} (contatto {})", error.label, row + 1);
				Some(error)
			})
		})
		.collect()
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
	})
	.into_response();

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
	let mut submission = match parse_submission(multipart).await {
		Ok(s) => s,
		Err(Rejection::Internal(detail)) => return enrollment_error(&detail),
		Err(Rejection::Invalid(errors)) => return enrollment_invalid(errors),
	};

	// Both dated against the same instant, so a request that lands a millisecond after
	// midnight cannot be judged against one day and printed with another.
	let today = Local::now().date_naive();

	// Before validating, because normalization is what decides *which* fields are in play:
	// it clears the sections the two toggles turned off, so a parent who filled in the minor
	// block and then unticked it is not asked to fix fields nobody will read. It also runs
	// before the emails, which is where the old copy of this logic — inside the PDF
	// generator — was too late to help.
	validation::normalize(&mut submission.form);

	let mut errors = validation::validate(&submission.form, today);
	errors.extend(validation::APPLICANT_EMAIL.check(
		&submission.applicant_email,
		true,
		today,
		None,
	));
	errors.extend(validate_contacts(&submission.contacts, today));
	if !errors.is_empty() {
		return enrollment_invalid(errors);
	}

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
	})
	.render()
	{
		Ok(body) => body,
		Err(e) => return enrollment_error(&format!("recap template failed: {e}")),
	};

	let association_body = match (AssociationNotice {
		applicant_email: &submission.applicant_email,
		form: &submission.form,
		contacts: &submission.contacts,
		certificate_name: &certificate_name,
	})
	.render()
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
	})
	.into_response();

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
///
/// Reserved for failures that are ours — a template that would not render, a PDF that would
/// not compile, an SMTP server that would not answer. Anything the applicant can fix goes
/// through [`enrollment_invalid`] instead, which says what and where.
fn enrollment_error(detail: &str) -> Response {
	error!(detail, "enrollment failed");
	HtmlTemplate(EnrollmentErrorTemplate {
		message: "Non è stato possibile completare l'invio. Riprova tra qualche minuto: \
		          se il problema resta, mandaci una mail a shineparkour@yahoo.it."
			.to_string(),
	})
	.into_response()
}

/// Returns the list of fields to fix, and tells the page which ones they are.
///
/// The `HX-Trigger` payload is what lets the browser do the part a fragment cannot: mark
/// each offending input, move to the step holding the first one, and focus it. Same `200 OK`
/// reasoning as above — the applicant's data has to stay on screen.
fn enrollment_invalid(errors: Vec<FieldError>) -> Response {
	warn!(
		count = errors.len(),
		fields = errors
			.iter()
			.map(|e| e.field.as_str())
			.collect::<Vec<_>>()
			.join(","),
		"enrollment rejected: invalid fields"
	);

	let fields: Vec<&str> = errors.iter().map(|e| e.field.as_str()).collect();
	let payload = serde_json::json!({ "enrollmentInvalid": { "fields": fields } });

	let mut response = HtmlTemplate(EnrollmentInvalidTemplate { errors }).into_response();

	// Field names come from the rule table and are ASCII, so this only fails if somebody
	// adds one with a character a header cannot carry — in which case the page still shows
	// the list, it just stops highlighting.
	if let Ok(value) = HeaderValue::from_str(&payload.to_string()) {
		response.headers_mut().insert("HX-Trigger", value);
	}
	response
}
