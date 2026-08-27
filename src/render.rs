use askama::Template;
use axum::{
	http::StatusCode,
	http::header::{CACHE_CONTROL, HeaderValue},
	response::{Html, IntoResponse, Response},
};

pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
	T: Template,
{
	fn into_response(self) -> Response {
		match self.0.render() {
			// A page's URL never changes when its markup does, and with no explicit lifetime
			// a browser falls back to *heuristic* freshness: it reuses the copy it has
			// without asking, so a deployed change can stay invisible on a phone that
			// visited the site before. `no-cache` still lets the copy be stored — it only
			// forces the conditional request that notices the change.
			Ok(html) => (
				[(CACHE_CONTROL, HeaderValue::from_static("no-cache"))],
				Html(html),
			)
				.into_response(),
			Err(err) => (
				StatusCode::INTERNAL_SERVER_ERROR,
				format!("Errore rendering template: {err}"),
			)
				.into_response(),
		}
	}
}
