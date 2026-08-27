use askama::Template;
use axum::http::HeaderMap;
use axum::http::header::{HeaderValue, USER_AGENT, VARY};
use axum::response::IntoResponse;

use crate::render::HtmlTemplate;
use crate::user_agent::is_mobile_firefox;

/// The certificate request form as everybody else gets it: `ServeDir` sends it with no
/// `Content-Disposition`, and the `download` attribute on the link does the rest. Tested
/// working on Safari and Chrome for iOS, and on Chrome and Firefox for desktop.
const CERTIFICATE_URL: &str = "/static/documents/shine_richiesta_certificato_medico_25_26.pdf";

/// The same file through the mount that adds `Content-Disposition: attachment`, for the
/// browsers that ignore the `download` attribute — see `is_mobile_firefox`. Only the URL
/// changes: the button is the same button, so nothing about the page looks different.
const CERTIFICATE_URL_ATTACHMENT: &str = "/documents/shine_richiesta_certificato_medico_25_26.pdf";

#[derive(Template)]
#[template(path = "showcase/index.html")]
pub struct IndexTemplate {
	/// Which of the two mounts the certificate download points at.
	certificate_url: &'static str,
}

/// The page varies with the `User-Agent`, so any shared cache in front of us has to be told:
/// without `Vary` it would hand one visitor's variant to everybody.
pub async fn index_handler(headers: HeaderMap) -> impl IntoResponse {
	let ignores_download_attribute = headers
		.get(USER_AGENT)
		.and_then(|value| value.to_str().ok())
		.is_some_and(is_mobile_firefox);

	let certificate_url = if ignores_download_attribute {
		CERTIFICATE_URL_ATTACHMENT
	} else {
		CERTIFICATE_URL
	};

	(
		[(VARY, HeaderValue::from_static("User-Agent"))],
		HtmlTemplate(IndexTemplate { certificate_url }),
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
