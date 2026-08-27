use askama::Template;
use axum::response::IntoResponse;

use crate::render::HtmlTemplate;

#[derive(Template)]
#[template(path = "showcase/index.html")]
pub struct IndexTemplate;

pub async fn index_handler() -> impl IntoResponse {
	HtmlTemplate(IndexTemplate)
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

#[cfg(test)]
mod tests {
	use super::*;
	use axum::body::to_bytes;
	use axum::http::header::CACHE_CONTROL;

	/// `pdf-download.js` finds the download button and its note by id, and reads the
	/// attachment URL from a data attribute. Renaming any of the three in the markup breaks
	/// the script silently — the page still looks right, and Firefox on mobile quietly goes
	/// back to being unable to save the file.
	#[tokio::test]
	async fn the_homepage_carries_what_the_download_script_looks_for() {
		let response = index_handler().await.into_response();

		assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), "no-cache");

		let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
		let page = String::from_utf8(body.to_vec()).unwrap();

		assert!(page.contains("id=\"certificate-download\""));
		assert!(page.contains("data-attachment-url=\"/documents/"));
		// Hidden in the markup: the script is the only thing that reveals it.
		assert!(page.contains("id=\"certificate-note\" class=\"steps-list__hint\" hidden"));
		assert!(page.contains("/static/scripts/pdf-download.js"));
	}
}
