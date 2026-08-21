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
