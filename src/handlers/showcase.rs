use askama::Template;
use axum::http::HeaderMap;
use axum::http::header::{HeaderValue, USER_AGENT, VARY};
use axum::response::IntoResponse;

use crate::render::HtmlTemplate;
use crate::user_agent::is_mobile_firefox;

#[derive(Template)]
#[template(path = "showcase/index.html")]
pub struct IndexTemplate {
	/// Adds the note next to the certificate download that explains how to save a PDF the
	/// browser insists on opening in its viewer. See `is_mobile_firefox`.
	pdf_viewer_hint: bool,
}

/// The page varies with the `User-Agent`, so any shared cache in front of us has to be told:
/// without `Vary` it would hand one visitor's variant to everybody.
pub async fn index_handler(headers: HeaderMap) -> impl IntoResponse {
	let pdf_viewer_hint = headers
		.get(USER_AGENT)
		.and_then(|value| value.to_str().ok())
		.is_some_and(is_mobile_firefox);

	(
		[(VARY, HeaderValue::from_static("User-Agent"))],
		HtmlTemplate(IndexTemplate { pdf_viewer_hint }),
	)
}

// Legal pages. Served as full standalone pages so they are reachable at their own URL;
// the homepage footer also loads them into the global dialog via htmx + `hx-select`,
// which extracts only the `#legal-content` article from the same response.

#[derive(Template)]
#[template(path = "showcase/privacy_policy.html")]
pub struct PrivacyPolicyTemplate;

pub async fn privacy_policy_handler() -> impl IntoResponse {
	HtmlTemplate(PrivacyPolicyTemplate)
}

#[derive(Template)]
#[template(path = "showcase/statute.html")]
pub struct StatuteTemplate;

pub async fn statute_handler() -> impl IntoResponse {
	HtmlTemplate(StatuteTemplate)
}
