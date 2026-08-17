use askama::Template;
use axum::{
	extract::{Form, Path, State},
	http::{HeaderValue, StatusCode, header},
	response::{IntoResponse, Response},
};

use crate::pdf::membership_2026_27;
use crate::render::HtmlTemplate;
use crate::state::AppState;

const PDF_FILENAME: &str = "Modulo_Tesseramento_Shine_2026_27.pdf";

// Template HTML
#[derive(Template)]
#[template(path = "showcase/index.html")]
pub struct IndexTemplate;

#[derive(Template)]
#[template(path = "showcase/enrollment.html")]
pub struct EnrollmentTemplate;

#[derive(Template)]
#[template(path = "showcase/membership_form.html")]
pub struct MembershipTemplate;

#[derive(Template)]
#[template(path = "partials/membership_success.html")]
pub struct MembershipSuccessTemplate;

#[derive(Template)]
#[template(path = "partials/membership_error.html")]
pub struct MembershipErrorTemplate {
	pub message: String,
}

// Handlers
pub async fn index_handler() -> impl IntoResponse {
	HtmlTemplate(IndexTemplate)
}

pub async fn enrollment_handler() -> impl IntoResponse {
	HtmlTemplate(EnrollmentTemplate)
}

pub async fn membership_handler() -> impl IntoResponse {
	HtmlTemplate(MembershipTemplate)
}

/// Renders the membership PDF, parks it in [`AppState`] and answers HTMX with a
/// fragment for `#form-feedback` plus an `HX-Trigger` header. The page script
/// turns that `triggerDownload` event into an `<a download>` click.
pub async fn membership_form_post_handler(
	State(state): State<AppState>,
	Form(form): Form<membership_2026_27::templates::MembershipForm>,
) -> Response {
	let pdf_bytes =
		match tokio::task::spawn_blocking(move || membership_2026_27::generator::generate(form))
			.await
		{
			Ok(Ok(bytes)) => bytes,
			Ok(Err(detail)) => return membership_error(&detail),
			Err(join_error) => return membership_error(&format!("Task interrotto: {join_error}")),
		};

	let id = state.insert_pdf(pdf_bytes);
	let trigger = format!(
		r#"{{"triggerDownload":{{"url":"/membership_form/download/{id}","filename":"{PDF_FILENAME}"}}}}"#
	);

	let mut response = HtmlTemplate(MembershipSuccessTemplate).into_response();
	match HeaderValue::from_str(&trigger) {
		Ok(value) => {
			response.headers_mut().insert("HX-Trigger", value);
			response
		}
		// Without the trigger the browser would never start the download, so the
		// submission has effectively failed.
		Err(e) => membership_error(&format!("HX-Trigger non valido: {e}")),
	}
}

/// Serves a previously generated PDF exactly once.
pub async fn membership_pdf_download_handler(
	State(state): State<AppState>,
	Path(id): Path<String>,
) -> Response {
	match state.take_pdf(&id) {
		Some(bytes) => (
			StatusCode::OK,
			[
				(header::CONTENT_TYPE, "application/pdf".to_string()),
				(
					header::CONTENT_DISPOSITION,
					format!("attachment; filename=\"{PDF_FILENAME}\""),
				),
			],
			bytes,
		)
			.into_response(),
		None => (
			StatusCode::NOT_FOUND,
			"PDF non disponibile o scaduto. Compila di nuovo il modulo.",
		)
			.into_response(),
	}
}

/// Builds the `#form-feedback` error fragment.
///
/// Returns `200 OK` on purpose: HTMX ignores the body of non-2xx responses by
/// default, so a 4xx/5xx here would leave the user staring at an unchanged page.
/// `detail` is logged rather than shown, to keep internals off the page.
fn membership_error(detail: &str) -> Response {
	eprintln!("membership PDF generation failed: {detail}");
	HtmlTemplate(MembershipErrorTemplate {
		message: "Controlla i dati inseriti e riprova. Se il problema persiste, contattaci."
			.to_string(),
	})
	.into_response()
}
