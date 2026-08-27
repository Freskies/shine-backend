use askama::Template;
use axum::http::HeaderMap;
use axum::http::header::{HeaderValue, USER_AGENT, VARY};
use axum::response::IntoResponse;

use crate::render::HtmlTemplate;
use crate::user_agent::{PdfDownload, pdf_download};

/// The certificate request form served inline: `ServeDir` sends it with no
/// `Content-Disposition`, and the `download` attribute on the link does the rest. Tested
/// working on Safari and Chrome for iOS, and on Chrome and Firefox for desktop.
const CERTIFICATE_URL: &str = "/static/documents/shine_richiesta_certificato_medico_25_26.pdf";

/// The same file through the mount that adds `Content-Disposition: attachment`. Only the URL
/// changes: the button is the same button, so nothing about the page looks different.
const CERTIFICATE_URL_ATTACHMENT: &str = "/documents/shine_richiesta_certificato_medico_25_26.pdf";

#[derive(Template)]
#[template(path = "showcase/index.html")]
pub struct IndexTemplate {
	/// Which of the two mounts the certificate download points at.
	certificate_url: &'static str,
	/// Adds the note telling the visitor to open the page in another browser. Only for the
	/// ones that cannot save the file by either route — see `PdfDownload::Unsupported`.
	suggest_other_browser: bool,
}

/// The page varies with the `User-Agent`, so any shared cache in front of us has to be told:
/// without `Vary` it would hand one visitor's variant to everybody.
pub async fn index_handler(headers: HeaderMap) -> impl IntoResponse {
	let support = headers
		.get(USER_AGENT)
		.and_then(|value| value.to_str().ok())
		.map_or(PdfDownload::Attribute, pdf_download);

	// The unsupported case keeps the inline URL: the attachment is what produces "Frame load
	// interrupted" there, while inline at least opens the PDF in the viewer.
	let certificate_url = match support {
		PdfDownload::Header => CERTIFICATE_URL_ATTACHMENT,
		PdfDownload::Attribute | PdfDownload::Unsupported => CERTIFICATE_URL,
	};

	(
		[(VARY, HeaderValue::from_static("User-Agent"))],
		HtmlTemplate(IndexTemplate {
			certificate_url,
			suggest_other_browser: support == PdfDownload::Unsupported,
		}),
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

#[cfg(test)]
mod tests {
	use super::*;
	use axum::body::to_bytes;

	const IOS_FIREFOX: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) \
		AppleWebKit/605.1.15 (KHTML, like Gecko) FxiOS/127.0 Mobile/15E148 Safari/605.1.15";
	const ANDROID_FIREFOX: &str =
		"Mozilla/5.0 (Android 14; Mobile; rv:143.0) Gecko/143.0 Firefox/143.0";
	const IOS_SAFARI: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) \
		AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1";

	/// The homepage as a given browser receives it, `Vary` checked along the way.
	async fn page(user_agent: &str) -> String {
		let mut headers = HeaderMap::new();
		headers.insert(USER_AGENT, HeaderValue::from_str(user_agent).unwrap());

		let response = index_handler(headers).await.into_response();
		assert_eq!(response.headers().get(VARY).unwrap(), "User-Agent");

		let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
		String::from_utf8(body.to_vec()).unwrap()
	}

	/// The notice, matched on a fragment of its Italian text so rewording it fails loudly
	/// rather than silently making this test check nothing.
	const NOTICE: &str = "apri questa pagina con";

	#[tokio::test]
	async fn ios_firefox_is_sent_inline_and_told_to_switch_browser() {
		let page = page(IOS_FIREFOX).await;

		assert!(page.contains("href=\"/static/documents/shine_richiesta"));
		assert!(page.contains(NOTICE));
	}

	#[tokio::test]
	async fn android_firefox_is_sent_to_the_attachment_mount_without_a_notice() {
		let page = page(ANDROID_FIREFOX).await;

		assert!(page.contains("href=\"/documents/shine_richiesta"));
		assert!(!page.contains(NOTICE));
	}

	#[tokio::test]
	async fn a_browser_that_honours_the_attribute_sees_the_page_unchanged() {
		let safari = page(IOS_SAFARI).await;

		assert!(safari.contains("href=\"/static/documents/shine_richiesta"));
		assert!(!safari.contains(NOTICE));

		// Nothing else on the page is allowed to move with the `User-Agent`: the two variants
		// differ by exactly the mount prefix, and by nothing else.
		let android = page(ANDROID_FIREFOX).await;
		assert_eq!(android.len() + "/static".len(), safari.len());
	}
}
