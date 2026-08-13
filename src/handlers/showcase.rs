use askama::Template;
use axum::{
	extract::Form,
	http::{header, StatusCode},
	response::{IntoResponse, Response},
};

use crate::pdf::membership_2026_27;
use crate::render::HtmlTemplate;

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

pub async fn membership_pdf_handler(
	Form(form): Form<membership_2026_27::templates::MembershipForm>,
) -> Response {
	match tokio::task::spawn_blocking(move || membership_2026_27::generator::generate(form)).await {
		Ok(Ok(pdf_bytes)) => (
			StatusCode::OK,
			[
				(header::CONTENT_TYPE, "application/pdf"),
				(
					header::CONTENT_DISPOSITION,
					"inline; filename=\"tesseramento-shine.pdf\"",
				),
			],
			pdf_bytes,
		)
			.into_response(),
		_ => (
			StatusCode::INTERNAL_SERVER_ERROR,
			"Errore durante la generazione del PDF",
		)
			.into_response(),
	}
}